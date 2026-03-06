use std::collections::BTreeMap;
use std::process::Command;

use oauth::{
    exchange_code_with_curl, load_client_config, load_token, merge_token_response,
    refresh_token_with_curl, store_token, OAuthClientConfig, TokenResponse,
};

use crate::config::{load_provider_client_config, AccountEntry, Provider};
use crate::domain::{Attachment, MessageDetail, MessageSummary};
use crate::error::AppError;
use crate::json::{parse as parse_json, JsonValue};
use crate::render::{extract_links, html_to_readable_text, preview_text};
use crate::time::{epoch_millis_to_local_timestamp, local_timestamp_to_epoch_seconds};

use super::ListQuery;

const TOOL_NAME: &str = "mail";
const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const API_BASE: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const GMAIL_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
const CLIENT_ID_ENV: &str = "PA_MAIL_GOOGLE_CLIENT_ID";
const CLIENT_SECRET_ENV: &str = "PA_MAIL_GOOGLE_CLIENT_SECRET";

pub fn supports_label_filter() -> bool {
    true
}

pub fn list_messages(
    account: &AccountEntry,
    query: &ListQuery,
) -> Result<Vec<MessageSummary>, AppError> {
    let session = GmailSession::new(account)?;
    let label_id = match query.label.as_deref() {
        Some(label) => Some(session.resolve_label_id(label)?),
        None => None,
    };

    let q = build_gmail_query(query)?;
    let mut params = vec![(
        "maxResults".to_string(),
        query.limit.unwrap_or(20).to_string(),
    )];
    if !q.is_empty() {
        params.push(("q".to_string(), q));
    }
    if let Some(label_id) = label_id {
        params.push(("labelIds".to_string(), label_id));
    }

    let response = session.api_get("/messages", &params)?;
    let root = parse_json(&response).map_err(|error| {
        AppError::query(format!("failed to parse Gmail list response: {error}"))
    })?;
    let message_refs = root
        .get("messages")
        .and_then(JsonValue::as_array)
        .map(|items| items.to_vec())
        .unwrap_or_default();

    let mut summaries = Vec::new();
    for item in message_refs {
        let id = item
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| AppError::query("Gmail list response is missing message id"))?;

        let detail = session.fetch_message_metadata(id)?;
        if !message_matches_time_window(&detail.internal_date_millis, query)? {
            continue;
        }

        summaries.push(MessageSummary {
            id: detail.id,
            date: detail.date,
            from: detail.from,
            to: detail.to,
            subject: detail.subject,
            labels: detail.labels,
            body_preview: preview_text(&detail.preview_source, 250),
            thread_id: detail.thread_id,
        });
    }

    Ok(summaries)
}

pub fn get_message(account: &AccountEntry, message_id: &str) -> Result<MessageDetail, AppError> {
    let session = GmailSession::new(account)?;
    let response = session.api_get_message(message_id, "full")?;
    let root = parse_json(&response).map_err(|error| {
        AppError::query(format!("failed to parse Gmail message response: {error}"))
    })?;
    let detail = parse_message_detail(&session, &root, account.email.clone())?;
    Ok(detail)
}

pub fn provider_name() -> &'static str {
    Provider::Google.as_str()
}

pub fn load_oauth_client() -> Result<OAuthClientConfig, AppError> {
    if let Ok(config) = load_provider_client_config(&Provider::Google) {
        return Ok(config);
    }

    load_client_config(CLIENT_ID_ENV, CLIENT_SECRET_ENV)
        .map_err(|error| AppError::config(format!("{error}")))
}

pub fn build_authorization_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&access_type=offline&prompt=consent",
        AUTH_ENDPOINT,
        percent_encode(client_id),
        percent_encode(redirect_uri),
        percent_encode(GMAIL_READONLY_SCOPE),
        percent_encode(state),
    )
}

pub fn exchange_code(code: &str, redirect_uri: &str) -> Result<TokenResponse, AppError> {
    let client = load_oauth_client()?;
    exchange_code_with_curl(TOKEN_ENDPOINT, &client, code, redirect_uri)
        .map_err(|error| AppError::config(format!("{error}")))
}

struct GmailSession {
    access_token: String,
    label_map: LabelMap,
}

impl GmailSession {
    fn new(account: &AccountEntry) -> Result<Self, AppError> {
        let client = load_oauth_client()?;
        let previous = load_token(TOOL_NAME, &account.email)
            .map_err(|error| AppError::config(format!("{error}")))?;
        let refresh_token = previous
            .refresh_token
            .clone()
            .ok_or_else(|| AppError::config("stored token is missing refresh_token"))?;
        let refreshed = refresh_token_with_curl(TOKEN_ENDPOINT, &client, &refresh_token)
            .map_err(|error| AppError::config(format!("{error}")))?;
        let merged = merge_token_response(&previous, &refreshed);
        store_token(TOOL_NAME, &account.email, &merged.raw_json).map_err(|error| {
            AppError::config(format!("failed to store refreshed token: {error}"))
        })?;

        let mut session = Self {
            access_token: merged.access_token,
            label_map: LabelMap::default(),
        };
        session.label_map = session.fetch_labels()?;
        Ok(session)
    }

    fn fetch_labels(&self) -> Result<LabelMap, AppError> {
        let response = self.api_get("/labels", &[])?;
        let root = parse_json(&response).map_err(|error| {
            AppError::query(format!("failed to parse Gmail labels response: {error}"))
        })?;
        let mut label_map = LabelMap::default();

        for label in root
            .get("labels")
            .and_then(JsonValue::as_array)
            .unwrap_or(&[])
        {
            let Some(id) = label.get("id").and_then(JsonValue::as_str) else {
                continue;
            };
            let Some(name) = label.get("name").and_then(JsonValue::as_str) else {
                continue;
            };

            let display = display_label_name(name);
            label_map.id_to_name.insert(id.to_string(), display.clone());
            label_map
                .name_to_id
                .insert(name.to_lowercase(), id.to_string());
            label_map
                .name_to_id
                .insert(display.to_lowercase(), id.to_string());
        }

        Ok(label_map)
    }

    fn resolve_label_id(&self, requested: &str) -> Result<String, AppError> {
        self.label_map
            .name_to_id
            .get(&requested.to_lowercase())
            .cloned()
            .ok_or_else(|| AppError::query(format!("label {requested} not found")))
    }

    fn fetch_message_metadata(&self, message_id: &str) -> Result<ListMessageDetail, AppError> {
        let response = self.api_get_message_with_params(
            message_id,
            &[
                ("format".to_string(), "metadata".to_string()),
                ("metadataHeaders".to_string(), "From".to_string()),
                ("metadataHeaders".to_string(), "To".to_string()),
                ("metadataHeaders".to_string(), "Subject".to_string()),
            ],
        )?;
        let root = parse_json(&response).map_err(|error| {
            AppError::query(format!("failed to parse Gmail metadata response: {error}"))
        })?;

        let id = required_str(&root, "id")?.to_string();
        let thread_id = root
            .get("threadId")
            .and_then(JsonValue::as_str)
            .map(|value| value.to_string());
        let internal_date_millis = required_str(&root, "internalDate")?
            .parse::<i64>()
            .map_err(|_| AppError::query("Gmail message internalDate was invalid"))?;
        let date = epoch_millis_to_local_timestamp(internal_date_millis)?;
        let snippet = root
            .get("snippet")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string();
        let headers = root
            .get("payload")
            .and_then(|value| value.get("headers"))
            .and_then(JsonValue::as_array)
            .ok_or_else(|| AppError::query("Gmail metadata response is missing headers"))?;

        let header_map = parse_headers(headers);
        let labels = root
            .get("labelIds")
            .and_then(JsonValue::as_array)
            .map(|items| self.map_labels(items))
            .unwrap_or_default();

        Ok(ListMessageDetail {
            id,
            thread_id,
            internal_date_millis,
            date,
            from: header_map.get("from").cloned().unwrap_or_default(),
            to: header_map.get("to").cloned().unwrap_or_default(),
            subject: header_map.get("subject").cloned().unwrap_or_default(),
            labels,
            preview_source: snippet,
        })
    }

    fn api_get(&self, path: &str, params: &[(String, String)]) -> Result<String, AppError> {
        let (status, body, stderr) = self.api_get_with_status(path, params)?;
        match status {
            200 => Ok(body),
            404 => Err(AppError::query("Gmail resource not found")),
            code => {
                let message = if stderr.is_empty() {
                    format!("Gmail API request failed with status {code}")
                } else {
                    format!("Gmail API request failed with status {code}: {stderr}")
                };
                Err(AppError::query(message))
            }
        }
    }

    fn api_get_with_status(
        &self,
        path: &str,
        params: &[(String, String)],
    ) -> Result<(u16, String, String), AppError> {
        let mut url = format!("{API_BASE}{path}");
        if !params.is_empty() {
            let query = params
                .iter()
                .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&query);
        }

        let output = Command::new("curl")
            .arg("--silent")
            .arg("--show-error")
            .arg("-H")
            .arg(format!("Authorization: Bearer {}", self.access_token))
            .arg("-H")
            .arg("Accept: application/json")
            .arg("-w")
            .arg("\n%{http_code}")
            .arg(url)
            .output()
            .map_err(|error| AppError::config(format!("failed to run curl: {error}")))?;

        let stdout = String::from_utf8(output.stdout).map_err(|error| {
            AppError::config(format!("curl output was not valid UTF-8: {error}"))
        })?;
        let (body, status) = stdout.rsplit_once('\n').ok_or_else(|| {
            AppError::config("failed to parse curl response status for Gmail API")
        })?;
        let status = status
            .trim()
            .parse::<u16>()
            .map_err(|_| AppError::config("failed to parse Gmail API status code"))?;
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Ok((status, body.to_string(), stderr))
    }

    fn api_get_message(&self, message_id: &str, format: &str) -> Result<String, AppError> {
        self.api_get_message_with_params(message_id, &[("format".to_string(), format.to_string())])
    }

    fn api_get_message_with_params(
        &self,
        message_id: &str,
        params: &[(String, String)],
    ) -> Result<String, AppError> {
        let path = format!("/messages/{message_id}");
        let (status, body, stderr) = self.api_get_with_status(&path, params)?;
        match status {
            200 => Ok(body),
            404 => Err(AppError::query(format!(
                "message with id {message_id} not found"
            ))),
            code => {
                let message = if stderr.is_empty() {
                    format!("Gmail API request failed with status {code}")
                } else {
                    format!("Gmail API request failed with status {code}: {stderr}")
                };
                Err(AppError::query(message))
            }
        }
    }

    fn map_labels(&self, items: &[JsonValue]) -> Vec<String> {
        items
            .iter()
            .filter_map(JsonValue::as_str)
            .map(|id| {
                self.label_map
                    .id_to_name
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| display_label_name(id))
            })
            .collect()
    }
}

#[derive(Default)]
struct LabelMap {
    id_to_name: BTreeMap<String, String>,
    name_to_id: BTreeMap<String, String>,
}

struct ListMessageDetail {
    id: String,
    thread_id: Option<String>,
    internal_date_millis: i64,
    date: String,
    from: String,
    to: String,
    subject: String,
    labels: Vec<String>,
    preview_source: String,
}

fn parse_message_detail(
    session: &GmailSession,
    root: &JsonValue,
    account_email: String,
) -> Result<MessageDetail, AppError> {
    let id = required_str(root, "id")?.to_string();
    let thread_id = root
        .get("threadId")
        .and_then(JsonValue::as_str)
        .map(|value| value.to_string());
    let internal_date_millis = required_str(root, "internalDate")?
        .parse::<i64>()
        .map_err(|_| AppError::query("Gmail message internalDate was invalid"))?;
    let date = epoch_millis_to_local_timestamp(internal_date_millis)?;
    let labels = root
        .get("labelIds")
        .and_then(JsonValue::as_array)
        .map(|items| session.map_labels(items))
        .unwrap_or_default();

    let payload = root
        .get("payload")
        .ok_or_else(|| AppError::query("Gmail message response is missing payload"))?;
    let headers = payload
        .get("headers")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| AppError::query("Gmail message response is missing headers"))?;
    let header_map = parse_headers(headers);

    let mut collector = PartCollector::default();
    collector.visit_part(payload)?;

    let body_text = if !collector.text_bodies.is_empty() {
        collector.text_bodies.join("\n\n")
    } else if !collector.html_bodies.is_empty() {
        collector
            .html_bodies
            .iter()
            .map(|body| html_to_readable_text(body))
            .collect::<Vec<_>>()
            .join("\n\n")
    } else {
        root.get("snippet")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string()
    };
    let links = extract_links(&body_text);

    Ok(MessageDetail {
        account: account_email,
        id,
        thread_id,
        date,
        from: header_map.get("from").cloned().unwrap_or_default(),
        to: header_map.get("to").cloned().unwrap_or_default(),
        cc: header_map.get("cc").cloned(),
        subject: header_map.get("subject").cloned().unwrap_or_default(),
        labels,
        body_text,
        links,
        attachments: collector.attachments,
    })
}

#[derive(Default)]
struct PartCollector {
    text_bodies: Vec<String>,
    html_bodies: Vec<String>,
    attachments: Vec<Attachment>,
}

impl PartCollector {
    fn visit_part(&mut self, part: &JsonValue) -> Result<(), AppError> {
        let mime_type = part
            .get("mimeType")
            .and_then(JsonValue::as_str)
            .unwrap_or("");
        let filename = part
            .get("filename")
            .and_then(JsonValue::as_str)
            .unwrap_or("");

        if !filename.is_empty() {
            let size_bytes = part
                .get("body")
                .and_then(|body| body.get("size"))
                .and_then(JsonValue::as_u64)
                .unwrap_or(0) as usize;

            self.attachments.push(Attachment {
                name: filename.to_string(),
                mime_type: mime_type.to_string(),
                size_bytes,
            });
        }

        if let Some(data) = part
            .get("body")
            .and_then(|body| body.get("data"))
            .and_then(JsonValue::as_str)
        {
            let decoded = decode_base64_url(data)?;
            let text = String::from_utf8_lossy(&decoded).to_string();
            if mime_type.starts_with("text/plain") {
                self.text_bodies.push(text);
            } else if mime_type.starts_with("text/html") {
                self.html_bodies.push(text);
            }
        }

        if let Some(parts) = part.get("parts").and_then(JsonValue::as_array) {
            for child in parts {
                self.visit_part(child)?;
            }
        }

        Ok(())
    }
}

fn message_matches_time_window(
    internal_date_millis: &i64,
    query: &ListQuery,
) -> Result<bool, AppError> {
    let seconds = internal_date_millis / 1000;
    let since = local_timestamp_to_epoch_seconds(&query.since)?;
    if seconds < since {
        return Ok(false);
    }

    if let Some(until) = &query.until {
        let until = local_timestamp_to_epoch_seconds(until)?;
        if seconds >= until {
            return Ok(false);
        }
    }

    Ok(true)
}

fn build_gmail_query(query: &ListQuery) -> Result<String, AppError> {
    let since = local_timestamp_to_epoch_seconds(&query.since)?;
    let mut filters = vec![format!("after:{}", since.saturating_sub(1))];
    if let Some(until) = &query.until {
        let until = local_timestamp_to_epoch_seconds(until)?;
        filters.push(format!("before:{until}"));
    }
    Ok(filters.join(" "))
}

fn parse_headers(headers: &[JsonValue]) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for header in headers {
        let Some(name) = header.get("name").and_then(JsonValue::as_str) else {
            continue;
        };
        let Some(value) = header.get("value").and_then(JsonValue::as_str) else {
            continue;
        };
        values.insert(name.to_lowercase(), value.to_string());
    }
    values
}

fn required_str<'a>(value: &'a JsonValue, key: &str) -> Result<&'a str, AppError> {
    value
        .get(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| AppError::query(format!("Gmail response is missing field: {key}")))
}

fn display_label_name(value: &str) -> String {
    if value.chars().all(|ch| ch.is_ascii_uppercase() || ch == '_') {
        value.to_lowercase()
    } else {
        value.to_string()
    }
}

fn decode_base64_url(input: &str) -> Result<Vec<u8>, AppError> {
    let mut normalized = input.replace('-', "+").replace('_', "/");
    while normalized.len() % 4 != 0 {
        normalized.push('=');
    }

    let mut output = Vec::new();
    let bytes = normalized.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let a = decode_base64_digit(bytes[index])?;
        let b = decode_base64_digit(bytes[index + 1])?;
        let c = decode_base64_digit(bytes[index + 2])?;
        let d = decode_base64_digit(bytes[index + 3])?;

        output.push((a << 2) | (b >> 4));
        if bytes[index + 2] != b'=' {
            output.push((b << 4) | (c >> 2));
        }
        if bytes[index + 3] != b'=' {
            output.push((c << 6) | d);
        }

        index += 4;
    }

    Ok(output)
}

fn decode_base64_digit(value: u8) -> Result<u8, AppError> {
    match value {
        b'A'..=b'Z' => Ok(value - b'A'),
        b'a'..=b'z' => Ok(26 + value - b'a'),
        b'0'..=b'9' => Ok(52 + value - b'0'),
        b'+' => Ok(62),
        b'/' => Ok(63),
        b'=' => Ok(0),
        _ => Err(AppError::query("failed to decode Gmail message body")),
    }
}

fn percent_encode(input: &str) -> String {
    let mut output = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char)
            }
            _ => output.push_str(&format!("%{:02X}", byte)),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_label_name_normalizes_gmail_system_labels() {
        assert_eq!(display_label_name("UNREAD"), "unread");
        assert_eq!(display_label_name("Project X"), "Project X");
    }

    #[test]
    fn base64_url_decoder_handles_unpadded_input() {
        let decoded = decode_base64_url("SGVsbG8").unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "Hello");
    }
}
