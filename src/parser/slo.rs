// use serde::Deserialize;
// use std::collections::HashMap;

// use super::alert::{AlertConditionSpec, AlertNotificationTargetSpec, AlertPolicySpec};
// use super::common::{BudgetingMethod, DurationShorthand};
// use super::objective::Objective;
// use super::sli::SLISpec;
// use super::validation::{ValidationError, ValidationResult};

// #[derive(Debug, Deserialize)]
// #[serde(untagged)]
// pub enum AlertPolicy {
//     Inline(AlertPolicySpec),
//     Reference(String),
// }

// #[derive(Debug, Deserialize)]
// pub struct SLOSpec {
//     pub description: Option<String>,
//     pub service: Option<String>,
//     pub indicator: Option<SLISpec>,
//     #[serde(rename = "indicatorRef")]
//     pub indicator_ref: Option<String>,
//     #[serde(rename = "timeWindow")]
//     pub time_window: Option<Vec<TimeWindow>>,
//     #[serde(rename = "budgetingMethod")]
//     pub budgeting_method: BudgetingMethod,
//     pub objectives: Vec<Objective>,
//     #[serde(rename = "alertPolicy")]
//     pub alert_policy: Option<Vec<AlertPolicy>>,
// }

// impl SLOSpec {
//     pub fn is_composite(&self) -> bool {
//         self.objectives
//             .iter()
//             .any(|objective| objective.indicator.is_some() || objective.indicator_ref.is_some())
//     }

//     pub fn validate(
//         &self,
//         sli_map: &HashMap<String, SLISpec>,
//         alert_policy_map: &HashMap<String, AlertPolicySpec>,
//         condition_map: &HashMap<String, AlertConditionSpec>,
//         notification_target_map: &HashMap<String, AlertNotificationTargetSpec>,
//         path: &str,
//     ) -> ValidationResult {
//         if self.is_composite() {
//             if self.indicator.is_some() || self.indicator_ref.is_some() {
//                 return Err(ValidationError::new(
//                     format!("{path}.indicator"),
//                     "indicator or indicatorRef must be moved into objectives for composite SLOs",
//                 ));
//             }

//             self.objectives.iter().try_for_each(|objective| {
//                 if let Some(indicator) = &objective.indicator {
//                     if indicator.threshold_metric.is_some() {
//                         return Err(ValidationError::new(
//                             format!("{path}.objectives"),
//                             "thresholdMetrics not allowed for composite SLOs.",
//                         ));
//                     }
//                 }

//                 if let Some(indicator_ref) = &objective.indicator_ref {
//                     let indicator = sli_map.get(indicator_ref).ok_or_else(|| {
//                         ValidationError::new(
//                             format!("{path}.objectives"),
//                             format!("Indicator reference `{}` not found.", indicator_ref),
//                         )
//                     })?;
//                     if indicator.threshold_metric.is_some() {
//                         return Err(ValidationError::new(
//                             format!("{path}.objectives"),
//                             "thresholdMetrics not allowed for composite SLOs.",
//                         ));
//                     }
//                 }

//                 Ok(())
//             })?;
//         }
//         if !self.is_composite() {
//             if self.indicator.is_some() && self.indicator_ref.is_some() {
//                 return Err(ValidationError::new(
//                     format!("{path}.indicator"),
//                     "Cannot specify both indicator and indicatorRef.",
//                 ));
//             }

//             if self.indicator.is_none() && self.indicator_ref.is_none() {
//                 return Err(ValidationError::new(
//                     format!("{path}.indicator"),
//                     "Must specify either indicator or indicatorRef in SLOSpec when not using composite SLOs.",
//                 ));
//             }

//             if let Some(indicator) = &self.indicator {
//                 indicator.validate(&format!("{path}.indicator"))?;
//             }

//             if let Some(indicator_ref) = &self.indicator_ref {
//                 let indicator = sli_map.get(indicator_ref).ok_or_else(|| {
//                     ValidationError::new(
//                         format!("{path}.indicatorRef"),
//                         format!("Indicator reference `{}` not found.", indicator_ref),
//                     )
//                 })?;
//                 indicator.validate(&format!("{path}.indicatorRef[{}]", indicator_ref))?;
//             }
//         }

//         if let Some(time_window) = &self.time_window {
//             if time_window.len() != 1 {
//                 return Err(ValidationError::new(
//                     format!("{path}.timeWindow"),
//                     "timeWindow must contain exactly one item.",
//                 ));
//             }
//             time_window[0].validate(&format!("{path}.timeWindow[0]"))?;
//         }

//         if self.objectives.is_empty() {
//             return Err(ValidationError::new(
//                 format!("{path}.objectives"),
//                 "objectives must contain at least one item.",
//             ));
//         }

//         if let Some(indicator) = &self.indicator {
//             if indicator.threshold_metric.is_some() && self.objectives.len() != 1 {
//                 return Err(ValidationError::new(
//                     format!("{path}.objectives"),
//                     "Only one objective is allowed when using a `thresholdMetric`.",
//                 ));
//             }
//         }

//         if let Some(indicator_ref) = &self.indicator_ref {
//             let indicator = sli_map.get(indicator_ref).ok_or_else(|| {
//                 ValidationError::new(
//                     format!("{path}.indicatorRef"),
//                     format!("Indicator reference `{}` not found.", indicator_ref),
//                 )
//             })?;

//             if indicator.threshold_metric.is_some() && self.objectives.len() != 1 {
//                 return Err(ValidationError::new(
//                     format!("{path}.objectives"),
//                     "Only one objective is allowed when using a `thresholdMetric`.",
//                 ));
//             }
//         }

//         for (i, obj) in self.objectives.iter().enumerate() {
//             obj.validate(
//                 &self.budgeting_method,
//                 sli_map,
//                 &format!("{path}.objectives[{}]", i),
//             )?;

//             if let Some(alert_policy) = &self.alert_policy {
//                 for (i, policy) in alert_policy.iter().enumerate() {
//                     match policy {
//                         AlertPolicy::Inline(alert_policy) => {
//                             alert_policy.validate(condition_map, notification_target_map, path)?;
//                         }
//                         AlertPolicy::Reference(ref_name) => {
//                             let alert_policy = alert_policy_map.get(ref_name).ok_or_else(|| {
//                                 ValidationError::new(
//                                     format!("{path}.alertPolicies.{}", i),
//                                     format!("Alert Policy reference `{}` not found.", ref_name),
//                                 )
//                             })?;
//                             alert_policy.validate(condition_map, notification_target_map, path)?;
//                         }
//                     }
//                 }
//             }
//         }
//         Ok(())
//     }
// }

// #[derive(Debug, Deserialize)]
// pub struct TimeWindow {
//     pub duration: DurationShorthand,
//     pub calendar: Option<CalendarDetails>,
//     #[serde(rename = "isRolling")]
//     pub is_rolling: bool,
// }

// impl TimeWindow {
//     pub fn validate(&self, path: &str) -> ValidationResult {
//         if self.is_rolling && self.calendar.is_some() {
//             return Err(ValidationError::new(
//                 format!("{path}.calendar"),
//                 "Calendar details can only be specified for Calendar Aligned time windows.",
//             ));
//         }

//         if !self.is_rolling && self.calendar.is_none() {
//             return Err(ValidationError::new(
//                 format!("{path}.calendar"),
//                 "Calendar details must be specified for Calendar Aligned time windows.",
//             ));
//         }

//         Ok(())
//     }
// }

// #[derive(Debug, Deserialize)]
// pub struct CalendarDetails {
//     #[serde(rename = "startTime")]
//     pub start_time: String,
//     #[serde(rename = "timeZone")]
//     pub time_zone: String,
// }

// #[cfg(test)]

// mod tests {
//     use crate::parser::sli::{MetricSource, RatioMetric, RawType, ThresholdMetric};

//     use super::*;
//     use serde_yaml;

//     #[test]
//     fn minimal_valid_slo() {
//         let yaml = r#"
//         service: payment-service
//         indicator:
//           thresholdMetric:
//             metric_source:
//               metric_source_ref: "datadoge"
//               type_: "datadoge"
//         budgetingMethod: Occurrences
//         timeWindow:
//             - duration: 30d
//               isRolling: true
//         objectives:
//             - target: 0.95
//               op: lt
//               value: 200
//         "#;

//         let slo: SLOSpec = serde_yaml::from_str(yaml).unwrap();
//         let sli_map = HashMap::new();
//         let alert_policy_map = HashMap::new();
//         let condition_map = HashMap::new();
//         let notification_target_map = HashMap::new();

//         let result = slo.validate(
//             &sli_map,
//             &alert_policy_map,
//             &condition_map,
//             &notification_target_map,
//             "SLO",
//         );

//         assert!(result.is_ok(), "Expected valid SLO to pass validation");
//     }

//     #[test]
//     fn calendar_window() {
//         let yaml = r#"
//         description: "Monthly uptime tracking"
//         service: backend-api
//         indicatorRef: availability-sli
//         budgetingMethod: Timeslices
//         timeWindow:
//             - duration: 1M
//               calendar:
//                 startTime: "2025-04-01 00:00:00"
//                 timeZone: "UTC"
//               isRolling: false
//         objectives:
//             - targetPercent: 99.5
//               timeSliceTarget: 0.99
//               timeSliceWindow: 5m
//         "#;

//         let slo: SLOSpec = serde_yaml::from_str(yaml).unwrap();
//         let mut sli_map = HashMap::new();
//         let alert_policy_map = HashMap::new();
//         let condition_map = HashMap::new();
//         let notification_target_map = HashMap::new();

//         sli_map.insert(
//             "availability-sli".to_string(),
//             SLISpec {
//                 threshold_metric: Some(ThresholdMetric {
//                     metric_source: MetricSource {
//                         metric_source_ref: Some("datadoge".to_string()),
//                         type_: Some("datadoge".to_string()),
//                         spec: None,
//                     },
//                 }),
//                 description: None,
//                 ratio_metric: None,
//                 // Add other fields as necessary
//             },
//         );

//         let result = slo.validate(
//             &sli_map,
//             &alert_policy_map,
//             &condition_map,
//             &notification_target_map,
//             "SLO",
//         );

//         assert!(result.is_ok(), "Expected valid SLO to pass validation");
//     }

//     #[test]
//     fn composite_slo() {
//         let yaml = r#"
//         service: composite-service
//         budgetingMethod: RatioTimeslices
//         timeWindow:
//             - duration: 7d
//               isRolling: true
//         objectives:
//             - target: 0.98
//               indicatorRef: slo-1
//               compositeWeight: 1
//               timeSliceTarget: 0.95
//               timeSliceWindow: 1h
//             - targetPercent: 99.9
//               indicatorRef: slo-2
//               compositeWeight: 2
//               timeSliceTarget: 0.97
//               timeSliceWindow: 1h
//         "#;

//         let slo: SLOSpec = serde_yaml::from_str(yaml).unwrap();
//         let mut sli_map = HashMap::new();
//         let alert_policy_map = HashMap::new();
//         let condition_map = HashMap::new();
//         let notification_target_map = HashMap::new();

//         sli_map.insert(
//             "slo-2".to_string(),
//             SLISpec {
//                 threshold_metric: None,
//                 description: None,
//                 ratio_metric: Some(RatioMetric {
//                     counter: Some(true),
//                     good: None,
//                     bad: None,
//                     total: None,
//                     raw_type: Some(RawType::Success),
//                     raw: Some(MetricSource {
//                         metric_source_ref: Some("datadoge".to_string()),
//                         type_: Some("datadoge".to_string()),
//                         spec: None,
//                     }),
//                 }),
//             },
//         );

//         sli_map.insert(
//             "slo-1".to_string(),
//             SLISpec {
//                 threshold_metric: None,
//                 description: None,
//                 ratio_metric: Some(RatioMetric {
//                     counter: Some(true),
//                     good: Some(MetricSource {
//                         metric_source_ref: Some("datadoge".to_string()),
//                         type_: Some("datadoge".to_string()),
//                         spec: None,
//                     }),
//                     bad: None,
//                     total: Some(MetricSource {
//                         metric_source_ref: Some("datadoge".to_string()),
//                         type_: Some("datadoge".to_string()),
//                         spec: None,
//                     }),
//                     raw_type: None,
//                     raw: None,
//                 }),
//             },
//         );

//         let result = slo.validate(
//             &sli_map,
//             &alert_policy_map,
//             &condition_map,
//             &notification_target_map,
//             "SLO",
//         );

//         println!("{:?}", result);
//         assert!(result.is_ok(), "Expected valid SLO to pass validation");
//     }

//     #[test]
//     fn alert_policy_ref() {
//         let yaml = r#"
//         service: alerts-service
//         indicatorRef: error-rate-sli
//         budgetingMethod: Occurrences
//         timeWindow:
//             - duration: 30d
//               isRolling: true
//         objectives:
//             - target: 0.99
//         alertPolicies:
//             - alertPolicyRef: high-error-rate-alert
//         "#;

//         let slo: SLOSpec = serde_yaml::from_str(yaml).unwrap();
//         let mut sli_map = HashMap::new();
//         let alert_policy_map = HashMap::new();
//         let condition_map = HashMap::new();
//         let notification_target_map = HashMap::new();

//         sli_map.insert(
//             "error-rate-sli".to_string(),
//             SLISpec {
//                 threshold_metric: Some(ThresholdMetric {
//                     metric_source: MetricSource {
//                         metric_source_ref: Some("datadoge".to_string()),
//                         type_: Some("datadoge".to_string()),
//                         spec: None,
//                     },
//                 }),
//                 description: None,
//                 ratio_metric: None,
//                 // Add other fields as necessary
//             },
//         );

//         let result = slo.validate(
//             &sli_map,
//             &alert_policy_map,
//             &condition_map,
//             &notification_target_map,
//             "SLO",
//         );

//         println!("{:?}", result);
//         assert!(result.is_ok(), "Expected valid SLO to pass validation");
//     }
// }
