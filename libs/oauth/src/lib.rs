use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenStatus {
    Missing,
    Present,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackConfig {
    pub state: String,
    pub redirect_uri: String,
}

#[derive(Debug)]
pub struct LoopbackListener {
    listener: TcpListener,
    pub config: LoopbackConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackResult {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthClientConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
    pub raw_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthError {
    MissingHome,
    Io(String),
    ListenerBind(String),
    ListenerAcceptTimeout,
    CallbackMalformed,
    StateMismatch,
    MissingCode,
    MissingEnv(String),
    ProcessFailure(String),
    TokenParseFailed,
}

impl Display for OAuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHome => write!(f, "HOME is not set"),
            Self::Io(message) => write!(f, "{message}"),
            Self::ListenerBind(message) => write!(f, "{message}"),
            Self::ListenerAcceptTimeout => write!(f, "timed out waiting for OAuth callback"),
            Self::CallbackMalformed => write!(f, "received malformed OAuth callback"),
            Self::StateMismatch => write!(f, "received OAuth callback with unexpected state"),
            Self::MissingCode => write!(f, "received OAuth callback without authorization code"),
            Self::MissingEnv(name) => write!(f, "required environment variable is missing: {name}"),
            Self::ProcessFailure(message) => write!(f, "{message}"),
            Self::TokenParseFailed => write!(f, "failed to parse OAuth token response"),
        }
    }
}

impl Error for OAuthError {}

pub fn tool_paths(tool: &str) -> Result<ToolPaths, OAuthError> {
    let home = env::var_os("HOME").ok_or(OAuthError::MissingHome)?;
    let home = PathBuf::from(home);

    Ok(ToolPaths {
        config_dir: home.join(".config").join("pa").join(tool),
        data_dir: home.join(".local").join("share").join("pa").join(tool),
        cache_dir: home.join(".cache").join("pa").join(tool),
    })
}

pub fn account_token_path(tool: &str, account_email: &str) -> Result<PathBuf, OAuthError> {
    let paths = tool_paths(tool)?;
    Ok(paths
        .data_dir
        .join("tokens")
        .join(format!("{}.token", sanitize_filename(account_email))))
}

pub fn token_status(tool: &str, account_email: &str) -> Result<TokenStatus, OAuthError> {
    let token_path = account_token_path(tool, account_email)?;

    if token_path.exists() {
        Ok(TokenStatus::Present)
    } else {
        Ok(TokenStatus::Missing)
    }
}

pub fn load_client_config(
    client_id_env: &str,
    client_secret_env: &str,
) -> Result<OAuthClientConfig, OAuthError> {
    let client_id =
        env::var(client_id_env).map_err(|_| OAuthError::MissingEnv(client_id_env.to_string()))?;
    let client_secret = env::var(client_secret_env)
        .map_err(|_| OAuthError::MissingEnv(client_secret_env.to_string()))?;

    Ok(OAuthClientConfig {
        client_id,
        client_secret,
    })
}

pub fn store_token(
    tool: &str,
    account_email: &str,
    token_json: &str,
) -> Result<PathBuf, OAuthError> {
    let token_path = account_token_path(tool, account_email)?;
    if let Some(parent) = token_path.parent() {
        fs::create_dir_all(parent).map_err(|error| OAuthError::Io(error.to_string()))?;
    }
    fs::write(&token_path, token_json).map_err(|error| OAuthError::Io(error.to_string()))?;
    Ok(token_path)
}

pub fn start_loopback_listener() -> Result<LoopbackListener, OAuthError> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| {
        OAuthError::ListenerBind(format!("failed to bind loopback listener: {error}"))
    })?;
    let address = listener.local_addr().map_err(|error| {
        OAuthError::ListenerBind(format!("failed to resolve listener address: {error}"))
    })?;
    let state = generate_state()?;

    Ok(LoopbackListener {
        listener,
        config: LoopbackConfig {
            state,
            redirect_uri: format!("http://127.0.0.1:{}/oauth/callback", address.port()),
        },
    })
}

impl LoopbackListener {
    pub fn wait_for_callback(self, timeout: Duration) -> Result<CallbackResult, OAuthError> {
        self.listener
            .set_nonblocking(true)
            .map_err(|error| OAuthError::Io(error.to_string()))?;

        let start = Instant::now();
        loop {
            match self.listener.accept() {
                Ok((mut stream, _addr)) => {
                    let request_line = read_request_line(&mut stream)?;
                    let result = parse_callback_from_request_line(&request_line)?;
                    let body = "<html><body><h1>Authorization received</h1><p>You can close this tab and return to the terminal.</p></body></html>";
                    write_http_response(&mut stream, 200, "OK", body)?;

                    if result.state != self.config.state {
                        return Err(OAuthError::StateMismatch);
                    }

                    return Ok(result);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() >= timeout {
                        return Err(OAuthError::ListenerAcceptTimeout);
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(error) => return Err(OAuthError::Io(error.to_string())),
            }
        }
    }
}

pub fn exchange_code_with_curl(
    token_endpoint: &str,
    client: &OAuthClientConfig,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, OAuthError> {
    let body = format!(
        "code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
        percent_encode(code),
        percent_encode(&client.client_id),
        percent_encode(&client.client_secret),
        percent_encode(redirect_uri),
    );

    let output = std::process::Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--fail")
        .arg("-X")
        .arg("POST")
        .arg(token_endpoint)
        .arg("-H")
        .arg("Content-Type: application/x-www-form-urlencoded")
        .arg("--data")
        .arg(body)
        .output()
        .map_err(|error| OAuthError::ProcessFailure(format!("failed to run curl: {error}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "curl exited with a non-zero status during token exchange".to_string()
        } else {
            format!("token exchange failed: {stderr}")
        };
        return Err(OAuthError::ProcessFailure(message));
    }

    let raw_json = String::from_utf8(output.stdout).map_err(|error| {
        OAuthError::ProcessFailure(format!("token response was not valid UTF-8: {error}"))
    })?;

    parse_token_response(&raw_json)
}

fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn generate_state() -> Result<String, OAuthError> {
    let mut buffer = [0u8; 16];
    if let Ok(mut file) = fs::File::open("/dev/urandom") {
        file.read_exact(&mut buffer)
            .map_err(|error| OAuthError::Io(error.to_string()))?;
        return Ok(hex_encode(&buffer));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| OAuthError::Io(error.to_string()))?;
    Ok(format!("{:x}{:x}", now.as_secs(), now.subsec_nanos()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(nibble_to_hex(byte >> 4));
        output.push(nibble_to_hex(byte & 0x0f));
    }
    output
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => unreachable!(),
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

fn read_request_line(stream: &mut TcpStream) -> Result<String, OAuthError> {
    let mut buffer = [0u8; 4096];
    let read = stream
        .read(&mut buffer)
        .map_err(|error| OAuthError::Io(error.to_string()))?;
    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
    let line = request
        .lines()
        .next()
        .ok_or(OAuthError::CallbackMalformed)?;
    Ok(line.to_string())
}

fn parse_callback_from_request_line(request_line: &str) -> Result<CallbackResult, OAuthError> {
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(OAuthError::CallbackMalformed)?;
    let target = parts.next().ok_or(OAuthError::CallbackMalformed)?;

    if method != "GET" {
        return Err(OAuthError::CallbackMalformed);
    }

    let (_, query) = target
        .split_once('?')
        .ok_or(OAuthError::CallbackMalformed)?;
    let params = parse_query_string(query);

    let state = params
        .get("state")
        .cloned()
        .ok_or(OAuthError::StateMismatch)?;
    let code = params.get("code").cloned().ok_or(OAuthError::MissingCode)?;

    Ok(CallbackResult { code, state })
}

fn parse_query_string(input: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for pair in input.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        values.insert(percent_decode(key), percent_decode(value));
    }
    values
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut index = 0;
    let mut output = Vec::with_capacity(bytes.len());

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let upper = decode_hex_digit(bytes[index + 1]);
                let lower = decode_hex_digit(bytes[index + 2]);
                if let (Some(upper), Some(lower)) = (upper, lower) {
                    output.push((upper << 4) | lower);
                    index += 3;
                } else {
                    output.push(bytes[index]);
                    index += 1;
                }
            }
            other => {
                output.push(other);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&output).to_string()
}

fn decode_hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(10 + value - b'a'),
        b'A'..=b'F' => Some(10 + value - b'A'),
        _ => None,
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status_code: u16,
    reason: &str,
    body: &str,
) -> Result<(), OAuthError> {
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status_code,
        reason,
        body.len(),
        body,
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|error| OAuthError::Io(error.to_string()))
}

fn parse_token_response(raw_json: &str) -> Result<TokenResponse, OAuthError> {
    let access_token =
        extract_json_string(raw_json, "access_token").ok_or(OAuthError::TokenParseFailed)?;
    let refresh_token = extract_json_string(raw_json, "refresh_token");
    let scope = extract_json_string(raw_json, "scope");
    let token_type = extract_json_string(raw_json, "token_type");
    let expires_in = extract_json_u64(raw_json, "expires_in");

    Ok(TokenResponse {
        access_token,
        refresh_token,
        scope,
        token_type,
        expires_in,
        raw_json: raw_json.to_string(),
    })
}

fn extract_json_string(raw_json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\"");
    let start = raw_json.find(&pattern)?;
    let rest = &raw_json[start + pattern.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;

    let mut escaped = false;
    let mut output = String::new();
    for ch in rest.chars() {
        if escaped {
            output.push(match ch {
                '"' => '"',
                '\\' => '\\',
                '/' => '/',
                'b' => '\u{0008}',
                'f' => '\u{000C}',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => return Some(output),
            other => output.push(other),
        }
    }

    None
}

fn extract_json_u64(raw_json: &str, key: &str) -> Option<u64> {
    let pattern = format!("\"{key}\"");
    let start = raw_json.find(&pattern)?;
    let rest = &raw_json[start + pattern.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let digits = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_paths_follow_repo_convention() {
        let paths = tool_paths("mail");
        assert!(paths.is_ok());

        let paths = paths.unwrap();
        let config = paths.config_dir.to_string_lossy();
        let data = paths.data_dir.to_string_lossy();
        let cache = paths.cache_dir.to_string_lossy();

        assert!(config.contains(".config/pa/mail"));
        assert!(data.contains(".local/share/pa/mail"));
        assert!(cache.contains(".cache/pa/mail"));
    }

    #[test]
    fn token_file_name_is_sanitized() {
        let token_path = account_token_path("mail", "me+label@example.com");
        assert!(token_path.is_ok());

        let token_path = token_path.unwrap();
        let token_path = token_path.to_string_lossy();
        assert!(token_path.ends_with("me_label_example_com.token"));
    }

    #[test]
    fn percent_encode_encodes_reserved_characters() {
        assert_eq!(percent_encode("hello world+mail"), "hello%20world%2Bmail");
    }

    #[test]
    fn callback_parser_extracts_code_and_state() {
        let result = parse_callback_from_request_line(
            "GET /oauth/callback?code=abc123&state=test-state HTTP/1.1",
        );
        assert_eq!(
            result,
            Ok(CallbackResult {
                code: "abc123".to_string(),
                state: "test-state".to_string(),
            })
        );
    }

    #[test]
    fn token_parser_extracts_expected_fields() {
        let raw = r#"{"access_token":"at","refresh_token":"rt","scope":"s","token_type":"Bearer","expires_in":3600}"#;
        let parsed = parse_token_response(raw);
        assert!(parsed.is_ok());

        let parsed = parsed.unwrap();
        assert_eq!(parsed.access_token, "at");
        assert_eq!(parsed.refresh_token.as_deref(), Some("rt"));
        assert_eq!(parsed.expires_in, Some(3600));
    }
}
