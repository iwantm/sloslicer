// use std::collections::HashMap;

// use serde::Deserialize;

// use super::alert::{AlertConditionSpec, AlertNotificationTargetSpec, AlertPolicySpec};
// use super::data_source::DataSourceSpec;
// use super::sli::SLISpec;
// use super::slo::SLOSpec;

// #[derive(Debug, Deserialize)]
// pub struct Document {
//     #[serde(rename = "apiVersion")]
//     pub api_version: String,
//     pub kind: Kind,
//     pub metadata: Metadata,
//     pub spec: Spec,
// }

// #[derive(Debug, Deserialize)]
// pub struct Metadata {
//     pub name: String,
//     #[serde(rename = "displayName")]
//     pub display_name: Option<String>,
//     pub labels: Option<HashMap<String, StringOrVec>>,
//     pub annotations: Option<HashMap<String, String>>,
// }

// #[derive(Debug, Deserialize)]
// #[serde(untagged)]
// pub enum StringOrVec {
//     Single(String),
//     Multiple(Vec<String>),
// }

// #[derive(Debug, Deserialize)]
// pub enum Kind {
//     #[serde(rename = "DataSource")]
//     DataSource,
//     #[serde(rename = "SLO")]
//     SLO,
//     #[serde(rename = "SLI")]
//     SLI,
//     #[serde(rename = "AlertPolicy")]
//     AlertPolicy,
//     #[serde(rename = "AlertCondition")]
//     AlertCondition,
//     #[serde(rename = "AlertNotificationTarget")]
//     AlertNotificationTarget,
//     #[serde(rename = "Service")]
//     Service,
// }

// #[derive(Debug, Deserialize)]
// #[serde(tag = "kind", content = "spec")]
// pub enum Spec {
//     DataSource(DataSourceSpec),
//     SLO(SLOSpec),
//     SLI(SLISpec),
//     AlertPolicy(AlertPolicySpec),
//     AlertCondition(AlertConditionSpec),
//     AlertNotificationTarget(AlertNotificationTargetSpec),
//     // Service(ServiceSpec),
// }
