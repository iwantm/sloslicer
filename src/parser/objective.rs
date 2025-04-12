use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::common::{BudgetingMethod, DurationShorthand, Operator};
use super::sli::SLISpec;
use super::validation::{ValidationError, ValidationResult};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TimeSliceWindow {
    Numeric(u32),
    DurationShorthand(DurationShorthand),
}

impl Serialize for TimeSliceWindow {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TimeSliceWindow::Numeric(value) => {
                let shorthand = format!("{}m", value);
                serializer.serialize_str(&shorthand)
            }
            TimeSliceWindow::DurationShorthand(duration) => {
                serializer.serialize_str(duration.as_str())
            }
        }
    }
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
    pub time_slice_window: Option<TimeSliceWindow>,
    pub indicator: Option<SLISpec>,
    #[serde(rename = "indicatorRef")]
    pub indicator_ref: Option<String>,
    #[serde(rename = "compositeWeight")]
    pub composite_weight: Option<f64>,
}

impl Objective {
    pub fn validate(
        &self,
        budgeting_method: &BudgetingMethod,
        sli_map: &HashMap<String, SLISpec>,
        is_composite: bool,
        path: &str,
    ) -> ValidationResult {
        if let Some(op) = &self.op {
            if matches!(op, Operator::Invalid) {
                return Err(ValidationError::new(
                    format!("{path}.op"),
                    "Invalid operator specified.",
                ));
            }
        }

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

        if let Some(target_percent) = self.target_percent {
            if target_percent < 0.0 || target_percent > 100.0 {
                return Err(ValidationError::new(
                    format!("{path}.targetPercent"),
                    "Target percent must be between 0 and 100.",
                ));
            }
        }

        if let Some(target) = self.target {
            if target < 0.0 || target > 1.0 {
                return Err(ValidationError::new(
                    format!("{path}.target"),
                    "Target must be between 0 and 1.",
                ));
            }
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
            if let Some(time_slice_target) = self.time_slice_target {
                if time_slice_target < 0.0 || time_slice_target > 1.0 {
                    return Err(ValidationError::new(
                        format!("{path}.timeSliceTarget"),
                        "TimeSlice target must be between 0 and 1.",
                    ));
                }
            }
        }

        if self.op.is_some() && self.value.is_none() {
            return Err(ValidationError::new(
                format!("{path}.value"),
                "Value must be specified when using an operator.",
            ));
        }

        if self.value.is_some() && self.op.is_none() {
            return Err(ValidationError::new(
                format!("{path}.op"),
                "Operator must be specified when using a value.",
            ));
        }

        if self.indicator.is_some() && self.indicator_ref.is_some() {
            return Err(ValidationError::new(
                format!("{path}.indicator"),
                "Cannot specify both indicator and indicatorRef.",
            ));
        }

        if let Some(indicator) = &self.indicator {
            if indicator.is_threshold_metric() && (self.op.is_none() || self.value.is_none()) {
                return Err(ValidationError::new(
                    format!("{path}.indicator"),
                    "op and value must be specified when using a thresholdMetric.",
                ));
            }

            indicator.validate(&format!("{path}.indicator"))?;
        }

        if let Some(indicator_ref) = &self.indicator_ref {
            let indicator = sli_map.get(indicator_ref).ok_or_else(|| {
                ValidationError::new(
                    format!("{path}.indicatorRef"),
                    format!("Indicator reference `{}` not found.", indicator_ref),
                )
            })?;

            if indicator.is_threshold_metric() && (self.op.is_none() || self.value.is_none()) {
                return Err(ValidationError::new(
                    format!("{path}.indicator"),
                    "op and value must be specified when using a thresholdMetric.",
                ));
            }

            indicator.validate(&format!("{path}.indicator"))?;
        }

        if is_composite {
            if self.indicator.is_none() && self.indicator_ref.is_none() {
                return Err(ValidationError::new(
                    format!("{path}.indicator"),
                    "Indicator or indicatorRef must be specified for composite objectives.",
                ));
            }
        }

        if !is_composite {
            if self.indicator.is_some() || self.indicator_ref.is_some() {
                return Err(ValidationError::new(
                    format!("{path}.indicator"),
                    "Indicator or indicatorRef must not be specified for non-composite objectives.",
                ));
            }
            if self.composite_weight.is_some() {
                return Err(ValidationError::new(
                    format!("{path}.compositeWeight"),
                    "Composite weight must not be specified for single objectives.",
                ));
            }
        }

        if let Some(composite_weight) = self.composite_weight {
            if composite_weight < 0.0 {
                return Err(ValidationError::new(
                    format!("{path}.compositeWeight"),
                    "Composite weight must be greater than or equal to 0.",
                ));
            }
        };

        Ok(())
    }
}

#[cfg(test)]

mod happy_path_tests {
    use crate::parser::sli::{MetricSource, RatioMetric, ThresholdMetric};

    use super::*;

    fn default_sli_map() -> HashMap<String, SLISpec> {
        let mut sli_map = HashMap::new();
        sli_map.insert(
            "threshold_metric".to_string(),
            SLISpec {
                threshold_metric: Some(ThresholdMetric {
                    metric_source: MetricSource {
                        metric_source_ref: Some("some_errors".to_string()),
                        type_: Some("datadoge".to_string()),
                        spec: None,
                    },
                }),
                ratio_metric: None,
                description: None,
            },
        );

        sli_map.insert(
            "ratio_metric".to_string(),
            SLISpec {
                description: None,
                threshold_metric: None,
                ratio_metric: Some(RatioMetric {
                    counter: Some(true),
                    good: Some(MetricSource {
                        metric_source_ref: Some("good".to_string()),
                        type_: Some("datadoge".to_string()),
                        spec: None,
                    }),
                    bad: Some(MetricSource {
                        metric_source_ref: Some("bad".to_string()),
                        type_: Some("datadoge".to_string()),
                        spec: None,
                    }),
                    total: None,
                    raw_type: None,
                    raw: None,
                }),
            },
        );

        sli_map
    }

    #[test]
    fn test_valid_target_percent_objective() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result =
            objective.validate(&BudgetingMethod::Occurrences, &sli_map, false, "test_path");

        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_composite_objective_indicator_ref() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: Some("threshold_metric".to_string()),
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result = objective.validate(&BudgetingMethod::Occurrences, &sli_map, true, "test_path");

        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_composite_objective_indicator_inline() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: Some(SLISpec {
                description: None,
                ratio_metric: None,
                threshold_metric: Some(ThresholdMetric {
                    metric_source: MetricSource {
                        metric_source_ref: Some("some_errors".to_string()),
                        type_: Some("datadoge".to_string()),
                        spec: None,
                    },
                }),
            }),
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result = objective.validate(&BudgetingMethod::Occurrences, &sli_map, true, "test_path");

        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_target_objective() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: Some(0.75),
            target_percent: None,
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result =
            objective.validate(&BudgetingMethod::Occurrences, &sli_map, false, "test_path");

        assert!(result.is_ok());
    }
}

mod unhappy_path_tests {
    use super::*;
    use crate::parser::sli::{MetricSource, RatioMetric, ThresholdMetric};

    fn default_sli_map() -> HashMap<String, SLISpec> {
        let mut sli_map = HashMap::new();
        sli_map.insert(
            "threshold_metric".to_string(),
            SLISpec {
                threshold_metric: Some(ThresholdMetric {
                    metric_source: MetricSource {
                        metric_source_ref: Some("some_errors".to_string()),
                        type_: Some("datadoge".to_string()),
                        spec: None,
                    },
                }),
                ratio_metric: None,
                description: None,
            },
        );

        sli_map.insert(
            "ratio_metric".to_string(),
            SLISpec {
                description: None,
                threshold_metric: None,
                ratio_metric: Some(RatioMetric {
                    counter: Some(true),
                    good: Some(MetricSource {
                        metric_source_ref: Some("good".to_string()),
                        type_: Some("datadoge".to_string()),
                        spec: None,
                    }),
                    bad: Some(MetricSource {
                        metric_source_ref: Some("bad".to_string()),
                        type_: Some("datadoge".to_string()),
                        spec: None,
                    }),
                    total: None,
                    raw_type: None,
                    raw: None,
                }),
            },
        );

        sli_map
    }

    #[test]
    fn test_invalid_target_both() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: Some(0.75),
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result =
            objective.validate(&BudgetingMethod::Occurrences, &sli_map, false, "test_path");

        assert!(result.is_err_and(|e| {
            e.path == "test_path.target"
                && e.message == "Cannot specify both target and targetPercent."
        }));
    }

    #[test]
    fn test_missing_target() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: None,
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result =
            objective.validate(&BudgetingMethod::Occurrences, &sli_map, false, "test_path");

        assert!(result.is_err_and(|e| {
            e.path == "test_path.target"
                && e.message == "Must specify either target or targetPercent."
        }));
    }

    #[test]
    fn test_timeslice_required() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result = objective.validate(
            &BudgetingMethod::RatioTimeslices,
            &sli_map,
            false,
            "test_path",
        );

        assert!(result.is_err_and(|e| {
            e.path == "test_path.timeSliceTarget"
                && e.message == "TimeSlices budgeting requires timeSliceTarget and timeSliceWindow."
        }));
    }

    #[test]
    fn invalid_timeslice_target() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: Some(1.5),
            time_slice_window: Some(TimeSliceWindow::Numeric(5)),
            indicator: None,
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result = objective.validate(&BudgetingMethod::Timeslices, &sli_map, false, "test_path");

        assert!(result.is_err_and(|e| {
            e.path == "test_path.timeSliceTarget"
                && e.message == "TimeSlice target must be between 0 and 1."
        }));
    }

    #[test]
    fn test_op_without_value() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: None,
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result =
            objective.validate(&BudgetingMethod::Occurrences, &sli_map, false, "test_path");

        assert!(result.is_err_and(|e| {
            e.path == "test_path.value"
                && e.message == "Value must be specified when using an operator."
        }));
    }

    #[test]
    fn test_value_without_op() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: None,
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result =
            objective.validate(&BudgetingMethod::Occurrences, &sli_map, false, "test_path");

        assert!(result.is_err_and(|e| {
            e.path == "test_path.op"
                && e.message == "Operator must be specified when using a value."
        }));
    }

    #[test]
    fn test_composite_requires_indicator() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: None,
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result = objective.validate(&BudgetingMethod::Occurrences, &sli_map, true, "test_path");

        assert!(result.is_err_and(|e| {
            e.path == "test_path.indicator"
                && e.message
                    == "Indicator or indicatorRef must be specified for composite objectives."
        }));
    }

    #[test]
    fn test_invalid_indicator_and_ref() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: Some(SLISpec {
                description: None,
                ratio_metric: None,
                threshold_metric: Some(ThresholdMetric {
                    metric_source: MetricSource {
                        metric_source_ref: Some("some_errors".to_string()),
                        type_: Some("datadoge".to_string()),
                        spec: None,
                    },
                }),
            }),
            indicator_ref: Some("threshold_metric".to_string()),
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result = objective.validate(&BudgetingMethod::Occurrences, &sli_map, true, "test_path");

        assert!(result.is_err_and(|e| {
            e.path == "test_path.indicator"
                && e.message == "Cannot specify both indicator and indicatorRef."
        }));
    }

    #[test]
    fn invalid_composite_weight() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: Some("threshold_metric".to_string()),
            composite_weight: Some(-1.0),
        };

        let sli_map = default_sli_map();
        let result = objective.validate(&BudgetingMethod::Occurrences, &sli_map, true, "test_path");

        assert!(result.is_err_and(|e| {
            e.path == "test_path.compositeWeight"
                && e.message == "Composite weight must be greater than or equal to 0."
        }));
    }

    #[test]
    fn test_single_objective_weight() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: None,
            indicator_ref: None,
            composite_weight: Some(1.0),
        };

        let sli_map = default_sli_map();
        let result =
            objective.validate(&BudgetingMethod::Occurrences, &sli_map, false, "test_path");

        assert!(result.is_err_and(|e| {
            e.path == "test_path.compositeWeight"
                && e.message == "Composite weight must not be specified for single objectives."
        }));
    }
}
