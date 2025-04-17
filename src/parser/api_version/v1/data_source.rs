use serde::{Deserialize, Serialize};

use super::common::{Kind, Metadata};
use crate::utils::{errors::ParserError, validation_context::ValidationContext};

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct DataSourceDoc {
    pub kind: Option<Kind>,
    pub metadata: Metadata,
    pub spec: DataSourceSpec,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct DataSourceSpec {
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "connectionDetails")]
    pub connection_details: Option<serde_yaml::Value>,
}

impl DataSourceDoc {
    pub fn validate(&self, path: Option<&str>, ctx: &mut ValidationContext) {
        let path = path.unwrap_or("");

        if !matches!(self.kind, Some(Kind::DataSource)) {
            ctx.push(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Expected kind to be DataSource.".to_string(),
            });
        };

        if self.spec.type_.trim().is_empty() {
            ctx.push(ParserError::Validation {
                path: format!("{path}.spec.type"),
                message: "Type must be a non-empty string.".to_string(),
            });
        }
    }
}
