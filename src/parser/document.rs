use core::str;
use std::collections::HashMap;

use serde::Deserialize;

use super::{
    api_version::v1::{
        alert_condition::AlertConditionDoc, alert_notification_target::AlertNotificationTargetDoc,
        alert_policy::AlertPolicyDoc, data_source::DataSourceDoc, service::ServiceDoc, sli::SLIDoc,
        slo::SLODoc,
    },
    errors::{ParserError, ParserResult},
};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum TypedDocument {
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

#[derive(Debug, Deserialize)]
pub struct Document {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    #[serde(flatten)]
    pub doc: TypedDocument,
}

impl Document {
    pub fn parse(yaml: &str) -> ParserResult<Self> {
        Ok(serde_yaml::from_str(yaml)?)
    }

    pub fn validate(
        &self,
        path: &str,
        sli_map: Option<&HashMap<String, SLIDoc>>,
        alert_policy_map: Option<&HashMap<String, AlertPolicyDoc>>,
        condition_map: Option<&HashMap<String, AlertConditionDoc>>,
        notification_target_map: Option<&HashMap<String, AlertNotificationTargetDoc>>,
    ) -> ParserResult<()> {
        if !self.api_version.ends_with("v1") {
            return Err(ParserError::Validation {
                path: path.to_string(),
                message: "Currently only v1 of the spec is supported".to_string(),
            });
        }
        match &self.doc {
            TypedDocument::DataSource(data_source_doc) => data_source_doc.validate(Some(path))?,
            TypedDocument::Slo(slodoc) => {
                return slodoc.validate(
                    Some(path),
                    sli_map,
                    alert_policy_map,
                    condition_map,
                    notification_target_map,
                );
            }
            TypedDocument::Sli(slidoc) => return slidoc.validate(false, Some(path)),
            TypedDocument::AlertPolicy(alert_policy_doc) => {
                return alert_policy_doc.validate(
                    condition_map,
                    notification_target_map,
                    Some(path),
                );
            }

            TypedDocument::AlertCondition(alert_condition_doc) => {
                return alert_condition_doc.validate(Some(path));
            }
            TypedDocument::AlertNotificationTarget(alert_notification_target_doc) => {
                return alert_notification_target_doc.validate(Some(path));
            }

            TypedDocument::Service(service_doc) => return service_doc.validate(path),
            TypedDocument::InvalidKind => {
                return Err(ParserError::Validation {
                    path: path.to_string(),
                    message: "Couldn't match kind".to_string(),
                });
            }
        }

        Ok(())
    }
}

pub fn parse_kind(yaml: &str) -> ParserResult<Document> {
    Ok(serde_yaml::from_str(yaml)?)
}
