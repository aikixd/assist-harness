use crate::cli::PendingArgs;
use crate::commands::inbox::{scan_pending, PendingScanOptions};
use crate::config::{find_peer, load_cursor, load_peers, resolve_bot};
use crate::domain::PeerStatus;
use crate::error::AppError;

pub fn run(args: PendingArgs) -> Result<String, AppError> {
    let bot = resolve_bot(args.bot.as_deref())?;
    let peers = load_peers(&bot.alias)?;
    let trusted = peers
        .into_iter()
        .filter(|peer| peer.status == PeerStatus::Trusted)
        .collect::<Vec<_>>();
    if trusted.is_empty() {
        return Ok("0".to_string());
    }

    if let Some(alias) = args.peer.as_deref() {
        let peer = find_peer(&trusted, alias)
            .ok_or_else(|| AppError::query(format!("peer {alias} is not configured")))?;
        if peer.status != PeerStatus::Trusted {
            return Err(AppError::query(format!("peer {alias} is not trusted")));
        }
    }

    let result = scan_pending(
        &bot,
        &trusted,
        load_cursor(&bot.alias)?,
        &PendingScanOptions {
            peer_alias: args.peer.as_deref(),
            limit: None,
            collect_messages: false,
        },
    )?;

    Ok(result.matched_count.to_string())
}
