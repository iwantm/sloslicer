use crate::parser::document::Document;
use crate::utils::errors::ParserResult;

pub fn validate(file: String) -> ParserResult<()> {
    let contents = std::fs::read_to_string(&file)?;

    let (name, doc) = Document::parse(&contents, &file)?;

    match doc.validate(&file, None, None, None, None) {
        Ok(_) => println!("✅ Document: {name} is valid."),
        Err(e) => eprintln!("🙅 Document: {name} is invalid. {e}"),
    }

    Ok(())
}

pub fn parse(file: String) -> ParserResult<()> {
    let contents = std::fs::read_to_string(&file)?;
    let doc = Document::parse(&contents, &file)?;
    println!("{}", serde_json::to_string_pretty(&doc)?);
    Ok(())
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Validate { file: String },
    Parse { file: String },
}
