pub mod args;
pub mod commands;

use crate::utils::errors::ParserResult;
use args::Cli;
use clap::Parser;
use commands::{parse, validate};

pub fn run() -> ParserResult<()> {
    let cli = Cli::parse();

    match cli.command {
        commands::Commands::Validate { file } => validate(file),
        commands::Commands::Parse { file } => parse(file),
    }
}
