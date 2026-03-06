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
    // Temporary best-effort fallback until we swap in a maintained HTML-to-text library.
    let mut output = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                output.push(' ');
            }
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }

    decode_basic_entities(&normalize_whitespace(&output))
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

fn decode_basic_entities(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}
