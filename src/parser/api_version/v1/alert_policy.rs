use super::alert_condition::AlertConditionDoc;
use super::alert_notification_target::AlertNotificationTargetDoc;

use super::common::{Kind, Metadata};
use crate::utils::errors::{ParserError, ParserResult};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AlertPolicyDoc {
    pub kind: Kind,
    pub metadata: Metadata,
    pub spec: AlertPolicySpec,
}

impl AlertPolicyDoc {
    pub fn validate(
        &self,
        condition_map: Option<&HashMap<String, AlertConditionDoc>>,
        notification_target_map: Option<&HashMap<String, AlertNotificationTargetDoc>>,
        path: Option<&str>,
    ) -> ParserResult<()> {
        let path = path.unwrap_or("AlertPolicy");

        if !matches!(self.kind, Kind::AlertPolicy) {
            return Err(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "SLI must have a kind.".to_string(),
            });
        }

        self.spec.validate(
            condition_map,
            notification_target_map,
            &format!("{path}.spec"),
        )
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum AlertCondition {
    Inline(Box<AlertConditionDoc>),
    Reference(AlertConditionRef),
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AlertConditionRef {
    #[serde(rename = "conditionRef")]
    pub condition_ref: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum NotificationTarget {
    Inline(AlertNotificationTargetDoc),
    Reference(NotificationTargetRef),
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct NotificationTargetRef {
    #[serde(rename = "targetRef")]
    pub target_ref: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AlertPolicySpec {
    pub description: Option<String>,
    #[serde(default, rename = "alertWhenNoData")]
    pub alert_when_no_data: bool,
    #[serde(default, rename = "alertWhenResolved")]
    pub alert_when_resolved: bool,
    #[serde(default, rename = "alertWhenBreaching")]
    pub alert_when_breaching: bool,
    pub(crate) conditions: Vec<AlertCondition>,
    #[serde(rename = "notificationTargets")]
    pub notification_targets: Vec<NotificationTarget>,
}

impl AlertPolicySpec {
    pub fn validate(
        &self,
        condition_map: Option<&HashMap<String, AlertConditionDoc>>,
        notification_target_map: Option<&HashMap<String, AlertNotificationTargetDoc>>,
        path: &str,
    ) -> ParserResult<()> {
        //currently condition only accepts a single value

        if self.conditions.len() != 1 {
            return Err(ParserError::Validation {
                path: format!("{path}.condition"),
                message: "Condition must contain exactly one item.".to_string(),
            });
        }

        match &self.conditions[0] {
            AlertCondition::Reference(reference) => {
                let reference = &reference.condition_ref;
                if let Some(condition_map) = condition_map {
                    condition_map
                        .get(reference)
                        .ok_or_else(|| ParserError::Validation {
                            path: format!("{path}.conditions"),
                            message: format!("Condition reference `{}` not found.", reference),
                        })?;
                }
            }
            AlertCondition::Inline(inline_condition) => {
                inline_condition.validate(Some(path))?;
            }
        }

        if self.notification_targets.is_empty() {
            return Err(ParserError::Validation {
                path: format!("{path}.notificationTargets"),
                message: "Notification targets must not be empty.".to_string(),
            });
        }

        for (i, target) in self.notification_targets.iter().enumerate() {
            match target {
                NotificationTarget::Reference(reference) => {
                    let reference = &reference.target_ref;
                    if let Some(notification_target_map) = notification_target_map {
                        notification_target_map.get(reference).ok_or_else(|| {
                            ParserError::Validation {
                                path: format!("{path}.notificationTargets[{}].targetRef", i),
                                message: format!(
                                    "Notification target reference `{}` not found.",
                                    reference
                                ),
                            }
                        })?;
                    }
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

    use super::super::alert_condition::{AlertConditionSpec, Condition, CondtionKind};
    use super::super::alert_notification_target::AlertNotificationTargetSpec;
    use super::super::common::{DurationShorthand, Operator};

    fn create_condition() -> AlertConditionDoc {
        AlertConditionDoc {
            spec: AlertConditionSpec {
                condition: Condition {
                    kind: CondtionKind::Burnrate,
                    op: Some(Operator::Gte),
                    threshold: Some(1.5),
                    lookback_window: Some(DurationShorthand("5m".to_string())),
                    alert_after: Some(DurationShorthand("0m".to_string())),
                },
                description: Some("High CPU usage".to_string()),
                severity: "high".to_string(),
            },
            kind: Kind::AlertCondition,
            metadata: Metadata {
                name: "test".to_string(),
                display_name: None,
                labels: None,
                annotations: None,
            },
        }
    }
    fn create_target() -> AlertNotificationTargetDoc {
        AlertNotificationTargetDoc {
            spec: AlertNotificationTargetSpec {
                target: "slack".to_string(),
                description: Some("Slack channel".to_string()),
            },
            kind: Kind::AlertNotificationTarget,
            metadata: Metadata {
                name: "test".to_string(),
                display_name: None,
                labels: None,
                annotations: None,
            },
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

        let policy: AlertPolicyDoc = serde_yaml::from_str(yaml).unwrap();

        let result = policy.validate(Some(&condition_map), Some(&notification_target_map), None);
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

        let mut notification_target_map = HashMap::new();
        notification_target_map.insert("pagerduty-notify".to_string(), create_target());

        let policy: AlertPolicyDoc = serde_yaml::from_str(yaml).unwrap();

        let result = policy.validate(None, Some(&notification_target_map), None);

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

        let mut notification_target_map = HashMap::new();
        notification_target_map.insert("pagerduty-notify".to_string(), create_target());

        let policy: AlertPolicyDoc = serde_yaml::from_str(yaml).unwrap();

        let result = policy.validate(None, Some(&notification_target_map), None);

        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod unhappy_path_tests {
    use super::super::{
        alert_condition::{AlertConditionSpec, Condition, CondtionKind},
        common::{DurationShorthand, Operator},
    };

    use super::*;

    fn create_condition() -> AlertConditionDoc {
        AlertConditionDoc {
            spec: AlertConditionSpec {
                condition: Condition {
                    kind: CondtionKind::Burnrate,
                    op: Some(Operator::Gte),
                    threshold: Some(1.5),
                    lookback_window: Some(DurationShorthand("5m".to_string())),
                    alert_after: Some(DurationShorthand("0m".to_string())),
                },
                description: Some("High CPU usage".to_string()),
                severity: "high".to_string(),
            },
            kind: Kind::AlertCondition,
            metadata: Metadata {
                name: "test".to_string(),
                display_name: None,
                labels: None,
                annotations: None,
            },
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

        let policy: Result<AlertPolicyDoc, serde_yaml::Error> = serde_yaml::from_str(yaml);

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

        let alert_policy: AlertPolicyDoc = serde_yaml::from_str(yaml).unwrap();

        let result = alert_policy.validate(None, None, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message } if
                path == "AlertPolicy.spec.condition"
                    && message == "Condition must contain exactly one item."
            ))
        );
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

        let alert_policy: AlertPolicyDoc = serde_yaml::from_str(yaml).unwrap();

        let result = alert_policy.validate(Some(&condition_map), None, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "AlertPolicy.spec.notificationTargets"
                    && message == "Notification targets must not be empty."
            ))
        );
    }
}
