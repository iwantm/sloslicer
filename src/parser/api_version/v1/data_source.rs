use serde::Deserialize;

use super::common::{Kind, Metadata};
use crate::parser::errors::{ParserError, ParserResult};

#[derive(Debug, Deserialize, PartialEq)]
pub struct DataSourceDoc {
    pub kind: Kind,
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
    pub fn validate(&self, path: Option<&str>) -> ParserResult<()> {
        let path = path.unwrap_or("DataSource");

        if !matches!(self.kind, Kind::DataSource) {
            return Err(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Invalid kind specified. Expected `DataSource`.".to_string(),
            });
        }

        if self.spec.type_.trim().is_empty() {
            return Err(ParserError::Validation {
                path: format!("{path}.spec.type"),
                message: "Type must be a non-empty string.".to_string(),
            });
        }

        Ok(())
    }
}
