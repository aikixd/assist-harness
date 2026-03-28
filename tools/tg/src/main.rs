mod cli;
mod commands;
mod config;
mod domain;
mod error;
mod interactive;
mod json;
mod output;
mod providers;
mod time;

use cli::Command;
use error::AppError;

fn main() {
    let exit_code = match run() {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            0
        }
        Err(error) => {
            println!("{error}");
            1
        }
    };

    std::process::exit(exit_code);
}

fn run() -> Result<String, AppError> {
    let command = cli::parse(std::env::args().skip(1))?;

    match command {
        Command::Help(topic) => Ok(cli::help_text(topic)),
        Command::ConfigBotAdd => commands::config::bot_add::run(),
        Command::Bots => commands::bots::run(),
        Command::Auth(args) => commands::auth::run(args),
        Command::Peers(args) => commands::peers::run(args),
        Command::PeersRevoke(args) => commands::peers::revoke(args),
        Command::Recv(args) => commands::recv::run(args),
        Command::Send(args) => commands::send::run(args),
    }
}
