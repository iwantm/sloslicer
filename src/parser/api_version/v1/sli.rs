use serde::Deserialize;

use super::common::{Kind, Metadata};
use crate::parser::errors::{ParserError, ParserResult};

#[derive(Debug, Deserialize, PartialEq)]
pub struct SLIDoc {
    pub kind: Option<Kind>,
    pub metadata: Metadata,
    pub spec: SLISpec,
}

impl SLIDoc {
    pub fn validate(&self, is_inline: bool, path: Option<&str>) -> ParserResult<()> {
        let path = path.unwrap_or("SLI");

        if is_inline && self.kind.is_some() {
            return Err(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Inline SLI must not have a kind.".to_string(),
            });
        }

        self.spec.validate(&format!("{path}.spec"))
    }
}

#[derive(Debug, Deserialize, PartialEq)]
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

    pub fn validate(&self, path: &str) -> ParserResult<()> {
        if self.threshold_metric.is_some() && self.ratio_metric.is_some() {
            return Err(ParserError::Validation {
                path: format!("{path}.thresholdMetric, {path}.ratioMetric"),
                message: "Cannot specify both thresholdMetric and ratioMetric.".to_string(),
            });
        }

        if self.threshold_metric.is_none() && self.ratio_metric.is_none() {
            return Err(ParserError::Validation {
                path: format!("{path}.thresholdMetric, {path}.ratioMetric"),
                message: "Must specify either thresholdMetric or ratioMetric.".to_string(),
            });
        }

        if let Some(threshold_metric) = &self.threshold_metric {
            threshold_metric.validate(&format!("{path}.ratioMetric"))?;
        }

        if let Some(ratio_metric) = &self.ratio_metric {
            ratio_metric.validate(&format!("{path}.ratioMetric"))?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ThresholdMetric {
    #[serde(rename = "metricSource")]
    pub metric_source: MetricSource,
}

impl ThresholdMetric {
    pub fn validate(&self, path: &str) -> ParserResult<()> {
        self.metric_source
            .validate(&format!("{path}.metricSource"))?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct RatioMetric {
    pub counter: Option<bool>,
    pub good: Option<MetricSource>,
    pub bad: Option<MetricSource>,
    pub total: Option<MetricSource>,
    #[serde(rename = "rawType")]
    pub raw_type: Option<RawType>,
    pub raw: Option<MetricSource>,
}

impl RatioMetric {
    pub fn validate(&self, path: &str) -> ParserResult<()> {
        if self.raw.is_some() && (self.good.is_some() || self.bad.is_some() || self.total.is_some())
        {
            return Err(ParserError::Validation {
                path: format!("{path}.raw"),
                message: "Cannot specify raw with good, bad, or total.".to_string(),
            });
        }

        if self.good.is_some() && self.bad.is_some() {
            return Err(ParserError::Validation {
                path: format!("{path}.good, {path}.bad"),
                message: "Cannot specify both good and bad.".to_string(),
            });
        }

        if (self.good.is_some() || self.bad.is_some()) && self.total.is_none() {
            return Err(ParserError::Validation {
                path: format!("{path}.total"),
                message: "Must specify total when using good or bad.".to_string(),
            });
        }

        if self.total.is_some() && (self.good.is_none() && self.bad.is_none()) {
            return Err(ParserError::Validation {
                path: format!("{path}.total"),
                message: "Must specify good or bad when using total.".to_string(),
            });
        }

        if self.raw.is_some() && self.raw_type.is_none() {
            return Err(ParserError::Validation {
                path: format!("{path}.rawType"),
                message: "Must specify rawType when using raw.".to_string(),
            });
        }

        if self.raw.is_some() && self.counter.is_some() {
            println!("Counter ignored when using raw.") // replace with warning later.
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct MetricSource {
    #[serde(rename = "metricSourceRef")]
    pub metric_source_ref: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub spec: Option<serde_yaml::Value>,
}

impl MetricSource {
    pub fn validate(&self, path: &str) -> ParserResult<()> {
        if self.metric_source_ref.is_none() && self.type_.is_none() {
            return Err(ParserError::Validation {
                path: format!("{path}.metricSourceRef"),
                message: "Must specify one of type or metricSourceRef.".to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum RawType {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failure")]
    Failure,
}

#[cfg(test)]
mod happy_path_tests {
    use super::*;
    use serde_yaml;

    #[test]
    fn test_sli_doc_with_type() {
        let yaml = r#"
        kind: SLI
        metadata:
            name: http-requests
        spec:
            thresholdMetric:
                metricSource:
                    type: Prometheus
                    spec:
                        query: sum(rate(http_requests_total[5m]))
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(false, None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_sli_doc_with_ref() {
        let yaml = r#"
        metadata:
            name: rate-errors
        spec:
            thresholdMetric:
                metricSource:
                    metricSourceRef: prometheus-datasource
                    spec:
                        query: rate_errors_total
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(true, None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_sli_ratio_good() {
        let yaml = r#"
        metadata:
            name: good/total
        spec:
            ratioMetric:
                counter: true
                good:
                    metricSource:
                        type: Datadog
                        spec:
                            query: good_requests
                total:
                    metricSource:
                        type: Datadog
                        spec:
                            query: total_requests
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(true, None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_sli_ratio_bad() {
        let yaml = r#"
        metadata:
            name: good/total
        spec:
            ratioMetric:
                counter: false
                bad:
                    metricSource:
                        metricSourceRef: error-metric
                total:
                    metricSource:
                        type: Prometheus
                        spec:
                            query: total_queries
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(true, None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_sli_raw() {
        let yaml = r#"
        metadata:
            name: good/total
        spec:
            ratioMetric:
                rawType: success
                raw:
                    metricSource:
                        type: Prometheus
                        spec:
                            query: precomputed_ratio
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(true, None);
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod unhappy_path_tests {
    use super::*;
    use serde_yaml;

    #[test]
    fn no_metric() {
        let yaml = r#"
        kind: SLI
        metadata:
            name: http-requests
        spec:
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(false, None);
        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "SLI.spec.thresholdMetric, SLI.spec.ratioMetric"
                    && message == "Must specify either thresholdMetric or ratioMetric."
            ))
        );
    }

    #[test]
    fn both_types() {
        let yaml = r#"
        kind: SLI
        metadata:
            name: http-requests
        spec:
            thresholdMetric:
                metricSource:
                    type: Prometheus
                    spec:
                        query: something
            ratioMetric:
                rawType: success
                raw:
                    metricSource:
                        type: Prometheus
                        spec:
                            query: precomputed_ratio
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(false, None);
        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "SLI.spec.thresholdMetric, SLI.spec.ratioMetric"
                    && message == "Cannot specify both thresholdMetric and ratioMetric."
            ))
        );
    }

    #[test]
    fn invalid_ratio_good_bad() {
        let yaml = r#"
        kind: SLI
        metadata:
            name: http-requests
        spec:
            ratioMetric:
                good:
                    metricSource:
                        type: Prometheus
                bad:
                    metricSource:
                        type: Prometheus
                total:
                    metricSource:
                        type: Prometheus
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(false, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
            if path == "SLI.spec.ratioMetric.good, SLI.spec.ratioMetric.bad"
                    && message == "Cannot specify both good and bad."
            ))
        );
    }

    #[test]
    fn invalid_ratio_good_no_total() {
        let yaml = r#"
        kind: SLI
        metadata:
            name: http-requests
        spec:
            ratioMetric:
                good:
                    metricSource:
                        type: Prometheus
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(false, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "SLI.spec.ratioMetric.total"
                    && message == "Must specify total when using good or bad."
            ))
        );
    }

    #[test]
    fn invalid_ratio_no_good_no_bad() {
        let yaml = r#"
        kind: SLI
        metadata:
            name: http-requests
        spec:
            ratioMetric:
                total:
                    metricSource:
                        type: Prometheus
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(false, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "SLI.spec.ratioMetric.total"
                    && message == "Must specify good or bad when using total."
            ))
        );
    }

    #[test]
    fn invalid_ratio_raw_no_type() {
        let yaml = r#"
        kind: SLI
        metadata:
            name: http-requests
        spec:
            ratioMetric:
                raw:
                    metricSource:
                        type: Prometheus
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(false, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "SLI.spec.ratioMetric.rawType"
                    && message == "Must specify rawType when using raw."
            ))
        );
    }

    #[test]
    fn invalid_ratio_raw_and_good() {
        let yaml = r#"
        kind: SLI
        metadata:
            name: http-requests
        spec:
            ratioMetric:
                rawType: failure
                raw:
                    metricSource:
                        type: Prometheus
                good:
                    metricSource:
                        type: Prometheus
      "#;

        let sli: SLIDoc = serde_yaml::from_str(yaml).unwrap();

        let result = sli.validate(false, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message } if
                path == "SLI.spec.ratioMetric.raw"
                    && message == "Cannot specify raw with good, bad, or total."
            ))
        );
    }
}
