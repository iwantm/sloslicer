use serde::Deserialize;

use super::document::Metadata;
use crate::parser::validation::{ValidationError, ValidationResult};

#[derive(Debug, Deserialize, PartialEq)]
pub struct ServiceDoc {
    pub kind: String,
    pub metadata: Metadata,
    pub spec: ServiceSpec,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ServiceSpec {
    pub description: Option<String>,
}

impl ServiceDoc {
    pub fn validate(&self, path: &str) -> ValidationResult {
        if self.kind != "Service" {
            return Err(ValidationError::new(
                format!("{path}.kind"),
                "Invalid kind specified. Expected `Service`.",
            ));
        }

        Ok(())
    }
}
