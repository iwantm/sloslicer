use super::common::{DurationShorthand, Kind, Metadata, Operator};
use serde::{Deserialize, Serialize};

use crate::utils::{errors::ParserError, validation_context::ValidationContext};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AlertConditionDoc {
    pub kind: Option<Kind>,
    pub metadata: Metadata,
    pub spec: AlertConditionSpec,
}

impl AlertConditionDoc {
    pub fn validate(&self, path: Option<&str>, ctx: &mut ValidationContext) {
        let path = path.unwrap_or("");

        if !matches!(self.kind, Some(Kind::AlertCondition)) {
            ctx.push(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Expected kind to be AlertCondition.".to_string(),
            });
        }

        self.spec.validate(&format!("{path}.spec"), ctx);
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub enum CondtionKind {
    #[serde(rename = "burnrate")]
    Burnrate,
}

fn default_kind() -> CondtionKind {
    CondtionKind::Burnrate
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Condition {
    pub kind: CondtionKind,
    pub op: Option<Operator>,
    pub threshold: Option<f64>,
    pub lookback_window: Option<DurationShorthand>,
    pub alert_after: Option<DurationShorthand>,
}

impl Condition {
    pub fn validate(&self, path: &str, ctx: &mut ValidationContext) {
        let path = format!("{path}.condition");

        if matches!(self.op, Some(Operator::Invalid)) {
            ctx.push(ParserError::Validation {
                path: format!("{path}.op"),
                message: "Invalid operator specified.".to_string(),
            });
        }

        if matches!(self.kind, CondtionKind::Burnrate) {
            if self.op.is_none() {
                ctx.push(ParserError::Validation {
                    path: format!("{path}.op"),
                    message: "Operator must be specified for burnrate condition.".to_string(),
                });
            }

            if self.threshold.is_none() {
                ctx.push(ParserError::Validation {
                    path: format!("{path}.threshold"),
                    message: "Threshold must be specified for burnrate condition.".to_string(),
                });
            }
            if self.lookback_window.is_none() {
                ctx.push(ParserError::Validation {
                    path: format!("{path}.lookbackWindow"),
                    message: "lookbackWindow must be specified for burnrate condition.".to_string(),
                });
            }
            if self.alert_after.is_none() {
                ctx.push(ParserError::Validation {
                    path: format!("{path}.alertAfter"),
                    message: "alertAfter must be specified for burnrate condition.".to_string(),
                });
            }
        } else {
            ctx.push(ParserError::Validation {
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
        #[derive(Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AlertConditionSpec {
    pub description: Option<String>,
    pub severity: String,
    pub condition: Condition,
}

impl AlertConditionSpec {
    pub fn validate(&self, path: &str, ctx: &mut ValidationContext) {
        if self.severity.trim().is_empty() {
            ctx.push(ParserError::Validation {
                path: format!("{path}.severity"),
                message: "Severity must be a non-empty string.".to_string(),
            });
        }

        self.condition.validate(path, ctx)
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

        let expected = AlertConditionDoc {
            kind: Some(Kind::AlertCondition),
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

        let alert_condition: AlertConditionDoc = serde_yaml::from_str(yaml).unwrap();
        let mut validation_context = ValidationContext::new();

        alert_condition.validate(Some("test"), &mut validation_context);

        assert!(validation_context.result().is_ok());

        assert!(expected == alert_condition);
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

        let expected = AlertConditionDoc {
            kind: Some(Kind::AlertCondition),
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

        let alert_condition: AlertConditionDoc = serde_yaml::from_str(yaml).unwrap();
        let mut validation_context = ValidationContext::new();

        alert_condition.validate(None, &mut validation_context);
        assert!(expected == alert_condition);
        assert!(validation_context.result().is_ok());
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

        let alert_condition: Result<AlertConditionDoc, serde_yaml::Error> =
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

        let alert_condition: AlertConditionDoc = serde_yaml::from_str(yaml).unwrap();

        let mut validation_context = ValidationContext::new();
        alert_condition.validate(None, &mut validation_context);

        assert!(validation_context.result().is_err_and(
            |e| matches!(&e[0], ParserError::Validation { path, message }
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

        let alert_condition: AlertConditionDoc = serde_yaml::from_str(yaml).unwrap();

        let mut validation_context = ValidationContext::new();
        alert_condition.validate(None, &mut validation_context);

        assert!(
            validation_context.result().is_err_and(|e| matches!(&e[0], ParserError::Validation { path, message }
            if path == "AlertCondition.spec.condition.op" && message == "Invalid operator specified."))
        );
    }
}
