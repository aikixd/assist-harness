use crate::config::load_bots;
use crate::domain::BotStatus;
use crate::error::AppError;
use crate::output::join_blocks;
use crate::providers::get_me;

pub fn run() -> Result<String, AppError> {
    let bots = load_bots()?;
    if bots.is_empty() {
        return Ok("no bots configured".to_string());
    }

    let mut blocks = Vec::new();
    for bot in bots {
        let peer_count = crate::config::load_peers(&bot.alias)?
            .into_iter()
            .filter(|peer| peer.status.as_str() == "trusted")
            .count();
        let (status, detail) = match get_me(&bot) {
            Ok(identity) => (
                BotStatus {
                    alias: bot.alias.clone(),
                    username: identity.username,
                    status: "ready".to_string(),
                    trusted_peers: peer_count,
                },
                None,
            ),
            Err(error) => {
                let detail = error.to_string();
                let status = if detail.to_lowercase().contains("unauthorized") {
                    "invalid_token"
                } else {
                    "api_error"
                };
                (
                    BotStatus {
                        alias: bot.alias.clone(),
                        username: None,
                        status: status.to_string(),
                        trusted_peers: peer_count,
                    },
                    Some(detail),
                )
            }
        };

        let mut lines = vec![
            format!("bot: {}", status.alias),
            format!("status: {}", status.status),
            format!("trusted_peers: {}", status.trusted_peers),
        ];
        if let Some(username) = status.username {
            lines.insert(1, format!("username: {username}"));
        }
        if let Some(detail) = detail {
            lines.push(format!("detail: {detail}"));
        }
        blocks.push(lines.join("\n"));
    }

    Ok(join_blocks(&blocks))
}
