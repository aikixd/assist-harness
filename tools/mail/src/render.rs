use html_to_markdown_rs::{convert, ConversionOptions};
use std::collections::BTreeSet;

const VISIBLE_LINK_LIMIT: usize = 120;
const TRACKING_THRESHOLD: f32 = 1.0;
const BOILERPLATE_THRESHOLD: f32 = 1.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyCandidate {
    pub mime_type: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedBody {
    pub body_text: String,
    pub links: Vec<String>,
    pub cleanup_metadata: Vec<String>,
    pub stripped_tracking_links: usize,
    pub stripped_boilerplate_blocks: usize,
    pub body_structure_supported: bool,
    pub body_structure_note: Option<String>,
}

pub fn preview_text(input: &str, max_chars: usize) -> String {
    let normalized = preview_normalize(input);
    normalized.chars().take(max_chars).collect()
}

pub fn render_message_body(
    account_email: &str,
    message_id: &str,
    subject_hint: Option<&str>,
    candidates: &[BodyCandidate],
    unsupported_structure: Option<&str>,
    snippet: Option<&str>,
    raw_body: bool,
) -> RenderedBody {
    if let Some(mime_type) = unsupported_structure {
        let note = unsupported_body_note(mime_type, account_email, message_id);
        return RenderedBody {
            body_text: note.clone(),
            links: Vec::new(),
            cleanup_metadata: Vec::new(),
            stripped_tracking_links: 0,
            stripped_boilerplate_blocks: 0,
            body_structure_supported: false,
            body_structure_note: Some(note),
        };
    }

    let selected_body = match select_body_candidate(candidates, snippet) {
        Some(SelectedBody::Html(html)) => convert_html_to_readable_text(&html),
        Some(SelectedBody::Plain(text)) | Some(SelectedBody::Snippet(text)) => {
            normalize_initial_text(&text)
        }
        None => String::new(),
    };

    if raw_body {
        let body_text = normalize_structure(&separate_inline_labels(&selected_body));
        return RenderedBody {
            links: extract_all_links(&body_text),
            body_text,
            cleanup_metadata: Vec::new(),
            stripped_tracking_links: 0,
            stripped_boilerplate_blocks: 0,
            body_structure_supported: true,
            body_structure_note: None,
        };
    }

    let (link_processed, retained_links, stripped_tracking_links) = process_links(&selected_body);
    let structured = normalize_structure(&separate_inline_labels(&link_processed));
    let subject_tokens = subject_tokens(subject_hint.unwrap_or_default());
    let (body_without_boilerplate, stripped_boilerplate_blocks) =
        strip_boilerplate_blocks(&structured, &subject_tokens);
    let body_text = normalize_structure(&body_without_boilerplate);

    let mut cleanup_metadata = Vec::new();
    if stripped_tracking_links > 0 {
        cleanup_metadata.push(format!(
            "stripped {} suspected tracking {}",
            stripped_tracking_links,
            pluralize(stripped_tracking_links, "link", "links")
        ));
    }
    if stripped_boilerplate_blocks > 0 {
        cleanup_metadata.push(format!(
            "stripped {} suspected boilerplate {}",
            stripped_boilerplate_blocks,
            pluralize(stripped_boilerplate_blocks, "block", "blocks")
        ));
    }

    RenderedBody {
        body_text,
        links: retained_links,
        cleanup_metadata,
        stripped_tracking_links,
        stripped_boilerplate_blocks,
        body_structure_supported: true,
        body_structure_note: None,
    }
}

enum SelectedBody {
    Plain(String),
    Html(String),
    Snippet(String),
}

fn select_body_candidate(
    candidates: &[BodyCandidate],
    snippet: Option<&str>,
) -> Option<SelectedBody> {
    for candidate in candidates {
        if candidate.mime_type.starts_with("text/html") && is_meaningful_body(&candidate.text) {
            return Some(SelectedBody::Html(candidate.text.clone()));
        }
    }

    for candidate in candidates {
        if candidate.mime_type.starts_with("text/plain") && is_meaningful_body(&candidate.text) {
            return Some(SelectedBody::Plain(candidate.text.clone()));
        }
    }

    for candidate in candidates {
        if candidate.mime_type.starts_with("text/html") {
            return Some(SelectedBody::Html(candidate.text.clone()));
        }
        if candidate.mime_type.starts_with("text/plain") {
            return Some(SelectedBody::Plain(candidate.text.clone()));
        }
    }

    snippet
        .filter(|value| is_meaningful_body(value))
        .map(|value| SelectedBody::Snippet(value.to_string()))
}

fn is_meaningful_body(text: &str) -> bool {
    !text.is_empty() && !text.trim().is_empty()
}

fn convert_html_to_readable_text(html: &str) -> String {
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

    let html = normalize_html_breaks(html);
    match convert(&html, Some(options)) {
        Ok(markdown) => sanitize_markdown_output(&normalize_initial_text(&markdown)),
        Err(_) => normalize_initial_text(&html),
    }
}

fn normalize_html_breaks(input: &str) -> String {
    input
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<BR>", "\n")
        .replace("<BR/>", "\n")
        .replace("<BR />", "\n")
}

fn normalize_initial_text(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    separate_inline_labels(&decode_basic_entities(
        &normalized
            .chars()
            .filter(|ch| !is_unwanted_invisible(*ch))
            .collect::<String>(),
    ))
}

fn preview_normalize(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_structure(input: &str) -> String {
    let mut lines = Vec::new();

    for raw_line in input.lines() {
        let line = collapse_inline_whitespace(raw_line.trim());
        if line.is_empty() {
            lines.push(String::new());
            continue;
        }

        if is_horizontal_rule_line(&line) {
            continue;
        }

        if is_presentational_table_line(&line) {
            continue;
        }

        let line = clean_table_content_line(&line);
        for expanded in expand_short_label_line(&line) {
            lines.push(expanded);
        }
    }

    while matches!(lines.first(), Some(line) if line.is_empty()) {
        lines.remove(0);
    }
    while matches!(lines.last(), Some(line) if line.is_empty()) {
        lines.pop();
    }

    let mut output = Vec::new();
    let mut previous_blank = false;
    for line in lines {
        if line.is_empty() {
            if !previous_blank {
                output.push(String::new());
            }
            previous_blank = true;
            continue;
        }

        output.push(line);
        previous_blank = false;
    }

    output.join("\n")
}

fn collapse_inline_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_presentational_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    if !trimmed.contains('|') {
        return false;
    }

    trimmed
        .chars()
        .all(|ch| matches!(ch, '|' | '-' | ':' | ' '))
}

fn is_horizontal_rule_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 8 && trimmed.chars().all(|ch| matches!(ch, '-' | '_' | ' '))
}

fn clean_table_content_line(line: &str) -> String {
    if !line.contains('|') {
        return line.to_string();
    }

    let cells = line
        .split('|')
        .map(str::trim)
        .filter(|cell| !cell.is_empty())
        .filter(|cell| !cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
        .collect::<Vec<_>>();

    if cells.is_empty() {
        line.to_string()
    } else {
        cells.join(" | ")
    }
}

fn expand_short_label_line(line: &str) -> Vec<String> {
    for label in ["Quote", "Summary", "Note"] {
        if let Some(rest) = line.strip_prefix(label) {
            if !rest.is_empty()
                && !rest.starts_with(' ')
                && rest
                    .chars()
                    .next()
                    .map(|ch| ch.is_uppercase())
                    .unwrap_or(false)
            {
                return vec![label.to_string(), String::new(), rest.trim().to_string()];
            }
        }
    }

    vec![line.to_string()]
}

fn separate_inline_labels(input: &str) -> String {
    let mut output = String::new();
    let mut index = 0;

    while index < input.len() {
        let remaining = &input[index..];
        let mut matched = false;

        for label in ["Quote", "Summary", "Note"] {
            if !remaining.starts_with(label) {
                continue;
            }

            let next = remaining[label.len()..].chars().next();
            let next_is_content = next.map(|ch| ch.is_uppercase()).unwrap_or(false);
            let prev_char = output.chars().last();
            let prev_supports_split = output.ends_with("- ")
                || prev_char.is_none()
                || matches!(
                    prev_char.unwrap(),
                    '\n' | ':' | ';' | '.' | '!' | '?' | ')' | '>'
                );

            if next_is_content && prev_supports_split {
                if output.ends_with("- ") {
                    output.truncate(output.len() - 2);
                }
                if !output.is_empty() && !output.ends_with("\n\n") {
                    if output.ends_with('\n') {
                        output.push('\n');
                    } else {
                        output.push_str("\n\n");
                    }
                }
                output.push_str(label);
                output.push_str("\n\n");
                index += label.len();
                matched = true;
                break;
            }
        }

        if matched {
            continue;
        }

        let ch = remaining
            .chars()
            .next()
            .expect("non-empty remaining input must have a character");
        output.push(ch);
        index += ch.len_utf8();
    }

    output
}

fn process_links(input: &str) -> (String, Vec<String>, usize) {
    let (markdown_processed, mut links, mut stripped_count) = process_markdown_links(input);
    let (raw_processed, raw_links, raw_stripped) = process_raw_urls(&markdown_processed);
    links.extend(raw_links);
    stripped_count += raw_stripped;
    (raw_processed, dedupe_links(&links), stripped_count)
}

fn extract_all_links(input: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut current = String::new();

    for ch in input.chars() {
        if ch.is_whitespace() {
            if looks_like_url(&current) {
                let (core, _) = trim_url_token(&current);
                links.push(core);
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }

    if looks_like_url(&current) {
        let (core, _) = trim_url_token(&current);
        links.push(core);
    }

    dedupe_links(&links)
}

fn process_markdown_links(input: &str) -> (String, Vec<String>, usize) {
    let mut output = String::new();
    let mut links = Vec::new();
    let mut stripped = 0;
    let mut index = 0;

    while index < input.len() {
        let remaining = &input[index..];
        if !remaining.starts_with('[') {
            let ch = remaining
                .chars()
                .next()
                .expect("non-empty remaining input must have a character");
            output.push(ch);
            index += ch.len_utf8();
            continue;
        }

        let Some(text_end) = remaining.find("](") else {
            output.push('[');
            index += 1;
            continue;
        };
        let text = &remaining[1..text_end];
        let url_start = text_end + 2;
        let Some(url_end_rel) = remaining[url_start..].find(')') else {
            output.push('[');
            index += 1;
            continue;
        };
        let url = &remaining[url_start..url_start + url_end_rel];
        let score = score_tracking_link(url, Some(text));
        let visible = collapse_inline_whitespace(text.trim());

        if score > TRACKING_THRESHOLD {
            stripped += 1;
            if !visible.is_empty() {
                output.push_str(&visible);
            }
        } else {
            if !visible.is_empty() {
                output.push_str(&visible);
            } else {
                output.push_str(&trim_visible_link(url, VISIBLE_LINK_LIMIT));
            }
            links.push(url.to_string());
        }

        index += url_start + url_end_rel + 1;
    }

    (output, links, stripped)
}

fn process_raw_urls(input: &str) -> (String, Vec<String>, usize) {
    let mut output = String::new();
    let mut token = String::new();
    let mut links = Vec::new();
    let mut stripped = 0;

    for ch in input.chars() {
        if ch.is_whitespace() {
            flush_token(&mut token, &mut output, &mut links, &mut stripped);
            output.push(ch);
        } else {
            token.push(ch);
        }
    }

    flush_token(&mut token, &mut output, &mut links, &mut stripped);

    (output, links, stripped)
}

fn flush_token(
    token: &mut String,
    output: &mut String,
    links: &mut Vec<String>,
    stripped: &mut usize,
) {
    if token.is_empty() {
        return;
    }

    let current = std::mem::take(token);
    let (core, had_trim) = trim_url_token(&current);
    if !looks_like_url(&core) {
        output.push_str(&current);
        return;
    }

    let score = score_tracking_link(&core, None);
    if score > TRACKING_THRESHOLD {
        *stripped += 1;
        if !had_trim {
            return;
        }
        return;
    }

    links.push(core.clone());
    output.push_str(&trim_visible_link(&core, VISIBLE_LINK_LIMIT));
}

fn trim_url_token(input: &str) -> (String, bool) {
    let trimmed =
        input.trim_matches(|ch: char| matches!(ch, '(' | ')' | '[' | ']' | '"' | '\'' | ',' | ';'));
    (trimmed.to_string(), trimmed.len() != input.len())
}

fn looks_like_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://")
}

fn trim_visible_link(url: &str, max_len: usize) -> String {
    let count = url.chars().count();
    if count <= max_len {
        return url.to_string();
    }

    let mut shortened = url
        .chars()
        .take(max_len.saturating_sub(3))
        .collect::<String>();
    shortened.push_str("...");
    shortened
}

fn dedupe_links(links: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();

    for link in links {
        if seen.insert(link.clone()) {
            output.push(link.clone());
        }
    }

    output
}

fn score_tracking_link(url: &str, text_hint: Option<&str>) -> f32 {
    let parsed = ParsedUrl::parse(url);
    if parsed.host.is_empty() {
        return 0.0;
    }

    let mut score = 0.0;

    if is_known_redirect_host(&parsed.host) {
        score += 1.25;
    }
    if has_redirect_style_path(&parsed.path) {
        score += 0.75;
    }
    if has_explicit_redirect_param(&parsed.query_params) {
        score += 0.9;
    }

    score += tracking_param_score(&parsed.query_params).min(0.8);
    score += signature_param_score(&parsed.query_params).min(0.7);

    let query_count = parsed.query_params.len();
    if query_count >= 10 {
        score += 0.5;
    } else if query_count >= 5 {
        score += 0.25;
    }

    let url_len = url.chars().count();
    if url_len > 500 {
        score += 0.8;
    } else if url_len > 240 {
        score += 0.5;
    } else if url_len > 120 {
        score += 0.25;
    }

    if has_blob_like_content(&parsed.path, &parsed.query) {
        score += 0.5;
    }

    if clean_text_href_mismatch(text_hint, &parsed.host) {
        score += 0.8;
    }

    if has_suspicious_subdomain_prefix(&parsed.host) {
        score += 0.35;
    }

    score
}

struct ParsedUrl {
    host: String,
    path: String,
    query: String,
    query_params: Vec<(String, String)>,
}

impl ParsedUrl {
    fn parse(input: &str) -> Self {
        let after_scheme = input
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(input);

        let host_end = after_scheme
            .find(['/', '?', '#'])
            .unwrap_or(after_scheme.len());
        let host = after_scheme[..host_end].to_lowercase();

        let rest = &after_scheme[host_end..];
        let (path, query) = if let Some((path, query_and_fragment)) = rest.split_once('?') {
            let query = query_and_fragment
                .split_once('#')
                .map(|(query, _)| query)
                .unwrap_or(query_and_fragment);
            (path.to_lowercase(), query.to_lowercase())
        } else {
            (rest.to_lowercase(), String::new())
        };

        let query_params = if query.is_empty() {
            Vec::new()
        } else {
            query
                .split('&')
                .filter(|segment| !segment.is_empty())
                .map(|segment| {
                    let (key, value) = segment.split_once('=').unwrap_or((segment, ""));
                    (key.to_string(), value.to_string())
                })
                .collect()
        };

        Self {
            host,
            path,
            query,
            query_params,
        }
    }
}

fn is_known_redirect_host(host: &str) -> bool {
    host.split('.')
        .any(|segment| matches!(segment, "click" | "track" | "lnk" | "redir" | "redirect"))
}

fn has_redirect_style_path(path: &str) -> bool {
    matches!(
        path,
        value if value.starts_with("/click")
            || value.starts_with("/track")
            || value.starts_with("/redirect")
            || value.starts_with("/out")
            || value.contains("/r/")
            || value.contains("/c/")
    )
}

fn has_explicit_redirect_param(query_params: &[(String, String)]) -> bool {
    query_params.iter().any(|(key, value)| {
        matches!(
            key.as_str(),
            "url" | "u" | "target" | "dest" | "destination" | "redirect" | "redir"
        ) && (value.contains("http") || value.contains("%3a%2f%2f"))
    })
}

fn tracking_param_score(query_params: &[(String, String)]) -> f32 {
    let mut score = 0.0;
    for (key, _) in query_params {
        if key.starts_with("utm_")
            || key.starts_with("mc_")
            || key.starts_with("vero_")
            || key.starts_with("hs_")
            || matches!(key.as_str(), "gclid" | "fbclid" | "msclkid")
        {
            score += 0.2;
        }
    }
    score
}

fn signature_param_score(query_params: &[(String, String)]) -> f32 {
    let mut score = 0.0;
    for (key, _) in query_params {
        if matches!(
            key.as_str(),
            "token" | "sig" | "signature" | "hash" | "hmac" | "expires"
        ) {
            score += 0.35;
        }
    }
    score
}

fn has_blob_like_content(path: &str, query: &str) -> bool {
    path.split('/')
        .chain(query.split('&'))
        .any(is_blob_like_segment)
}

fn is_blob_like_segment(segment: &str) -> bool {
    let compact = segment.trim_matches(|ch: char| {
        !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' && ch != '%'
    });
    compact.len() >= 24
        && compact
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '%' | '='))
}

fn clean_text_href_mismatch(text_hint: Option<&str>, host: &str) -> bool {
    let Some(text_hint) = text_hint else {
        return false;
    };
    let text = collapse_inline_whitespace(text_hint.trim()).to_lowercase();
    if text.is_empty() || text.contains(host) {
        return false;
    }

    text.starts_with("http://")
        || text.starts_with("https://")
        || text.contains(".com")
        || text.contains(".io")
        || text.contains(".org")
}

fn has_suspicious_subdomain_prefix(host: &str) -> bool {
    let Some(first) = host.split('.').next() else {
        return false;
    };
    first.starts_with("click")
        || first.starts_with("track")
        || first.starts_with("lnk")
        || first.starts_with("email")
        || first.starts_with("mg")
}

fn strip_boilerplate_blocks(input: &str, subject_tokens: &BTreeSet<String>) -> (String, usize) {
    let blocks = split_into_blocks(input);
    if blocks.is_empty() {
        return (String::new(), 0);
    }

    let mut kept = Vec::new();
    let mut stripped = 0;
    for (index, block) in blocks.iter().enumerate() {
        if score_boilerplate_block(block, index, blocks.len(), subject_tokens)
            > BOILERPLATE_THRESHOLD
        {
            stripped += 1;
        } else {
            kept.push(block.clone());
        }
    }

    if kept.is_empty() {
        return (blocks.join("\n\n"), 0);
    }

    (kept.join("\n\n"), stripped)
}

fn split_into_blocks(input: &str) -> Vec<String> {
    input
        .split("\n\n")
        .map(str::trim)
        .filter(|block| !block.is_empty())
        .map(|block| block.to_string())
        .collect()
}

fn score_boilerplate_block(
    block: &str,
    index: usize,
    total_blocks: usize,
    subject_tokens: &BTreeSet<String>,
) -> f32 {
    let lower = block.to_lowercase();
    let mut score = 0.0;

    if index + 2 >= total_blocks {
        score += 0.35;
    }
    if contains_any_phrase(
        &lower,
        &[
            "unsubscribe",
            "manage preferences",
            "email settings",
            "notification settings",
            "view in browser",
            "open in app",
        ],
    ) {
        score += 0.7;
    }
    if contains_any_phrase(
        &lower,
        &[
            "privacy policy",
            "terms of service",
            "all rights reserved",
            "confidentiality notice",
            "this email and any attachments",
        ],
    ) {
        score += 0.9;
    }
    if is_link_heavy_utility_block(block) {
        score += 0.45;
    }
    if looks_like_branding_tail(block) {
        score += 0.4;
    }
    if is_multi_line_utility_cluster(block) {
        score += 0.6;
    }
    if block.starts_with("---") || block.starts_with("***") {
        score += 0.25;
    }

    if looks_like_real_paragraph(block) {
        score -= 0.45;
    }
    if has_topic_overlap(&lower, subject_tokens) {
        score -= 0.4;
    }
    if index > 0 && index + 2 < total_blocks {
        score -= 0.5;
    }
    if looks_content_rich(block) {
        score -= 0.35;
    }

    score
}

fn contains_any_phrase(input: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| input.contains(phrase))
}

fn is_link_heavy_utility_block(block: &str) -> bool {
    let lower = block.to_lowercase();
    let linkish_count = block.matches("http://").count()
        + block.matches("https://").count()
        + block.matches('[').count();
    let utility_count = ["settings", "privacy", "unsubscribe", "help", "preferences"]
        .iter()
        .filter(|word| lower.contains(**word))
        .count();
    let word_count = word_count(block);

    linkish_count > 0 && utility_count > 0 && word_count <= 24
}

fn looks_like_branding_tail(block: &str) -> bool {
    let lines = block.lines().collect::<Vec<_>>();
    word_count(block) <= 18
        && lines.len() <= 3
        && !looks_like_real_paragraph(block)
        && (block.contains("http://") || block.contains("https://") || block.contains('['))
}

fn is_multi_line_utility_cluster(block: &str) -> bool {
    let lines = block
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    lines.len() >= 2
        && lines
            .iter()
            .all(|line| word_count(line) <= 4 && !line.ends_with('.'))
}

fn looks_like_real_paragraph(block: &str) -> bool {
    word_count(block) >= 12 && (block.contains('.') || block.contains('!') || block.contains('?'))
}

fn looks_content_rich(block: &str) -> bool {
    word_count(block) >= 8
        && block
            .split_whitespace()
            .filter(|word| {
                word.chars()
                    .next()
                    .map(|ch| ch.is_uppercase())
                    .unwrap_or(false)
            })
            .count()
            >= 2
}

fn has_topic_overlap(block_lower: &str, subject_tokens: &BTreeSet<String>) -> bool {
    if subject_tokens.is_empty() {
        return false;
    }

    let block_tokens = tokenize_for_overlap(block_lower);
    block_tokens
        .iter()
        .any(|token| subject_tokens.contains(token))
}

fn subject_tokens(subject: &str) -> BTreeSet<String> {
    tokenize_for_overlap(subject)
}

fn tokenize_for_overlap(input: &str) -> BTreeSet<String> {
    input
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 4)
        .map(|token| token.to_lowercase())
        .collect()
}

fn word_count(input: &str) -> usize {
    input.split_whitespace().count()
}

fn unsupported_body_note(mime_type: &str, account_email: &str, message_id: &str) -> String {
    format!(
        "message body structure {mime_type} is not supported yet\nreport this issue with account {account_email} and message id {message_id}"
    )
}

fn pluralize<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

fn sanitize_markdown_output(input: &str) -> String {
    let lines = input.lines().collect::<Vec<_>>();
    if lines.first().map(|line| line.trim()) == Some("---") {
        if let Some(closing_index) = lines.iter().enumerate().skip(1).find_map(|(index, line)| {
            if line.trim() == "---" {
                Some(index)
            } else {
                None
            }
        }) {
            let metadata_lines = &lines[1..closing_index];
            if !metadata_lines.is_empty()
                && metadata_lines.iter().all(|line| {
                    let trimmed = line.trim_start().to_lowercase();
                    trimmed.starts_with("meta-")
                        || trimmed.starts_with("title:")
                        || trimmed.starts_with("description:")
                        || trimmed.starts_with("viewport:")
                        || trimmed.starts_with("author:")
                })
            {
                return lines[closing_index + 1..].join("\n");
            }
        }
    }

    input.to_string()
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
    fn preview_flattens_clean_body() {
        assert_eq!(preview_text("Hello\n\nworld", 50), "Hello world");
    }

    #[test]
    fn normalize_structure_collapses_excess_blank_lines() {
        let normalized = normalize_structure("A\n\n\n\nB");
        assert_eq!(normalized, "A\n\nB");
    }

    #[test]
    fn normalize_structure_removes_presentational_table_lines() {
        let normalized = normalize_structure("Title\n| --- | --- |\nBody");
        assert_eq!(normalized, "Title\nBody");
    }

    #[test]
    fn normalize_structure_cleans_table_content_rows() {
        let normalized = normalize_structure("| Go to Notion → |\n| D | Dima Sorkin's Space |");
        assert_eq!(normalized, "Go to Notion →\nD | Dima Sorkin's Space");
    }

    #[test]
    fn normalize_structure_splits_short_labels_from_attached_text() {
        let normalized = normalize_structure("QuoteReally excited to share this one.");
        assert_eq!(normalized, "Quote\n\nReally excited to share this one.");
    }

    #[test]
    fn normalize_initial_text_separates_inline_quote_labels() {
        let normalized = normalize_initial_text("Linkedin:QuoteFor the last few months...");
        assert!(normalized.contains("Linkedin:\n\nQuote\n\nFor the last few months..."));
    }

    #[test]
    fn markdown_tracking_links_are_stripped_but_text_is_kept() {
        let input = "[Go to Notion](https://click.example.com/redirect?url=https%3A%2F%2Fnotion.so&gclid=1)";
        let (processed, links, stripped) = process_links(input);
        assert_eq!(processed, "Go to Notion");
        assert!(links.is_empty());
        assert_eq!(stripped, 1);
    }

    #[test]
    fn raw_links_are_trimmed_and_retained_when_not_tracking() {
        let url = "https://example.com/some/really/long/path/that/keeps/going/for/a/while/and/has/useful/content";
        let (processed, links, stripped) = process_links(url);
        assert!(processed.contains("https://example.com/"));
        assert_eq!(links, vec![url.to_string()]);
        assert_eq!(stripped, 0);
    }

    #[test]
    fn render_message_body_reports_tracking_and_boilerplate_metadata() {
        let rendered = render_message_body(
            "me@example.com",
            "msg-1",
            Some("Weekly update"),
            &[BodyCandidate {
                mime_type: "text/html".to_string(),
                text: "<p>Hello world.</p><p><a href=\"https://click.example.com/redirect?url=https%3A%2F%2Fexample.com&gclid=1\">Open</a></p><p>unsubscribe</p>".to_string(),
            }],
            None,
            None,
            false,
        );

        assert!(rendered
            .cleanup_metadata
            .iter()
            .any(|item| item.contains("suspected tracking")));
        assert!(rendered
            .cleanup_metadata
            .iter()
            .any(|item| item.contains("suspected boilerplate")));
    }

    #[test]
    fn unsupported_structure_returns_note() {
        let rendered = render_message_body(
            "me@example.com",
            "msg-1",
            Some("Subject"),
            &[],
            Some("multipart/report"),
            None,
            false,
        );

        assert!(!rendered.body_structure_supported);
        assert!(rendered.body_text.contains("multipart/report"));
        assert!(rendered.body_text.contains("msg-1"));
    }

    #[test]
    fn raw_body_mode_skips_tracking_and_boilerplate_stripping() {
        let rendered = render_message_body(
            "me@example.com",
            "msg-1",
            Some("Weekly update"),
            &[BodyCandidate {
                mime_type: "text/html".to_string(),
                text: "<p>Hello world.</p><p><a href=\"https://click.example.com/redirect?url=https%3A%2F%2Fexample.com&gclid=1\">Open</a></p><p>unsubscribe</p>".to_string(),
            }],
            None,
            None,
            true,
        );

        assert!(rendered.cleanup_metadata.is_empty());
        assert_eq!(rendered.stripped_tracking_links, 0);
        assert!(rendered.body_text.contains("Open"));
    }
}
