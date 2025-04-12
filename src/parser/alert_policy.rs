use super::alert_condition::{AlertConditionDocument, AlertConditionSpec};
use super::alert_notification_target::{
    AlertNotificationTargetDocument, AlertNotificationTargetSpec,
};

use super::document::{Kind, Metadata};
use super::validation::{ValidationError, ValidationResult};
use serde::Deserialize;

use std::collections::HashMap;

#[derive(Debug, Deserialize, PartialEq)]
pub struct AlertPolicyDocument {
    pub kind: Kind,
    pub metadata: Metadata,
    pub spec: AlertPolicySpec,
}

impl AlertPolicyDocument {
    pub fn validate(
        &self,
        condition_map: &HashMap<String, AlertConditionSpec>,
        notification_target_map: &HashMap<String, AlertNotificationTargetSpec>,
        path: Option<String>,
    ) -> ValidationResult {
        if !matches!(self.kind, Kind::AlertPolicy) {
            return Err(ValidationError::new(
                "kind",
                "Invalid kind specified. Expected `AlertPolicy`.",
            ));
        }

        match path {
            Some(path) => self
                .spec
                .validate(condition_map, notification_target_map, &path),
            None => self
                .spec
                .validate(condition_map, notification_target_map, "alert_policy"),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AlertCondition {
    Inline(AlertConditionDocument),
    Reference(AlertConditionRef),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AlertConditionRef {
    #[serde(rename = "conditionRef")]
    pub condition_ref: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum NotificationTarget {
    Inline(AlertNotificationTargetDocument),
    Reference(NotificationTargetRef),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct NotificationTargetRef {
    #[serde(rename = "targetRef")]
    pub target_ref: String,
}

#[derive(Debug, Deserialize, PartialEq)]
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
        kind: AlertPolicy
        metadata:
            name: cpu-usage-policy
        spec:
            description: CPU usage policy
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

        let policy: AlertPolicyDocument = serde_yaml::from_str(yaml).unwrap();

        let result = policy.validate(&condition_map, &notification_target_map, None);
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_alert_policy_spec_inline_condition() {
        let yaml = r#"
        kind: AlertPolicy
        metadata:
            name: latency-policy
        spec:
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

        let condition_map = HashMap::new();

        let mut notification_target_map = HashMap::new();
        notification_target_map.insert("pagerduty-notify".to_string(), create_target());

        let policy: AlertPolicyDocument = serde_yaml::from_str(yaml).unwrap();

        let result = policy.validate(&condition_map, &notification_target_map, None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_alert_policy_spec_inline_condition_and_notification_target() {
        let yaml = r#"
        kind: AlertPolicy
        metadata:
            name: latency-policy
        spec:
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
                - kind: AlertNotificationTarget
                  metadata:
                    name: slack-notify
                  spec:
                    target: slack
                    description: Slack channel
        "#;

        let condition_map = HashMap::new();

        let mut notification_target_map = HashMap::new();
        notification_target_map.insert("pagerduty-notify".to_string(), create_target());

        let policy: AlertPolicyDocument = serde_yaml::from_str(yaml).unwrap();

        let result = policy.validate(&condition_map, &notification_target_map, None);

        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod unhappy_path_tests {
    use crate::parser::{
        alert_condition::{Condition, CondtionKind},
        common::{DurationShorthand, Operator},
    };

    use super::*;

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
    fn test_missing_conditions() {
        let yaml = r#"
        kind: AlertPolicy
        metadata:
            name: missing-conditions
        spec:
            alertWhenNoData: true
            alertWhenResolved: true
            alertWhenBreaching: true
            notificationTargets:
                - targetRef: slack
        "#;

        let policy: Result<AlertPolicyDocument, serde_yaml::Error> = serde_yaml::from_str(yaml);

        assert!(policy.is_err_and(|e| e.to_string().contains("spec: missing field `conditions`")));
    }

    #[test]
    fn test_multiple_conditions() {
        let yaml = r#"
        kind: AlertPolicy
        metadata:
            name: too-many-conditions
        spec:
            alertWhenNoData: true
            alertWhenResolved: false
            alertWhenBreaching: true
            conditions:
                - conditionRef: cpu-usage
                - conditionRef: memory-usage
            notificationTargets:
                - targetRef: opsgenie
        "#;

        let condition_map = HashMap::new();
        let notification_target_map = HashMap::new();

        let alert_policy: AlertPolicyDocument = serde_yaml::from_str(yaml).unwrap();

        let result = alert_policy.validate(&condition_map, &notification_target_map, None);

        println!("{:?}", result);
        assert!(result.is_err_and(|e| {
            e.path == "alert_policy.condition"
                && e.message == "Condition must contain exactly one item."
        }));
    }

    #[test]
    fn test_empty_notifications() {
        let yaml = r#"
        kind: AlertPolicy
        metadata:
            name: no-targets
        spec:
            alertWhenNoData: false
            alertWhenResolved: false
            alertWhenBreaching: true
            conditions:
                - conditionRef: slo-alert
            notificationTargets: []
        "#;

        let mut condition_map = HashMap::new();
        condition_map.insert("slo-alert".to_string(), create_condition());
        let notification_target_map = HashMap::new();

        let alert_policy: AlertPolicyDocument = serde_yaml::from_str(yaml).unwrap();

        let result = alert_policy.validate(&condition_map, &notification_target_map, None);

        println!("{:?}", result);
        assert!(result.is_err_and(|e| {
            e.path == "alert_policy.notificationTargets"
                && e.message == "Notification targets must not be empty."
        }));
    }
}
