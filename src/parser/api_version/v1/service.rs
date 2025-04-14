use serde::{Deserialize, Serialize};

use super::common::Metadata;
use crate::utils::errors::{ParserError, ParserResult};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ServiceDoc {
    pub kind: String,
    pub metadata: Metadata,
    pub spec: ServiceSpec,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ServiceSpec {
    pub description: Option<String>,
}

impl ServiceDoc {
    pub fn validate(&self, path: &str) -> ParserResult<()> {
        if self.kind != "Service" {
            return Err(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Expected kind to be Service.".to_string(),
            });
        }

        Ok(())
    }
}
