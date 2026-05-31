use crate::config::BotConfig;
use crate::domain::{MessageKind, PeerRecord, RecvMessage};
use crate::error::AppError;
use crate::providers::{get_updates, TelegramMessage, TelegramUpdate};

const BATCH_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingScanOptions<'a> {
    pub peer_alias: Option<&'a str>,
    pub limit: Option<usize>,
    pub collect_messages: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingScanResult {
    pub matched_count: usize,
    pub messages: Vec<RecvMessage>,
    pub highest_seen_update_id: Option<u64>,
}

pub fn scan_pending(
    bot: &BotConfig,
    trusted: &[PeerRecord],
    start_offset: u64,
    options: &PendingScanOptions<'_>,
) -> Result<PendingScanResult, AppError> {
    scan_pending_with(trusted, start_offset, options, |offset, limit| {
        get_updates(bot, offset, limit)
    })
}

pub fn scan_pending_with<F>(
    trusted: &[PeerRecord],
    start_offset: u64,
    options: &PendingScanOptions<'_>,
    mut fetch_updates: F,
) -> Result<PendingScanResult, AppError>
where
    F: FnMut(u64, usize) -> Result<Vec<TelegramUpdate>, AppError>,
{
    let mut offset = start_offset;
    let mut highest_seen_update_id = None;
    let mut matched_count = 0usize;
    let mut messages = Vec::new();

    'outer: loop {
        let updates = fetch_updates(offset, BATCH_LIMIT)?;
        if updates.is_empty() {
            break;
        }

        for update in &updates {
            highest_seen_update_id = Some(
                highest_seen_update_id.map_or(update.update_id, |current: u64| {
                    current.max(update.update_id)
                }),
            );

            let Some(message) = &update.message else {
                continue;
            };
            if message.chat_type != "private" {
                continue;
            }

            let Some(peer) = trusted.iter().find(|peer| peer.chat_id == message.chat_id) else {
                continue;
            };
            if let Some(alias) = options.peer_alias {
                if peer.alias != alias {
                    continue;
                }
            }

            matched_count += 1;
            if options.collect_messages {
                messages.push(build_recv_message(update, message, peer));
            }

            if options.limit.is_some_and(|limit| matched_count >= limit) {
                break 'outer;
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

    Ok(PendingScanResult {
        matched_count,
        messages,
        highest_seen_update_id,
    })
}

fn build_recv_message(
    update: &TelegramUpdate,
    message: &TelegramMessage,
    peer: &PeerRecord,
) -> RecvMessage {
    let (kind, text, content_type) = match (&message.text, &message.content_type) {
        (Some(text), _) => (MessageKind::Text, text.clone(), None),
        (None, Some(content_type)) => (
            MessageKind::Unsupported,
            format!("unsupported trusted message type: {content_type}"),
            Some(content_type.clone()),
        ),
        (None, None) => (
            MessageKind::Unsupported,
            "unsupported trusted message type: non_text_message".to_string(),
            Some("non_text_message".to_string()),
        ),
    };

    RecvMessage {
        peer_alias: peer.alias.clone(),
        chat_id: peer.chat_id,
        update_id: update.update_id,
        message_id: Some(message.message_id),
        date: message.date.clone(),
        from: message.from_name.clone(),
        text,
        kind,
        content_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PeerStatus;

    fn peer(alias: &str, chat_id: i64) -> PeerRecord {
        PeerRecord {
            bot_alias: "main".to_string(),
            alias: alias.to_string(),
            status: PeerStatus::Trusted,
            chat_id,
            user_id: chat_id,
            username: None,
            display_name: None,
            paired_at: "2026-03-29T10:00".to_string(),
            pairing_update_id: 1,
        }
    }

    fn message(
        update_id: u64,
        chat_id: i64,
        text: Option<&str>,
        content_type: Option<&str>,
    ) -> TelegramUpdate {
        TelegramUpdate {
            update_id,
            message: Some(TelegramMessage {
                message_id: update_id + 100,
                date: "2026-03-29T10:10".to_string(),
                chat_id,
                chat_type: "private".to_string(),
                user_id: chat_id,
                from_name: format!("user-{chat_id}"),
                username: None,
                text: text.map(|value| value.to_string()),
                content_type: content_type.map(|value| value.to_string()),
            }),
        }
    }

    #[test]
    fn scan_counts_trusted_messages_without_collecting() {
        let trusted = vec![peer("owner", 1)];
        let result = scan_pending_with(
            &trusted,
            0,
            &PendingScanOptions {
                peer_alias: None,
                limit: None,
                collect_messages: false,
            },
            |_offset, _limit| Ok(vec![message(10, 1, Some("hello"), None)]),
        )
        .expect("scan");

        assert_eq!(result.matched_count, 1);
        assert!(result.messages.is_empty());
        assert_eq!(result.highest_seen_update_id, Some(10));
    }

    #[test]
    fn scan_ignores_untrusted_messages() {
        let trusted = vec![peer("owner", 1)];
        let result = scan_pending_with(
            &trusted,
            0,
            &PendingScanOptions {
                peer_alias: None,
                limit: None,
                collect_messages: false,
            },
            |_offset, _limit| Ok(vec![message(10, 2, Some("hello"), None)]),
        )
        .expect("scan");

        assert_eq!(result.matched_count, 0);
        assert_eq!(result.highest_seen_update_id, Some(10));
    }

    #[test]
    fn scan_collects_unsupported_trusted_messages() {
        let trusted = vec![peer("owner", 1)];
        let result = scan_pending_with(
            &trusted,
            0,
            &PendingScanOptions {
                peer_alias: None,
                limit: None,
                collect_messages: true,
            },
            |_offset, _limit| Ok(vec![message(10, 1, None, Some("sticker"))]),
        )
        .expect("scan");

        assert_eq!(result.matched_count, 1);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].content_type.as_deref(), Some("sticker"));
    }

    #[test]
    fn scan_respects_peer_filter() {
        let trusted = vec![peer("owner", 1), peer("other", 2)];
        let result = scan_pending_with(
            &trusted,
            0,
            &PendingScanOptions {
                peer_alias: Some("owner"),
                limit: None,
                collect_messages: false,
            },
            |_offset, _limit| {
                Ok(vec![
                    message(10, 1, Some("a"), None),
                    message(11, 2, Some("b"), None),
                ])
            },
        )
        .expect("scan");

        assert_eq!(result.matched_count, 1);
        assert_eq!(result.highest_seen_update_id, Some(11));
    }

    #[test]
    fn scan_stops_after_limit_and_tracks_processed_updates_only() {
        let trusted = vec![peer("owner", 1)];
        let result = scan_pending_with(
            &trusted,
            0,
            &PendingScanOptions {
                peer_alias: None,
                limit: Some(1),
                collect_messages: true,
            },
            |_offset, _limit| {
                Ok(vec![
                    message(10, 1, Some("first"), None),
                    message(11, 1, Some("second"), None),
                ])
            },
        )
        .expect("scan");

        assert_eq!(result.matched_count, 1);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.highest_seen_update_id, Some(10));
    }
}
