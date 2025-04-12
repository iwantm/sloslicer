use super::validation::{ValidationError, ValidationResult};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NotificationTargetSpec {
    Inline(AlertNotificationTargetSpec),
    Reference(String),
}

#[derive(Debug, Deserialize)]
pub struct AlertNotificationTargetSpec {
    target: String,
    description: Option<String>,
}

impl AlertNotificationTargetSpec {
    pub fn validate(&self, path: &str) -> ValidationResult {
        if self.target.is_empty() {
            return Err(ValidationError::new(
                format!("{path}.target"),
                "Target must not be empty.",
            ));
        }
        Ok(())
    }
}
