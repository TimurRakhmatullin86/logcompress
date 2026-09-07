use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompressConfig {
    pub top_k: usize,
    pub context_lines: usize,
    pub min_score: f32,
    pub max_output_lines: usize,
    pub field_boosts: HashMap<String, f32>,
    pub always_include_errors: bool,
    pub format: InputFormat,
    pub trace_expansion: bool,
    pub trace_fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Auto,
    JsonLines,
    PlainText,
}

impl Default for CompressConfig {
    fn default() -> Self {
        Self {
            top_k: 200,
            context_lines: 2,
            min_score: 0.01,
            max_output_lines: 1000,
            field_boosts: default_field_boosts(),
            always_include_errors: true,
            format: InputFormat::Auto,
            trace_expansion: true,
            trace_fields: vec![
                "trace_id".into(),
                "request_id".into(),
                "correlation_id".into(),
            ],
        }
    }
}

fn default_field_boosts() -> HashMap<String, f32> {
    let mut m = HashMap::new();
    m.insert("error_id".into(), 3.0);
    m.insert("trace_id".into(), 3.0);
    m.insert("span_id".into(), 2.5);
    m.insert("request_id".into(), 2.5);
    m.insert("correlation_id".into(), 2.5);
    m.insert("transaction_id".into(), 2.5);
    m.insert("order_id".into(), 2.0);
    m.insert("payment_id".into(), 2.0);
    m.insert("user_id".into(), 1.5);
    m.insert("session_id".into(), 1.5);
    m
}
