use super::{
    super::super::validation::{ValidationError, ValidationResult},
    document::{Kind, Metadata},
};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct AlertNotificationTargetDocument {
    pub kind: Kind,
    pub metadata: Metadata,
    pub spec: AlertNotificationTargetSpec,
}

impl AlertNotificationTargetDocument {
    pub fn validate(&self, path: Option<String>) -> ValidationResult {
        if !matches!(self.kind, Kind::AlertNotificationTarget) {
            return Err(ValidationError::new(
                "kind",
                "Invalid kind specified. Expected `AlertNotificationTarget`.",
            ));
        }

        match path {
            Some(path) => self.spec.validate(&path),
            None => self.spec.validate("alert_notification_target"),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AlertNotificationTargetSpec {
    pub target: String,
    pub description: Option<String>,
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

#[cfg(test)]

mod happy_path_tests {
    use super::super::document::StringOrVec;

    use super::*;

    #[test]
    fn test_alert_notification_target_spec() {
        let expected = AlertNotificationTargetDocument {
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

        let alert_condition: AlertNotificationTargetDocument = serde_yaml::from_str(yaml).unwrap();

        let result = alert_condition.validate(None);

        assert!(expected == alert_condition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_alert_notification_target_spec_full() {
        let expected = AlertNotificationTargetDocument {
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

        let alert_condition: AlertNotificationTargetDocument = serde_yaml::from_str(yaml).unwrap();

        let result = alert_condition.validate(None);

        assert!(expected == alert_condition);
        assert!(result.is_ok());
    }
}
