use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CompressResult {
    pub lines: Vec<OutputLine>,
    pub stats: CompressStats,
    pub elapsed_us: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputLine {
    pub line_number: usize,
    pub content: String,
    pub score: f32,
    pub reason: MatchReason,
}

#[derive(Debug, Clone, Serialize)]
pub enum MatchReason {
    QueryMatch,
    FieldMatch { field: String },
    ErrorLevel,
    TraceExpansion { trace_id: String },
    Context { of_line: usize },
}

#[derive(Debug, Clone, Serialize)]
pub struct CompressStats {
    pub input_lines: usize,
    pub output_lines: usize,
    pub compression_ratio: f32,
    pub unique_query_terms: usize,
    pub candidate_lines: usize,
}
