use serde::Deserialize;
use std::collections::HashMap;

use super::alert::AlertPolicySpec;
use super::common::{DurationShorthand, Operator};
use super::sli::SLISpec;
use super::validation::{ValidationError, ValidationResult};

#[derive(Debug, Deserialize)]
pub struct SLOSpec {
    pub description: Option<String>,
    pub service: Option<String>,
    pub indicator: Option<SLISpec>,
    #[serde(rename = "indicatorRef")]
    pub indicator_ref: Option<String>,
    #[serde(rename = "timeWindow")]
    pub time_window: Option<Vec<TimeWindow>>,
    #[serde(rename = "budgetingMethod")]
    pub budgeting_method: BudgetingMethod,
    pub objectives: Vec<Objective>,
    #[serde(rename = "alertPolicy")]
    pub alert_policy: Option<Vec<AlertPolicySpec>>,
}

impl SLOSpec {
    pub fn is_composite(&self) -> bool {
        self.objectives
            .iter()
            .any(|objective| objective.indicator.is_some() || objective.indicator_ref.is_some())
    }

    pub fn validate(&self, sli_map: &HashMap<String, SLISpec>, path: &str) -> ValidationResult {
        if self.is_composite() && (self.indicator.is_some() || self.indicator_ref.is_some()) {
            return Err(ValidationError::new(
                format!("{path}.indicator"),
                "indicator or indicatorRef must be moved into objectives for composite SLOs",
            ));
        }

        if !self.is_composite() {
            if self.indicator.is_some() && self.indicator_ref.is_some() {
                return Err(ValidationError::new(
                    format!("{path}.indicator"),
                    "Cannot specify both indicator and indicatorRef.",
                ));
            }

            if self.indicator.is_none() && self.indicator_ref.is_none() {
                return Err(ValidationError::new(
                    format!("{path}.indicator"),
                    "Must specify either indicator or indicatorRef in SLOSpec when not using composite SLOs.",
                ));
            }

            if let Some(indicator) = &self.indicator {
                indicator.validate(&format!("{path}.indicator"))?;
            }

            if let Some(indicator_ref) = &self.indicator_ref {
                let indicator = sli_map.get(indicator_ref).ok_or_else(|| {
                    ValidationError::new(
                        format!("{path}.indicatorRef"),
                        format!("Indicator reference `{}` not found.", indicator_ref),
                    )
                })?;
                indicator.validate(&format!("{path}.indicatorRef[{}]", indicator_ref))?;
            }
        }

        if let Some(time_window) = &self.time_window {
            if time_window.len() != 1 {
                return Err(ValidationError::new(
                    format!("{path}.timeWindow"),
                    "timeWindow must contain exactly one item.",
                ));
            }
            time_window[0].validate(&format!("{path}.timeWindow[0]"))?;
        }

        if self.objectives.is_empty() {
            return Err(ValidationError::new(
                format!("{path}.objectives"),
                "objectives must contain at least one item.",
            ));
        }

        if let Some(indicator) = &self.indicator {
            if indicator.threshold_metric.is_some() && self.objectives.len() != 1 {
                return Err(ValidationError::new(
                    format!("{path}.objectives"),
                    "Only one objective is allowed when using a `thresholdMetric`.",
                ));
            }
        }

        if let Some(indicator_ref) = &self.indicator_ref {
            let indicator = sli_map.get(indicator_ref).ok_or_else(|| {
                ValidationError::new(
                    format!("{path}.indicatorRef"),
                    format!("Indicator reference `{}` not found.", indicator_ref),
                )
            })?;

            if indicator.threshold_metric.is_some() && self.objectives.len() != 1 {
                return Err(ValidationError::new(
                    format!("{path}.objectives"),
                    "Only one objective is allowed when using a `thresholdMetric`.",
                ));
            }
        }

        for (i, obj) in self.objectives.iter().enumerate() {
            obj.validate(
                &self.budgeting_method,
                sli_map,
                &format!("{path}.objectives[{}]", i),
            )?;
        }
        Ok(())
    }
}

fn default_composite_weight() -> f64 {
    1.0
}

#[derive(Debug, Deserialize)]
pub enum BudgetingMethod {
    #[serde(rename = "Occurrences")]
    Occurrences,
    #[serde(rename = "Timeslices")]
    Timeslices,
    #[serde(rename = "RatioTimeslices")]
    RatioTimeslices,
}

#[derive(Debug, Deserialize)]
pub struct Objective {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub op: Option<Operator>,
    pub value: Option<f64>,
    pub target: Option<f64>,
    #[serde(rename = "targetPercent")]
    pub target_percent: Option<f64>,
    #[serde(rename = "timeSliceTarget")]
    pub time_slice_target: Option<f64>,
    #[serde(rename = "timeSliceWindow")]
    pub time_slice_window: Option<DurationShorthand>,
    pub indicator: Option<SLISpec>,
    #[serde(rename = "indicatorRef")]
    pub indicator_ref: Option<String>,
    #[serde(default = "default_composite_weight", rename = "compositeWeight")]
    pub composite_weight: f64,
}

impl Objective {
    pub fn validate(
        &self,
        budgeting_method: &BudgetingMethod,
        sli_map: &HashMap<String, SLISpec>,
        path: &str,
    ) -> ValidationResult {
        if self.target.is_some() && self.target_percent.is_some() {
            return Err(ValidationError::new(
                format!("{path}.target"),
                "Cannot specify both target and targetPercent.",
            ));
        }
        if self.target.is_none() && self.target_percent.is_none() {
            return Err(ValidationError::new(
                format!("{path}.target"),
                "Must specify either target or targetPercent.",
            ));
        }

        if let Some(indicator) = &self.indicator {
            indicator.validate(&format!("{path}.indicator"))?;
        }

        if let Some(indicator_ref) = &self.indicator_ref {
            let indicator = sli_map.get(indicator_ref).ok_or_else(|| {
                ValidationError::new(
                    format!("{path}.indicatorRef"),
                    format!("Indicator reference `{}` not found.", indicator_ref),
                )
            })?;

            indicator.validate(&format!("{path}.indicator"))?;
        }

        if matches!(
            budgeting_method,
            BudgetingMethod::Timeslices | BudgetingMethod::RatioTimeslices
        ) {
            if self.time_slice_target.is_none() || self.time_slice_window.is_none() {
                return Err(ValidationError::new(
                    format!("{path}.timeSliceTarget"),
                    "TimeSlices budgeting requires timeSliceTarget and timeSliceWindow.",
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct TimeWindow {
    pub duration: DurationShorthand,
    pub calendar: Option<CalendarDetails>,
    #[serde(rename = "isRolling")]
    pub is_rolling: bool,
}

impl TimeWindow {
    pub fn validate(&self, path: &str) -> ValidationResult {
        if self.is_rolling && self.calendar.is_some() {
            return Err(ValidationError::new(
                format!("{path}.calendar"),
                "Calendar details can only be specified for Calendar Aligned time windows.",
            ));
        }

        if !self.is_rolling && self.calendar.is_none() {
            return Err(ValidationError::new(
                format!("{path}.calendar"),
                "Calendar details must be specified for Calendar Aligned time windows.",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CalendarDetails {
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "timeZone")]
    pub time_zone: String,
}

#[cfg(test)]
mod time_window_tests {
    use super::*;
    use serde_yaml;

    #[test]
    fn test_time_window_rolling_valid() {
        let yaml = r#"
        duration: 1h
        isRolling: true
        "#;

        let time_window: TimeWindow = serde_yaml::from_str(yaml).unwrap();
        let result = time_window.validate("timeWindow");
        assert!(
            result.is_ok(),
            "Expected valid TimeWindow to pass validation"
        );
    }

    #[test]
    fn test_time_window_rolling_invalid() {
        let yaml = r#"
        duration: 1h
        isRolling: false
        "#;

        let time_window: TimeWindow = serde_yaml::from_str(yaml).unwrap();
        let result = time_window.validate("timeWindow");
        assert!(
            result.is_err(),
            "Expected invalid TimeWindow to fail validation"
        );
    }

    #[test]
    fn test_calendar_alligned_valid() {
        let yaml = r#"
        duration: 1h
        isRolling: false
        calendar:
          startTime: "2023-01-01T00:00:00Z"
          timeZone: "UTC"
        "#;

        let time_window: TimeWindow = serde_yaml::from_str(yaml).unwrap();
        let result = time_window.validate("timeWindow");
        assert!(
            result.is_ok(),
            "Expected valid TimeWindow to pass validation"
        );
    }

    #[test]
    fn test_calendar_alligned_invalid() {
        let yaml = r#"
        duration: 1h
        isRolling: true
        calendar:
          startTime: "2023-01-01T00:00:00Z"
          timeZone: "UTC"
        "#;

        let time_window: TimeWindow = serde_yaml::from_str(yaml).unwrap();
        let result = time_window.validate("timeWindow");
        assert!(
            result.is_err(),
            "Expected invalid TimeWindow to fail validation"
        );
    }
}

#[cfg(test)]
mod objective_test {
    use crate::parser::sli::{MetricSource, ThresholdMetric};

    use super::*;
    use serde_yaml;

    // Valid Test Cases
    #[test]
    fn test_valid_objective_with_target_and_ref() {
        let yaml = r#"
        target: 0.99
        indicatorRef: latency_indicator
        "#;

        let objective: Objective = serde_yaml::from_str(yaml).unwrap();
        let mut sli_map = HashMap::new();

        sli_map.insert(
            "latency_indicator".to_string(),
            SLISpec {
                threshold_metric: Some(ThresholdMetric {
                    metric_source: MetricSource {
                        metric_source_ref: Some("datadoge".to_string()),
                        type_: Some("datadoge".to_string()),
                        spec: None,
                    },
                }),
                description: None,
                ratio_metric: None,
                // Add other fields as necessary
            },
        );

        let result = objective.validate(&BudgetingMethod::Occurrences, &sli_map, "objective");
        assert!(
            result.is_ok(),
            "Expected valid target objective to pass validation"
        );
    }

    #[test]
    fn test_valid_objective_with_target_percent_inline_sli() {
        let yaml = r#"
        targetPercent: 99.9
        indicator:
          thresholdMetric:
            metric_source:
              metric_source_ref: "datadoge"
              type_: "datadoge"
        "#;

        let objective: Objective = serde_yaml::from_str(yaml).unwrap();
        let sli_map = HashMap::new();
        let result = objective.validate(&BudgetingMethod::Occurrences, &sli_map, "objective");
        assert!(
            result.is_ok(),
            "Expected valid target objective to pass validation"
        );
    }

    #[test]
    fn test_valid_threshold_objective() {
        let yaml = r#"
        op: lte
        value: 500
        target: 0.95
        "#;

        let objective: Objective = serde_yaml::from_str(yaml).unwrap();
        let sli_map = HashMap::new();
        let result = objective.validate(&BudgetingMethod::Occurrences, &sli_map, "objective");
        assert!(
            result.is_ok(),
            "Expected valid target objective to pass validation"
        );
    }

    #[test]
    fn test_valid_timeslice_objective() {
        let yaml = r#"
        targetPercent: 99.9
        timeSliceTarget: 0.9
        timeSliceWindow: 5m
        "#;

        let objective: Objective = serde_yaml::from_str(yaml).unwrap();
        let sli_map = HashMap::new();
        let result = objective.validate(&BudgetingMethod::Occurrences, &sli_map, "objective");
        assert!(
            result.is_ok(),
            "Expected valid target objective to pass validation"
        );
    }

    #[test]
    fn test_all_fields_set() {
        let yaml = r#"
        displayName: "Latency Objective"
        op: gte
        value: 200
        targetPercent: 99.5
        compositeWeight: 2
        "#;

        let objective: Objective = serde_yaml::from_str(yaml).unwrap();
        let sli_map = HashMap::new();

        let result: Result<(), ValidationError> =
            objective.validate(&BudgetingMethod::Occurrences, &sli_map, "objective");
        assert!(
            result.is_ok(),
            "Expected valid target objective to pass validation"
        );
    }

    // Invalid Test Cases
}
