use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(value) if *value >= 0.0 => Some(*value as u64),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.as_object()?.get(key)
    }
}

pub fn parse(input: &str) -> Result<JsonValue, String> {
    let mut parser = Parser {
        chars: input.chars().collect(),
        index: 0,
    };
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.index != parser.chars.len() {
        return Err("unexpected trailing characters in JSON".to_string());
    }
    Ok(value)
}

struct Parser {
    chars: Vec<char>,
    index: usize,
}

impl Parser {
    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        let Some(ch) = self.peek() else {
            return Err("unexpected end of JSON".to_string());
        };

        match ch {
            'n' => self.parse_null(),
            't' | 'f' => self.parse_bool(),
            '"' => self.parse_string().map(JsonValue::String),
            '[' => self.parse_array(),
            '{' => self.parse_object(),
            '-' | '0'..='9' => self.parse_number(),
            _ => Err(format!("unexpected character in JSON: {ch}")),
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        self.expect_keyword("null")?;
        Ok(JsonValue::Null)
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.starts_with("true") {
            self.expect_keyword("true")?;
            Ok(JsonValue::Bool(true))
        } else {
            self.expect_keyword("false")?;
            Ok(JsonValue::Bool(false))
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut output = String::new();

        while let Some(ch) = self.next() {
            match ch {
                '"' => return Ok(output),
                '\\' => {
                    let escaped = self
                        .next()
                        .ok_or_else(|| "unexpected end of escape sequence".to_string())?;
                    match escaped {
                        '"' => output.push('"'),
                        '\\' => output.push('\\'),
                        '/' => output.push('/'),
                        'b' => output.push('\u{0008}'),
                        'f' => output.push('\u{000C}'),
                        'n' => output.push('\n'),
                        'r' => output.push('\r'),
                        't' => output.push('\t'),
                        'u' => {
                            let codepoint = self.parse_hex_u16()?;
                            let decoded = char::from_u32(codepoint as u32)
                                .ok_or_else(|| "invalid unicode escape".to_string())?;
                            output.push(decoded);
                        }
                        other => {
                            return Err(format!("unsupported escape sequence: \\{other}"));
                        }
                    }
                }
                other => output.push(other),
            }
        }

        Err("unterminated JSON string".to_string())
    }

    fn parse_hex_u16(&mut self) -> Result<u16, String> {
        let mut value = 0u16;
        for _ in 0..4 {
            let ch = self
                .next()
                .ok_or_else(|| "unexpected end of unicode escape".to_string())?;
            let digit = ch
                .to_digit(16)
                .ok_or_else(|| "invalid unicode escape".to_string())?;
            value = (value << 4) | digit as u16;
        }
        Ok(value)
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.expect('[')?;
        self.skip_whitespace();

        let mut values = Vec::new();
        if self.peek() == Some(']') {
            self.index += 1;
            return Ok(JsonValue::Array(values));
        }

        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();

            match self.next() {
                Some(',') => {}
                Some(']') => break,
                Some(other) => {
                    return Err(format!("unexpected character in array: {other}"));
                }
                None => return Err("unterminated JSON array".to_string()),
            }
        }

        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.expect('{')?;
        self.skip_whitespace();

        let mut values = BTreeMap::new();
        if self.peek() == Some('}') {
            self.index += 1;
            return Ok(JsonValue::Object(values));
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(':')?;
            let value = self.parse_value()?;
            values.insert(key, value);
            self.skip_whitespace();

            match self.next() {
                Some(',') => {}
                Some('}') => break,
                Some(other) => {
                    return Err(format!("unexpected character in object: {other}"));
                }
                None => return Err("unterminated JSON object".to_string()),
            }
        }

        Ok(JsonValue::Object(values))
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.index;

        if self.peek() == Some('-') {
            self.index += 1;
        }

        while matches!(self.peek(), Some('0'..='9')) {
            self.index += 1;
        }

        if self.peek() == Some('.') {
            self.index += 1;
            while matches!(self.peek(), Some('0'..='9')) {
                self.index += 1;
            }
        }

        if matches!(self.peek(), Some('e' | 'E')) {
            self.index += 1;
            if matches!(self.peek(), Some('+' | '-')) {
                self.index += 1;
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.index += 1;
            }
        }

        let value = self.chars[start..self.index]
            .iter()
            .collect::<String>()
            .parse::<f64>()
            .map_err(|_| "invalid JSON number".to_string())?;

        Ok(JsonValue::Number(value))
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), String> {
        if self.starts_with(keyword) {
            self.index += keyword.len();
            Ok(())
        } else {
            Err(format!("expected {keyword}"))
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.next() {
            Some(ch) if ch == expected => Ok(()),
            Some(ch) => Err(format!("expected {expected}, got {ch}")),
            None => Err(format!("expected {expected}, got end of JSON")),
        }
    }

    fn starts_with(&self, value: &str) -> bool {
        self.chars[self.index..]
            .iter()
            .collect::<String>()
            .starts_with(value)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n' | '\r' | '\t')) {
            self.index += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn next(&mut self) -> Option<char> {
        let value = self.peek()?;
        self.index += 1;
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_json() {
        let value = parse(r#"{"a":[1,{"b":"x"}],"c":true}"#).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object.get("c"), Some(&JsonValue::Bool(true)));
        let array = object.get("a").unwrap().as_array().unwrap();
        assert_eq!(array.len(), 2);
    }
}
