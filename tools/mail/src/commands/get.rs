use crate::cli::GetArgs;
use crate::config::{find_account, load_accounts};
use crate::domain::MessageDetail;
use crate::error::AppError;
use crate::output::json_string;
use crate::providers::get_message;

pub fn run(args: GetArgs) -> Result<String, AppError> {
    let accounts = load_accounts()?;
    if accounts.is_empty() {
        return Ok("no accounts configured".to_string());
    }

    let account = find_account(&accounts, &args.account)
        .ok_or_else(|| AppError::query(format!("account {} is not configured", args.account)))?;

    if !account.is_ready() {
        return Err(AppError::query(format!(
            "account {} is not ready: {}",
            account.email, account.status
        )));
    }

    let message = get_message(account, &args.id)?;

    if args.json {
        Ok(format_json(&message))
    } else {
        Ok(format_text(&message))
    }
}

fn format_text(message: &MessageDetail) -> String {
    let mut lines = vec![
        format!("acc: {}", message.account),
        format!("id: {}", message.id),
    ];

    if let Some(thread_id) = &message.thread_id {
        lines.push(format!("thread_id: {thread_id}"));
    }

    lines.push(format!("date: {}", message.date));
    lines.push(format!("from: {}", message.from));
    lines.push(format!("to: {}", message.to));
    lines.push(format!("cc: {}", message.cc.clone().unwrap_or_default()));
    lines.push(format!("subject: {}", message.subject));

    if !message.labels.is_empty() {
        lines.push(format!("labels: {}", message.labels.join(", ")));
    }

    lines.push(String::new());
    lines.push("body_text:".to_string());
    lines.push(message.body_text.clone());

    if !message.links.is_empty() {
        lines.push(String::new());
        lines.push("links:".to_string());
        for link in &message.links {
            lines.push(format!("- {link}"));
        }
    }

    if !message.attachments.is_empty() {
        lines.push(String::new());
        lines.push("attachments:".to_string());
        for attachment in &message.attachments {
            lines.push(format!(
                "- {} | {} | {}",
                attachment.name, attachment.mime_type, attachment.size_bytes
            ));
        }
    }

    lines.join("\n")
}

fn format_json(message: &MessageDetail) -> String {
    let thread_id = message
        .thread_id
        .as_ref()
        .map(|value| json_string(value))
        .unwrap_or_else(|| "null".to_string());
    let cc = message
        .cc
        .as_ref()
        .map(|value| json_string(value))
        .unwrap_or_else(|| "null".to_string());
    let labels = message
        .labels
        .iter()
        .map(|label| json_string(label))
        .collect::<Vec<_>>()
        .join(", ");
    let links = message
        .links
        .iter()
        .map(|link| json_string(link))
        .collect::<Vec<_>>()
        .join(", ");
    let attachments = message
        .attachments
        .iter()
        .map(|attachment| {
            format!(
                "{{\"name\":{},\"mime_type\":{},\"size_bytes\":{}}}",
                json_string(&attachment.name),
                json_string(&attachment.mime_type),
                attachment.size_bytes,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        concat!(
            "{{",
            "\"account\":{},",
            "\"id\":{},",
            "\"thread_id\":{},",
            "\"date\":{},",
            "\"from\":{},",
            "\"to\":{},",
            "\"cc\":{},",
            "\"subject\":{},",
            "\"labels\":[{}],",
            "\"body_text\":{},",
            "\"links\":[{}],",
            "\"attachments\":[{}]",
            "}}"
        ),
        json_string(&message.account),
        json_string(&message.id),
        thread_id,
        json_string(&message.date),
        json_string(&message.from),
        json_string(&message.to),
        cc,
        json_string(&message.subject),
        labels,
        json_string(&message.body_text),
        links,
        attachments,
    )
}

#[cfg(test)]
mod tests {
    use crate::domain::{Attachment, MessageDetail};

    use super::*;

    #[test]
    fn text_format_includes_body_and_attachments() {
        let message = MessageDetail {
            account: "personal@gmail.com".to_string(),
            id: "msg-1".to_string(),
            thread_id: Some("thread-1".to_string()),
            date: "2026-03-06T12:14".to_string(),
            from: "someone@example.com".to_string(),
            to: "personal@gmail.com".to_string(),
            cc: None,
            subject: "Quick question".to_string(),
            labels: vec!["inbox".to_string(), "unread".to_string()],
            body_text: "Can you share the docs link?".to_string(),
            links: vec!["https://example.com/docs".to_string()],
            attachments: vec![Attachment {
                name: "spec.pdf".to_string(),
                mime_type: "application/pdf".to_string(),
                size_bytes: 48213,
            }],
        };

        let output = format_text(&message);
        assert!(output.contains("body_text:"));
        assert!(output.contains("attachments:"));
        assert!(output.contains("spec.pdf | application/pdf | 48213"));
    }
}
