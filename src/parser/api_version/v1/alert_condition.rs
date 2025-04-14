use super::common::{DurationShorthand, Operator};
use super::document::{Kind, Metadata};
use serde::Deserialize;

use crate::parser::errors::{ParserError, ParserResult};

#[derive(Debug, Deserialize, PartialEq)]
pub struct AlertConditionDocument {
    pub kind: Kind,
    pub metadata: Metadata,
    pub spec: AlertConditionSpec,
}

impl AlertConditionDocument {
    pub fn validate(&self, path: Option<&str>) -> ParserResult<()> {
        let path = path.unwrap_or("AlertCondition");

        if !matches!(self.kind, Kind::AlertCondition) {
            return Err(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Expected kind to be AlertCondition.".to_string(),
            });
        }

        self.spec.validate(&format!("{path}.spec"))
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum CondtionKind {
    #[serde(rename = "burnrate")]
    Burnrate,
}

fn default_kind() -> CondtionKind {
    CondtionKind::Burnrate
}

#[derive(Debug, PartialEq)]
pub struct Condition {
    pub kind: CondtionKind,
    pub op: Option<Operator>,
    pub threshold: Option<f64>,
    pub lookback_window: Option<DurationShorthand>,
    pub alert_after: Option<DurationShorthand>,
}

impl Condition {
    pub fn validate(&self, path: &str) -> ParserResult<()> {
        let path = format!("{path}.condition");

        if matches!(self.op, Some(Operator::Invalid)) {
            return Err(ParserError::Validation {
                path: format!("{path}.op"),
                message: "Invalid operator specified.".to_string(),
            });
        }

        if matches!(self.kind, CondtionKind::Burnrate) {
            if self.op.is_none() {
                return Err(ParserError::Validation {
                    path: format!("{path}.op"),
                    message: "Operator must be specified for burnrate condition.".to_string(),
                });
            }

            if self.threshold.is_none() {
                return Err(ParserError::Validation {
                    path: format!("{path}.threshold"),
                    message: "Threshold must be specified for burnrate condition.".to_string(),
                });
            }
            if self.lookback_window.is_none() {
                return Err(ParserError::Validation {
                    path: format!("{path}.lookbackWindow"),
                    message: "lookbackWindow must be specified for burnrate condition.".to_string(),
                });
            }
            if self.alert_after.is_none() {
                return Err(ParserError::Validation {
                    path: format!("{path}.alertAfter"),
                    message: "alertAfter must be specified for burnrate condition.".to_string(),
                });
            }

            Ok(())
        } else {
            Err(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Unsupported condition kind.".to_string(),
            })
        }
    }
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConditionFields {
            #[serde(default = "default_kind")]
            kind: CondtionKind,
            op: Option<Operator>,
            threshold: Option<f64>,
            #[serde(rename = "lookbackWindow")]
            lookback_window: Option<DurationShorthand>,
            #[serde(rename = "alertAfter")]
            alert_after: Option<DurationShorthand>,
        }

        let mut fields = ConditionFields::deserialize(deserializer)?;

        if matches!(fields.kind, CondtionKind::Burnrate) && fields.alert_after.is_none() {
            fields.alert_after = Some(DurationShorthand("0m".to_string()));
        }

        Ok(Condition {
            kind: fields.kind,
            op: fields.op,
            threshold: fields.threshold,
            lookback_window: fields.lookback_window,
            alert_after: fields.alert_after,
        })
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AlertConditionSpec {
    pub description: Option<String>,
    pub severity: String,
    pub condition: Condition,
}

impl AlertConditionSpec {
    pub fn validate(&self, path: &str) -> ParserResult<()> {
        if self.severity.trim().is_empty() {
            return Err(ParserError::Validation {
                path: format!("{path}.severity"),
                message: "Severity must be a non-empty string.".to_string(),
            });
        }

        self.condition.validate(path)
    }
}

#[cfg(test)]

mod happy_path_tests {
    use super::*;

    #[test]
    fn test_minimal_valid_config() {
        let yaml = r#"
        kind: AlertCondition
        metadata:
            name: test
        spec:
            severity: page
            condition:
                kind: burnrate
                op: gte
                threshold: 1.5
                lookbackWindow: 5m
        "#;

        let expected = AlertConditionDocument {
            kind: Kind::AlertCondition,
            metadata: Metadata {
                name: "test".to_string(),
                display_name: None,
                labels: None,
                annotations: None,
            },
            spec: AlertConditionSpec {
                description: None,
                severity: "page".to_string(),
                condition: Condition {
                    kind: CondtionKind::Burnrate,
                    op: Some(Operator::Gte),
                    threshold: Some(1.5),
                    lookback_window: Some(DurationShorthand("5m".to_string())),
                    alert_after: Some(DurationShorthand("0m".to_string())),
                },
            },
        };

        let alert_condition: AlertConditionDocument = serde_yaml::from_str(yaml).unwrap();

        let result = alert_condition.validate(Some("test"));

        assert!(expected == alert_condition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_full_valid_config() {
        let yaml = r#"
        kind: AlertCondition
        metadata:
            name: test
        spec:
            description: Breach if memory usage is too high for too long
            severity: sev1
            condition:
                kind: burnrate
                op: lte
                threshold: 0.8
                lookbackWindow: 30m
                alertAfter: 5m
        "#;

        let expected = AlertConditionDocument {
            kind: Kind::AlertCondition,
            metadata: Metadata {
                name: "test".to_string(),
                display_name: None,
                labels: None,
                annotations: None,
            },
            spec: AlertConditionSpec {
                description: Some("Breach if memory usage is too high for too long".to_string()),
                severity: "sev1".to_string(),
                condition: Condition {
                    kind: CondtionKind::Burnrate,
                    op: Some(Operator::Lte),
                    threshold: Some(0.8),
                    lookback_window: Some(DurationShorthand("30m".to_string())),
                    alert_after: Some(DurationShorthand("5m".to_string())),
                },
            },
        };

        let alert_condition: AlertConditionDocument = serde_yaml::from_str(yaml).unwrap();

        let result = alert_condition.validate(None);
        assert!(expected == alert_condition);
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod unhappy_path_tests {
    use super::*;
    #[test]
    fn test_missing_severity() {
        let yaml = r#"
        kind: AlertCondition
        metadata:
            name: test
        spec:
            condition:
                kind: burnrate
                op: lte
                threshold: 0.8
                lookbackWindow: 30m
        "#;

        let alert_condition: Result<AlertConditionDocument, serde_yaml::Error> =
            serde_yaml::from_str(yaml);

        assert!(
            alert_condition.is_err_and(|e| { e.to_string().contains("missing field `severity`") })
        );
    }

    #[test]
    fn test_missing_op() {
        let yaml = r#"
        kind: AlertCondition
        metadata:
            name: test
        spec:
            severity: sev1
            condition:
                kind: burnrate
                threshold: 0.8
                lookbackWindow: 30m
        "#;

        let alert_condition: AlertConditionDocument = serde_yaml::from_str(yaml).unwrap();

        let validation_result = alert_condition.validate(None);
        assert!(validation_result.is_err_and(
            |e| matches!(e, ParserError::Validation { path, message }
            if path == "AlertCondition.spec.condition.op"
                && message == "Operator must be specified for burnrate condition.")
        ));
    }

    #[test]
    fn invalid_op() {
        let yaml = r#"
        kind: AlertCondition
        metadata:
            name: test
        spec:
            severity: sev1
            condition:
                kind: burnrate
                op: between
                threshold: 0.8
                lookbackWindow: 30m
        "#;

        let alert_condition: AlertConditionDocument = serde_yaml::from_str(yaml).unwrap();

        let validation_result = alert_condition.validate(None);
        print!("validation_result: {:?}", validation_result);

        assert!(
            validation_result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
            if path == "AlertCondition.spec.condition.op" && message == "Invalid operator specified."))
        );
    }
}
