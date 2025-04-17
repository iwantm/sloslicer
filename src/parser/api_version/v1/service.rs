use serde::{Deserialize, Serialize};

use super::common::{Kind, Metadata};
use crate::utils::{errors::ParserError, validation_context::ValidationContext};

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
    pub fn validate(&self, ctx: &mut ValidationContext) {
        if !matches!(self.kind, Some(Kind::Service)) {
            ctx.push(ParserError::Validation {
                path: ".kind".to_string(),
                message: "Expected kind to be Service.".to_string(),
            });
        };
    }
}
