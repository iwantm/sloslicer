use thiserror::Error;

#[derive(Error, Debug)]

pub enum ParserError {
    #[error("Failed to deserialise yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Validation failed at {path} with message: {message}")]
    Validation { path: String, message: String },
}

pub type ParserResult<T> = Result<T, ParserError>;
