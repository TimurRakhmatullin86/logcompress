use std::collections::HashMap;

pub struct FastIndex {
    postings: HashMap<u64, Vec<u32>>,
    doc_freq: HashMap<u64, u32>,
    total_docs: u32,
}

fn hash_token(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        let b = if b.is_ascii_uppercase() { b + 32 } else { b };
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn tokenize_hashed(text: &[u8]) -> Vec<u64> {
    let mut tokens = Vec::new();
    let mut start = None;

    for (i, &b) in text.iter().enumerate() {
        let is_token = b.is_ascii_alphanumeric() || b == b'_';
        match (is_token, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if i - s >= 2 {
                    tokens.push(hash_token(&text[s..i]));
                }
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        if text.len() - s >= 2 {
            tokens.push(hash_token(&text[s..]));
        }
    }

    tokens
}

impl FastIndex {
    pub fn new(total_docs: u32) -> Self {
        Self {
            postings: HashMap::new(),
            doc_freq: HashMap::new(),
            total_docs,
        }
    }

    pub fn add_doc(&mut self, doc_id: u32, token_hashes: &[u64]) {
        let mut unique = token_hashes.to_vec();
        unique.sort_unstable();
        unique.dedup();
        for h in unique {
            self.postings.entry(h).or_default().push(doc_id);
            *self.doc_freq.entry(h).or_insert(0) += 1;
        }
    }

    pub fn idf(&self, term_hash: u64) -> f32 {
        let df = self.doc_freq.get(&term_hash).copied().unwrap_or(0) as f32;
        if df == 0.0 {
            return 0.0;
        }
        let n = self.total_docs as f32;
        ((n + 1.0) / (df + 1.0)).ln() + 1.0
    }

    pub fn candidate_lines(&self, query_hashes: &[u64]) -> Vec<u32> {
        let mut seen = HashMap::new();
        for &h in query_hashes {
            if let Some(postings) = self.postings.get(&h) {
                for &line_idx in postings {
                    *seen.entry(line_idx).or_insert(0u32) += 1;
                }
            }
        }
        let mut candidates: Vec<u32> = seen.into_keys().collect();
        candidates.sort_unstable();
        candidates
    }
}

pub fn score_line_fast(
    line_hashes: &[u64],
    query_hashes: &[u64],
    query_idf: &[(u64, f32)],
    index: &FastIndex,
) -> f32 {
    if line_hashes.is_empty() || query_idf.is_empty() {
        return 0.0;
    }

    let total = line_hashes.len() as f32;
    let query_set: std::collections::HashSet<u64> = query_hashes.iter().copied().collect();

    let mut line_tfidf: HashMap<u64, f32> = HashMap::new();
    for &h in line_hashes {
        if query_set.contains(&h) {
            *line_tfidf.entry(h).or_insert(0.0) += 1.0;
        }
    }

    if line_tfidf.is_empty() {
        return 0.0;
    }

    for (h, count) in line_tfidf.iter_mut() {
        let tf = *count / total;
        let idf = index.idf(*h);
        *count = tf * idf;
    }

    let mut dot = 0.0f32;
    let mut mag_q = 0.0f32;
    let mut mag_l = 0.0f32;

    for &(qh, qv) in query_idf {
        mag_q += qv * qv;
        if let Some(&lv) = line_tfidf.get(&qh) {
            dot += qv * lv;
        }
    }
    for &lv in line_tfidf.values() {
        mag_l += lv * lv;
    }

    let mag = mag_q.sqrt() * mag_l.sqrt();
    if mag == 0.0 {
        0.0
    } else {
        (dot / mag).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        assert_eq!(hash_token(b"payment"), hash_token(b"payment"));
        assert_ne!(hash_token(b"payment"), hash_token(b"order"));
    }

    #[test]
    fn hash_case_insensitive() {
        assert_eq!(hash_token(b"ERROR"), hash_token(b"error"));
        assert_eq!(hash_token(b"Payment"), hash_token(b"payment"));
    }

    #[test]
    fn tokenize_hashed_basic() {
        let hashes = tokenize_hashed(b"payment_id=42 failed");
        assert_eq!(hashes.len(), 3); // payment_id, 42, failed
    }

    #[test]
    fn fast_index_basic() {
        let mut idx = FastIndex::new(3);
        idx.add_doc(0, &tokenize_hashed(b"payment failed timeout"));
        idx.add_doc(1, &tokenize_hashed(b"order created successfully"));
        idx.add_doc(2, &tokenize_hashed(b"payment processed"));

        let q = tokenize_hashed(b"payment failed");
        let candidates = idx.candidate_lines(&q);
        assert!(candidates.contains(&0));
        assert!(candidates.contains(&2));
    }

    #[test]
    fn score_fast_matching() {
        let mut idx = FastIndex::new(3);
        let h0 = tokenize_hashed(b"payment failed timeout");
        let h1 = tokenize_hashed(b"order created successfully");
        idx.add_doc(0, &h0);
        idx.add_doc(1, &h1);
        idx.add_doc(2, &tokenize_hashed(b"payment processed"));

        let qh = tokenize_hashed(b"payment failed");
        let query_idf: Vec<(u64, f32)> = qh
            .iter()
            .map(|&h| {
                let tf = 1.0 / qh.len() as f32;
                (h, tf * idx.idf(h))
            })
            .collect();

        let s0 = score_line_fast(&h0, &qh, &query_idf, &idx);
        let s1 = score_line_fast(&h1, &qh, &query_idf, &idx);

        assert!(s0 > 0.5, "matching should be high: {s0}");
        assert!(s1 < 0.01, "non-matching should be ~0: {s1}");
    }
}
