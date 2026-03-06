mod cli;
mod commands;
mod config;
mod domain;
mod error;
mod interactive;
mod output;
mod providers;

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
        Command::Help => Ok(cli::help_text()),
        Command::ConfigProvider => commands::config::provider::run(),
        Command::ConfigAccountAdd => commands::config::account_add::run(),
        Command::Accounts => commands::accounts::run(),
        Command::List(args) => commands::list::run(args),
        Command::Get(args) => commands::get::run(args),
    }
}
