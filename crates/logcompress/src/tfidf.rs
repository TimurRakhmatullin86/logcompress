use std::collections::HashMap;

use crate::index::InvertedIndex;

pub fn compute_query_tfidf(query_tokens: &[String], index: &InvertedIndex) -> HashMap<String, f32> {
    let total = query_tokens.len() as f32;
    if total == 0.0 {
        return HashMap::new();
    }

    let mut tf: HashMap<&str, f32> = HashMap::new();
    for token in query_tokens {
        *tf.entry(token.as_str()).or_insert(0.0) += 1.0;
    }

    let mut tfidf = HashMap::new();
    for (&term, &count) in &tf {
        let tf_val = count / total;
        let idf_val = index.idf(term);
        if idf_val > 0.0 {
            tfidf.insert(term.to_string(), tf_val * idf_val);
        }
    }

    tfidf
}

pub fn score_line(
    line_tokens: &[String],
    query_tfidf: &HashMap<String, f32>,
    index: &InvertedIndex,
) -> f32 {
    if line_tokens.is_empty() || query_tfidf.is_empty() {
        return 0.0;
    }

    let total = line_tokens.len() as f32;

    let mut line_vec: HashMap<&str, f32> = HashMap::new();
    for token in line_tokens {
        if query_tfidf.contains_key(token.as_str()) {
            *line_vec.entry(token.as_str()).or_insert(0.0) += 1.0;
        }
    }

    for (term, count) in line_vec.iter_mut() {
        let tf = *count / total;
        let idf = index.idf(term);
        *count = tf * idf;
    }

    cosine_similarity(&query_tfidf_to_ref(query_tfidf), &line_vec)
}

fn query_tfidf_to_ref(m: &HashMap<String, f32>) -> HashMap<&str, f32> {
    m.iter().map(|(k, &v)| (k.as_str(), v)).collect()
}

fn cosine_similarity(a: &HashMap<&str, f32>, b: &HashMap<&str, f32>) -> f32 {
    let dot: f32 = a
        .iter()
        .filter_map(|(k, v)| b.get(k).map(|bv| v * bv))
        .sum();

    let mag_a: f32 = a.values().map(|v| v * v).sum::<f32>().sqrt();
    let mag_b: f32 = b.values().map(|v| v * v).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_index() -> InvertedIndex {
        let mut idx = InvertedIndex::new(4);
        idx.add_doc(0, &["payment".into(), "failed".into(), "timeout".into()]);
        idx.add_doc(
            1,
            &["order".into(), "created".into(), "successfully".into()],
        );
        idx.add_doc(2, &["payment".into(), "processed".into()]);
        idx.add_doc(3, &["error".into(), "database".into(), "connection".into()]);
        idx
    }

    #[test]
    fn query_tfidf_basic() {
        let idx = build_test_index();
        let query_tokens = vec!["payment".into(), "failed".into()];
        let qt = compute_query_tfidf(&query_tokens, &idx);
        assert!(qt.contains_key("payment"));
        assert!(qt.contains_key("failed"));
        assert!(
            *qt.get("failed").unwrap() > *qt.get("payment").unwrap(),
            "rarer 'failed' should have higher TF-IDF than common 'payment'"
        );
    }

    #[test]
    fn score_matching_line() {
        let idx = build_test_index();
        let query_tokens = vec!["payment".into(), "failed".into()];
        let qt = compute_query_tfidf(&query_tokens, &idx);

        let line0_tokens = vec!["payment".into(), "failed".into(), "timeout".into()];
        let line1_tokens = vec!["order".into(), "created".into(), "successfully".into()];

        let score0 = score_line(&line0_tokens, &qt, &idx);
        let score1 = score_line(&line1_tokens, &qt, &idx);

        assert!(
            score0 > 0.5,
            "matching line should score high, got {score0}"
        );
        assert!(
            score1 < 0.01,
            "non-matching line should score ~0, got {score1}"
        );
    }

    #[test]
    fn score_partial_match() {
        let idx = build_test_index();
        let query_tokens = vec!["payment".into(), "failed".into()];
        let qt = compute_query_tfidf(&query_tokens, &idx);

        let partial_tokens = vec!["payment".into(), "processed".into()];
        let score = score_line(&partial_tokens, &qt, &idx);

        assert!(
            score > 0.0 && score < 0.9,
            "partial match should be moderate, got {score}"
        );
    }

    #[test]
    fn empty_query() {
        let idx = build_test_index();
        let qt = compute_query_tfidf(&[], &idx);
        assert!(qt.is_empty());
    }
}
