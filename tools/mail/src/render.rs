use html_to_markdown_rs::{convert, ConversionOptions};
use std::collections::BTreeSet;

pub fn preview_text(input: &str, max_chars: usize) -> String {
    let normalized = normalize_whitespace(input);
    let mut output = String::new();
    for ch in normalized.chars().take(max_chars) {
        output.push(ch);
    }
    output
}

pub fn html_to_readable_text(html: &str) -> String {
    let options = ConversionOptions {
        strip_tags: vec![
            "script".to_string(),
            "style".to_string(),
            "head".to_string(),
            "svg".to_string(),
            "noscript".to_string(),
        ],
        skip_images: true,
        ..Default::default()
    };

    match convert(html, Some(options)) {
        Ok(markdown) => normalize_markdown_whitespace(&sanitize_markdown_output(
            &decode_basic_entities(&markdown),
        )),
        Err(_) => decode_basic_entities(&normalize_whitespace(html)),
    }
}

pub fn extract_links(text: &str) -> Vec<String> {
    let mut links = BTreeSet::new();
    for token in text.split_whitespace() {
        if token.starts_with("http://") || token.starts_with("https://") {
            links.insert(
                token
                    .trim_matches(|ch: char| matches!(ch, ')' | '(' | ',' | '.' | ';' | '"'))
                    .to_string(),
            );
        }
    }
    links.into_iter().collect()
}

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_markdown_whitespace(input: &str) -> String {
    let mut lines = input.lines().map(str::trim).collect::<Vec<_>>();
    while matches!(lines.first(), Some(line) if line.is_empty()) {
        lines.remove(0);
    }
    while matches!(lines.last(), Some(line) if line.is_empty()) {
        lines.pop();
    }

    let mut output = Vec::new();
    let mut previous_blank = false;
    for line in lines {
        let is_blank = line.is_empty();
        if is_blank {
            if !previous_blank {
                output.push(String::new());
            }
        } else {
            output.push(line.to_string());
        }
        previous_blank = is_blank;
    }

    output.join("\n")
}

fn sanitize_markdown_output(input: &str) -> String {
    let cleaned = input
        .chars()
        .filter(|ch| !is_unwanted_invisible(*ch))
        .collect::<String>();

    let lines = cleaned.lines().collect::<Vec<_>>();
    if lines.len() >= 3
        && lines[0].trim() == "---"
        && lines[2].trim() == "---"
        && lines[1].trim_start().starts_with("meta-")
    {
        return lines[3..].join("\n");
    }

    cleaned
}

fn is_unwanted_invisible(ch: char) -> bool {
    matches!(
        ch,
        '\u{00AD}'
            | '\u{034F}'
            | '\u{061C}'
            | '\u{17B4}'
            | '\u{17B5}'
            | '\u{180E}'
            | '\u{200B}'
            | '\u{200C}'
            | '\u{200D}'
            | '\u{2060}'
            | '\u{2061}'
            | '\u{2062}'
            | '\u{2063}'
            | '\u{FEFF}'
    )
}

fn decode_basic_entities(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_readable_text_strips_style_content() {
        let html = r#"<html><head><style>.x{color:red}</style></head><body><p>Hello&nbsp;world</p></body></html>"#;
        let rendered = html_to_readable_text(html);
        assert!(rendered.contains("Hello world"));
        assert!(!rendered.contains(".x{color:red}"));
    }

    #[test]
    fn html_to_readable_text_strips_meta_block_and_invisible_chars() {
        let input = "---\nmeta-viewport: width=device-width\n---\nHello\u{200C} world";
        let rendered = normalize_markdown_whitespace(&sanitize_markdown_output(input));
        assert_eq!(rendered, "Hello world");
    }
}
