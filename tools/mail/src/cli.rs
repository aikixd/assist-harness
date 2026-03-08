use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help(HelpTopic),
    ConfigProvider,
    ConfigAccountAdd,
    Accounts,
    List(ListArgs),
    Get(GetArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpTopic {
    General,
    ConfigProvider,
    ConfigAccountAdd,
    Accounts,
    List,
    Get,
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
        return Ok(Command::Help(HelpTopic::General));
    }

    match args[0].as_str() {
        "--help" | "-h" => Ok(Command::Help(HelpTopic::General)),
        "config" => parse_config(&args[1..]),
        "accounts" => parse_accounts(&args[1..]),
        "list" => parse_list(&args[1..]),
        "get" => parse_get(&args[1..]),
        other => Err(AppError::usage(format!(
            "unknown command: {other}\n\n{}",
            help_text(HelpTopic::General)
        ))),
    }
}

pub fn help_text(topic: HelpTopic) -> String {
    match topic {
        HelpTopic::General => general_help_text(),
        HelpTopic::ConfigProvider => config_provider_help_text(),
        HelpTopic::ConfigAccountAdd => config_account_add_help_text(),
        HelpTopic::Accounts => accounts_help_text(),
        HelpTopic::List => list_help_text(),
        HelpTopic::Get => get_help_text(),
    }
}

fn general_help_text() -> String {
    [
        "pa-mail",
        "",
        "Usage:",
        "  pa-mail --help",
        "  pa-mail <command> --help",
        "",
        "Commands:",
        "  pa-mail config provider",
        "    Store local OAuth app credentials for a provider.",
        "  pa-mail config account add",
        "    Add one mailbox account and complete OAuth setup.",
        "  pa-mail accounts",
        "    List configured accounts and their status.",
        "  pa-mail list --since <time> [--until <time>] [--account <email>] [--label <label>] [--limit <n>] [--json]",
        "    List recent matching messages.",
        "  pa-mail get <id> <email> [--json]",
        "    Fetch one message in detail.",
        "",
        "Required time format:",
        "  - Use local RFC3339-like timestamps: YYYY-MM-DDTHH:MM",
        "  - Example: 2026-03-06T14:30",
        "",
        "Time semantics:",
        "  - CLI time inputs are interpreted in the machine's local timezone.",
        "  - --since is inclusive.",
        "  - --until is exclusive.",
        "",
        "Storage:",
        "  - config: ~/.config/pa/mail/",
        "  - local data: ~/.local/share/pa/mail/",
        "  - cache: ~/.cache/pa/mail/",
        "",
        "Config file:",
        "  - accounts: ~/.config/pa/mail/accounts.txt",
        "  - providers: ~/.config/pa/mail/providers/",
        "",
        "Use `pa-mail <command> --help` for detailed command help.",
    ]
    .join("\n")
}

fn config_provider_help_text() -> String {
    [
        "pa-mail config provider",
        "",
        "Purpose:",
        "  Store local OAuth app credentials for a provider.",
        "",
        "Usage:",
        "  pa-mail config provider",
        "",
        "Interactive parameters:",
        "  provider",
        "    The provider name. V1 supports: google",
        "  google client id",
        "    OAuth client ID issued for your Desktop app.",
        "  google client secret",
        "    OAuth client secret for the same app.",
        "",
        "Storage:",
        "  Writes provider credentials under ~/.config/pa/mail/providers/",
    ]
    .join("\n")
}

fn config_account_add_help_text() -> String {
    [
        "pa-mail config account add",
        "",
        "Purpose:",
        "  Add one mailbox account and complete OAuth setup.",
        "",
        "Usage:",
        "  pa-mail config account add",
        "",
        "Interactive parameters:",
        "  account email",
        "    Mailbox email address for the account being added.",
        "  provider",
        "    The provider name. V1 supports: google",
        "",
        "Behavior:",
        "  - Prints local config and token paths before continuing.",
        "  - Starts a temporary loopback listener on 127.0.0.1 for OAuth callback.",
        "  - Opens no browser automatically; you follow the printed URL manually.",
    ]
    .join("\n")
}

fn accounts_help_text() -> String {
    [
        "pa-mail accounts",
        "",
        "Purpose:",
        "  List configured accounts and their current status.",
        "",
        "Usage:",
        "  pa-mail accounts",
        "",
        "Parameters:",
        "  none",
        "",
        "Output:",
        "  Each account starts with <email> - <provider> followed by status information.",
    ]
    .join("\n")
}

fn list_help_text() -> String {
    [
        "pa-mail list",
        "",
        "Purpose:",
        "  List recent matching messages in compact text by default.",
        "",
        "Usage:",
        "  pa-mail list --since <time> [--until <time>] [--account <email>] [--label <label>] [--limit <n>] [--json]",
        "",
        "Parameters:",
        "  --since <time>",
        "    Required. Lower time bound, inclusive.",
        "  --until <time>",
        "    Optional. Upper time bound, exclusive.",
        "  --account <email>",
        "    Optional. Restrict results to one mailbox.",
        "  --label <label>",
        "    Optional. Google-specific label filter.",
        "  --limit <n>",
        "    Optional. Per-account result limit.",
        "  --json",
        "    Optional. Emit structured JSON instead of text.",
        "",
        "Time format:",
        "  Use local RFC3339-like timestamps: YYYY-MM-DDTHH:MM",
        "  Example: 2026-03-06T14:30",
    ]
    .join("\n")
}

fn get_help_text() -> String {
    [
        "pa-mail get",
        "",
        "Purpose:",
        "  Fetch one message in detail.",
        "",
        "Usage:",
        "  pa-mail get <id> <email> [--json]",
        "",
        "Parameters:",
        "  <id>",
        "    Provider message id for the target message.",
        "  <email>",
        "    Mailbox email address that scopes the message lookup.",
        "  --json",
        "    Optional. Emit structured JSON instead of text.",
        "",
        "Behavior:",
        "  - Does not search across accounts implicitly.",
        "  - Prints `message with id <id> not found` when the target message is missing.",
    ]
    .join("\n")
}

fn parse_config(args: &[String]) -> Result<Command, AppError> {
    match args {
        [provider, flag] if provider == "provider" && is_help_flag(flag) => {
            Ok(Command::Help(HelpTopic::ConfigProvider))
        }
        [provider] if provider == "provider" => Ok(Command::ConfigProvider),
        [account, add, flag] if account == "account" && add == "add" && is_help_flag(flag) => {
            Ok(Command::Help(HelpTopic::ConfigAccountAdd))
        }
        [account, add] if account == "account" && add == "add" => Ok(Command::ConfigAccountAdd),
        _ => Err(AppError::usage(format!(
            "supported config commands:\n  pa-mail config provider\n  pa-mail config account add\n\n{}",
            help_text(HelpTopic::General)
        ))),
    }
}

fn parse_accounts(args: &[String]) -> Result<Command, AppError> {
    match args {
        [] => Ok(Command::Accounts),
        [flag] if is_help_flag(flag) => Ok(Command::Help(HelpTopic::Accounts)),
        _ => Err(AppError::usage(format!(
            "pa-mail accounts does not accept extra arguments\n\n{}",
            help_text(HelpTopic::General)
        ))),
    }
}

fn parse_list(args: &[String]) -> Result<Command, AppError> {
    if args.len() == 1 && is_help_flag(&args[0]) {
        return Ok(Command::Help(HelpTopic::List));
    }

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
                    help_text(HelpTopic::General)
                )));
            }
        }

        index += 1;
    }

    let since = since.ok_or_else(|| {
        AppError::usage(format!(
            "pa-mail list requires --since <time>\n\n{}",
            help_text(HelpTopic::List)
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
    if args.len() == 1 && is_help_flag(&args[0]) {
        return Ok(Command::Help(HelpTopic::Get));
    }

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
                help_text(HelpTopic::General)
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
            help_text(HelpTopic::Get)
        )));
    }

    let id = id.ok_or_else(|| {
        AppError::usage(format!(
            "pa-mail get requires <id> <email>\n\n{}",
            help_text(HelpTopic::Get)
        ))
    })?;
    let account = account.ok_or_else(|| {
        AppError::usage(format!(
            "pa-mail get requires <id> <email>\n\n{}",
            help_text(HelpTopic::Get)
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

fn is_help_flag(value: &str) -> bool {
    matches!(value, "--help" | "-h")
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

    #[test]
    fn config_provider_is_supported() {
        let result = parse(["config".to_string(), "provider".to_string()]);
        assert_eq!(result, Ok(Command::ConfigProvider));
    }

    #[test]
    fn list_help_is_supported() {
        let result = parse(["list".to_string(), "--help".to_string()]);
        assert_eq!(result, Ok(Command::Help(HelpTopic::List)));
    }

    #[test]
    fn get_help_is_supported() {
        let result = parse(["get".to_string(), "--help".to_string()]);
        assert_eq!(result, Ok(Command::Help(HelpTopic::Get)));
    }

    #[test]
    fn config_account_add_help_is_supported() {
        let result = parse([
            "config".to_string(),
            "account".to_string(),
            "add".to_string(),
            "--help".to_string(),
        ]);
        assert_eq!(result, Ok(Command::Help(HelpTopic::ConfigAccountAdd)));
    }

    #[test]
    fn general_help_mentions_required_time_format() {
        let help = help_text(HelpTopic::General);
        assert!(help.contains("Required time format:"));
        assert!(help.contains("YYYY-MM-DDTHH:MM"));
    }

    #[test]
    fn help_command_is_not_supported() {
        let result = parse(["help".to_string()]);
        assert!(result.is_err());
    }
}
