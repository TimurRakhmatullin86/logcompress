mod config;
mod context;
mod fast_index;
#[allow(dead_code)]
mod index;
mod parser;
mod result;
#[allow(dead_code)]
mod tfidf;

pub use config::{CompressConfig, InputFormat};
pub use result::{CompressResult, CompressStats, MatchReason, OutputLine};

use std::collections::HashSet;
use std::time::Instant;

use rayon::prelude::*;

use fast_index::{score_line_fast, tokenize_hashed, FastIndex};

pub fn compress(query: &str, input: &str, config: &CompressConfig) -> CompressResult {
    let start = Instant::now();
    let raw_lines: Vec<&str> = input.lines().collect();
    let total = raw_lines.len();

    if total == 0 || query.trim().is_empty() {
        return empty_result(total, start);
    }

    let is_json = match config.format {
        InputFormat::Auto => {
            if let Some(first) = raw_lines.first() {
                parser::detect_format(first) == InputFormat::JsonLines
            } else {
                false
            }
        }
        InputFormat::JsonLines => true,
        InputFormat::PlainText => false,
    };

    // Phase 1: parallel tokenization, then sequential index build
    let line_hashes: Vec<Vec<u64>> = raw_lines
        .par_iter()
        .map(|line| tokenize_hashed(line.as_bytes()))
        .collect();

    let mut idx = FastIndex::new(total as u32);
    for (i, hashes) in line_hashes.iter().enumerate() {
        idx.add_doc(i as u32, hashes);
    }

    let query_hashes = tokenize_hashed(query.as_bytes());

    // Compute query IDF
    let mut unique_hashes: Vec<u64> = query_hashes.clone();
    unique_hashes.sort_unstable();
    unique_hashes.dedup();
    let query_idf: Vec<(u64, f32)> = unique_hashes
        .iter()
        .filter_map(|&h| {
            let idf = idx.idf(h);
            if idf > 0.0 {
                let tf = query_hashes.iter().filter(|&&x| x == h).count() as f32
                    / query_hashes.len() as f32;
                Some((h, tf * idf))
            } else {
                None
            }
        })
        .collect();

    let unique_query_terms = query_idf.len();

    if query_idf.is_empty() {
        return empty_result(total, start);
    }

    let candidates = idx.candidate_lines(&query_hashes);
    let candidate_count = candidates.len();

    // Phase 2: score candidates, JSON-parse only candidates for field boost
    let boost_field_names: Vec<String> = config.field_boosts.keys().cloned().collect();
    let query_token_strings: Vec<String> = parser::tokenize(query);
    let query_token_set: HashSet<&str> = query_token_strings.iter().map(|s| s.as_str()).collect();

    let mut scored: Vec<(usize, f32, MatchReason)> = Vec::with_capacity(candidates.len());

    for &line_idx in &candidates {
        let li = line_idx as usize;
        let mut score = score_line_fast(&line_hashes[li], &query_hashes, &query_idf, &idx);
        let mut reason = MatchReason::QueryMatch;

        if is_json {
            if let Some(fields) = parser::parse_json_fields_only(raw_lines[li], &boost_field_names)
            {
                for (field_name, field_value) in &fields.field_values {
                    if let Some(&boost) = config.field_boosts.get(field_name) {
                        let value_tokens = parser::tokenize(field_value);
                        let has_overlap = value_tokens
                            .iter()
                            .any(|t| query_token_set.contains(t.as_str()));
                        if has_overlap {
                            score *= boost;
                            reason = MatchReason::FieldMatch {
                                field: field_name.clone(),
                            };
                        }
                    }
                }
            }
        }

        if score >= config.min_score {
            scored.push((li, score, reason));
        }
    }

    // Phase 2b: include ERROR lines (fast substring scan, no JSON parse)
    if config.always_include_errors && is_json {
        for (i, line) in raw_lines.iter().enumerate() {
            if parser::fast_is_error_line(line) {
                let already = scored.iter().any(|(idx, _, _)| *idx == i);
                if !already {
                    scored.push((i, 0.1, MatchReason::ErrorLevel));
                }
            }
        }
    }

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(config.top_k);

    if let Some(&(_, top_score, _)) = scored.first() {
        if top_score > 0.0 {
            let cutoff = top_score * 0.25;
            scored
                .retain(|(_, s, reason)| *s >= cutoff || matches!(reason, MatchReason::ErrorLevel));
        }
    }

    // Trace-based expansion: pull all lines sharing trace_id with high-scoring hits
    if config.trace_expansion && is_json && !scored.is_empty() {
        let mut trace_ids: HashSet<String> = HashSet::new();
        let scored_set: HashSet<usize> = scored.iter().map(|(idx, _, _)| *idx).collect();

        for &(li, score, _) in &scored {
            if score < config.min_score {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw_lines[li]) {
                if let Some(obj) = val.as_object() {
                    for field in &config.trace_fields {
                        if let Some(v) = obj.get(field.as_str()).and_then(|v| v.as_str()) {
                            trace_ids.insert(v.to_string());
                        }
                    }
                }
            }
        }

        if !trace_ids.is_empty() {
            let needles: Vec<String> = config
                .trace_fields
                .iter()
                .map(|f| format!(r#""{}":"#, f))
                .collect();

            let trace_matches: Vec<(usize, String)> = raw_lines
                .par_iter()
                .enumerate()
                .filter_map(|(i, line)| {
                    if scored_set.contains(&i) {
                        return None;
                    }
                    for (fi, needle) in needles.iter().enumerate() {
                        if line.contains(needle.as_str()) {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                                if let Some(obj) = val.as_object() {
                                    let field = &config.trace_fields[fi];
                                    if let Some(v) =
                                        obj.get(field.as_str()).and_then(|v| v.as_str())
                                    {
                                        if trace_ids.contains(v) {
                                            return Some((i, v.to_string()));
                                        }
                                    }
                                }
                            }
                            return None;
                        }
                    }
                    None
                })
                .collect();

            for (li, trace_id) in trace_matches {
                scored.push((li, 0.05, MatchReason::TraceExpansion { trace_id }));
            }
        }
    }

    let hit_lines: Vec<usize> = scored.iter().map(|(idx, _, _)| *idx).collect();
    let hit_set: HashSet<usize> = hit_lines.iter().copied().collect();

    let mut ranges = context::expand_context(&hit_lines, config.context_lines, total);
    context::merge_ranges(&mut ranges);
    let all_line_indices = context::ranges_to_line_set(&ranges);

    let mut output_lines: Vec<OutputLine> = Vec::with_capacity(all_line_indices.len());

    for &line_idx in &all_line_indices {
        if line_idx >= total {
            continue;
        }

        let (score, reason) = if hit_set.contains(&line_idx) {
            let entry = scored.iter().find(|(idx, _, _)| *idx == line_idx).unwrap();
            (entry.1, entry.2.clone())
        } else {
            let nearest_hit = hit_lines
                .iter()
                .min_by_key(|&&h| (h as isize - line_idx as isize).unsigned_abs())
                .copied()
                .unwrap_or(line_idx);
            (
                0.0,
                MatchReason::Context {
                    of_line: nearest_hit + 1,
                },
            )
        };

        output_lines.push(OutputLine {
            line_number: line_idx + 1,
            content: raw_lines[line_idx].to_string(),
            score,
            reason,
        });
    }

    output_lines.truncate(config.max_output_lines);
    let output_count = output_lines.len();

    CompressResult {
        lines: output_lines,
        stats: CompressStats {
            input_lines: total,
            output_lines: output_count,
            compression_ratio: if total > 0 {
                output_count as f32 / total as f32
            } else {
                0.0
            },
            unique_query_terms,
            candidate_lines: candidate_count,
        },
        elapsed_us: start.elapsed().as_micros() as u64,
    }
}

pub fn compress_default(query: &str, input: &str) -> CompressResult {
    compress(query, input, &CompressConfig::default())
}

fn empty_result(input_lines: usize, start: Instant) -> CompressResult {
    CompressResult {
        lines: Vec::new(),
        stats: CompressStats {
            input_lines,
            output_lines: 0,
            compression_ratio: 0.0,
            unique_query_terms: 0,
            candidate_lines: 0,
        },
        elapsed_us: start.elapsed().as_micros() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_json_logs() -> String {
        let mut lines = Vec::new();
        for i in 0..100 {
            let level = if i == 42 {
                "ERROR"
            } else if i % 20 == 0 {
                "WARN"
            } else {
                "INFO"
            };
            let msg = if i == 42 {
                "payment 42 failed: timeout connecting to payment gateway"
            } else if i == 41 {
                "payment 42 processing: sending request to gateway"
            } else if i == 43 {
                "payment 42 retry: scheduling retry in 5s"
            } else {
                "health check OK"
            };
            lines.push(format!(
                r#"{{"timestamp":"2026-09-06T10:{i:02}:00Z","level":"{level}","service":"payment","message":"{msg}","trace_id":"tr-{i:04}"}}"#,
            ));
        }
        lines.join("\n")
    }

    fn sample_plain_logs() -> String {
        let mut lines = Vec::new();
        for i in 0..100 {
            if i == 42 {
                lines.push(format!("2026-09-06 10:{i:02}:00 ERROR payment 42 failed: timeout connecting to payment gateway"));
            } else if i == 41 {
                lines.push(format!(
                    "2026-09-06 10:{i:02}:00 INFO payment 42 processing: sending request"
                ));
            } else if i == 43 {
                lines.push(format!(
                    "2026-09-06 10:{i:02}:00 INFO payment 42 retry: scheduling retry in 5s"
                ));
            } else {
                lines.push(format!(
                    "2026-09-06 10:{i:02}:00 INFO health check OK service=api"
                ));
            }
        }
        lines.join("\n")
    }

    #[test]
    fn compress_json_finds_payment() {
        let logs = sample_json_logs();
        let result = compress_default("why did payment 42 fail?", &logs);

        assert!(result.stats.output_lines > 0, "should find some lines");
        assert!(
            result.stats.output_lines < 20,
            "should be compressed, got {}",
            result.stats.output_lines
        );

        let has_line_42 = result.lines.iter().any(|l| l.line_number == 43);
        assert!(
            has_line_42,
            "should include the error line (line 43, 0-indexed 42)"
        );

        let content_42 = result
            .lines
            .iter()
            .find(|l| l.content.contains("payment 42 failed"));
        assert!(content_42.is_some(), "should contain the failure message");
    }

    #[test]
    fn compress_plain_finds_payment() {
        let logs = sample_plain_logs();
        let result = compress_default("why did payment 42 fail?", &logs);

        assert!(result.stats.output_lines > 0);
        assert!(result.stats.output_lines < 20);

        let has_failure = result
            .lines
            .iter()
            .any(|l| l.content.contains("payment 42 failed"));
        assert!(has_failure, "should find the payment failure line");
    }

    #[test]
    fn compression_ratio() {
        let logs = sample_json_logs();
        let result = compress_default("payment 42 failed", &logs);
        assert!(
            result.stats.compression_ratio < 0.5,
            "ratio should be < 50%, got {}",
            result.stats.compression_ratio
        );
    }

    #[test]
    fn empty_query() {
        let logs = sample_json_logs();
        let result = compress_default("", &logs);
        assert_eq!(result.stats.output_lines, 0);
    }

    #[test]
    fn empty_input() {
        let result = compress_default("payment failed", "");
        assert_eq!(result.stats.output_lines, 0);
        assert_eq!(result.stats.input_lines, 0);
    }

    #[test]
    fn includes_context_lines() {
        let logs = sample_plain_logs();
        let config = CompressConfig {
            context_lines: 2,
            always_include_errors: false,
            ..Default::default()
        };
        let result = compress("payment 42 failed", &logs, &config);

        let line_numbers: Vec<usize> = result.lines.iter().map(|l| l.line_number).collect();
        let has_context = result
            .lines
            .iter()
            .any(|l| matches!(l.reason, MatchReason::Context { .. }));
        assert!(
            has_context,
            "should include context lines, got lines: {:?}",
            line_numbers
        );
    }

    #[test]
    fn stats_populated() {
        let logs = sample_json_logs();
        let result = compress_default("payment failed", &logs);
        assert_eq!(result.stats.input_lines, 100);
        assert!(result.stats.unique_query_terms > 0);
        assert!(result.stats.candidate_lines > 0);
        assert!(result.elapsed_us > 0);
    }
}
