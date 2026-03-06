pub fn join_blocks(blocks: &[String]) -> String {
    blocks.join("\n\n")
}

pub fn escape_json(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());

    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }

    escaped
}

pub fn json_string(input: &str) -> String {
    format!("\"{}\"", escape_json(input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_string_escapes_special_characters() {
        let escaped = json_string("hello\n\"mail\"");
        assert_eq!(escaped, "\"hello\\n\\\"mail\\\"\"");
    }
}
