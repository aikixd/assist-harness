use crate::cli::AuthArgs;
use crate::config::{
    load_cursor, load_peers, remove_peer, resolve_bot, store_cursor, store_peer, validate_alias,
    validate_auth_key,
};
use crate::domain::{PeerRecord, PeerStatus};
use crate::error::AppError;
use crate::interactive::{prompt, read_stdin};
use crate::providers::get_updates;
use crate::time::current_local_timestamp;

const BATCH_LIMIT: usize = 100;

pub fn run(args: AuthArgs) -> Result<String, AppError> {
    validate_alias(&args.alias)?;
    let bot = resolve_bot(args.bot.as_deref())?;
    let raw_key = if args.stdin {
        read_stdin()?
    } else {
        prompt("auth key")?
    };
    let auth_key = validate_auth_key(&raw_key)?;
    let peers = load_peers(&bot.alias)?;
    let start_offset = load_cursor(&bot.alias)?;

    let mut offset = start_offset;
    let mut highest_seen: Option<u64> = None;
    let mut matches = Vec::new();

    loop {
        let updates = get_updates(&bot, offset, BATCH_LIMIT)?;
        if updates.is_empty() {
            break;
        }

        for update in &updates {
            highest_seen = Some(highest_seen.map_or(update.update_id, |current: u64| {
                current.max(update.update_id)
            }));

            let Some(message) = &update.message else {
                continue;
            };
            if message.chat_type != "private" {
                continue;
            }
            let Some(text) = &message.text else {
                continue;
            };
            if text.trim() == auth_key {
                matches.push((update.update_id, message.clone()));
            }
        }

        if updates.len() < BATCH_LIMIT {
            break;
        }

        offset = updates
            .last()
            .map(|update| update.update_id + 1)
            .unwrap_or(offset);
    }

    match matches.len() {
        0 => Err(AppError::query(
            "auth key not found in pending updates; resend the key in Telegram and try again",
        )),
        1 => {
            let (pairing_update_id, message) = matches.into_iter().next().expect("single match");

            if let Some(existing_alias) = peers
                .iter()
                .find(|peer| peer.alias == args.alias && peer.chat_id != message.chat_id)
                .map(|peer| peer.alias.clone())
            {
                return Err(AppError::query(format!(
                    "peer alias {} is already bound to a different chat",
                    existing_alias
                )));
            }

            let existing = peers.iter().find(|peer| peer.chat_id == message.chat_id);
            let peer = PeerRecord {
                bot_alias: bot.alias.clone(),
                alias: args.alias.clone(),
                status: PeerStatus::Trusted,
                chat_id: message.chat_id,
                user_id: message.user_id,
                username: message.username.clone(),
                display_name: Some(message.from_name.clone()),
                paired_at: current_local_timestamp()?,
                pairing_update_id,
            };
            store_peer(&peer)?;
            if let Some(existing) = existing {
                if existing.alias != peer.alias {
                    remove_peer(&bot.alias, &existing.alias)?;
                }
            }

            if let Some(highest_seen) = highest_seen {
                store_cursor(&bot.alias, highest_seen + 1)?;
            }

            let result = if existing.is_some() {
                format!(
                    "peer refreshed: {}\nbot: {}\nchat_id: {}",
                    peer.alias, peer.bot_alias, peer.chat_id
                )
            } else {
                format!(
                    "peer trusted: {}\nbot: {}\nchat_id: {}",
                    peer.alias, peer.bot_alias, peer.chat_id
                )
            };

            Ok(result)
        }
        _ => Err(AppError::query(
            "auth key matched multiple pending updates; send a fresh key and try again",
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::PeerStatus;

    use super::*;

    #[test]
    fn refreshed_peer_keeps_trusted_status() {
        let peer = PeerRecord {
            bot_alias: "main".to_string(),
            alias: "owner".to_string(),
            status: PeerStatus::Trusted,
            chat_id: 1,
            user_id: 2,
            username: None,
            display_name: None,
            paired_at: "2026-03-28T10:00".to_string(),
            pairing_update_id: 9,
        };
        assert_eq!(peer.status, PeerStatus::Trusted);
    }
}
