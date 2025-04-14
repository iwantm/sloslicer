use thiserror::Error;

#[derive(Error, Debug)]

pub enum ParserError {
    #[error("Failed to deserialise yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Failed to serialise json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Validation failed at {path} with message: {message}")]
    Validation { path: String, message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ParserResult<T> = Result<T, ParserError>;
