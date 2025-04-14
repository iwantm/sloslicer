use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::alert_condition::AlertConditionDoc;
use super::alert_notification_target::AlertNotificationTargetDoc;
use super::alert_policy::AlertPolicyDoc;
use super::common::{BudgetingMethod, DurationShorthand, Kind, Metadata};
use super::objective::Objective;
use super::sli::SLIDoc;
use crate::utils::errors::{ParserError, ParserResult};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AlertPolicyRef {
    #[serde(rename = "alertPolicyRef")]
    pub alert_policy_ref: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AlertPolicy {
    Inline(Box<AlertPolicyDoc>),
    Reference(AlertPolicyRef),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SLODoc {
    pub kind: Option<Kind>,
    pub metadata: Metadata,
    pub spec: SLOSpec,
}

impl SLODoc {
    pub fn validate(
        &self,
        path: Option<&str>,
        sli_map: Option<&HashMap<String, SLIDoc>>,
        alert_policy_map: Option<&HashMap<String, AlertPolicyDoc>>,
        condition_map: Option<&HashMap<String, AlertConditionDoc>>,
        notification_target_map: Option<&HashMap<String, AlertNotificationTargetDoc>>,
    ) -> ParserResult<()> {
        let path = path.unwrap_or("SLO");

        self.spec.validate(
            sli_map,
            alert_policy_map,
            condition_map,
            notification_target_map,
            &format!("{path}.spec"),
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SLOSpec {
    pub description: Option<String>,
    pub service: String,
    pub indicator: Option<SLIDoc>,
    #[serde(rename = "indicatorRef")]
    pub indicator_ref: Option<String>,
    #[serde(rename = "timeWindow")]
    pub time_window: Option<Vec<TimeWindow>>,
    #[serde(rename = "budgetingMethod")]
    pub budgeting_method: BudgetingMethod,
    pub objectives: Vec<Objective>,
    #[serde(rename = "alertPolicies")]
    pub alert_policies: Option<Vec<AlertPolicy>>,
}

impl SLOSpec {
    pub fn is_composite(&self) -> bool {
        self.objectives.len() > 1
    }

    fn validate_service(&self, path: &str) -> ParserResult<()> {
        if self.service.trim().is_empty() {
            return Err(ParserError::Validation {
                path: format!("{path}.service"),
                message: "Service must be a non-empty string.".to_string(),
            });
        }

        Ok(())
    }

    fn validate_budgeting_method(&self, path: &str) -> ParserResult<()> {
        if self.budgeting_method == BudgetingMethod::Unknown {
            return Err(ParserError::Validation {
                path: format!("{path}.budgetingMethod"),
                message: "Invalid budgeting method.".to_string(),
            });
        }

        Ok(())
    }

    fn validate_indicators(
        &self,
        is_composite: bool,
        sli_map: Option<&HashMap<String, SLIDoc>>,
        path: &str,
    ) -> ParserResult<()> {
        if is_composite && (self.indicator.is_some() || self.indicator_ref.is_some()) {
            return Err(ParserError::Validation {
                path: format!("{path}.indicator"),
                message: "indicator is not allowed for composite SLOs.".to_string(),
            });
        }
        if !is_composite {
            match (&self.indicator, &self.indicator_ref) {
                (Some(_), Some(_)) => {
                    return Err(ParserError::Validation {
                        path: format!("{path}.indicator, {path}.indicatorRef"),
                        message: "Cannot define both indicator and indicatorRef.".to_string(),
                    });
                }
                (None, None) => {
                    return Err(ParserError::Validation {
                        path: format!("{path}.indicator, {path}.indicatorRef"),
                        message: "Must define either indicator or indicatorRef.".to_string(),
                    });
                }
                (Some(indicator), None) => {
                    return indicator.validate(true, Some(&format!("{path}.indicator")));
                }
                (None, Some(indicator_ref)) => match sli_map {
                    Some(sli_map) => {
                        let _ =
                            sli_map
                                .get(indicator_ref)
                                .ok_or_else(|| ParserError::Validation {
                                    path: format!("{path}.indicatorRef"),
                                    message: format!(
                                        "Indicator reference `{}` not found.",
                                        indicator_ref
                                    ),
                                })?;

                        return Ok(());
                    }
                    None => {
                        return Err(ParserError::Validation {
                            path: format!("{path}.indicatorRef"),
                            message: "SLI map is required for indicatorRef validation.".to_string(),
                        });
                    }
                },
            }
        }
        Ok(())
    }

    fn validate_time_window(&self, path: &str) -> ParserResult<()> {
        if let Some(time_window) = &self.time_window {
            if time_window.len() != 1 {
                return Err(ParserError::Validation {
                    path: format!("{path}.timeWindow"),
                    message: "timeWindow must contain exactly one item.".to_string(),
                });
            }
            return time_window[0].validate(&format!("{path}.timeWindow[0]"));
        }

        Ok(())
    }

    fn validate_objectives(
        &self,
        is_composite: bool,
        sli_map: Option<&HashMap<String, SLIDoc>>,
        path: &str,
    ) -> ParserResult<()> {
        if !is_composite {
            if self.objectives.len() != 1 {
                return Err(ParserError::Validation {
                    path: format!("{path}.objectives"),
                    message: "objectives must contain exactly one item.".to_string(),
                });
            }
            return self.objectives[0].validate(
                &self.budgeting_method,
                sli_map,
                is_composite,
                path,
            );
        }

        for (i, objective) in self.objectives.iter().enumerate() {
            objective.validate(
                &self.budgeting_method,
                sli_map,
                is_composite,
                &format!("{path}.objectives[{}]", i),
            )?;
        }

        Ok(())
    }

    fn validate_alert_policy(
        &self,
        alert_policy_map: Option<&HashMap<String, AlertPolicyDoc>>,
        condition_map: Option<&HashMap<String, AlertConditionDoc>>,
        notification_target_map: Option<&HashMap<String, AlertNotificationTargetDoc>>,
        path: &str,
    ) -> ParserResult<()> {
        if let Some(alert_policy) = &self.alert_policies {
            for (i, policy) in alert_policy.iter().enumerate() {
                match policy {
                    AlertPolicy::Inline(policy) => policy.validate(
                        condition_map,
                        notification_target_map,
                        Some(&format!("{path}.alertPolicy[{}]", i)),
                    )?,
                    AlertPolicy::Reference(policy_ref) => {
                        if let Some(alert_policy_map) = alert_policy_map {
                            alert_policy_map
                                .get(&policy_ref.alert_policy_ref)
                                .ok_or_else(|| ParserError::Validation {
                                    path: format!("{path}.alertPolicy[{}]", i),
                                    message: format!(
                                        "Alert policy reference `{}` not found.",
                                        policy_ref.alert_policy_ref
                                    ),
                                })?;
                        } else {
                            return Err(ParserError::Validation {
                                path: format!("{path}.alertPolicy[{}]", i),
                                message: "Alert policy map is required for alert policy reference validation.".to_string(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn validate(
        &self,
        sli_map: Option<&HashMap<String, SLIDoc>>,
        alert_policy_map: Option<&HashMap<String, AlertPolicyDoc>>,
        condition_map: Option<&HashMap<String, AlertConditionDoc>>,
        notification_target_map: Option<&HashMap<String, AlertNotificationTargetDoc>>,
        path: &str,
    ) -> ParserResult<()> {
        let is_composite = self.is_composite();
        self.validate_service(path)?;
        self.validate_budgeting_method(path)?;
        self.validate_indicators(is_composite, sli_map, path)?;
        self.validate_time_window(path)?;
        self.validate_objectives(is_composite, sli_map, path)?;
        self.validate_alert_policy(
            alert_policy_map,
            condition_map,
            notification_target_map,
            path,
        )?;
        Ok(())
    }
}
#[derive(Debug, Deserialize, Serialize)]
pub struct TimeWindow {
    pub duration: DurationShorthand,
    pub calendar: Option<CalendarDetails>,
    #[serde(rename = "isRolling")]
    pub is_rolling: bool,
}

impl TimeWindow {
    pub fn validate(&self, path: &str) -> ParserResult<()> {
        if self.is_rolling && self.calendar.is_some() {
            return Err(ParserError::Validation {
                path: format!("{path}.calendar"),
                message:
                    "Calendar details can only be specified for Calendar Aligned time windows."
                        .to_string(),
            });
        }

        if !self.is_rolling && self.calendar.is_none() {
            return Err(ParserError::Validation {
                path: format!("{path}.calendar"),
                message: "Calendar details must be specified for Calendar Aligned time windows."
                    .to_string(),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CalendarDetails {
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "timeZone")]
    pub time_zone: String,
}

#[cfg(test)]
mod happy_path_tests {
    use super::super::{
        alert_condition::{AlertConditionSpec, Condition, CondtionKind},
        alert_notification_target::AlertNotificationTargetSpec,
        alert_policy::{AlertCondition, AlertPolicySpec, NotificationTarget},
        common::Operator,
        sli::{MetricSource, RatioMetric, RawType, SLISpec, ThresholdMetric},
    };

    use super::*;
    use serde_yaml;

    #[test]
    fn minimal_valid_slo() {
        let yaml = r#"
        kind: SLO
        metadata:
            name: foo
        spec:
            service: foo
            budgetingMethod: Occurrences
            indicator:
                metadata:
                    name: foo-indicator
                spec:
                    ratioMetric:
                        counter: true
                        good:
                            metricSource:
                                metricSourceRef: datadog-datasource
                                type: Datadog
                                spec:
                                    query: sum:trace.http.request.hits.by_http_status{*}.as_count()
                        total:
                            metricSource:
                                metricSourceRef: datadog-datasource
                                type: Datadog
                                spec:
                                    query: sum:trace.http.request.hits.by_http_status{*}.as_count()
            objectives:
                - displayName: Foo Total Errors
                  target: 0.98
        "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();

        let result = slo.validate(None, None, None, None, None);

        assert!(result.is_ok(), "Expected valid SLO to pass validation");
    }

    #[test]
    fn calendar_window() {
        let yaml = r#"
        kind: SLO
        metadata:
            name: foo
        spec:
            description: "Monthly uptime tracking"
            service: backend-api
            indicatorRef: availability-sli
            budgetingMethod: Timeslices
            timeWindow:
                - duration: 1M
                  calendar:
                    startTime: "2025-04-01 00:00:00"
                    timeZone: "UTC"
                  isRolling: false
            objectives:
                - targetPercent: 99.5
                  timeSliceTarget: 0.99
                  timeSliceWindow: 5m
        "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();
        let mut sli_map = HashMap::new();

        sli_map.insert(
            "availability-sli".to_string(),
            SLIDoc {
                kind: Some(Kind::Slo),
                metadata: Metadata {
                    name: "availability-sli".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: SLISpec {
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
            },
        );

        let result = slo.validate(None, Some(&sli_map), None, None, None);

        assert!(result.is_ok(), "Expected valid SLO to pass validation");
    }

    #[test]
    fn composite_slo() {
        let yaml = r#"
        kind: SLO
        metadata:
            name: foo
        spec:
            service: composite-service
            budgetingMethod: RatioTimeslices
            timeWindow:
                - duration: 7d
                  isRolling: true
            objectives:
                - target: 0.98
                  indicatorRef: slo-1
                  compositeWeight: 1
                  timeSliceTarget: 0.95
                  timeSliceWindow: 1h
                - targetPercent: 99.9
                  indicatorRef: slo-2
                  compositeWeight: 2
                  timeSliceTarget: 0.97
                  timeSliceWindow: 1h
        "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();
        let mut sli_map = HashMap::new();

        sli_map.insert(
            "slo-2".to_string(),
            SLIDoc {
                kind: Some(Kind::Slo),
                metadata: Metadata {
                    name: "availability-sli".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: SLISpec {
                    threshold_metric: None,
                    description: None,
                    ratio_metric: Some(RatioMetric {
                        counter: Some(true),
                        good: None,
                        bad: None,
                        total: None,
                        raw_type: Some(RawType::Success),
                        raw: Some(MetricSource {
                            metric_source_ref: Some("datadoge".to_string()),
                            type_: Some("datadoge".to_string()),
                            spec: None,
                        }),
                    }),
                },
            },
        );

        sli_map.insert(
            "slo-1".to_string(),
            SLIDoc {
                kind: Some(Kind::Slo),
                metadata: Metadata {
                    name: "availability-sli".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: SLISpec {
                    threshold_metric: None,
                    description: None,
                    ratio_metric: Some(RatioMetric {
                        counter: Some(true),
                        good: Some(MetricSource {
                            metric_source_ref: Some("datadoge".to_string()),
                            type_: Some("datadoge".to_string()),
                            spec: None,
                        }),
                        bad: None,
                        total: Some(MetricSource {
                            metric_source_ref: Some("datadoge".to_string()),
                            type_: Some("datadoge".to_string()),
                            spec: None,
                        }),
                        raw_type: None,
                        raw: None,
                    }),
                },
            },
        );

        let result = slo.validate(None, Some(&sli_map), None, None, None);

        assert!(result.is_ok(), "Expected valid SLO to pass validation");
    }

    #[test]
    fn alert_policy_ref() {
        let yaml = r#"
        kind: SLO
        metadata:
            name: foo
        spec:
            service: alerts-service
            indicatorRef: error-rate-sli
            budgetingMethod: Occurrences
            timeWindow:
                - duration: 30d
                  isRolling: true
            objectives:
                - target: 0.99
            alertPolicies:
                - alertPolicyRef: high-error-rate-alert
        "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();
        let mut sli_map = HashMap::new();
        let mut alert_policy_map = HashMap::new();

        sli_map.insert(
            "error-rate-sli".to_string(),
            SLIDoc {
                kind: Some(Kind::Slo),
                metadata: Metadata {
                    name: "availability-sli".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: SLISpec {
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
            },
        );

        alert_policy_map.insert(
            "high-error-rate-alert".to_string(),
            AlertPolicyDoc {
                kind: Kind::AlertPolicy,
                metadata: Metadata {
                    name: "high-error-rate-alert".to_string(),
                    display_name: None,
                    labels: None,
                    annotations: None,
                },
                spec: AlertPolicySpec {
                    description: None,
                    alert_when_no_data: false,
                    alert_when_resolved: false,
                    alert_when_breaching: false,
                    conditions: vec![AlertCondition::Inline(Box::new(AlertConditionDoc {
                        kind: Kind::AlertCondition,
                        metadata: Metadata {
                            name: "high-error-rate-alert".to_string(),
                            display_name: None,
                            labels: None,
                            annotations: None,
                        },
                        spec: AlertConditionSpec {
                            description: None,
                            severity: "sev1".to_string(),
                            condition: Condition {
                                kind: CondtionKind::Burnrate,
                                op: Some(Operator::Gte),
                                threshold: Some(0.8),
                                lookback_window: Some(DurationShorthand("5m".to_string())),
                                alert_after: Some(DurationShorthand("0m".to_string())),
                            },
                        },
                    }))],
                    notification_targets: vec![NotificationTarget::Inline(
                        AlertNotificationTargetDoc {
                            kind: Kind::AlertNotificationTarget,
                            metadata: Metadata {
                                name: "notification-target".to_string(),
                                display_name: None,
                                labels: None,
                                annotations: None,
                            },
                            spec: AlertNotificationTargetSpec {
                                description: None,
                                target: "slack".to_string(),
                            },
                        },
                    )],
                },
            },
        );

        let result = slo.validate(None, Some(&sli_map), Some(&alert_policy_map), None, None);

        assert!(result.is_ok(), "Expected valid SLO to pass validation");
    }

    #[test]
    fn everything_inline() {
        let yaml = r#"
            kind: SLO
            metadata:
                name: foo
            spec:
                description: "Monthly uptime tracking"
                service: foo
                budgetingMethod: Occurrences
                indicator:
                    metadata:
                        name: foo-indicator
                    spec:
                        ratioMetric:
                            counter: true
                            good:
                                metricSource:
                                    metricSourceRef: datadog-datasource
                                    type: Datadog
                                    spec:
                                        query: sum:trace.http.request.hits.by_http_status{*}.as_count()
                            total:
                                metricSource:
                                    metricSourceRef: datadog-datasource
                                    type: Datadog
                                    spec:
                                        query: sum:trace.http.request.hits.by_http_status{*}.as_count()
                objectives:
                    - displayName: Foo Total Errors
                      target: 0.98
                alertPolicies:
                    - kind: AlertPolicy
                      metadata:
                        name: latency-policy
                      spec:
                        description: Inlined condition, referenced target
                        alertWhenNoData: false
                        alertWhenResolved: true
                        alertWhenBreaching: false
                        conditions:
                            - kind: AlertCondition
                              metadata:
                                name: high-latency
                              spec:
                                severity: page
                                condition:
                                    kind: burnrate
                                    op: gte
                                    threshold: 1.5
                                    lookbackWindow: 5m
                        notificationTargets:
                            - kind: AlertNotificationTarget
                              metadata:
                                name: slack-notification
                              spec:
                                description: Slack notification
                                target: slack
            "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();

        let result = slo.validate(None, None, None, None, None);

        assert!(result.is_ok(), "Expected valid SLO to pass validation");
    }
}

#[cfg(test)]
mod unhappy_path_tests {

    use super::*;
    use serde_yaml;

    #[test]
    fn missing_indicator() {
        let yaml = r#"
        kind: SLO
        metadata:
            name: foo
        spec:
            service: missing-indicator
            budgetingMethod: Occurrences
            objectives:
            - target: 0.99
        "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();

        let result = slo.validate(None, None, None, None, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message } if
                path == "SLO.spec.indicator, SLO.spec.indicatorRef"
                    && message == "Must define either indicator or indicatorRef."
            ))
        );
    }

    #[test]
    fn double_indicator() {
        let yaml = r#"
        kind: SLO
        metadata:
            name: foo
        spec:
            service: missing-indicator
            budgetingMethod: Occurrences
            indicatorRef: missing-indicator
            indicator:
                metadata:
                    name: foo-indicator
                spec:
                    ratioMetric:
                        counter: true
                        good:
                            metricSource:
                                metricSourceRef: datadog-datasource
                                type: Datadog
                                spec:
                                    query: sum:trace.http.request.hits.by_http_status{*}.as_count()
                        total:
                            metricSource:
                                metricSourceRef: datadog-datasource
                                type: Datadog
                                spec:
                                    query: sum:trace.http.request.hits.by_http_status{*}.as_count()
            objectives:
            - target: 0.99
        "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();

        let result = slo.validate(None, None, None, None, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message } if
                path == "SLO.spec.indicator, SLO.spec.indicatorRef"
                    && message == "Cannot define both indicator and indicatorRef."
            ))
        );
    }

    #[test]
    fn alert_policy_not_in_map() {
        let yaml = r#"
        kind: SLO
        metadata:
            name: foo
        spec:
            service: missing-indicator
            budgetingMethod: Occurrences
            indicator:
                metadata:
                    name: foo-indicator
                spec:
                    ratioMetric:
                        counter: true
                        good:
                            metricSource:
                                metricSourceRef: datadog-datasource
                                type: Datadog
                                spec:
                                    query: sum:trace.http.request.hits.by_http_status{*}.as_count()
                        total:
                            metricSource:
                                metricSourceRef: datadog-datasource
                                type: Datadog
                                spec:
                                    query: sum:trace.http.request.hits.by_http_status{*}.as_count()
            alertPolicies:
                - alertPolicyRef: not-found
            objectives:
                - target: 0.99
        "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();

        let result = slo.validate(None, None, Some(&HashMap::new()), None, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message } if
                path == "SLO.spec.alertPolicy[0]"
                    && message == "Alert policy reference `not-found` not found."
            ))
        );
    }

    #[test]
    fn indicator_ref_not_found() {
        let yaml: &str = r#"
        kind: SLO
        metadata:
            name: foo
        spec:
            service: foo
            budgetingMethod: Occurrences
            indicatorRef: not-found
            objectives:
                - displayName: Foo Total Errors
                  target: 0.98
        "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();

        let result = slo.validate(None, Some(&HashMap::new()), None, None, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message }
                if path == "SLO.spec.indicatorRef"
                    && message == "Indicator reference `not-found` not found."
            ))
        );
    }

    #[test]
    fn top_level_indicator_composite() {
        let yaml: &str = r#"
        kind: SLO
        metadata:
            name: foo
        spec:
            service: bad-composite
            budgetingMethod: RatioTimeslices
            indicator:
                metadata:
                    name: foo-indicator
                spec:
                    ratioMetric:
                        counter: true
                        good:
                            metricSource:
                                metricSourceRef: datadog-datasource
                                type: Datadog
                                spec:
                                    query: sum:trace.http.request.hits.by_http_status{*}.as_count()
                        total:
                            metricSource:
                                metricSourceRef: datadog-datasource
                                type: Datadog
                                spec:
                                    query: sum:trace.http.request.hits.by_http_status{*}.as_count()
            timeWindow:
                - duration: 1w
                  isRolling: true
            objectives:
                - target: 0.9
                  indicatorRef: foo
                  timeSliceTarget: 0.9
                  timeSliceWindow: 5m
                - target: 0.95
                  indicatorRef: bar
                  timeSliceTarget: 0.9
                  timeSliceWindow: 5m
        "#;

        let slo: SLODoc = serde_yaml::from_str(yaml).unwrap();

        let result = slo.validate(None, Some(&HashMap::new()), None, None, None);

        assert!(
            result.is_err_and(|e| matches!(e, ParserError::Validation { path, message } if
                path == "SLO.spec.indicator"
                    && message == "indicator is not allowed for composite SLOs."
            ))
        );
    }
}
