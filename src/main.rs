use utils::errors::ParserResult;

mod cli;
mod parser;
mod utils;

fn main() -> ParserResult<()> {
    cli::run()
}
