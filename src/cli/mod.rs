pub mod args;
pub mod commands;

use crate::utils::errors::ParserResult;
use args::Cli;
use clap::Parser;
use commands::validate;

pub fn run() -> ParserResult<()> {
    let cli = Cli::parse();

    match cli.command {
        commands::Commands::Validate {
            file,
            recursive,
            quiet,
        } => validate(file, recursive, quiet),
    }
}
