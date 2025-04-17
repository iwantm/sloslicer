use super::alert_condition::AlertConditionDoc;
use super::alert_notification_target::AlertNotificationTargetDoc;

use super::common::{Kind, Metadata};
use crate::parser::document::Document;
use crate::utils::errors::ParserError;
use crate::utils::validation_context::ValidationContext;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct AlertPolicyDoc {
    pub kind: Option<Kind>,
    pub metadata: Metadata,
    pub spec: AlertPolicySpec,
}

impl AlertPolicyDoc {
    pub fn validate(
        &self,
        path: Option<&str>,
        document_map: &HashMap<String, Document>,
        ctx: &mut ValidationContext,
    ) {
        let path = path.unwrap_or("");

        if !matches!(self.kind, Some(Kind::AlertPolicy)) {
            ctx.push(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Expected kind to be AlertPolicy.".to_string(),
            });
        };

        self.spec
            .validate(&format!("{path}.spec"), document_map, ctx);
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
#[serde(untagged)]
pub enum AlertCondition {
    Reference(AlertConditionRef),
    Inline(Box<AlertConditionDoc>),
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct AlertConditionRef {
    #[serde(rename = "conditionRef")]
    pub condition_ref: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
#[serde(untagged)]
pub enum NotificationTarget {
    Reference(NotificationTargetRef),
    Inline(AlertNotificationTargetDoc),
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct NotificationTargetRef {
    #[serde(rename = "targetRef")]
    pub target_ref: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
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
        path: &str,
        document_map: &HashMap<String, Document>,
        ctx: &mut ValidationContext,
    ) {
        //currently condition only accepts a single value

        if self.conditions.len() != 1 {
            ctx.push(ParserError::Validation {
                path: format!("{path}.condition"),
                message: "Condition must contain exactly one item.".to_string(),
            });
        }

        match &self.conditions[0] {
            AlertCondition::Reference(reference) => {
                let reference = &reference.condition_ref;

                if let Some(document) = document_map.get(reference) {
                    match document {
                        Document::AlertCondition(alert_condition_doc) => {
                            alert_condition_doc.validate(Some(path), ctx);
                        }
                        _ => {
                            ctx.push(ParserError::Validation {
                                path: format!("{path}.conditions[0].targetRef"),
                                message: format!("Condition reference `{}` wrong type.", reference),
                            });
                        }
                    }
                } else {
                    ctx.push(ParserError::Validation {
                        path: format!("{path}.conditions[0].targetRef"),
                        message: format!("Condition reference `{}` not found.", reference),
                    });
                }
            }
            AlertCondition::Inline(inline_condition) => {
                inline_condition.validate(Some(&format!("{}.conditions.[0]", path)), ctx);
            }
        }

        if self.notification_targets.is_empty() {
            ctx.push(ParserError::Validation {
                path: format!("{path}.notificationTargets"),
                message: "Notification targets must not be empty.".to_string(),
            });
        }

        for (i, target) in self.notification_targets.iter().enumerate() {
            match target {
                NotificationTarget::Reference(reference) => {
                    let reference = &reference.target_ref;

                    if let Some(document) = document_map.get(reference) {
                        match document {
                            Document::AlertNotificationTarget(alert_notification) => {
                                alert_notification.validate(Some(path), ctx);
                            }
                            _ => {
                                ctx.push(ParserError::Validation {
                                    path: format!("{path}.notificationTargets[{}].targetRef", i),
                                    message: format!(
                                        "Condition reference `{}` is wrong kind.",
                                        reference
                                    ),
                                });
                            }
                        }
                    } else {
                        ctx.push(ParserError::Validation {
                            path: format!("{path}.notificationTargets[{}].targetRef", i),
                            message: format!(
                                "Notification target reference `{}` not found.",
                                reference
                            ),
                        });
                    }
                }
                NotificationTarget::Inline(inline_target) => {
                    inline_target
                        .spec
                        .validate(&format!("{path}.notificationTargets[{}]", i), ctx);
                }
            }
        }
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
            kind: Some(Kind::AlertCondition),
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
            kind: Some(Kind::AlertNotificationTarget),
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

        let mut document_map: HashMap<String, Document> = HashMap::new();
        document_map.insert(
            "cpu-usage-breach".to_string(),
            Document::AlertCondition(create_condition()),
        );

        document_map.insert(
            "on-call-slack".to_string(),
            Document::AlertNotificationTarget(create_target()),
        );

        let policy: AlertPolicyDoc = serde_yaml::from_str(yaml).unwrap();

        let mut validation_context = ValidationContext::new();
        policy.validate(None, &document_map, &mut validation_context);

        assert!(validation_context.result().is_ok());
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

        let mut document_map: HashMap<String, Document> = HashMap::new();
        document_map.insert(
            "pagerduty-notify".to_string(),
            Document::AlertNotificationTarget(create_target()),
        );

        let policy: AlertPolicyDoc = serde_yaml::from_str(yaml).unwrap();

        let mut validation_context = ValidationContext::new();
        policy.validate(None, &document_map, &mut validation_context);

        assert!(validation_context.result().is_ok());
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

        let mut document_map: HashMap<String, Document> = HashMap::new();
        document_map.insert(
            "pagerduty-notify".to_string(),
            Document::AlertNotificationTarget(create_target()),
        );

        let policy: AlertPolicyDoc = serde_yaml::from_str(yaml).unwrap();

        let mut validation_context = ValidationContext::new();
        policy.validate(None, &document_map, &mut validation_context);

        assert!(validation_context.result().is_ok());
    }
}

#[cfg(test)]
mod unhappy_path_tests {

    use super::*;

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

        let mut validation_context = ValidationContext::new();
        alert_policy.validate(None, &HashMap::new(), &mut validation_context);

        assert!(validation_context.result().is_err_and(
            |e| matches!(&e[0], ParserError::Validation { path, message } if
                path == "AlertPolicy.spec.condition"
                    && message == "Condition must contain exactly one item."
            )
        ));
    }
}
