use core::str;
use std::collections::HashMap;

use serde::Deserialize;

use super::alert_policy::AlertPolicySpec;
use super::data_source::DataSourceSpec;
use super::sli::SLISpec;
use super::slo::SLOSpec;
use crate::parser::alert_condition::AlertConditionSpec;
use crate::parser::alert_notification_target::AlertNotificationTargetSpec;

#[derive(Debug, Deserialize, PartialEq)]
pub enum Kind {
    #[serde(rename = "DataSource")]
    DataSource,
    #[serde(rename = "SLO")]
    SLO,
    #[serde(rename = "SLI")]
    SLI,
    #[serde(rename = "AlertPolicy")]
    AlertPolicy,
    #[serde(rename = "AlertCondition")]
    AlertCondition,
    #[serde(rename = "AlertNotificationTarget")]
    AlertNotificationTarget,
    #[serde(rename = "Service")]
    Service,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", content = "spec")]
pub enum Spec {
    // DataSource(DataSourceSpec),
    SLO(SLOSpec),
    SLI(SLISpec),
    AlertPolicy(AlertPolicySpec),
    AlertCondition(AlertConditionSpec),
    AlertNotificationTarget(AlertNotificationTargetSpec),
    // Service(ServiceSpec),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Metadata {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub labels: Option<HashMap<String, StringOrVec>>,
    pub annotations: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}
