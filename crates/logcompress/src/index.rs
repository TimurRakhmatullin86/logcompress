use std::collections::HashMap;

pub struct InvertedIndex {
    postings: HashMap<String, Vec<usize>>,
    doc_freq: HashMap<String, usize>,
    total_docs: usize,
}

impl InvertedIndex {
    pub fn new(total_docs: usize) -> Self {
        Self {
            postings: HashMap::new(),
            doc_freq: HashMap::new(),
            total_docs,
        }
    }

    pub fn add_doc(&mut self, doc_id: usize, tokens: &[String]) {
        let mut seen = HashMap::new();
        for token in tokens {
            seen.entry(token.clone()).or_insert(false);
        }

        for token in seen.keys() {
            self.postings.entry(token.clone()).or_default().push(doc_id);
            *self.doc_freq.entry(token.clone()).or_insert(0) += 1;
        }
    }

    pub fn idf(&self, term: &str) -> f32 {
        let df = self.doc_freq.get(term).copied().unwrap_or(0) as f32;
        if df == 0.0 {
            return 0.0;
        }
        let n = self.total_docs as f32;
        ((n + 1.0) / (df + 1.0)).ln() + 1.0
    }

    pub fn postings_for(&self, term: &str) -> &[usize] {
        self.postings.get(term).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn candidate_lines(&self, query_tokens: &[String]) -> Vec<usize> {
        let mut seen = HashMap::new();
        for token in query_tokens {
            for &line_idx in self.postings_for(token) {
                *seen.entry(line_idx).or_insert(0usize) += 1;
            }
        }
        let mut candidates: Vec<usize> = seen.into_keys().collect();
        candidates.sort_unstable();
        candidates
    }

    #[allow(dead_code)]
    pub fn total_docs(&self) -> usize {
        self.total_docs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_index() {
        let mut idx = InvertedIndex::new(3);
        idx.add_doc(0, &["payment".into(), "failed".into()]);
        idx.add_doc(1, &["order".into(), "created".into()]);
        idx.add_doc(2, &["payment".into(), "success".into()]);

        assert_eq!(idx.postings_for("payment"), &[0, 2]);
        assert_eq!(idx.postings_for("order"), &[1]);
        assert!(idx.postings_for("unknown").is_empty());
    }

    #[test]
    fn idf_computation() {
        let mut idx = InvertedIndex::new(100);
        for i in 0..100 {
            idx.add_doc(i, &["common".into()]);
        }
        for i in 0..5 {
            idx.add_doc(i, &["rare".into()]);
        }

        let idf_common = idx.idf("common");
        let idf_rare = idx.idf("rare");
        assert!(idf_rare > idf_common, "rare term should have higher IDF");
    }

    #[test]
    fn candidate_lines_union() {
        let mut idx = InvertedIndex::new(5);
        idx.add_doc(0, &["payment".into()]);
        idx.add_doc(1, &["failed".into()]);
        idx.add_doc(2, &["payment".into(), "failed".into()]);

        let candidates = idx.candidate_lines(&["payment".into(), "failed".into()]);
        assert_eq!(candidates, vec![0, 1, 2]);
    }

    #[test]
    fn idf_missing_term() {
        let idx = InvertedIndex::new(10);
        assert_eq!(idx.idf("nonexistent"), 0.0);
    }
}
