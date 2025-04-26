use std::collections::HashMap;

use serde::Deserialize;
use serde_yaml::{Deserializer, Value};

use crate::parser::document::Document;
use crate::utils::errors::{ParserError, ParserResult};
use crate::utils::validation_context::{ValidationContext, ValidationResult};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

pub type DocumentParseResult = (
    ValidationResult,
    HashMap<String, Document>,
    HashMap<String, ValidationContext>,
);

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

        let files_vec: Vec<PathBuf> = files_iter
            .par_bridge()
            .filter_map(|file| {
                if matches!(
                    file.path().extension().and_then(|e| e.to_str()),
                    Some("yaml" | "yml")
                ) {
                    Some(file.path().to_path_buf())
                } else {
                    None
                }
            })
            .collect();

        files.extend(files_vec);
    } else {
        return Err(ParserError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No such file or directory: {}", path_string),
        )));
    }

    Ok(files)
}

pub fn parse_files(file: &PathBuf) -> ParserResult<DocumentParseResult> {
    let mut docs = HashMap::new();
    let mut ctxs = HashMap::new();

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
        let mut ctx = ValidationContext::new();
        let value = Value::deserialize(doc)?;

        let (name, parsed_doc) = Document::parse(value, path_string, &mut ctx)?;
        docs.insert(name.clone(), parsed_doc);
        ctxs.insert(name, ctx);
    }

    Ok((validation_result, docs, ctxs))
}

pub fn validate(path_string: String, recursive: bool, quiet: bool) -> ParserResult<()> {
    // let mut results = vec![];

    let files: Vec<PathBuf> = find_documents(&path_string, recursive)?;

    let parsed_results: Vec<DocumentParseResult> = files
        .par_iter()
        .map(|file| {
            parse_files(file)
                .map_err(|e| eprintln!("{}", e))
                .expect("Failed to parse file")
        })
        .collect();

    let mut all_docs = HashMap::new();
    for (_result, docs, _ctxs) in &parsed_results {
        all_docs.extend(docs.clone());
    }
    let all_docs = Arc::new(all_docs);

    let results: Vec<_> = parsed_results
        .into_par_iter()
        .map(|(mut result, docs, mut ctxs)| {
            for (name, doc) in &docs {
                let parse_ctx = ctxs.get_mut(name).unwrap();
                let mut ctx = ValidationContext::new();
                ctx.combine(parse_ctx);

                doc.validate(name, &all_docs, &mut ctx);

                match ctx.result() {
                    Ok(_) => result.add_valid(name),
                    Err(e) => result.add_invalid(name, e),
                }
            }

            if !quiet {
                print!("{}", result);
            }

            result
        })
        .collect();

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
