use super::commands::Commands;
use clap::{Parser, command};

#[derive(Parser)]
#[command(
    name = "slo-slicer 🔪",
    version,
    about = "OpenSLO YAML validator + parser."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
