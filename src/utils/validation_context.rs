use std::{collections::HashMap, fmt::Display};

use super::errors::ParserError;

pub struct ValidationContext {
    errors: Vec<ParserError>,
}

impl ValidationContext {
    pub fn new() -> Self {
        Self { errors: vec![] }
    }

    pub fn push(&mut self, error: ParserError) {
        self.errors.push(error);
    }

    pub fn combine(&mut self, other: &mut Self) {
        self.errors.append(&mut other.errors);
    }

    pub fn result(self) -> Result<(), Vec<ParserError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }
}

pub struct ValidationResult {
    path: String,
    valid: Vec<String>,
    invalid: HashMap<String, Vec<ParserError>>,
}

impl ValidationResult {
    pub fn new(path: String) -> Self {
        Self {
            path,
            valid: vec![],
            invalid: HashMap::new(),
        }
    }
    pub fn add_valid(&mut self, name: &str) {
        self.valid.push(name.to_string());
    }

    pub fn add_invalid(&mut self, name: &str, errors: Vec<ParserError>) {
        self.invalid.insert(name.to_string(), errors);
    }

    pub fn result(&self) -> Result<(), ()> {
        if self.invalid.is_empty() {
            Ok(())
        } else {
            Err(())
        }
    }
}

impl Display for ValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[📄] {}:", self.path)?;

        for file in &self.valid {
            writeln!(f, "   ↳[✅] {}", file)?;
        }

        for (file, errors) in &self.invalid {
            writeln!(f, "   ↳[❌] {}:", file)?;
            for err in errors {
                writeln!(f, "        ↳ {}", err)?;
            }
        }
        Ok(())
    }
}
