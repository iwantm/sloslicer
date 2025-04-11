use std::collections::HashMap;

use serde::Deserialize;

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
    // AlertPolicy(AlertPolicySpec),
    // AlertCondition(AlertConditionSpec),
    // AlertNotificationTarget(AlertNotificationTargetSpec),
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
    pub indicatior: Option<SLISpec>,
    #[serde(rename = "indicatorRef")]
    pub indicator_ref: Option<String>,
    #[serde(rename = "timeWindow")]
    pub time_window: Option<Vec<TimeWindow>>,
    #[serde(rename = "budgetingMethod")]
    pub budgeting_method: BudgetingMethod,
    pub objectives: Vec<Objective>,
    // #[serde(rename = "alertPolicy")]
    // pub alert_policy: Option<Vec<AlertPolicySpec>>,
}

impl SLOSpec {
    pub fn validate(&self, sli_map: &HashMap<String, SLISpec>) -> Result<(), String> {
        if self.indicatior.is_some() && self.indicator_ref.is_some() {
            return Err("Cannot specify both indicator and indicatorRef".to_string());
        }

        if let Some(time_window) = &self.time_window {
            if time_window.len() != 1 {
                return Err("timeWindow must contain exactly one item.".to_string());
            }
            time_window[0].validate()?;
        }

        if self.objectives.is_empty() {
            return Err("objectives must contain at least one item.".to_string());
        }

        if let Some(indicator) = &self.indicatior {
            if indicator.threshold_metric.is_some() && self.objectives.len() != 1 {
                return Err(
                    "Only one objective is allowed when using a thresholdMetric.".to_string(),
                );
            }

            indicator.validate()?;
        }

        if let Some(indicator_ref) = &self.indicator_ref {
            let sli = sli_map.get(indicator_ref).ok_or_else(|| {
                format!(
                    "Indicator {} not found in the provided SLI map.",
                    indicator_ref
                )
            })?;
            sli.validate()?;
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
    pub fn validate(&self) -> Result<(), String> {
        match self {
            TimeWindow::Rolling(rolling) => {
                if !rolling.is_rolling {
                    return Err("Rolling time window must have isRolling set to true.".to_string());
                }
            }
            TimeWindow::CalendarAligned(calendar) => {
                if calendar.is_rolling {
                    return Err(
                        "Calendar-aligned time window must have isRolling set to false."
                            .to_string(),
                    );
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
    pub indicatior: Option<SLISpec>,
    #[serde(rename = "indicatorRef")]
    pub indicator_ref: Option<String>,
    #[serde(default = "default_composite_weight", rename = "compositeWeight")]
    pub composite_weight: f64,
}

impl Objective {
    pub fn validate(&self) -> Result<(), String> {
        if self.target.is_some() && self.target_percent.is_some() {
            return Err("Cannot specify both target and targetPercent.".to_string());
        }
        if self.target.is_none() && self.target_percent.is_none() {
            return Err("Must specify either target or targetPercent.".to_string());
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
    pub fn validate(&self) -> Result<(), String> {
        if self.threshold_metric.is_some() && self.ratio_metric.is_some() {
            return Err("Cannot specify both thresholdMetric and ratioMetric.".to_string());
        }
        if self.threshold_metric.is_none() && self.ratio_metric.is_none() {
            return Err("Must specify either thresholdMetric or ratioMetric.".to_string());
        }

        if let Some(ratio_metric) = &self.ratio_metric {
            ratio_metric.validate()?;
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
    pub fn validate(&self) -> Result<(), String> {
        if self.good.is_some() && self.bad.is_some() {
            return Err("Cannot specify both good and bad.".to_string());
        }

        if (self.good.is_some() || self.bad.is_some()) && self.total.is_none() {
            return Err("Must specify total when using good or bad.".to_string());
        }

        if self.raw.is_some() && (self.good.is_some() || self.bad.is_some() || self.total.is_some())
        {
            return Err("Cannot specify raw with good, bad, or total.".to_string());
        }

        if self.raw.is_some() && self.raw_type.is_none() {
            return Err("Must specify rawType when using raw.".to_string());
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
