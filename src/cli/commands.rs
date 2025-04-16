use std::collections::HashMap;

use serde::Deserialize;
use serde_yaml::{Deserializer, Value};

use crate::parser::document::Document;
use crate::utils::errors::ParserResult;

pub fn validate(file: String) -> ParserResult<()> {
    let contents = std::fs::read_to_string(&file)?;

    let mut docs = HashMap::new();

    for doc in Deserializer::from_str(&contents) {
        let value = Value::deserialize(doc)?;

        let (name, parsed_doc) = Document::parse(value, &file)?;
        docs.insert(name, parsed_doc);
    }

    for (name, doc) in &docs {
        match doc.validate(&format!("{}.{}", file, name), &docs) {
            Ok(_) => println!("✅ Document: {name} is valid."),
            Err(e) => eprintln!("🙅 Document: {name} is invalid. {e}"),
        }
    }

    Ok(())
}

pub fn parse(file: String) -> ParserResult<()> {
    let contents = std::fs::read_to_string(&file)?;
    let mut docs = HashMap::new();

    for doc in Deserializer::from_str(&contents) {
        let value = Value::deserialize(doc)?;

        let (name, parsed_doc) = Document::parse(value, &file)?;
        docs.insert(name, parsed_doc);
    }

    for (name, doc) in docs {
        println!("{name}: {}", serde_json::to_string_pretty(&doc)?);
    }

    Ok(())
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Validate { file: String },
    Parse { file: String },
}
