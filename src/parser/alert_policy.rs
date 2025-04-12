use super::alert_condition::AlertConditionSpec;
use super::alert_notification_target::{AlertNotificationTargetSpec, NotificationTargetSpec};
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
