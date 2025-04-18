use std::collections::HashMap;

use serde::Deserialize;
use serde_yaml::{Deserializer, Value};

use crate::parser::document::Document;
use crate::utils::errors::{ParserError, ParserResult};
use crate::utils::validation_context::{ValidationContext, ValidationResult};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn find_documents(path_string: &str, recursive: bool) -> ParserResult<Vec<PathBuf>> {
    let path = Path::new(&path_string);
    let mut files = vec![];

    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        let walker = if recursive {
            WalkDir::new(path).into_iter()
        } else {
            WalkDir::new(path).max_depth(1).into_iter()
        };

        let files_iter = walker.filter_map(|entry| {
            entry
                .ok()
                .and_then(|e| if e.path().is_file() { Some(e) } else { None })
        });

        for file in files_iter {
            let extension = file.path().extension().and_then(|e| e.to_str());

            if matches!(extension, Some("yaml" | "yml")) {
                files.push(file.path().to_path_buf());
            }
        }
    } else {
        return Err(ParserError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No such file or directory: {}", path_string),
        )));
    }

    Ok(files)
}

pub fn parse_files(
    file: &PathBuf,
    ctx: &mut ValidationContext,
) -> ParserResult<(ValidationResult, HashMap<String, Document>)> {
    let mut docs = HashMap::new();

    let contents = std::fs::read_to_string(file)?;
    let path_string = match file.as_path().to_str() {
        Some(path_string) => path_string,
        None => {
            return Err(ParserError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Couldn't parse path string.".to_string(),
            )));
        }
    };

    let validation_result = ValidationResult::new(path_string.to_string());

    for doc in Deserializer::from_str(&contents) {
        let value = Value::deserialize(doc)?;

        let (name, parsed_doc) = Document::parse(value, path_string, ctx)?;
        docs.insert(name, parsed_doc);
    }

    Ok((validation_result, docs))
}

pub fn validate(path_string: String, recursive: bool, quiet: bool) -> ParserResult<()> {
    let files: Vec<PathBuf> = find_documents(&path_string, recursive)?;
    let mut results = vec![];
    let mut all_docs = HashMap::new();

    for file in &files {
        let mut file_ctx = ValidationContext::new();
        let (mut result, docs) = parse_files(file, &mut file_ctx)?;
        all_docs.extend(docs.clone());

        for (name, doc) in &docs {
            let mut ctx = ValidationContext::new();
            ctx.combine(&mut file_ctx);
            doc.validate(&all_docs, &mut ctx);
            match ctx.result() {
                Ok(_) => {
                    result.add_valid(name);
                }
                Err(e) => {
                    result.add_invalid(name, e);
                }
            }
        }
        if !quiet {
            print!("{}", result);
        }
        results.push(result);
    }

    if results.iter().any(|f| f.result().is_err()) {
        std::process::exit(1);
    } else {
        Ok(())
    }
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Validate {
        file: String,
        #[clap(long)]
        recursive: bool,
        #[clap(long)]
        quiet: bool,
    },
}
