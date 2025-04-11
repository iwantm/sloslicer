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
        println!("Validating SLOSpec: {:?}", self);
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
#[serde(untagged)]
pub enum TimeWindow {
    Rolling(RollingTimeWindow),
    CalendarAligned(CalendarAlignedTimeWindow),
}

impl TimeWindow {
    pub fn validate(&self, path: &str) -> ValidationResult {
        match self {
            TimeWindow::Rolling(rolling) => {
                if !rolling.is_rolling {
                    return Err(ValidationError::new(
                        format!("{path}.isRolling"),
                        "Rolling time window must have isRolling set to true.",
                    ));
                }
            }
            TimeWindow::CalendarAligned(calendar) => {
                if calendar.is_rolling {
                    return Err(ValidationError::new(
                        format!("{path}.isRolling"),
                        "Calendar-aligned time window must have isRolling set to false.",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CalendarAlignedTimeWindow {
    pub duration: DurationShorthand,
    pub calendar: CalendarDetails,
    #[serde(default, rename = "isRolling")]
    pub is_rolling: bool,
}

#[derive(Debug, Deserialize)]
pub struct RollingTimeWindow {
    pub duration: DurationShorthand,
    #[serde(default = "default_is_rolling", rename = "isRolling")]
    pub is_rolling: bool,
}

#[derive(Debug, Deserialize)]
pub struct CalendarDetails {
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "timeZone")]
    pub time_zone: String,
}

fn default_is_rolling() -> bool {
    true
}
