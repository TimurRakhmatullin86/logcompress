use crate::config::InputFormat;

pub fn tokenize(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut start = None;

    for (i, &b) in bytes.iter().enumerate() {
        let is_token_char = b.is_ascii_alphanumeric() || b == b'_';
        match (is_token_char, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if i - s >= 2 {
                    let slice = &text[s..i];
                    tokens.push(fast_lowercase(slice));
                }
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        if bytes.len() - s >= 2 {
            tokens.push(fast_lowercase(&text[s..]));
        }
    }

    tokens
}

fn fast_lowercase(s: &str) -> String {
    if s.bytes().all(|b| !b.is_ascii_uppercase()) {
        s.to_string()
    } else {
        s.to_ascii_lowercase()
    }
}

pub fn detect_format(first_line: &str) -> InputFormat {
    let trimmed = first_line.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        InputFormat::JsonLines
    } else {
        InputFormat::PlainText
    }
}

#[allow(dead_code)]
pub struct ParsedLine {
    pub tokens: Vec<String>,
    pub json_fields: Option<JsonFields>,
}

pub struct JsonFields {
    #[allow(dead_code)]
    pub level: Option<String>,
    pub field_values: Vec<(String, String)>,
}

#[allow(dead_code)]
pub fn parse_line(line: &str, is_json: bool, boost_fields: &[String]) -> ParsedLine {
    let tokens = tokenize(line);

    let json_fields = if is_json {
        parse_json_fields(line, boost_fields)
    } else {
        None
    };

    ParsedLine {
        tokens,
        json_fields,
    }
}

fn parse_json_fields(line: &str, boost_fields: &[String]) -> Option<JsonFields> {
    let val: serde_json::Value = serde_json::from_str(line).ok()?;
    let obj = val.as_object()?;

    let level = obj
        .get("level")
        .or_else(|| obj.get("severity"))
        .or_else(|| obj.get("log_level"))
        .or_else(|| obj.get("loglevel"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase());

    let mut field_values = Vec::new();
    for field_name in boost_fields {
        if let Some(v) = obj.get(field_name.as_str()) {
            let val_str = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            field_values.push((field_name.clone(), val_str));
        }
    }

    Some(JsonFields {
        level,
        field_values,
    })
}

#[allow(dead_code)]
pub fn is_error_level(level: &str) -> bool {
    matches!(
        level,
        "ERROR" | "FATAL" | "CRITICAL" | "PANIC" | "ALERT" | "EMERGENCY"
    )
}

pub fn parse_json_fields_only(line: &str, boost_fields: &[String]) -> Option<JsonFields> {
    parse_json_fields(line, boost_fields)
}

pub fn fast_is_error_line(line: &str) -> bool {
    line.contains("\"ERROR\"")
        || line.contains("\"FATAL\"")
        || line.contains("\"CRITICAL\"")
        || line.contains("\"PANIC\"")
        || line.contains("\"error\"")
        || line.contains("\"fatal\"")
        || line.contains("\"critical\"")
}

#[allow(dead_code)]
pub fn is_warn_level(level: &str) -> bool {
    level == "WARN" || level == "WARNING"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_basic() {
        let tokens = tokenize("payment_id=42 failed with timeout");
        assert!(tokens.contains(&"payment_id".to_string()));
        assert!(tokens.contains(&"42".to_string()));
        assert!(tokens.contains(&"failed".to_string()));
        assert!(tokens.contains(&"timeout".to_string()));
    }

    #[test]
    fn tokenize_filters_short() {
        let tokens = tokenize("a b cc dd");
        assert!(!tokens.contains(&"a".to_string()));
        assert!(!tokens.contains(&"b".to_string()));
        assert!(tokens.contains(&"cc".to_string()));
        assert!(tokens.contains(&"dd".to_string()));
    }

    #[test]
    fn detect_json() {
        assert_eq!(
            detect_format(r#"{"level":"ERROR","message":"fail"}"#),
            InputFormat::JsonLines
        );
    }

    #[test]
    fn detect_plain() {
        assert_eq!(
            detect_format("2026-09-06 10:00:00 ERROR payment failed"),
            InputFormat::PlainText
        );
    }

    #[test]
    fn parse_json_level() {
        let boost_fields = vec!["trace_id".to_string()];
        let parsed = parse_line(
            r#"{"level":"ERROR","trace_id":"abc123","message":"payment timeout"}"#,
            true,
            &boost_fields,
        );
        let fields = parsed.json_fields.unwrap();
        assert_eq!(fields.level.as_deref(), Some("ERROR"));
        assert!(fields
            .field_values
            .iter()
            .any(|(k, v)| k == "trace_id" && v == "abc123"));
    }

    #[test]
    fn parse_plain_no_json() {
        let parsed = parse_line("just plain text here", false, &[]);
        assert!(parsed.json_fields.is_none());
        assert!(!parsed.tokens.is_empty());
    }
}
