use super::alert_condition::{AlertConditionDocument, AlertConditionSpec};
use super::alert_notification_target::{
    AlertNotificationTargetDocument, AlertNotificationTargetSpec,
};

use super::validation::{ValidationError, ValidationResult};
use serde::Deserialize;

use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AlertCondition {
    Inline(AlertConditionDocument),
    Reference(AlertConditionRef),
}

#[derive(Debug, Deserialize)]
pub struct AlertConditionRef {
    #[serde(rename = "conditionRef")]
    pub condition_ref: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NotificationTarget {
    Inline(AlertNotificationTargetDocument),
    Reference(NotificationTargetRef),
}

#[derive(Debug, Deserialize)]
pub struct NotificationTargetRef {
    #[serde(rename = "targetRef")]
    pub target_ref: String,
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
    conditions: Vec<AlertCondition>,
    #[serde(rename = "notificationTargets")]
    pub notification_targets: Vec<NotificationTarget>,
}

impl AlertPolicySpec {
    pub fn validate(
        &self,
        condition_map: &HashMap<String, AlertConditionSpec>,
        notification_target_map: &HashMap<String, AlertNotificationTargetSpec>,
        path: &str,
    ) -> ValidationResult {
        //currently condition only accepts a single value

        if self.conditions.len() != 1 {
            return Err(ValidationError::new(
                format!("{path}.condition"),
                "Condition must contain exactly one item.",
            ));
        }

        match &self.conditions[0] {
            AlertCondition::Reference(reference) => {
                let reference = &reference.condition_ref;
                let condition = condition_map.get(reference).ok_or_else(|| {
                    return ValidationError::new(
                        format!("{path}.conditions"),
                        format!("Condition reference `{}` not found.", reference),
                    );
                })?;

                condition.validate(&format!("{path}.conditions[{}].conditionRef", reference))?;
            }
            AlertCondition::Inline(inline_condition) => {}
        }

        if self.notification_targets.is_empty() {
            return Err(ValidationError::new(
                format!("{path}.notificationTargets"),
                "Notification targets must not be empty.",
            ));
        }

        for (i, target) in self.notification_targets.iter().enumerate() {
            match target {
                NotificationTarget::Reference(reference) => {
                    let reference = &reference.target_ref;

                    let notification_target =
                        notification_target_map.get(reference).ok_or_else(|| {
                            return ValidationError::new(
                                format!("{path}.notificationTargets[{}].targetRef", i),
                                format!("Notification target reference `{}` not found.", reference),
                            );
                        })?;

                    notification_target
                        .validate(&format!("{path}.notificationTargets[{}].targetRef", i))?;
                }
                NotificationTarget::Inline(inline_target) => {
                    inline_target
                        .spec
                        .validate(&format!("{path}.notificationTargets[{}]", i))?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod happy_path_tests {
    use super::*;

    use crate::parser::alert_condition::{AlertConditionSpec, Condition, CondtionKind};
    use crate::parser::common::{DurationShorthand, Operator};

    fn create_condition() -> AlertConditionSpec {
        AlertConditionSpec {
            condition: Condition {
                kind: CondtionKind::Burnrate,
                op: Some(Operator::Gte),
                threshold: Some(1.5),
                lookback_window: Some(DurationShorthand("5m".to_string())),
                alert_after: Some(DurationShorthand("0m".to_string())),
            },
            description: Some("High CPU usage".to_string()),
            severity: "high".to_string(),
        }
    }
    fn create_target() -> AlertNotificationTargetSpec {
        AlertNotificationTargetSpec {
            target: "slack".to_string(),
            description: Some("Slack channel".to_string()),
        }
    }

    #[test]
    fn test_alert_policy_spec_referenced_condition() {
        let yaml = r#"
        description: Simple reference policy
        alertWhenNoData: true
        alertWhenResolved: false
        alertWhenBreaching: true
        conditions:
            - conditionRef: cpu-usage-breach
        notificationTargets:
            - targetRef: on-call-slack
        "#;

        let mut condition_map = HashMap::new();
        condition_map.insert("cpu-usage-breach".to_string(), create_condition());

        let mut notification_target_map = HashMap::new();
        notification_target_map.insert("on-call-slack".to_string(), create_target());

        let policy: AlertPolicySpec = serde_yaml::from_str(yaml).unwrap();

        let result = policy.validate(&condition_map, &notification_target_map, "test");
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_alert_policy_spec_inline_condition() {
        let yaml = r#"
        description: Inlined condition, referenced target
        alertWhenNoData: false
        alertWhenResolved: true
        alertWhenBreaching: false
        conditions:
            - kind: AlertCondition
              metadata:
                name: high-latency
              spec:
                severity: page
                condition:
                    kind: burnrate
                    op: gte
                    threshold: 1.5
                    lookbackWindow: 5m
        notificationTargets:
            - targetRef: pagerduty-notify
        "#;

        let mut notification_target_map = HashMap::new();
        notification_target_map.insert("pagerduty-notify".to_string(), create_target());

        let policy: Result<AlertPolicySpec, serde_yaml::Error> = serde_yaml::from_str(yaml);

        assert!(policy.is_ok());
        // assert!(false == true);
    }
}
