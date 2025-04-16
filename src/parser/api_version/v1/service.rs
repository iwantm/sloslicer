use serde::{Deserialize, Serialize};

use super::common::{Kind, Metadata};
use crate::utils::errors::ParserResult;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ServiceDoc {
    pub kind: Option<Kind>,
    pub metadata: Metadata,
    pub spec: ServiceSpec,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct ServiceSpec {
    pub description: Option<String>,
}

impl ServiceDoc {
    pub fn validate(&self, path: &str) -> ParserResult<()> {
        Ok(())
    }
}
