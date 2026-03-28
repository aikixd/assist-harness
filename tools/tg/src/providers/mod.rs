use std::process::Command;

use crate::config::BotConfig;
use crate::error::AppError;
use crate::json::{parse as parse_json, JsonValue};
use crate::time::epoch_seconds_to_local_timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramBotIdentity {
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramUpdate {
    pub update_id: u64,
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramMessage {
    pub message_id: u64,
    pub date: String,
    pub chat_id: i64,
    pub chat_type: String,
    pub user_id: i64,
    pub from_name: String,
    pub username: Option<String>,
    pub text: Option<String>,
    pub content_type: Option<String>,
}

pub fn get_me(bot: &BotConfig) -> Result<TelegramBotIdentity, AppError> {
    let root = api_get(bot, "getMe", &[])?;
    let result = root
        .get("result")
        .ok_or_else(|| AppError::query("Telegram getMe response is missing result"))?;
    Ok(TelegramBotIdentity {
        username: result
            .get("username")
            .and_then(JsonValue::as_str)
            .map(|value| value.to_string()),
    })
}

pub fn get_updates(
    bot: &BotConfig,
    offset: u64,
    limit: usize,
) -> Result<Vec<TelegramUpdate>, AppError> {
    let root = api_get(
        bot,
        "getUpdates",
        &[
            ("offset".to_string(), offset.to_string()),
            ("limit".to_string(), limit.to_string()),
        ],
    )?;
    let items = root
        .get("result")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| AppError::query("Telegram getUpdates response is missing result"))?;

    let mut updates = Vec::new();
    for item in items {
        let update_id = item
            .get("update_id")
            .and_then(JsonValue::as_u64)
            .ok_or_else(|| AppError::query("Telegram update is missing update_id"))?;
        let message = item.get("message").map(parse_message).transpose()?;
        updates.push(TelegramUpdate { update_id, message });
    }

    Ok(updates)
}

pub fn send_message(bot: &BotConfig, chat_id: i64, text: &str) -> Result<u64, AppError> {
    let root = api_post(
        bot,
        "sendMessage",
        &[
            ("chat_id".to_string(), chat_id.to_string()),
            ("text".to_string(), text.to_string()),
        ],
    )?;
    root.get("result")
        .and_then(|value| value.get("message_id"))
        .and_then(JsonValue::as_u64)
        .ok_or_else(|| AppError::query("Telegram sendMessage response is missing message_id"))
}

fn parse_message(value: &JsonValue) -> Result<TelegramMessage, AppError> {
    let message_id = value
        .get("message_id")
        .and_then(JsonValue::as_u64)
        .ok_or_else(|| AppError::query("Telegram message is missing message_id"))?;
    let date = value
        .get("date")
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| AppError::query("Telegram message is missing date"))?;
    let chat = value
        .get("chat")
        .ok_or_else(|| AppError::query("Telegram message is missing chat"))?;
    let from = value
        .get("from")
        .ok_or_else(|| AppError::query("Telegram message is missing from"))?;

    let chat_id = chat
        .get("id")
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| AppError::query("Telegram chat is missing id"))?;
    let chat_type = chat
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| AppError::query("Telegram chat is missing type"))?
        .to_string();
    let user_id = from
        .get("id")
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| AppError::query("Telegram from is missing id"))?;
    let username = from
        .get("username")
        .and_then(JsonValue::as_str)
        .map(|value| value.to_string());

    let first_name = from
        .get("first_name")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let last_name = from
        .get("last_name")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let from_name = [first_name, last_name]
        .iter()
        .filter(|part| !part.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");

    Ok(TelegramMessage {
        message_id,
        date: epoch_seconds_to_local_timestamp(date)?,
        chat_id,
        chat_type,
        user_id,
        from_name: if from_name.is_empty() {
            username.clone().unwrap_or_else(|| user_id.to_string())
        } else {
            from_name
        },
        username,
        text: value
            .get("text")
            .and_then(JsonValue::as_str)
            .map(|text| text.to_string()),
        content_type: infer_content_type(value),
    })
}

fn infer_content_type(value: &JsonValue) -> Option<String> {
    if value.get("text").is_some() {
        return None;
    }

    for key in [
        "photo",
        "sticker",
        "voice",
        "video",
        "video_note",
        "document",
        "audio",
        "animation",
        "contact",
        "location",
        "poll",
    ] {
        if value.get(key).is_some() {
            return Some(key.to_string());
        }
    }

    Some("non_text_message".to_string())
}

fn api_get(
    bot: &BotConfig,
    method: &str,
    params: &[(String, String)],
) -> Result<JsonValue, AppError> {
    let mut url = endpoint(bot, method);
    if !params.is_empty() {
        let query = params
            .iter()
            .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
            .collect::<Vec<_>>()
            .join("&");
        url.push('?');
        url.push_str(&query);
    }
    parse_api_response(run_curl_get(&url)?)
}

fn api_post(
    bot: &BotConfig,
    method: &str,
    params: &[(String, String)],
) -> Result<JsonValue, AppError> {
    parse_api_response(run_curl_post(&endpoint(bot, method), params)?)
}

fn endpoint(bot: &BotConfig, method: &str) -> String {
    format!("https://api.telegram.org/bot{}/{}", bot.token, method)
}

fn run_curl_get(url: &str) -> Result<String, AppError> {
    let output = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("-w")
        .arg("\n%{http_code}")
        .arg(url)
        .output()
        .map_err(|error| AppError::config(format!("failed to run curl: {error}")))?;

    parse_http_output(output.stdout, output.stderr)
}

fn run_curl_post(url: &str, params: &[(String, String)]) -> Result<String, AppError> {
    let body = params
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&");

    let output = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("-X")
        .arg("POST")
        .arg("-H")
        .arg("Content-Type: application/x-www-form-urlencoded")
        .arg("--data")
        .arg(body)
        .arg("-w")
        .arg("\n%{http_code}")
        .arg(url)
        .output()
        .map_err(|error| AppError::config(format!("failed to run curl: {error}")))?;

    parse_http_output(output.stdout, output.stderr)
}

fn parse_http_output(stdout: Vec<u8>, stderr: Vec<u8>) -> Result<String, AppError> {
    let stdout = String::from_utf8(stdout)
        .map_err(|error| AppError::config(format!("curl output was not valid UTF-8: {error}")))?;
    let (body, status) = stdout
        .rsplit_once('\n')
        .ok_or_else(|| AppError::config("failed to parse Telegram API status code"))?;
    let status = status
        .trim()
        .parse::<u16>()
        .map_err(|_| AppError::config("failed to parse Telegram API status code"))?;
    let stderr = String::from_utf8_lossy(&stderr).trim().to_string();

    match status {
        200 => Ok(body.to_string()),
        code => {
            let message = if stderr.is_empty() {
                format!("Telegram API request failed with status {code}")
            } else {
                format!("Telegram API request failed with status {code}: {stderr}")
            };
            Err(AppError::query(message))
        }
    }
}

fn parse_api_response(body: String) -> Result<JsonValue, AppError> {
    let root = parse_json(&body).map_err(|error| {
        AppError::query(format!("failed to parse Telegram API response: {error}"))
    })?;
    let ok = root
        .get("ok")
        .and_then(JsonValue::as_bool)
        .ok_or_else(|| AppError::query("Telegram API response is missing ok"))?;
    if ok {
        Ok(root)
    } else {
        let description = root
            .get("description")
            .and_then(JsonValue::as_str)
            .unwrap_or("Telegram API request failed");
        Err(AppError::query(description))
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
    use crate::json::parse as parse_json;

    #[test]
    fn infer_content_type_prefers_text_messages() {
        let value = parse_json("{\"text\":\"hello\",\"sticker\":{}}").expect("json");
        assert_eq!(infer_content_type(&value), None);
    }

    #[test]
    fn parse_api_response_returns_description_on_error() {
        let result = parse_api_response(
            "{\"ok\":false,\"description\":\"Bad Request: invalid\"}".to_string(),
        );
        assert_eq!(result.unwrap_err().to_string(), "Bad Request: invalid");
    }
}
