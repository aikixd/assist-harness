use crate::cli::SendArgs;
use crate::config::{find_peer, load_peers, resolve_bot};
use crate::domain::PeerStatus;
use crate::error::AppError;
use crate::interactive::read_stdin;
use crate::output::json_string;
use crate::providers::send_message;

pub fn run(args: SendArgs) -> Result<String, AppError> {
    let bot = resolve_bot(args.bot.as_deref())?;
    let peers = load_peers(&bot.alias)?;
    let trusted = peers
        .into_iter()
        .filter(|peer| peer.status == PeerStatus::Trusted)
        .collect::<Vec<_>>();
    if trusted.is_empty() {
        return Err(AppError::query("no trusted peers configured"));
    }

    let peer = match args.peer.as_deref() {
        Some(alias) => find_peer(&trusted, alias)
            .cloned()
            .ok_or_else(|| AppError::query(format!("peer {alias} is not configured")))?,
        None if trusted.len() == 1 => trusted[0].clone(),
        None => {
            return Err(AppError::query(
                "multiple trusted peers configured; provide --peer <alias>",
            ))
        }
    };

    let text = match (args.text, args.stdin) {
        (Some(text), false) => text,
        (None, true) => read_stdin()?,
        _ => {
            return Err(AppError::usage(
                "pa-tg send requires exactly one of --text <text> or --stdin",
            ))
        }
    };

    if text.trim().is_empty() {
        return Err(AppError::usage("message text cannot be empty"));
    }

    let message_id = send_message(&bot, peer.chat_id, &text)?;

    if args.json {
        return Ok(format!(
            "{{\"bot\":{},\"peer\":{},\"chat_id\":{},\"message_id\":{}}}",
            json_string(&bot.alias),
            json_string(&peer.alias),
            peer.chat_id,
            message_id,
        ));
    }

    Ok(format!(
        "bot: {}\npeer: {}\nchat_id: {}\nmessage_id: {}",
        bot.alias, peer.alias, peer.chat_id, message_id
    ))
}
