use std::collections::HashSet;
use std::fs;
use std::path::Path;

use logcompress::{compress, CompressConfig};

#[derive(Debug)]
struct GroundTruthEntry {
    query: String,
    expected_lines: Vec<usize>,
    incident_kind: String,
    aggregate: bool,
}

fn parse_ground_truth(path: &Path) -> Vec<GroundTruthEntry> {
    let raw = fs::read_to_string(path).expect("read ground truth");
    let arr: serde_json::Value = serde_json::from_str(&raw).expect("parse ground truth JSON");
    let arr = arr.as_array().expect("ground truth should be array");

    arr.iter()
        .map(|entry| {
            let query = entry["query"].as_str().unwrap().to_string();
            let expected_lines: Vec<usize> = entry["expected_lines"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect();
            let incident_kind = entry["incident_kind"].as_str().unwrap().to_string();
            let aggregate = entry
                .get("aggregate")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            GroundTruthEntry {
                query,
                expected_lines,
                incident_kind,
                aggregate,
            }
        })
        .collect()
}

#[test]
fn benchmark_recall_precision() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata/benchmark");

    let logs_path = base.join("logs_100k.jsonl");
    let gt_path = base.join("ground_truth.json");

    if !logs_path.exists() {
        eprintln!(
            "Benchmark dataset not found at {:?} — skipping recall test",
            logs_path
        );
        eprintln!("Run: cd testdata/benchmark && rustc generate.rs -o generate && ./generate");
        return;
    }

    let logs = fs::read_to_string(&logs_path).expect("read logs");
    let ground_truth = parse_ground_truth(&gt_path);

    let config_single = CompressConfig {
        top_k: 500,
        context_lines: 2,
        max_output_lines: 2000,
        ..Default::default()
    };
    let config_aggregate = CompressConfig {
        top_k: 2000,
        context_lines: 0,
        max_output_lines: 5000,
        min_score: 0.005,
        ..Default::default()
    };

    let mut total_recall = 0.0f64;
    let mut total_precision = 0.0f64;
    let mut count = 0usize;
    let mut failures: Vec<String> = Vec::new();

    let mut per_kind_recall: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::new();

    let mut results_lines: Vec<String> = Vec::new();
    results_lines.push("# LogCompress Benchmark Results".to_string());
    results_lines.push(String::new());
    results_lines.push(format!("Dataset: {} lines", logs.lines().count()));
    results_lines.push(format!(
        "Ground truth: {} queries ({} per-incident + {} aggregate)",
        ground_truth.len(),
        ground_truth.iter().filter(|e| !e.aggregate).count(),
        ground_truth.iter().filter(|e| e.aggregate).count(),
    ));
    results_lines.push(String::new());
    results_lines.push("## Per-query results".to_string());
    results_lines.push(String::new());
    results_lines.push(
        "| # | Kind | Aggregate | Expected | Found | Recall | Precision | Time |".to_string(),
    );
    results_lines.push(
        "|---|------|-----------|----------|-------|--------|-----------|------|".to_string(),
    );

    for (i, entry) in ground_truth.iter().enumerate() {
        let config = if entry.aggregate {
            &config_aggregate
        } else {
            &config_single
        };
        let result = compress(&entry.query, &logs, config);

        let output_line_numbers: HashSet<usize> =
            result.lines.iter().map(|l| l.line_number).collect();
        let expected_set: HashSet<usize> = entry.expected_lines.iter().copied().collect();

        let found = expected_set.intersection(&output_line_numbers).count();
        let recall = if expected_set.is_empty() {
            1.0
        } else {
            found as f64 / expected_set.len() as f64
        };
        let precision = if output_line_numbers.is_empty() {
            0.0
        } else {
            found as f64 / output_line_numbers.len() as f64
        };

        total_recall += recall;
        total_precision += precision;
        count += 1;

        per_kind_recall
            .entry(entry.incident_kind.clone())
            .or_default()
            .push(recall);

        let agg_marker = if entry.aggregate { "yes" } else { "no" };
        results_lines.push(format!(
            "| {} | {} | {} | {} | {}/{} | {:.0}% | {:.0}% | {}us |",
            i,
            entry.incident_kind,
            agg_marker,
            expected_set.len(),
            found,
            output_line_numbers.len(),
            recall * 100.0,
            precision * 100.0,
            result.elapsed_us,
        ));

        if recall < 0.9 {
            failures.push(format!(
                "Query #{} ({}, {}): recall={:.0}% ({}/{} found), output={} lines",
                i,
                entry.incident_kind,
                if entry.aggregate { "agg" } else { "single" },
                recall * 100.0,
                found,
                expected_set.len(),
                output_line_numbers.len(),
            ));
        }
    }

    let avg_recall = total_recall / count as f64;
    let avg_precision = total_precision / count as f64;

    results_lines.push(String::new());
    results_lines.push("## Summary".to_string());
    results_lines.push(String::new());
    results_lines.push(format!("- **Average recall: {:.1}%**", avg_recall * 100.0));
    results_lines.push(format!(
        "- **Average precision: {:.1}%**",
        avg_precision * 100.0
    ));
    let passing = count - failures.len();
    results_lines.push(format!(
        "- Queries with recall >= 90%: {}/{}",
        passing, count
    ));
    results_lines.push(String::new());
    results_lines.push("## Per-kind recall".to_string());
    results_lines.push(String::new());

    let mut kinds: Vec<_> = per_kind_recall.keys().cloned().collect();
    kinds.sort();
    for kind in &kinds {
        let recalls = &per_kind_recall[kind];
        let avg = recalls.iter().sum::<f64>() / recalls.len() as f64;
        results_lines.push(format!(
            "- **{}**: {:.1}% avg recall ({} queries)",
            kind,
            avg * 100.0,
            recalls.len()
        ));
    }

    if !failures.is_empty() {
        results_lines.push(String::new());
        results_lines.push("## Failures (recall < 90%)".to_string());
        results_lines.push(String::new());
        for f in &failures {
            results_lines.push(format!("- {}", f));
        }
    }

    let results_md = results_lines.join("\n");
    let results_path = base.join("RESULTS.md");
    fs::write(&results_path, &results_md).expect("write RESULTS.md");

    eprintln!("\n{}\n", results_md);
    eprintln!("Results saved to {:?}", results_path);

    assert!(
        avg_recall >= 0.9,
        "BLOCKER: Average recall {:.1}% is below 90% threshold.\nFailures:\n{}",
        avg_recall * 100.0,
        failures.join("\n"),
    );
}
