use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    ConfigAccountAdd,
    Accounts,
    List(ListArgs),
    Get(GetArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListArgs {
    pub since: String,
    pub until: Option<String>,
    pub account: Option<String>,
    pub label: Option<String>,
    pub limit: Option<usize>,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetArgs {
    pub id: String,
    pub account: String,
    pub json: bool,
}

pub fn parse<I>(args: I) -> Result<Command, AppError>
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();

    if args.is_empty() {
        return Ok(Command::Help);
    }

    match args[0].as_str() {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "config" => parse_config(&args[1..]),
        "accounts" => parse_accounts(&args[1..]),
        "list" => parse_list(&args[1..]),
        "get" => parse_get(&args[1..]),
        other => Err(AppError::usage(format!(
            "unknown command: {other}\n\n{}",
            help_text()
        ))),
    }
}

pub fn help_text() -> String {
    [
        "pa-mail",
        "",
        "Commands:",
        "  pa-mail help",
        "  pa-mail config account add",
        "  pa-mail accounts",
        "  pa-mail list --since <time> [--until <time>] [--account <email>] [--label <label>] [--limit <n>] [--json]",
        "  pa-mail get <id> <email> [--json]",
        "",
        "Time semantics:",
        "  - CLI time inputs are interpreted in the machine's local timezone",
        "  - --since is inclusive",
        "  - --until is exclusive",
        "",
        "Storage:",
        "  - config: ~/.config/pa/mail/",
        "  - local data: ~/.local/share/pa/mail/",
        "  - cache: ~/.cache/pa/mail/",
        "",
        "Config file:",
        "  - ~/.config/pa/mail/accounts.txt",
        "  - one account per line: <email> <provider>",
        "  - example: personal@gmail.com google",
        "",
        "OAuth env vars:",
        "  - PA_MAIL_GOOGLE_CLIENT_ID",
        "  - PA_MAIL_GOOGLE_CLIENT_SECRET",
    ]
    .join("\n")
}

fn parse_config(args: &[String]) -> Result<Command, AppError> {
    match args {
        [account, add] if account == "account" && add == "add" => Ok(Command::ConfigAccountAdd),
        _ => Err(AppError::usage(format!(
            "supported config command: pa-mail config account add\n\n{}",
            help_text()
        ))),
    }
}

fn parse_accounts(args: &[String]) -> Result<Command, AppError> {
    if args.is_empty() {
        Ok(Command::Accounts)
    } else {
        Err(AppError::usage(format!(
            "pa-mail accounts does not accept extra arguments\n\n{}",
            help_text()
        )))
    }
}

fn parse_list(args: &[String]) -> Result<Command, AppError> {
    let mut since = None;
    let mut until = None;
    let mut account = None;
    let mut label = None;
    let mut limit = None;
    let mut json = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--since" => {
                since = Some(take_value(args, &mut index, "--since")?);
            }
            "--until" => {
                until = Some(take_value(args, &mut index, "--until")?);
            }
            "--account" => {
                account = Some(take_value(args, &mut index, "--account")?);
            }
            "--label" => {
                label = Some(take_value(args, &mut index, "--label")?);
            }
            "--limit" => {
                let value = take_value(args, &mut index, "--limit")?;
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| AppError::usage(format!("invalid value for --limit: {value}")))?;
                limit = Some(parsed);
            }
            "--json" => {
                json = true;
            }
            other => {
                return Err(AppError::usage(format!(
                    "unknown list argument: {other}\n\n{}",
                    help_text()
                )));
            }
        }

        index += 1;
    }

    let since = since.ok_or_else(|| {
        AppError::usage(format!(
            "pa-mail list requires --since <time>\n\n{}",
            help_text()
        ))
    })?;

    Ok(Command::List(ListArgs {
        since,
        until,
        account,
        label,
        limit,
        json,
    }))
}

fn parse_get(args: &[String]) -> Result<Command, AppError> {
    let mut id = None;
    let mut account = None;
    let mut json = false;

    for arg in args {
        if arg == "--json" {
            json = true;
            continue;
        }

        if arg.starts_with("--") {
            return Err(AppError::usage(format!(
                "unknown get argument: {arg}\n\n{}",
                help_text()
            )));
        }

        if id.is_none() {
            id = Some(arg.clone());
            continue;
        }

        if account.is_none() {
            account = Some(arg.clone());
            continue;
        }

        return Err(AppError::usage(format!(
            "pa-mail get accepts only <id> <email> [--json]\n\n{}",
            help_text()
        )));
    }

    let id = id.ok_or_else(|| {
        AppError::usage(format!(
            "pa-mail get requires <id> <email>\n\n{}",
            help_text()
        ))
    })?;
    let account = account.ok_or_else(|| {
        AppError::usage(format!(
            "pa-mail get requires <id> <email>\n\n{}",
            help_text()
        ))
    })?;

    Ok(Command::Get(GetArgs { id, account, json }))
}

fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, AppError> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| AppError::usage(format!("missing value for {flag}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_requires_since() {
        let result = parse(["list".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn list_parses_expected_flags() {
        let result = parse([
            "list".to_string(),
            "--since".to_string(),
            "2026-03-06T14:30".to_string(),
            "--account".to_string(),
            "personal@gmail.com".to_string(),
            "--label".to_string(),
            "inbox".to_string(),
            "--limit".to_string(),
            "5".to_string(),
            "--json".to_string(),
        ]);

        assert!(result.is_ok());
        let command = result.unwrap();

        let Command::List(args) = command else {
            panic!("expected list command");
        };

        assert_eq!(args.since, "2026-03-06T14:30");
        assert_eq!(args.account.as_deref(), Some("personal@gmail.com"));
        assert_eq!(args.label.as_deref(), Some("inbox"));
        assert_eq!(args.limit, Some(5));
        assert!(args.json);
    }

    #[test]
    fn get_requires_id_and_account() {
        let result = parse(["get".to_string(), "abc123".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn config_account_add_is_supported() {
        let result = parse([
            "config".to_string(),
            "account".to_string(),
            "add".to_string(),
        ]);
        assert_eq!(result, Ok(Command::ConfigAccountAdd));
    }
}
