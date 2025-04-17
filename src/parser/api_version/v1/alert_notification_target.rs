use serde::{Deserialize, Serialize};

use super::common::{Kind, Metadata};
use crate::utils::{errors::ParserError, validation_context::ValidationContext};

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct AlertNotificationTargetDoc {
    pub kind: Option<Kind>,
    pub metadata: Metadata,
    pub spec: AlertNotificationTargetSpec,
}

impl AlertNotificationTargetDoc {
    pub fn validate(&self, path: Option<&str>, ctx: &mut ValidationContext) {
        let path = path.unwrap_or("");

        if !matches!(self.kind, Some(Kind::AlertNotificationTarget)) {
            ctx.push(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Expected kind to be AlertNotificationTarget.".to_string(),
            });
        };

        self.spec.validate(&format!("{path}.spec"), ctx);
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct AlertNotificationTargetSpec {
    pub target: String,
    pub description: Option<String>,
}

impl AlertNotificationTargetSpec {
    pub fn validate(&self, path: &str, ctx: &mut ValidationContext) {
        if self.target.is_empty() {
            ctx.push(ParserError::Validation {
                path: format!("{path}.target"),
                message: "Target must not be empty.".to_string(),
            });
        }
    }
}

#[cfg(test)]
mod happy_path_tests {
    use super::super::common::StringOrVec;

    use super::*;

    #[test]
    fn test_alert_notification_target_spec() {
        let expected = AlertNotificationTargetDoc {
            kind: Some(Kind::AlertNotificationTarget),
            metadata: Metadata {
                name: "test".to_string(),
                display_name: None,
                labels: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("env".to_string(), StringOrVec::Single("test".to_string()));
                    Some(map)
                },
                annotations: None,
            },
            spec: AlertNotificationTargetSpec {
                target: "slack".to_string(),
                description: None,
            },
        };

        let yaml = r#"
        kind: AlertNotificationTarget
        metadata:
            name: test
            labels:
                env: test
        spec:
            target: slack
        "#;

        let alert_condition: AlertNotificationTargetDoc = serde_yaml::from_str(yaml).unwrap();
        let mut validation_context = ValidationContext::new();

        alert_condition.validate(None, &mut validation_context);

        assert!(expected == alert_condition);
        assert!(validation_context.result().is_ok());
    }

    #[test]
    fn test_alert_notification_target_spec_full() {
        let expected = AlertNotificationTargetDoc {
            kind: Some(Kind::AlertNotificationTarget),
            metadata: Metadata {
                name: "test".to_string(),
                display_name: None,
                labels: None,
                annotations: None,
            },
            spec: AlertNotificationTargetSpec {
                target: "slack".to_string(),
                description: Some("Slack channel".to_string()),
            },
        };

        let yaml = r#"
        kind: AlertNotificationTarget
        metadata:
            name: test
        spec:
            target: slack
            description: Slack channel
        "#;

        let alert_condition: AlertNotificationTargetDoc = serde_yaml::from_str(yaml).unwrap();
        let mut validation_context = ValidationContext::new();

        alert_condition.validate(None, &mut validation_context);

        assert!(expected == alert_condition);
        assert!(validation_context.result().is_ok());
    }
}
