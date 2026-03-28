use std::collections::BTreeMap;

use crate::cli::RecvArgs;
use crate::config::{find_peer, load_cursor, load_peers, resolve_bot, store_cursor};
use crate::domain::{MessageKind, PeerStatus, RecvBlock, RecvMessage};
use crate::error::AppError;
use crate::output::json_string;
use crate::providers::get_updates;

const BATCH_LIMIT: usize = 100;

pub fn run(args: RecvArgs) -> Result<String, AppError> {
    let bot = resolve_bot(args.bot.as_deref())?;
    let peers = load_peers(&bot.alias)?;
    let trusted = peers
        .into_iter()
        .filter(|peer| peer.status == PeerStatus::Trusted)
        .collect::<Vec<_>>();
    if trusted.is_empty() {
        return Ok("no peers configured".to_string());
    }

    let requested_peer = args.peer.as_deref();
    if let Some(alias) = requested_peer {
        let peer = find_peer(&trusted, alias)
            .ok_or_else(|| AppError::query(format!("peer {alias} is not configured")))?;
        if peer.status != PeerStatus::Trusted {
            return Err(AppError::query(format!("peer {alias} is not trusted")));
        }
    }

    let mut offset = load_cursor(&bot.alias)?;
    let mut highest_seen: Option<u64> = None;
    let limit = args.limit.unwrap_or(20);
    let mut messages = Vec::new();

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

            let Some(peer) = trusted.iter().find(|peer| peer.chat_id == message.chat_id) else {
                continue;
            };
            if let Some(alias) = requested_peer {
                if peer.alias != alias {
                    continue;
                }
            }

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

            messages.push(RecvMessage {
                peer_alias: peer.alias.clone(),
                chat_id: peer.chat_id,
                update_id: update.update_id,
                message_id: Some(message.message_id),
                date: message.date.clone(),
                from: message.from_name.clone(),
                text,
                kind,
                content_type,
            });
        }

        if messages.len() >= limit {
            messages.truncate(limit);
            break;
        }

        if updates.len() < BATCH_LIMIT {
            break;
        }
        offset = updates
            .last()
            .map(|update| update.update_id + 1)
            .unwrap_or(offset);
    }

    if let Some(highest_seen) = highest_seen {
        store_cursor(&bot.alias, highest_seen + 1)?;
    }

    if messages.is_empty() {
        return Ok("no messages".to_string());
    }

    let blocks = group_messages(&messages);
    if args.json {
        Ok(format_json(&blocks))
    } else {
        Ok(format_text(&blocks))
    }
}

fn group_messages(messages: &[RecvMessage]) -> Vec<RecvBlock> {
    let mut grouped = BTreeMap::<(String, i64), Vec<RecvMessage>>::new();
    for message in messages {
        grouped
            .entry((message.peer_alias.clone(), message.chat_id))
            .or_default()
            .push(message.clone());
    }

    grouped
        .into_iter()
        .map(|((peer_alias, chat_id), messages)| RecvBlock {
            peer_alias,
            chat_id,
            total: messages.len(),
            messages,
        })
        .collect()
}

fn format_text(blocks: &[RecvBlock]) -> String {
    let mut rendered = Vec::new();

    for block in blocks {
        let mut lines = vec![
            format!("peer: {}", block.peer_alias),
            format!("chat_id: {}", block.chat_id),
            format!("total: {}", block.total),
        ];

        for message in &block.messages {
            lines.push(String::new());
            lines.push("---".to_string());
            lines.push(format!("update_id: {}", message.update_id));
            if let Some(message_id) = message.message_id {
                lines.push(format!("message_id: {message_id}"));
            }
            lines.push(format!("date: {}", message.date));
            lines.push(format!("from: {}", message.from));
            if let Some(content_type) = &message.content_type {
                lines.push(format!("kind: unsupported"));
                lines.push(format!("content_type: {content_type}"));
            }
            lines.push(format!("text: {}", message.text));
            lines.push("---".to_string());
        }

        rendered.push(lines.join("\n"));
    }

    rendered.join("\n\n====\n\n")
}

fn format_json(blocks: &[RecvBlock]) -> String {
    let items = blocks
        .iter()
        .map(|block| {
            let messages = block
                .messages
                .iter()
                .map(|message| {
                    format!(
                        concat!(
                            "{{",
                            "\"update_id\":{},",
                            "\"message_id\":{},",
                            "\"date\":{},",
                            "\"from\":{},",
                            "\"kind\":{},",
                            "\"content_type\":{},",
                            "\"text\":{}",
                            "}}"
                        ),
                        message.update_id,
                        message
                            .message_id
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "null".to_string()),
                        json_string(&message.date),
                        json_string(&message.from),
                        json_string(match message.kind {
                            MessageKind::Text => "text",
                            MessageKind::Unsupported => "unsupported",
                        }),
                        message
                            .content_type
                            .as_ref()
                            .map(|value| json_string(value))
                            .unwrap_or_else(|| "null".to_string()),
                        json_string(&message.text),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "{{\"peer\":{},\"chat_id\":{},\"total\":{},\"messages\":[{}]}}",
                json_string(&block.peer_alias),
                block.chat_id,
                block.total,
                messages,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("{{\"peers\":[{}]}}", items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_text_includes_unsupported_placeholder() {
        let blocks = vec![RecvBlock {
            peer_alias: "owner".to_string(),
            chat_id: 1,
            total: 1,
            messages: vec![RecvMessage {
                peer_alias: "owner".to_string(),
                chat_id: 1,
                update_id: 10,
                message_id: Some(20),
                date: "2026-03-28T10:14".to_string(),
                from: "aikixd".to_string(),
                text: "unsupported trusted message type: sticker".to_string(),
                kind: MessageKind::Unsupported,
                content_type: Some("sticker".to_string()),
            }],
        }];

        let output = format_text(&blocks);
        assert!(output.contains("content_type: sticker"));
        assert!(output.contains("kind: unsupported"));
    }
}
