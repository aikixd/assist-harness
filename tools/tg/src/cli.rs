use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help(HelpTopic),
    ConfigBotAdd,
    Bots,
    Auth(AuthArgs),
    Peers(PeersArgs),
    PeersRevoke(PeersRevokeArgs),
    Recv(RecvArgs),
    Send(SendArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpTopic {
    General,
    ConfigBotAdd,
    Bots,
    Auth,
    Peers,
    Recv,
    Send,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthArgs {
    pub alias: String,
    pub bot: Option<String>,
    pub stdin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeersArgs {
    pub bot: Option<String>,
    pub all: bool,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeersRevokeArgs {
    pub alias: String,
    pub bot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecvArgs {
    pub bot: Option<String>,
    pub peer: Option<String>,
    pub limit: Option<usize>,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendArgs {
    pub bot: Option<String>,
    pub peer: Option<String>,
    pub text: Option<String>,
    pub stdin: bool,
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
        "bots" => parse_bots(&args[1..]),
        "auth" => parse_auth(&args[1..]),
        "peers" => parse_peers(&args[1..]),
        "recv" => parse_recv(&args[1..]),
        "send" => parse_send(&args[1..]),
        other => Err(AppError::usage(format!(
            "unknown command: {other}\n\n{}",
            help_text(HelpTopic::General)
        ))),
    }
}

pub fn help_text(topic: HelpTopic) -> String {
    match topic {
        HelpTopic::General => general_help_text(),
        HelpTopic::ConfigBotAdd => config_bot_add_help_text(),
        HelpTopic::Bots => bots_help_text(),
        HelpTopic::Auth => auth_help_text(),
        HelpTopic::Peers => peers_help_text(),
        HelpTopic::Recv => recv_help_text(),
        HelpTopic::Send => send_help_text(),
    }
}

fn general_help_text() -> String {
    [
        "pa-tg",
        "",
        "Usage:",
        "  pa-tg --help",
        "  pa-tg <command> --help",
        "",
        "Commands:",
        "  pa-tg config bot add",
        "    Store one bot token locally.",
        "  pa-tg bots",
        "    List configured bots and their status.",
        "  pa-tg auth --alias <alias> [--bot <name>] [--stdin]",
        "    Trust one private chat by matching a locally provided auth key.",
        "  pa-tg peers [--bot <name>] [--all] [--json]",
        "    List known peers for a bot.",
        "  pa-tg peers revoke <alias> [--bot <name>]",
        "    Mark one peer as revoked.",
        "  pa-tg recv [--bot <name>] [--peer <alias>] [--limit <n>] [--json]",
        "    Fetch trusted pending messages.",
        "  pa-tg send [--bot <name>] [--peer <alias>] (--text <text> | --stdin) [--json]",
        "    Send one text message to a trusted peer.",
        "",
        "Storage:",
        "  - config: ~/.config/pa/tg/",
        "  - local data: ~/.local/share/pa/tg/",
        "  - cache: ~/.cache/pa/tg/",
        "",
        "Notes:",
        "  - `recv` ignores unauthenticated chats and does not store them.",
        "  - `auth` requires a lowercase GUID key in canonical 8-4-4-4-12 form.",
    ]
    .join("\n")
}

fn config_bot_add_help_text() -> String {
    [
        "pa-tg config bot add",
        "",
        "Purpose:",
        "  Store one bot alias and token locally.",
        "",
        "Usage:",
        "  pa-tg config bot add",
        "",
        "Interactive parameters:",
        "  bot alias",
        "    Unique local bot name.",
        "  bot token",
        "    Telegram Bot API token.",
    ]
    .join("\n")
}

fn bots_help_text() -> String {
    [
        "pa-tg bots",
        "",
        "Purpose:",
        "  List configured bots and current API status.",
    ]
    .join("\n")
}

fn auth_help_text() -> String {
    [
        "pa-tg auth",
        "",
        "Purpose:",
        "  Trust one private chat by matching a locally provided auth key.",
        "",
        "Usage:",
        "  pa-tg auth --alias <alias> [--bot <name>] [--stdin]",
        "",
        "Parameters:",
        "  --alias <alias>",
        "    Required. Local peer alias to assign to the trusted chat.",
        "  --bot <name>",
        "    Optional when exactly one bot is configured.",
        "  --stdin",
        "    Optional. Read the auth key from stdin instead of prompting.",
    ]
    .join("\n")
}

fn peers_help_text() -> String {
    [
        "pa-tg peers",
        "",
        "Usage:",
        "  pa-tg peers [--bot <name>] [--all] [--json]",
        "  pa-tg peers revoke <alias> [--bot <name>]",
    ]
    .join("\n")
}

fn recv_help_text() -> String {
    [
        "pa-tg recv",
        "",
        "Purpose:",
        "  Fetch pending messages from trusted peers.",
        "",
        "Usage:",
        "  pa-tg recv [--bot <name>] [--peer <alias>] [--limit <n>] [--json]",
    ]
    .join("\n")
}

fn send_help_text() -> String {
    [
        "pa-tg send",
        "",
        "Purpose:",
        "  Send one text message to a trusted peer.",
        "",
        "Usage:",
        "  pa-tg send [--bot <name>] [--peer <alias>] (--text <text> | --stdin) [--json]",
    ]
    .join("\n")
}

fn parse_config(args: &[String]) -> Result<Command, AppError> {
    match args {
        [bot, add, flag] if bot == "bot" && add == "add" && is_help_flag(flag) => {
            Ok(Command::Help(HelpTopic::ConfigBotAdd))
        }
        [bot, add] if bot == "bot" && add == "add" => Ok(Command::ConfigBotAdd),
        _ => Err(AppError::usage(format!(
            "supported config commands:\n  pa-tg config bot add\n\n{}",
            help_text(HelpTopic::General)
        ))),
    }
}

fn parse_bots(args: &[String]) -> Result<Command, AppError> {
    match args {
        [] => Ok(Command::Bots),
        [flag] if is_help_flag(flag) => Ok(Command::Help(HelpTopic::Bots)),
        _ => Err(AppError::usage(format!(
            "pa-tg bots does not accept extra arguments\n\n{}",
            help_text(HelpTopic::General)
        ))),
    }
}

fn parse_auth(args: &[String]) -> Result<Command, AppError> {
    if args.len() == 1 && is_help_flag(&args[0]) {
        return Ok(Command::Help(HelpTopic::Auth));
    }

    let mut alias = None;
    let mut bot = None;
    let mut stdin = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--alias" => alias = Some(take_value(args, &mut index, "--alias")?),
            "--bot" => bot = Some(take_value(args, &mut index, "--bot")?),
            "--stdin" => stdin = true,
            other => {
                return Err(AppError::usage(format!(
                    "unknown auth argument: {other}\n\n{}",
                    help_text(HelpTopic::Auth)
                )))
            }
        }
        index += 1;
    }

    let alias = alias.ok_or_else(|| {
        AppError::usage(format!(
            "pa-tg auth requires --alias <alias>\n\n{}",
            help_text(HelpTopic::Auth)
        ))
    })?;

    Ok(Command::Auth(AuthArgs { alias, bot, stdin }))
}

fn parse_peers(args: &[String]) -> Result<Command, AppError> {
    if args.len() == 1 && is_help_flag(&args[0]) {
        return Ok(Command::Help(HelpTopic::Peers));
    }

    match args {
        [revoke, alias] if revoke == "revoke" => {
            return Ok(Command::PeersRevoke(PeersRevokeArgs {
                alias: alias.clone(),
                bot: None,
            }))
        }
        [revoke, alias, flag] if revoke == "revoke" && is_help_flag(flag) => {
            return Ok(Command::Help(HelpTopic::Peers))
        }
        _ => {}
    }

    let mut bot = None;
    let mut all = false;
    let mut json = false;
    let mut revoke_alias = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "revoke" => {
                index += 1;
                revoke_alias = Some(
                    args.get(index)
                        .cloned()
                        .ok_or_else(|| AppError::usage("missing alias for peers revoke"))?,
                );
            }
            "--bot" => bot = Some(take_value(args, &mut index, "--bot")?),
            "--all" => all = true,
            "--json" => json = true,
            other => {
                return Err(AppError::usage(format!(
                    "unknown peers argument: {other}\n\n{}",
                    help_text(HelpTopic::Peers)
                )))
            }
        }
        index += 1;
    }

    if let Some(alias) = revoke_alias {
        return Ok(Command::PeersRevoke(PeersRevokeArgs { alias, bot }));
    }

    Ok(Command::Peers(PeersArgs { bot, all, json }))
}

fn parse_recv(args: &[String]) -> Result<Command, AppError> {
    if args.len() == 1 && is_help_flag(&args[0]) {
        return Ok(Command::Help(HelpTopic::Recv));
    }

    let mut bot = None;
    let mut peer = None;
    let mut limit = None;
    let mut json = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--bot" => bot = Some(take_value(args, &mut index, "--bot")?),
            "--peer" => peer = Some(take_value(args, &mut index, "--peer")?),
            "--limit" => {
                let value = take_value(args, &mut index, "--limit")?;
                limit =
                    Some(value.parse::<usize>().map_err(|_| {
                        AppError::usage(format!("invalid value for --limit: {value}"))
                    })?);
            }
            "--json" => json = true,
            other => {
                return Err(AppError::usage(format!(
                    "unknown recv argument: {other}\n\n{}",
                    help_text(HelpTopic::Recv)
                )))
            }
        }
        index += 1;
    }

    Ok(Command::Recv(RecvArgs {
        bot,
        peer,
        limit,
        json,
    }))
}

fn parse_send(args: &[String]) -> Result<Command, AppError> {
    if args.len() == 1 && is_help_flag(&args[0]) {
        return Ok(Command::Help(HelpTopic::Send));
    }

    let mut bot = None;
    let mut peer = None;
    let mut text = None;
    let mut stdin = false;
    let mut json = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--bot" => bot = Some(take_value(args, &mut index, "--bot")?),
            "--peer" => peer = Some(take_value(args, &mut index, "--peer")?),
            "--text" => text = Some(take_value(args, &mut index, "--text")?),
            "--stdin" => stdin = true,
            "--json" => json = true,
            other => {
                return Err(AppError::usage(format!(
                    "unknown send argument: {other}\n\n{}",
                    help_text(HelpTopic::Send)
                )))
            }
        }
        index += 1;
    }

    if text.is_some() == stdin {
        return Err(AppError::usage(format!(
            "pa-tg send requires exactly one of --text <text> or --stdin\n\n{}",
            help_text(HelpTopic::Send)
        )));
    }

    Ok(Command::Send(SendArgs {
        bot,
        peer,
        text,
        stdin,
        json,
    }))
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
    fn auth_requires_alias() {
        let result = parse(["auth".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn peers_revoke_supports_bot_flag() {
        let result = parse([
            "peers".to_string(),
            "revoke".to_string(),
            "owner".to_string(),
            "--bot".to_string(),
            "main".to_string(),
        ]);

        assert_eq!(
            result,
            Ok(Command::PeersRevoke(PeersRevokeArgs {
                alias: "owner".to_string(),
                bot: Some("main".to_string()),
            }))
        );
    }

    #[test]
    fn send_requires_one_input_source() {
        let result = parse([
            "send".to_string(),
            "--text".to_string(),
            "hello".to_string(),
            "--stdin".to_string(),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn send_parses_json_flag() {
        let result = parse([
            "send".to_string(),
            "--peer".to_string(),
            "owner".to_string(),
            "--text".to_string(),
            "hello".to_string(),
            "--json".to_string(),
        ]);

        let Command::Send(args) = result.expect("expected send command") else {
            panic!("expected send command");
        };
        assert_eq!(args.peer.as_deref(), Some("owner"));
        assert_eq!(args.text.as_deref(), Some("hello"));
        assert!(args.json);
    }
}
