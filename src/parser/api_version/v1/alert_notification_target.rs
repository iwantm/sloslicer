use serde::Deserialize;

use super::common::{Kind, Metadata};
use crate::parser::errors::{ParserError, ParserResult};

#[derive(Debug, Deserialize, PartialEq)]
pub struct AlertNotificationTargetDoc {
    pub kind: Kind,
    pub metadata: Metadata,
    pub spec: AlertNotificationTargetSpec,
}

impl AlertNotificationTargetDoc {
    pub fn validate(&self, path: Option<&str>) -> ParserResult<()> {
        let path = path.unwrap_or("AlertNotificationTarget");

        if !matches!(self.kind, Kind::AlertNotificationTarget) {
            return Err(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Expected kind to be AlertNotificationTarget.".to_string(),
            });
        };

        self.spec.validate(&format!("{path}.spec"))
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AlertNotificationTargetSpec {
    pub target: String,
    pub description: Option<String>,
}

impl AlertNotificationTargetSpec {
    pub fn validate(&self, path: &str) -> ParserResult<()> {
        if self.target.is_empty() {
            return Err(ParserError::Validation {
                path: format!("{path}.target"),
                message: "Target must not be empty.".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod happy_path_tests {
    use super::super::common::StringOrVec;

    use super::*;

    #[test]
    fn test_alert_notification_target_spec() {
        let expected = AlertNotificationTargetDoc {
            kind: Kind::AlertNotificationTarget,
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

        let result = alert_condition.validate(None);

        assert!(expected == alert_condition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_alert_notification_target_spec_full() {
        let expected = AlertNotificationTargetDoc {
            kind: Kind::AlertNotificationTarget,
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

        let result = alert_condition.validate(None);

        assert!(expected == alert_condition);
        assert!(result.is_ok());
    }
}
