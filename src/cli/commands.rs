use crate::parser::document::Document;
use crate::utils::errors::ParserResult;

pub fn validate(file: String) -> ParserResult<()> {
    let contents = std::fs::read_to_string(&file)?;
    println!("{}", contents);
    let doc = Document::parse(&contents)?;
    doc.validate("<root>", None, None, None, None)?;
    println!("✅ Document is valid.");
    Ok(())
}

pub fn parse(file: String) -> ParserResult<()> {
    let contents = std::fs::read_to_string(&file)?;
    let doc = Document::parse(&contents)?;
    println!("{}", serde_json::to_string_pretty(&doc)?);
    Ok(())
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Validate { file: String },
    Parse { file: String },
}
