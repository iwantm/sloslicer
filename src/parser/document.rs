use core::str;

use serde::Deserialize;

use super::api_version::v1::{
    alert_condition::AlertConditionDoc, alert_notification_target::AlertNotificationTargetDoc,
    data_source::DataSourceDoc, service::ServiceDoc, sli::SLIDoc, slo::SLODoc,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", content = "TypedDocument")]
pub enum TypedDocument {
    DataSource(DataSourceDoc),
    SLO(SLODoc),
    SLI(SLIDoc),
    AlertPolicy(AlertConditionDoc),
    AlertCondition(AlertConditionDoc),
    AlertNotificationTarget(AlertNotificationTargetDoc),
    Service(ServiceDoc),
}

#[derive(Debug, Deserialize)]
pub struct Document {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
}

impl Document {
    pub fn parse(yaml: &str) {}
}
