use core::str;
use std::collections::HashMap;

use crate::utils::{
    errors::{ParserError, ParserResult},
    validation_context::ValidationContext,
};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use super::api_version::v1::{
    alert_condition::{AlertConditionDoc, AlertConditionSpec},
    alert_notification_target::{AlertNotificationTargetDoc, AlertNotificationTargetSpec},
    alert_policy::{AlertPolicyDoc, AlertPolicySpec},
    common::{Kind, Metadata},
    data_source::{DataSourceDoc, DataSourceSpec},
    service::{ServiceDoc, ServiceSpec},
    sli::{SLIDoc, SLISpec},
    slo::{SLODoc, SLOSpec},
};

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct RootDocument {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: Kind,
    pub metadata: Metadata,
    pub spec: serde_yaml::Value,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub enum Document {
    DataSource(DataSourceDoc),
    #[serde(rename = "SLO")]
    Slo(SLODoc),
    #[serde(rename = "SLI")]
    Sli(SLIDoc),
    AlertPolicy(AlertPolicyDoc),
    AlertCondition(AlertConditionDoc),
    AlertNotificationTarget(AlertNotificationTargetDoc),
    Service(ServiceDoc),
    InvalidKind,
}

impl TryFrom<RootDocument> for Document {
    type Error = ParserError;

    fn try_from(doc: RootDocument) -> ParserResult<Self> {
        match doc.kind {
            Kind::Slo => {
                let typed_spec: SLOSpec = serde_yaml::from_value(doc.spec)?;
                Ok(Document::Slo(SLODoc {
                    kind: Some(Kind::Slo),
                    metadata: doc.metadata,
                    spec: typed_spec,
                }))
            }
            Kind::Sli => {
                let typed_spec: SLISpec = serde_yaml::from_value(doc.spec)?;
                Ok(Document::Sli(SLIDoc {
                    kind: Some(Kind::Sli),
                    metadata: doc.metadata,
                    spec: typed_spec,
                }))
            }
            Kind::DataSource => {
                let typed_spec: DataSourceSpec = serde_yaml::from_value(doc.spec)?;
                Ok(Document::DataSource(DataSourceDoc {
                    kind: Some(Kind::DataSource),
                    metadata: doc.metadata,
                    spec: typed_spec,
                }))
            }
            Kind::AlertPolicy => {
                let typed_spec: AlertPolicySpec = serde_yaml::from_value(doc.spec)?;
                Ok(Document::AlertPolicy(AlertPolicyDoc {
                    kind: Some(Kind::AlertPolicy),
                    metadata: doc.metadata,
                    spec: typed_spec,
                }))
            }
            Kind::AlertCondition => {
                let typed_spec: AlertConditionSpec = serde_yaml::from_value(doc.spec)?;
                Ok(Document::AlertCondition(AlertConditionDoc {
                    kind: Some(Kind::AlertCondition),
                    metadata: doc.metadata,
                    spec: typed_spec,
                }))
            }
            Kind::AlertNotificationTarget => {
                let typed_spec: AlertNotificationTargetSpec = serde_yaml::from_value(doc.spec)?;
                Ok(Document::AlertNotificationTarget(
                    AlertNotificationTargetDoc {
                        kind: Some(Kind::AlertNotificationTarget),
                        metadata: doc.metadata,
                        spec: typed_spec,
                    },
                ))
            }
            Kind::Service => {
                let typed_spec: ServiceSpec = serde_yaml::from_value(doc.spec)?;
                Ok(Document::Service(ServiceDoc {
                    kind: Some(Kind::Service),
                    metadata: doc.metadata,
                    spec: typed_spec,
                }))
            }
        }
    }
}

impl Document {
    pub fn parse(yaml: Value, path: &str) -> ParserResult<(String, Self)> {
        let root: RootDocument = serde_yaml::from_value(yaml)?;

        if !root.api_version.ends_with("v1") {
            return Err(ParserError::Validation {
                path: path.to_string(),
                message: "Currently only v1 of the spec is supported".to_string(),
            });
        }

        Ok((root.metadata.name.clone(), Document::try_from(root)?))
    }

    pub fn validate(&self, document_map: &HashMap<String, Document>, ctx: &mut ValidationContext) {
        match self {
            Document::DataSource(data_source_doc) => data_source_doc.validate(None, ctx),
            Document::Slo(slodoc) => slodoc.validate(None, document_map, ctx),
            Document::Sli(slidoc) => slidoc.validate(None, false, ctx),
            Document::AlertPolicy(alert_policy_doc) => {
                alert_policy_doc.validate(None, document_map, ctx)
            }

            Document::AlertCondition(alert_condition_doc) => {
                alert_condition_doc.validate(None, ctx)
            }
            Document::AlertNotificationTarget(alert_notification_target_doc) => {
                alert_notification_target_doc.validate(None, ctx)
            }
            Document::Service(service_doc) => service_doc.validate(ctx),
            Document::InvalidKind => ctx.push(ParserError::Validation {
                path: "None".to_owned(),
                message: "Couldn't match kind".to_string(),
            }),
        }
    }
}
