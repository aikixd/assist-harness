use crate::cli::{PeersArgs, PeersRevokeArgs};
use crate::config::{load_peers, resolve_bot, revoke_peer};
use crate::domain::PeerStatus;
use crate::error::AppError;
use crate::output::{join_blocks, json_string};

pub fn run(args: PeersArgs) -> Result<String, AppError> {
    let bot = resolve_bot(args.bot.as_deref())?;
    let mut peers = load_peers(&bot.alias)?;
    if !args.all {
        peers.retain(|peer| peer.status == PeerStatus::Trusted);
    }

    if peers.is_empty() {
        return Ok("no peers configured".to_string());
    }

    if args.json {
        return Ok(format_json(&peers));
    }

    let blocks = peers
        .iter()
        .map(|peer| {
            let mut lines = vec![
                format!("peer: {}", peer.alias),
                format!("status: {}", peer.status.as_str()),
                format!("bot: {}", peer.bot_alias),
                format!("chat_id: {}", peer.chat_id),
                format!("user_id: {}", peer.user_id),
            ];
            if let Some(username) = &peer.username {
                lines.push(format!("username: {username}"));
            }
            if let Some(display_name) = &peer.display_name {
                lines.push(format!("display_name: {display_name}"));
            }
            lines.push(format!("paired_at: {}", peer.paired_at));
            lines.join("\n")
        })
        .collect::<Vec<_>>();

    Ok(join_blocks(&blocks))
}

pub fn revoke(args: PeersRevokeArgs) -> Result<String, AppError> {
    let bot = resolve_bot(args.bot.as_deref())?;
    revoke_peer(&bot.alias, &args.alias)?;
    Ok(format!("peer revoked: {}\nbot: {}", args.alias, bot.alias))
}

fn format_json(peers: &[crate::domain::PeerRecord]) -> String {
    let items = peers
        .iter()
        .map(|peer| {
            format!(
                concat!(
                    "{{",
                    "\"alias\":{},",
                    "\"status\":{},",
                    "\"bot\":{},",
                    "\"chat_id\":{},",
                    "\"user_id\":{},",
                    "\"username\":{},",
                    "\"display_name\":{},",
                    "\"paired_at\":{}",
                    "}}"
                ),
                json_string(&peer.alias),
                json_string(peer.status.as_str()),
                json_string(&peer.bot_alias),
                peer.chat_id,
                peer.user_id,
                peer.username
                    .as_ref()
                    .map(|value| json_string(value))
                    .unwrap_or_else(|| "null".to_string()),
                peer.display_name
                    .as_ref()
                    .map(|value| json_string(value))
                    .unwrap_or_else(|| "null".to_string()),
                json_string(&peer.paired_at),
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("{{\"peers\":[{}]}}", items)
}
