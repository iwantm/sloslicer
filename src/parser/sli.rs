use serde::Deserialize;

use super::validation::{ValidationError, ValidationResult};

#[derive(Debug, Deserialize)]
pub struct SLISpec {
    pub description: Option<String>,
    #[serde(rename = "thresholdMetric")]
    pub threshold_metric: Option<ThresholdMetric>,
    #[serde(rename = "ratioMetric")]
    pub ratio_metric: Option<RatioMetric>,
}

impl SLISpec {
    pub fn is_threshold_metric(&self) -> bool {
        self.threshold_metric.is_some()
    }

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
pub struct MetricSource {
    #[serde(rename = "metricSourceRef")]
    pub metric_source_ref: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub spec: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub enum RawType {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failure")]
    Failure,
}
