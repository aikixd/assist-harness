use crate::cli::ListArgs;
use crate::config::{find_account, load_accounts, AccountEntry};
use crate::domain::AccountMessageBlock;
use crate::error::AppError;
use crate::output::json_string;
use crate::providers::{list_messages, validate_list_query, ListQuery};

pub fn run(args: ListArgs) -> Result<String, AppError> {
    let accounts = load_accounts()?;
    if accounts.is_empty() {
        return Ok("no accounts configured".to_string());
    }

    let selected_accounts = select_accounts(&accounts, args.account.as_deref())?;
    let query = ListQuery {
        since: args.since,
        until: args.until,
        label: args.label,
        limit: args.limit,
    };

    for account in &selected_accounts {
        if !account.is_ready() {
            return Err(AppError::query(format!(
                "account {} is not ready: {}",
                account.email, account.status
            )));
        }
        validate_list_query(account, &query)?;
    }

    let mut blocks = Vec::new();
    for account in selected_accounts {
        let messages = list_messages(account, &query)?;
        blocks.push(AccountMessageBlock {
            account: account.email.clone(),
            unread: 0,
            total: messages.len(),
            messages,
        });
    }

    if args.json {
        Ok(format_json(&blocks))
    } else {
        Ok(format_text(&blocks))
    }
}

fn select_accounts<'a>(
    accounts: &'a [AccountEntry],
    requested_email: Option<&str>,
) -> Result<Vec<&'a AccountEntry>, AppError> {
    match requested_email {
        Some(email) => {
            let account = find_account(accounts, email)
                .ok_or_else(|| AppError::query(format!("account {email} is not configured")))?;
            Ok(vec![account])
        }
        None => Ok(accounts.iter().collect()),
    }
}

fn format_text(blocks: &[AccountMessageBlock]) -> String {
    let mut rendered = Vec::new();

    for block in blocks {
        let mut lines = vec![
            format!("acc: {}", block.account),
            format!("unread: {}", block.unread),
            format!("total: {}", block.total),
        ];

        for message in &block.messages {
            lines.push(String::new());
            lines.push("---".to_string());
            lines.push(format!("id: {}", message.id));
            lines.push(format!("date: {}", message.date));
            lines.push(format!("from: {}", message.from));
            lines.push(format!("to: {}", message.to));
            lines.push(format!("subject: {}", message.subject));
            if !message.labels.is_empty() {
                lines.push(format!("labels: {}", message.labels.join(", ")));
            }
            if let Some(thread_id) = &message.thread_id {
                lines.push(format!("thread_id: {thread_id}"));
            }
            lines.push(format!("body_preview: {}", message.body_preview));
            lines.push("---".to_string());
        }

        rendered.push(lines.join("\n"));
    }

    rendered.join("\n\n====\n\n")
}

fn format_json(blocks: &[AccountMessageBlock]) -> String {
    let items = blocks
        .iter()
        .map(|block| {
            let messages = block
                .messages
                .iter()
                .map(|message| {
                    let labels = message
                        .labels
                        .iter()
                        .map(|label| json_string(label))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let thread_id = message
                        .thread_id
                        .as_ref()
                        .map(|value| json_string(value))
                        .unwrap_or_else(|| "null".to_string());

                    format!(
                        "{{\"id\":{},\"date\":{},\"from\":{},\"to\":{},\"subject\":{},\"labels\":[{}],\"thread_id\":{},\"body_preview\":{}}}",
                        json_string(&message.id),
                        json_string(&message.date),
                        json_string(&message.from),
                        json_string(&message.to),
                        json_string(&message.subject),
                        labels,
                        thread_id,
                        json_string(&message.body_preview),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "{{\"account\":{},\"unread\":{},\"total\":{},\"messages\":[{}]}}",
                json_string(&block.account),
                block.unread,
                block.total,
                messages,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("{{\"accounts\":[{}]}}", items)
}

#[cfg(test)]
mod tests {
    use crate::domain::MessageSummary;

    use super::*;

    #[test]
    fn text_format_uses_expected_separators() {
        let blocks = vec![
            AccountMessageBlock {
                account: "a@example.com".to_string(),
                unread: 1,
                total: 1,
                messages: vec![MessageSummary {
                    id: "msg-1".to_string(),
                    date: "2026-03-06T12:14".to_string(),
                    from: "someone@example.com".to_string(),
                    to: "a@example.com".to_string(),
                    subject: "Hello".to_string(),
                    labels: vec!["inbox".to_string(), "unread".to_string()],
                    body_preview: "Preview".to_string(),
                    thread_id: None,
                }],
            },
            AccountMessageBlock {
                account: "b@example.com".to_string(),
                unread: 0,
                total: 0,
                messages: Vec::new(),
            },
        ];

        let output = format_text(&blocks);
        assert!(output.contains("acc: a@example.com"));
        assert!(output.contains("\n---\n"));
        assert!(output.contains("\n\n====\n\n"));
    }
}
