use serde::Deserialize;

use super::document::Metadata;
use crate::parser::validation::{ValidationError, ValidationResult};

#[derive(Debug, Deserialize, PartialEq)]
pub struct DataSourceDoc {
    pub kind: String,
    pub metadata: Metadata,
    pub spec: DataSourceSpec,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct DataSourceSpec {
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "connectionDetails")]
    pub connection_details: Option<serde_yaml::Value>,
}

impl DataSourceDoc {
    pub fn validate(&self, path: &str) -> ValidationResult {
        if self.kind != "DataSource" {
            return Err(ValidationError::new(
                format!("{path}.kind"),
                "Invalid kind specified. Expected `DataSource`.",
            ));
        }

        if self.spec.type_.trim().is_empty() {
            return Err(ValidationError::new(
                format!("{path}.spec.type"),
                "Type must be a non-empty string.",
            ));
        }

        Ok(())
    }
}
