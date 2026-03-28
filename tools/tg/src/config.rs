use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use oauth::{tool_paths, ToolPaths};

use crate::domain::{PeerRecord, PeerStatus};
use crate::error::AppError;

const TOOL_NAME: &str = "tg";
const BOTS_DIR: &str = "bots";
const PEERS_DIR: &str = "peers";
const CURSORS_DIR: &str = "cursors";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotConfig {
    pub alias: String,
    pub token: String,
}

impl Display for PeerStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn local_paths() -> Result<ToolPaths, AppError> {
    tool_paths(TOOL_NAME).map_err(|error| {
        AppError::config(format!("failed to resolve local storage paths: {error}"))
    })
}

pub fn ensure_local_storage() -> Result<ToolPaths, AppError> {
    let paths = local_paths()?;
    ensure_dir(&paths.config_dir)?;
    ensure_dir(&paths.config_dir.join(BOTS_DIR))?;
    ensure_dir(&paths.config_dir.join(PEERS_DIR))?;
    ensure_dir(&paths.data_dir)?;
    ensure_dir(&paths.data_dir.join(CURSORS_DIR))?;
    ensure_dir(&paths.cache_dir)?;
    Ok(paths)
}

pub fn validate_alias(alias: &str) -> Result<(), AppError> {
    if alias.trim().is_empty() {
        return Err(AppError::usage("alias cannot be empty"));
    }
    if !alias
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(AppError::usage(
            "alias must contain only ASCII letters, numbers, '-' or '_'",
        ));
    }
    Ok(())
}

pub fn validate_auth_key(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim().to_string();
    let bytes = trimmed.as_bytes();
    if bytes.len() != 36 {
        return Err(AppError::usage(
            "auth key must be a lowercase GUID in canonical 8-4-4-4-12 form",
        ));
    }

    for (index, byte) in bytes.iter().enumerate() {
        let is_dash = matches!(index, 8 | 13 | 18 | 23);
        if is_dash {
            if *byte != b'-' {
                return Err(AppError::usage(
                    "auth key must be a lowercase GUID in canonical 8-4-4-4-12 form",
                ));
            }
            continue;
        }

        let is_hex = matches!(byte, b'0'..=b'9' | b'a'..=b'f');
        if !is_hex {
            return Err(AppError::usage(
                "auth key must be a lowercase GUID in canonical 8-4-4-4-12 form",
            ));
        }
    }

    Ok(trimmed)
}

pub fn store_bot(alias: &str, token: &str) -> Result<PathBuf, AppError> {
    validate_alias(alias)?;
    if token.trim().is_empty() {
        return Err(AppError::usage("bot token cannot be empty"));
    }

    let paths = ensure_local_storage()?;
    let path = paths
        .config_dir
        .join(BOTS_DIR)
        .join(format!("{}.conf", alias.trim()));
    if path.exists() {
        return Err(AppError::config(format!(
            "bot alias {} already exists",
            alias.trim()
        )));
    }

    write_private_file(
        &path,
        &format!("alias={}\ntoken={}\n", alias.trim(), token.trim()),
    )?;
    Ok(path)
}

pub fn load_bots() -> Result<Vec<BotConfig>, AppError> {
    let paths = ensure_local_storage()?;
    let dir = paths.config_dir.join(BOTS_DIR);
    let mut bots = Vec::new();

    if !dir.exists() {
        return Ok(bots);
    }

    for entry in fs::read_dir(&dir).map_err(|error| {
        AppError::config(format!(
            "failed to read bot directory {}: {error}",
            dir.display()
        ))
    })? {
        let entry = entry
            .map_err(|error| AppError::config(format!("failed to read bot entry: {error}")))?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("conf") {
            continue;
        }
        bots.push(load_bot_from_path(&path)?);
    }

    bots.sort_by(|left, right| left.alias.cmp(&right.alias));
    Ok(bots)
}

pub fn resolve_bot(requested: Option<&str>) -> Result<BotConfig, AppError> {
    let bots = load_bots()?;
    if bots.is_empty() {
        return Err(AppError::query("no bots configured"));
    }

    match requested {
        Some(alias) => bots
            .into_iter()
            .find(|bot| bot.alias == alias)
            .ok_or_else(|| AppError::query(format!("bot {alias} is not configured"))),
        None if bots.len() == 1 => Ok(bots[0].clone()),
        None => Err(AppError::query(
            "multiple bots configured; provide --bot <name>",
        )),
    }
}

pub fn load_peers(bot_alias: &str) -> Result<Vec<PeerRecord>, AppError> {
    let paths = ensure_local_storage()?;
    let dir = paths.config_dir.join(PEERS_DIR).join(bot_alias);
    let mut peers = Vec::new();

    if !dir.exists() {
        return Ok(peers);
    }

    for entry in fs::read_dir(&dir).map_err(|error| {
        AppError::config(format!(
            "failed to read peers directory {}: {error}",
            dir.display()
        ))
    })? {
        let entry = entry
            .map_err(|error| AppError::config(format!("failed to read peer entry: {error}")))?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("conf") {
            continue;
        }
        peers.push(load_peer_from_path(bot_alias, &path)?);
    }

    peers.sort_by(|left, right| left.alias.cmp(&right.alias));
    Ok(peers)
}

pub fn find_peer<'a>(peers: &'a [PeerRecord], alias: &str) -> Option<&'a PeerRecord> {
    peers.iter().find(|peer| peer.alias == alias)
}

pub fn store_peer(peer: &PeerRecord) -> Result<PathBuf, AppError> {
    validate_alias(&peer.alias)?;
    validate_alias(&peer.bot_alias)?;

    let paths = ensure_local_storage()?;
    let dir = paths.config_dir.join(PEERS_DIR).join(&peer.bot_alias);
    ensure_dir(&dir)?;
    let path = dir.join(format!("{}.conf", peer.alias));

    let mut lines = vec![
        format!("alias={}", peer.alias),
        format!("status={}", peer.status.as_str()),
        format!("chat_id={}", peer.chat_id),
        format!("user_id={}", peer.user_id),
        format!("paired_at={}", peer.paired_at),
        format!("pairing_update_id={}", peer.pairing_update_id),
    ];
    if let Some(username) = &peer.username {
        lines.push(format!("username={username}"));
    }
    if let Some(display_name) = &peer.display_name {
        lines.push(format!("display_name={display_name}"));
    }

    write_private_file(&path, &(lines.join("\n") + "\n"))?;
    Ok(path)
}

pub fn revoke_peer(bot_alias: &str, alias: &str) -> Result<PathBuf, AppError> {
    let peers = load_peers(bot_alias)?;
    let mut peer = find_peer(&peers, alias)
        .cloned()
        .ok_or_else(|| AppError::query(format!("peer {alias} is not configured")))?;
    peer.status = PeerStatus::Revoked;
    store_peer(&peer)
}

pub fn remove_peer(bot_alias: &str, alias: &str) -> Result<(), AppError> {
    let path = ensure_local_storage()?
        .config_dir
        .join(PEERS_DIR)
        .join(bot_alias)
        .join(format!("{}.conf", alias));
    if !path.exists() {
        return Err(AppError::query(format!("peer {alias} is not configured")));
    }

    fs::remove_file(&path).map_err(|error| {
        AppError::config(format!(
            "failed to remove peer file {}: {error}",
            path.display()
        ))
    })
}

pub fn load_cursor(bot_alias: &str) -> Result<u64, AppError> {
    let path = ensure_local_storage()?
        .data_dir
        .join(CURSORS_DIR)
        .join(format!("{}.txt", bot_alias));
    if !path.exists() {
        return Ok(0);
    }

    let contents = fs::read_to_string(&path).map_err(|error| {
        AppError::config(format!(
            "failed to read cursor file {}: {error}",
            path.display()
        ))
    })?;
    contents
        .trim()
        .parse::<u64>()
        .map_err(|_| AppError::config(format!("cursor file {} is malformed", path.display())))
}

pub fn store_cursor(bot_alias: &str, offset: u64) -> Result<PathBuf, AppError> {
    let path = ensure_local_storage()?
        .data_dir
        .join(CURSORS_DIR)
        .join(format!("{}.txt", bot_alias));
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(&path, format!("{offset}\n")).map_err(|error| {
        AppError::config(format!(
            "failed to write cursor file {}: {error}",
            path.display()
        ))
    })?;
    Ok(path)
}

fn load_bot_from_path(path: &Path) -> Result<BotConfig, AppError> {
    let contents = fs::read_to_string(path).map_err(|error| {
        AppError::config(format!(
            "failed to read bot config {}: {error}",
            path.display()
        ))
    })?;
    let values = parse_key_values(&contents);
    let alias = values
        .get("alias")
        .cloned()
        .ok_or_else(|| AppError::config(format!("bot config {} is malformed", path.display())))?;
    let token = values
        .get("token")
        .cloned()
        .ok_or_else(|| AppError::config(format!("bot config {} is malformed", path.display())))?;
    Ok(BotConfig { alias, token })
}

fn load_peer_from_path(bot_alias: &str, path: &Path) -> Result<PeerRecord, AppError> {
    let contents = fs::read_to_string(path).map_err(|error| {
        AppError::config(format!(
            "failed to read peer config {}: {error}",
            path.display()
        ))
    })?;
    let values = parse_key_values(&contents);

    let alias = values
        .get("alias")
        .cloned()
        .ok_or_else(|| AppError::config(format!("peer config {} is malformed", path.display())))?;
    let status = match values.get("status").map(String::as_str) {
        Some("trusted") => PeerStatus::Trusted,
        Some("revoked") => PeerStatus::Revoked,
        _ => {
            return Err(AppError::config(format!(
                "peer config {} has invalid status",
                path.display()
            )))
        }
    };

    Ok(PeerRecord {
        bot_alias: bot_alias.to_string(),
        alias,
        status,
        chat_id: parse_i64(&values, "chat_id", path)?,
        user_id: parse_i64(&values, "user_id", path)?,
        username: values.get("username").cloned(),
        display_name: values.get("display_name").cloned(),
        paired_at: values.get("paired_at").cloned().ok_or_else(|| {
            AppError::config(format!("peer config {} is malformed", path.display()))
        })?,
        pairing_update_id: parse_u64(&values, "pairing_update_id", path)?,
    })
}

fn parse_key_values(contents: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values.insert(key.trim().to_string(), value.trim().to_string());
    }

    values
}

fn parse_i64(values: &BTreeMap<String, String>, key: &str, path: &Path) -> Result<i64, AppError> {
    values
        .get(key)
        .ok_or_else(|| AppError::config(format!("peer config {} is malformed", path.display())))?
        .parse::<i64>()
        .map_err(|_| {
            AppError::config(format!(
                "peer config {} has invalid {}",
                path.display(),
                key
            ))
        })
}

fn parse_u64(values: &BTreeMap<String, String>, key: &str, path: &Path) -> Result<u64, AppError> {
    values
        .get(key)
        .ok_or_else(|| AppError::config(format!("peer config {} is malformed", path.display())))?
        .parse::<u64>()
        .map_err(|_| {
            AppError::config(format!(
                "peer config {} has invalid {}",
                path.display(),
                key
            ))
        })
}

fn ensure_dir(path: &Path) -> Result<(), AppError> {
    fs::create_dir_all(path).map_err(|error| {
        AppError::config(format!(
            "failed to create directory {}: {error}",
            path.display()
        ))
    })
}

fn write_private_file(path: &Path, contents: &str) -> Result<(), AppError> {
    fs::write(path, contents).map_err(|error| {
        AppError::config(format!("failed to write {}: {error}", path.display()))
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, permissions).map_err(|error| {
            AppError::config(format!(
                "failed to set permissions on {}: {error}",
                path.display()
            ))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_key_validation_trims_whitespace() {
        let key = validate_auth_key(" 12345678-1234-1234-1234-123456789abc \n");
        assert_eq!(key, Ok("12345678-1234-1234-1234-123456789abc".to_string()));
    }

    #[test]
    fn auth_key_validation_rejects_uppercase() {
        assert!(validate_auth_key("12345678-1234-1234-1234-123456789abC").is_err());
    }

    #[test]
    fn parse_key_values_ignores_comments() {
        let values = parse_key_values("# a\nalias=main\ntoken=test\n");
        assert_eq!(values.get("alias").map(String::as_str), Some("main"));
        assert_eq!(values.get("token").map(String::as_str), Some("test"));
    }
}
