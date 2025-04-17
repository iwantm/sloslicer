use std::collections::HashMap;

use owo_colors::OwoColorize;
use serde::Deserialize;
use serde_yaml::{Deserializer, Value};

use crate::parser::document::Document;
use crate::utils::errors::ParserResult;
use crate::utils::validation_context::ValidationContext;

pub fn validate(file: String) -> ParserResult<()> {
    let contents = std::fs::read_to_string(&file)?;

    let mut docs = HashMap::new();

    for doc in Deserializer::from_str(&contents) {
        let value = Value::deserialize(doc)?;

        let (name, parsed_doc) = Document::parse(value, &file)?;
        docs.insert(name, parsed_doc);
    }

    let mut invalid = 0;
    let mut valid = 0;

    for (name, doc) in &docs {
        let mut ctx = ValidationContext::new();
        doc.validate(&docs, &mut ctx);
        match ctx.result() {
            Ok(_) => {
                valid += 1;
                println!("[✅] {}.{} - {}", file, name, "Valid".green())
            }
            Err(e) => {
                invalid += 1;
                eprintln!("[🙅] {}.{} - {}", file, name, "Invalid".red());
                for e in e {
                    eprintln!("↪ {}", e)
                }
            }
        }
    }

    println!(
        "\nValidation Summary: \nValid: {}\nInvalid: {}",
        valid, invalid
    );

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
