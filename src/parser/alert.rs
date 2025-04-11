use super::common::{DurationShorthand, Operator};
use super::validation::{ValidationError, ValidationResult};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AlertCondition {
    Inline(AlertConditionSpec),
    Reference(String),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NotificationTargetSpec {
    Inline(AlertNotificationTargetSpec),
    Reference(String),
}

#[derive(Debug, Deserialize)]
pub struct AlertPolicySpec {
    pub description: Option<String>,
    #[serde(default, rename = "alertWhenNoData")]
    pub alert_when_no_data: bool,
    #[serde(default, rename = "alertWhenResolved")]
    pub alert_when_resolved: bool,
    #[serde(default, rename = "alertWhenBreaching")]
    pub alert_when_breaching: bool,
    condition: Vec<AlertCondition>,
    #[serde(rename = "notificationTargets")]
    pub notification_targets: Vec<NotificationTargetSpec>,
}

impl AlertPolicySpec {
    pub fn validate(
        &self,
        condition_map: &HashMap<String, AlertConditionSpec>,
        notification_target_map: &HashMap<String, AlertNotificationTargetSpec>,
        path: &str,
    ) -> ValidationResult {
        //currently condition only accepts a single value
        if self.condition.len() != 1 {
            return Err(ValidationError::new(
                format!("{path}.condition"),
                "Condition must contain exactly one item.",
            ));
        }

        match &self.condition[0] {
            AlertCondition::Reference(reference) => {
                let condition = condition_map.get(reference).ok_or_else(|| {
                    return ValidationError::new(
                        format!("{path}.condition"),
                        format!("Condition reference `{}` not found.", reference),
                    );
                })?;

                condition.validate(&format!("{path}.condition[{}]", reference))?;
            }
            AlertCondition::Inline(inline_condition) => {
                inline_condition.validate(&format!("{path}.condition"))?;
            }
        }

        for (i, target) in self.notification_targets.iter().enumerate() {
            match target {
                NotificationTargetSpec::Reference(reference) => {
                    let notification_target =
                        notification_target_map.get(reference).ok_or_else(|| {
                            return ValidationError::new(
                                format!("{path}.notificationTargets"),
                                format!("Notification target reference `{}` not found.", reference),
                            );
                        })?;

                    notification_target
                        .validate(&format!("{path}.notificationTargets[{}]", reference))?;
                }
                NotificationTargetSpec::Inline(inline_target) => {
                    inline_target.validate(&format!("{path}.notificationTargets"))?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Condition {
    kind: String,
    op: Operator,
}

#[derive(Debug, Deserialize)]
pub struct AlertConditionSpec {
    description: Option<String>,
    severity: String,
    condition: Condition,
    threshold: f64,
    #[serde(rename = "lookbackWindow")]
    lookback_window: DurationShorthand,
    #[serde(rename = "alertAfter")]
    alert_after: DurationShorthand,
}

impl AlertConditionSpec {
    pub fn validate(&self, path: &str) -> ValidationResult {
        if matches!(self.condition.op, Operator::Lte) && self.threshold > 1.0 {
            return Err(ValidationError::new(
                format!("{path}.threshold"),
                "Threshold must be between 0 and 1 for Lte operator.",
            ));
        }

        Ok(())
    }
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
