use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::common::{BudgetingMethod, DurationShorthand, Operator};
use super::sli::{SLIDoc, SLISpec};
use crate::parser::errors::{ParserError, ParserResult};

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
    pub indicator: Option<SLIDoc>,
    #[serde(rename = "indicatorRef")]
    pub indicator_ref: Option<String>,
    #[serde(rename = "compositeWeight")]
    pub composite_weight: Option<f64>,
}

impl Objective {
    fn validate_target(&self, path: &str) -> ParserResult<()> {
        if self.target.is_some() && self.target_percent.is_some() {
            return Err(ParserError::Validation {
                path: format!("{path}.target, {path}.targetPercent"),
                message: "Cannot specify both target and targetPercent.".to_string(),
            });
        }

        if self.target.is_none() && self.target_percent.is_none() {
            return Err(ParserError::Validation {
                path: format!("{path}.target, {path}.targetPercent"),
                message: "Must specify either target or targetPercent.".to_string(),
            });
        }

        if let Some(target_percent) = self.target_percent {
            if target_percent <= 0.0 || target_percent >= 100.0 {
                return Err(ParserError::Validation {
                    path: format!("{path}.targetPercent"),
                    message: "Target percent must be between 0 and 100.".to_string(),
                });
            }
        }

        if let Some(target) = self.target {
            if target <= 0.0 || target >= 1.0 {
                return Err(ParserError::Validation {
                    path: format!("{path}.target"),
                    message: "Target must be between 0 and 1.".to_string(),
                });
            }
        }
        Ok(())
    }

    fn validate_timeslice(
        &self,
        budgeting_method: &BudgetingMethod,
        path: &str,
    ) -> ParserResult<()> {
        if matches!(
            budgeting_method,
            BudgetingMethod::Timeslices | BudgetingMethod::RatioTimeslices
        ) {
            if self.time_slice_target.is_none() || self.time_slice_window.is_none() {
                return Err(ParserError::Validation {
                    path: format!("{path}.timeSliceTarget, {path}.timeSliceWindow"),
                    message: "TimeSlices budgeting requires timeSliceTarget and timeSliceWindow."
                        .to_string(),
                });
            }

            if let Some(tst) = self.time_slice_target {
                if tst <= 0.0 || tst > 1.0 {
                    return Err(ParserError::Validation {
                        path: format!("{path}.timeSliceTarget"),
                        message: "TimeSlice target must be between 0 and 1.".to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_operators(&self, path: &str) -> ParserResult<()> {
        if self.op.is_some() && self.value.is_none() {
            return Err(ParserError::Validation {
                path: format!("{path}.value"),
                message: "Value must be specified when using an operator.".to_string(),
            });
        }

        if self.value.is_some() && self.op.is_none() {
            return Err(ParserError::Validation {
                path: format!("{path}.op"),
                message: "Operator must be specified when using a value.".to_string(),
            });
        }

        if let Some(op) = &self.op {
            if matches!(op, Operator::Invalid) {
                return Err(ParserError::Validation {
                    path: format!("{path}.op"),
                    message: "Invalid operator specified.".to_string(),
                });
            }
        }

        Ok(())
    }

    fn validate_indicators(
        &self,
        sli_map: Option<&HashMap<String, SLIDoc>>,
        is_composite: bool,
        path: &str,
    ) -> ParserResult<()> {
        if self.indicator.is_some() && self.indicator_ref.is_some() {
            return Err(ParserError::Validation {
                path: format!("{path}.indicator, {path}.indicatorRef"),
                message: "Cannot specify both indicator and indicatorRef.".to_string(),
            });
        }

        if is_composite {
            if self.indicator.is_none() && self.indicator_ref.is_none() {
                return Err(ParserError::Validation {
                    path: format!("{path}.indicator, {path}.indicatorRef"),
                    message:
                        "Indicator or indicatorRef must be specified for composite objectives."
                            .to_string(),
                });
            }

            if let Some(indicator) = &self.indicator {
                if indicator.spec.is_threshold_metric() {
                    return Err(ParserError::Validation {
                        path: format!("{path}.indicator"),
                        message: "Can't use thresholdMetric in composite objectives.".to_string(),
                    });
                }

                indicator.validate(true, Some(&format!("{path}.indicator")))?;
            }

            if let Some(indicator_ref) = &self.indicator_ref {
                if let Some(sli_map) = sli_map {
                    let indicator =
                        sli_map
                            .get(indicator_ref)
                            .ok_or_else(|| ParserError::Validation {
                                path: format!("{path}.indicatorRef"),
                                message: format!(
                                    "Indicator reference `{}` not found.",
                                    indicator_ref
                                ),
                            })?;

                    if indicator.spec.is_threshold_metric()
                        && (self.op.is_none() || self.value.is_none())
                    {
                        return Err(ParserError::Validation {
                            path: format!("{path}.indicator"),
                            message: "op and value must be specified when using a thresholdMetric."
                                .to_string(),
                        });
                    }
                }
            }

            if let Some(composite_weight) = self.composite_weight {
                if composite_weight < 0.0 {
                    return Err(ParserError::Validation {
                        path: format!("{path}.compositeWeight"),
                        message: "Composite weight must be greater than or equal to 0.".to_string(),
                    });
                }
            };
        }

        if !is_composite {
            if self.indicator.is_some() || self.indicator_ref.is_some() {
                return Err(ParserError::Validation {
                    path: format!("{path}.indicator"),
                    message: "Indicator or indicatorRef must not be specified for non-composite objectives.".to_string(),
                });
            }
            if self.composite_weight.is_some() {
                return Err(ParserError::Validation {
                    path: format!("{path}.compositeWeight"),
                    message: "Composite weight must not be specified for single objectives."
                        .to_string(),
                });
            }
        }

        Ok(())
    }

    pub fn validate(
        &self,
        budgeting_method: &BudgetingMethod,
        sli_map: Option<&HashMap<String, SLIDoc>>,
        is_composite: bool,
        path: &str,
    ) -> ParserResult<()> {
        self.validate_target(path)?;
        self.validate_timeslice(budgeting_method, path)?;
        self.validate_operators(path)?;

        if is_composite {
            if let Some(sli_map) = sli_map {
                self.validate_indicators(Some(sli_map), is_composite, path)?;
            }
        }

        self.validate_indicators(None, is_composite, path)?;

        Ok(())
    }
}

#[cfg(test)]

mod happy_path_tests {
    use super::super::{
        common::{Kind, Metadata},
        sli::{MetricSource, RatioMetric, ThresholdMetric},
    };

    use super::*;

    fn default_sli_map() -> HashMap<String, SLIDoc> {
        let mut sli_map = HashMap::new();
        sli_map.insert(
            "threshold_metric".to_string(),
            SLIDoc {
                spec: SLISpec {
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
                kind: Some(Kind::SLI),
                metadata: Metadata {
                    name: "Test SLI".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
            },
        );

        sli_map.insert(
            "ratio_metric".to_string(),
            SLIDoc {
                kind: Some(Kind::SLI),
                metadata: Metadata {
                    name: "Test SLI".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: SLISpec {
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

        let result = objective.validate(&BudgetingMethod::Occurrences, None, false, "test_path");

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
        let result = objective.validate(
            &BudgetingMethod::Occurrences,
            Some(&sli_map),
            true,
            "test_path",
        );

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
            indicator: Some(SLIDoc {
                kind: None,
                metadata: Metadata {
                    name: "Test SLI".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: SLISpec {
                    description: None,
                    ratio_metric: Some(RatioMetric {
                        counter: Some(true),
                        good: Some(MetricSource {
                            metric_source_ref: Some("good".to_string()),
                            type_: Some("datadoge".to_string()),
                            spec: None,
                        }),
                        bad: None,
                        total: Some(MetricSource {
                            metric_source_ref: Some("bad".to_string()),
                            type_: Some("datadoge".to_string()),
                            spec: None,
                        }),
                        raw_type: None,
                        raw: None,
                    }),
                    threshold_metric: None,
                },
            }),
            indicator_ref: None,
            composite_weight: None,
        };

        let result = objective.validate(&BudgetingMethod::Occurrences, None, true, "test_path");
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

        let result = objective.validate(&BudgetingMethod::Occurrences, None, false, "test_path");

        assert!(result.is_ok());
    }
}

mod unhappy_path_tests {
    use super::super::{
        common::{Kind, Metadata},
        sli::{MetricSource, RatioMetric, ThresholdMetric},
    };
    use super::*;

    fn default_sli_map() -> HashMap<String, SLIDoc> {
        let mut sli_map = HashMap::new();
        sli_map.insert(
            "threshold_metric".to_string(),
            SLIDoc {
                spec: SLISpec {
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
                kind: Some(Kind::SLI),
                metadata: Metadata {
                    name: "Test SLI".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
            },
        );

        sli_map.insert(
            "ratio_metric".to_string(),
            SLIDoc {
                kind: Some(Kind::SLI),
                metadata: Metadata {
                    name: "Test SLI".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: SLISpec {
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

        let result = objective.validate(&BudgetingMethod::Occurrences, None, false, "test_path");

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "test_path.target, test_path.targetPercent"
                    && message == "Cannot specify both target and targetPercent."
            ))
        );
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

        let result = objective.validate(&BudgetingMethod::Occurrences, None, false, "test_path");

        assert!(result.is_err_and(
            |e| matches!(e, ParserError::Validation {path, message} if path == "test_path.target, test_path.targetPercent"
                    && message == "Must specify either target or targetPercent."
            )
        ));
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

        let result =
            objective.validate(&BudgetingMethod::RatioTimeslices, None, false, "test_path");

        assert!(result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
            if path == "test_path.timeSliceTarget, test_path.timeSliceWindow" 
                && message == "TimeSlices budgeting requires timeSliceTarget and timeSliceWindow."
        )));
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

        let result = objective.validate(&BudgetingMethod::Timeslices, None, false, "test_path");

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message } if
                path == "test_path.timeSliceTarget"
                    && message == "TimeSlice target must be between 0 and 1."
            ))
        );
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

        let result = objective.validate(&BudgetingMethod::Occurrences, None, false, "test_path");

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message } if
                path == "test_path.value"
                    && message == "Value must be specified when using an operator."
            ))
        );
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

        let result = objective.validate(&BudgetingMethod::Occurrences, None, false, "test_path");

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message } if
                path == "test_path.op"
                    && message == "Operator must be specified when using a value."
            ))
        );
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

        let result = objective.validate(&BudgetingMethod::Occurrences, None, true, "test_path");

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "test_path.indicator, test_path.indicatorRef"
                    && message
                        == "Indicator or indicatorRef must be specified for composite objectives."
            ))
        );
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
            indicator: Some(SLIDoc {
                kind: None,
                metadata: Metadata {
                    name: "Test SLI".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: SLISpec {
                    description: None,
                    ratio_metric: None,
                    threshold_metric: Some(ThresholdMetric {
                        metric_source: MetricSource {
                            metric_source_ref: Some("some_errors".to_string()),
                            type_: Some("datadoge".to_string()),
                            spec: None,
                        },
                    }),
                },
            }),
            indicator_ref: Some("threshold_metric".to_string()),
            composite_weight: None,
        };

        let sli_map = default_sli_map();
        let result = objective.validate(
            &BudgetingMethod::Occurrences,
            Some(&sli_map),
            true,
            "test_path",
        );

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "test_path.indicator, test_path.indicatorRef"
                    && message == "Cannot specify both indicator and indicatorRef."
            ))
        );
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
        let result = objective.validate(
            &BudgetingMethod::Occurrences,
            Some(&sli_map),
            true,
            "test_path",
        );

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "test_path.compositeWeight"
                    && message == "Composite weight must be greater than or equal to 0."
            ))
        );
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

        let result = objective.validate(&BudgetingMethod::Occurrences, None, false, "test_path");

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "test_path.compositeWeight"
                    && message == "Composite weight must not be specified for single objectives."
            ))
        );
    }

    #[test]
    fn test_valid_composite_objective_threshold_matric() {
        let objective = Objective {
            display_name: Some("Test Objective".to_string()),
            op: Some(Operator::Lte),
            value: Some(0.95),
            target: None,
            target_percent: Some(90.0),
            time_slice_target: None,
            time_slice_window: None,
            indicator: Some(SLIDoc {
                kind: None,
                metadata: Metadata {
                    name: "Test SLI".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: SLISpec {
                    description: None,
                    ratio_metric: None,
                    threshold_metric: Some(ThresholdMetric {
                        metric_source: MetricSource {
                            metric_source_ref: Some("some_errors".to_string()),
                            type_: Some("datadoge".to_string()),
                            spec: None,
                        },
                    }),
                },
            }),
            indicator_ref: None,
            composite_weight: None,
        };

        let result = objective.validate(&BudgetingMethod::Occurrences, None, true, "test_path");

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "test_path.indicator"
                    && message == "Can't use thresholdMetric in composite objectives."
            ))
        );
    }
}
