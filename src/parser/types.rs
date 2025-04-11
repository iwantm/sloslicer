use std::{collections::HashMap, path};

use serde::Deserialize;

use crate::parser::validation::ValidationResult;

use super::validation::ValidationError;
#[derive(Debug, Deserialize)]
pub struct Document {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: Kind,
    pub metadata: Metadata,
    pub spec: Spec,
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub labels: Option<HashMap<String, StringOrVec>>,
    pub annotations: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
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
    DataSource(DataSourceSpec),
    SLO(SLOSpec),
    SLI(SLISpec),
    AlertPolicy(AlertPolicySpec),
    AlertCondition(AlertConditionSpec),
    AlertNotificationTarget(AlertNotificationTargetSpec),
    // Service(ServiceSpec),
}

#[derive(Debug, Deserialize)]
pub struct DataSourceSpec {
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "connectionDetails")]
    pub connection_details: serde_yaml::Value,
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
    pub duration: String,
    pub calendar: CalendarDetails,
    #[serde(default, rename = "isRolling")]
    pub is_rolling: bool,
}

#[derive(Debug, Deserialize)]
pub struct RollingTimeWindow {
    pub duration: String,
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

fn default_composite_weight() -> f64 {
    1.0
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
    pub time_slice_window: Option<String>,
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
pub enum Operator {
    #[serde(rename = "lte")]
    Lte,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "gt")]
    Gt,
}

#[derive(Debug, Deserialize)]
pub struct SLISpec {
    pub description: Option<String>,
    #[serde(rename = "thresholdMetric")]
    pub threshold_metric: Option<ThresholdMetric>,
    #[serde(rename = "ratioMetric")]
    pub ratio_metric: Option<RatioMetric>,
}

impl SLISpec {
    pub fn validate(&self, path: &str) -> ValidationResult {
        if self.threshold_metric.is_some() && self.ratio_metric.is_some() {
            return Err(ValidationError::new(
                format!("{path}.thresholdMetric"),
                "Cannot specify both thresholdMetric and ratioMetric.",
            ));
        }
        if self.threshold_metric.is_none() && self.ratio_metric.is_none() {
            return Err(ValidationError::new(
                format!("{path}.thresholdMetric"),
                "Must specify either thresholdMetric or ratioMetric.",
            ));
        }

        if let Some(ratio_metric) = &self.ratio_metric {
            ratio_metric.validate(&format!("{path}.ratioMetric"))?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct ThresholdMetric {
    pub metric_source: MetricSource,
}

#[derive(Debug, Deserialize)]
pub struct MetricSource {
    #[serde(rename = "metricSourceRef")]
    pub metric_source_ref: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub spec: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RatioMetric {
    pub counter: Option<bool>,
    pub good: Option<MetricSource>,
    pub bad: Option<MetricSource>,
    pub total: Option<MetricSource>,
    pub raw_type: Option<RawType>,
    pub raw: Option<MetricSource>,
}

impl RatioMetric {
    pub fn validate(&self, path: &str) -> ValidationResult {
        if self.good.is_some() && self.bad.is_some() {
            return Err(ValidationError::new(
                format!("{path}.good"),
                "Cannot specify both good and bad.",
            ));
        }

        if (self.good.is_some() || self.bad.is_some()) && self.total.is_none() {
            return Err(ValidationError::new(
                format!("{path}.good"),
                "Must specify total when using good or bad.",
            ));
        }

        if self.raw.is_some() && (self.good.is_some() || self.bad.is_some() || self.total.is_some())
        {
            return Err(ValidationError::new(
                format!("{path}.raw"),
                "Cannot specify raw with good, bad, or total.",
            ));
        }

        if self.raw.is_some() && self.raw_type.is_none() {
            return Err(ValidationError::new(
                format!("{path}.rawType"),
                "Must specify rawType when using raw.",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub enum RawType {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failure")]
    Failure,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AlertCondition {
    Inline(AlertConditionSpec),
    Reference(String),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NotificationTargetSpec {
    Inline(AlertNotificationTargetSpec),
    Reference(String),
}

#[derive(Debug, Deserialize)]
pub struct AlertPolicySpec {
    pub description: Option<String>,
    #[serde(default, rename = "alertWhenNoData")]
    pub alert_when_no_data: bool,
    #[serde(default, rename = "alertWhenResolved")]
    pub alert_when_resolved: bool,
    #[serde(default, rename = "alertWhenBreaching")]
    pub alert_when_breaching: bool,
    condition: Vec<AlertCondition>,
    #[serde(rename = "notificationTargets")]
    pub notification_targets: Vec<NotificationTargetSpec>,
}

impl AlertPolicySpec {
    pub fn validate(
        &self,
        condition_map: &HashMap<String, AlertConditionSpec>,
        notification_target_map: &HashMap<String, AlertNotificationTargetSpec>,
        path: &str,
    ) -> ValidationResult {
        //currently condition only accepts a single value
        if self.condition.len() != 1 {
            return Err(ValidationError::new(
                format!("{path}.condition"),
                "Condition must contain exactly one item.",
            ));
        }

        match &self.condition[0] {
            AlertCondition::Reference(reference) => {
                let condition = condition_map.get(reference).ok_or_else(|| {
                    return ValidationError::new(
                        format!("{path}.condition"),
                        format!("Condition reference `{}` not found.", reference),
                    );
                })?;

                condition.validate(&format!("{path}.condition[{}]", reference))?;
            }
            AlertCondition::Inline(inline_condition) => {
                inline_condition.validate(&format!("{path}.condition"))?;
            }
        }

        for (i, target) in self.notification_targets.iter().enumerate() {
            match target {
                NotificationTargetSpec::Reference(reference) => {
                    let notification_target =
                        notification_target_map.get(reference).ok_or_else(|| {
                            return ValidationError::new(
                                format!("{path}.notificationTargets"),
                                format!("Notification target reference `{}` not found.", reference),
                            );
                        })?;

                    notification_target
                        .validate(&format!("{path}.notificationTargets[{}]", reference))?;
                }
                NotificationTargetSpec::Inline(inline_target) => {
                    inline_target.validate(&format!("{path}.notificationTargets"))?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Condition {
    kind: String,
    op: Operator,
}

#[derive(Debug, Deserialize)]
pub struct AlertConditionSpec {
    description: Option<String>,
    severity: String,
    condition: Condition,
    threshold: f64,
    #[serde(rename = "lookbackWindow")]
    lookback_window: String,
    #[serde(rename = "alertAfter")]
    alert_after: String,
}

impl AlertConditionSpec {
    pub fn validate(&self, path: &str) -> ValidationResult {
        if matches!(self.condition.op, Operator::Lte) && self.threshold > 1.0 {
            return Err(ValidationError::new(
                format!("{path}.threshold"),
                "Threshold must be between 0 and 1 for Lte operator.",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct AlertNotificationTargetSpec {
    target: String,
    description: Option<String>,
}

impl AlertNotificationTargetSpec {
    pub fn validate(&self, path: &str) -> ValidationResult {
        if self.target.is_empty() {
            return Err(ValidationError::new(
                format!("{path}.target"),
                "Target must not be empty.",
            ));
        }
        Ok(())
    }
}
