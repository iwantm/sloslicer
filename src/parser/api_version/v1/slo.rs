use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::alert_policy::AlertPolicyDoc;
use super::common::{BudgetingMethod, DurationShorthand, Kind, Metadata};
use super::objective::Objective;
use super::sli::SLIDoc;
use crate::parser::document::Document;
use crate::utils::errors::ParserError;
use crate::utils::validation_context::ValidationContext;

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct AlertPolicyRef {
    #[serde(rename = "alertPolicyRef")]
    pub alert_policy_ref: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
#[serde(untagged)]
pub enum AlertPolicy {
    Inline(Box<AlertPolicyDoc>),
    Reference(AlertPolicyRef),
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct SLODoc {
    pub kind: Option<Kind>,
    pub metadata: Metadata,
    pub spec: SLOSpec,
}

impl SLODoc {
    pub fn validate(
        &self,
        path: Option<&str>,
        document_map: &HashMap<String, Document>,
        ctx: &mut ValidationContext,
    ) {
        let path = path.unwrap_or("");

        if self.kind.as_ref().is_some_and(|k| !matches!(k, Kind::Slo)) {
            ctx.push(ParserError::Validation {
                path: format!("{path}.kind"),
                message: "Expected kind to be SLO.".to_string(),
            });
        }

        self.spec
            .validate(&format!("{path}.spec"), document_map, ctx);
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
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

    fn validate_service(&self, path: &str, ctx: &mut ValidationContext) {
        if self.service.trim().is_empty() {
            ctx.push(ParserError::Validation {
                path: format!("{path}.service"),
                message: "Service must be a non-empty string.".to_string(),
            });
        }
    }

    fn validate_budgeting_method(&self, path: &str, ctx: &mut ValidationContext) {
        if self.budgeting_method == BudgetingMethod::Unknown {
            ctx.push(ParserError::Validation {
                path: format!("{path}.budgetingMethod"),
                message: "Invalid budgeting method.".to_string(),
            });
        }
    }

    fn validate_indicators(
        &self,
        path: &str,
        is_composite: bool,
        document_map: &HashMap<String, Document>,
        ctx: &mut ValidationContext,
    ) {
        if is_composite && (self.indicator.is_some() || self.indicator_ref.is_some()) {
            return ctx.push(ParserError::Validation {
                path: format!("{path}.indicator"),
                message: "indicator is not allowed for composite SLOs.".to_string(),
            });
        }
        if !is_composite {
            match (&self.indicator, &self.indicator_ref) {
                (Some(_), Some(_)) => {
                    ctx.push(ParserError::Validation {
                        path: format!("{path}.indicator, {path}.indicatorRef"),
                        message: "Cannot define both indicator and indicatorRef.".to_string(),
                    });
                }
                (None, None) => {
                    ctx.push(ParserError::Validation {
                        path: format!("{path}.indicator, {path}.indicatorRef"),
                        message: "Must define either indicator or indicatorRef.".to_string(),
                    });
                }
                (Some(indicator), None) => {
                    indicator.validate(Some(&format!("{path}.indicator")), true, ctx);
                }
                (None, Some(indicator_ref)) => {
                    if let Some(document) = document_map.get(indicator_ref) {
                        match document {
                            Document::Sli(slidoc) => slidoc.validate(Some(path), false, ctx),
                            _ => ctx.push(ParserError::Validation {
                                path: format!("{path}.indicatorRef"),
                                message: format!(
                                    "Indicator reference `{}` not found.",
                                    indicator_ref
                                ),
                            }),
                        };
                    } else {
                        ctx.push(ParserError::Validation {
                            path: format!("{path}.indicatorRef"),
                            message: format!("Indicator reference `{}` not found.", indicator_ref),
                        })
                    }
                }
            }
        }
    }

    fn validate_time_window(&self, path: &str, ctx: &mut ValidationContext) {
        if let Some(time_window) = &self.time_window {
            if time_window.len() != 1 {
                ctx.push(ParserError::Validation {
                    path: format!("{path}.timeWindow"),
                    message: "timeWindow must contain exactly one item.".to_string(),
                });
            }
            time_window[0].validate(&format!("{path}.timeWindow[0]"), ctx);
        }
    }

    fn validate_objectives(
        &self,
        path: &str,
        is_composite: bool,
        document_map: &HashMap<String, Document>,
        ctx: &mut ValidationContext,
    ) {
        if !is_composite {
            if self.objectives.len() != 1 {
                ctx.push(ParserError::Validation {
                    path: format!("{path}.objectives"),
                    message: "objectives must contain exactly one item.".to_string(),
                });
            }
            self.objectives[0].validate(
                path,
                &self.budgeting_method,
                document_map,
                is_composite,
                ctx,
            );
        }

        for (i, objective) in self.objectives.iter().enumerate() {
            objective.validate(
                &format!("{path}.objectives[{}]", i),
                &self.budgeting_method,
                document_map,
                is_composite,
                ctx,
            );
        }
    }

    fn validate_alert_policy(
        &self,
        path: &str,
        document_map: &HashMap<String, Document>,
        ctx: &mut ValidationContext,
    ) {
        if let Some(alert_policy) = &self.alert_policies {
            for (i, policy) in alert_policy.iter().enumerate() {
                match policy {
                    AlertPolicy::Inline(policy) => policy.validate(
                        Some(&format!("{path}.alertPolicies[{}]", i)),
                        document_map,
                        ctx,
                    ),
                    AlertPolicy::Reference(policy_ref) => {
                        if let Some(document) = document_map.get(&policy_ref.alert_policy_ref) {
                            match document {
                                Document::AlertPolicy(alert_policy_doc) => {
                                    alert_policy_doc.validate(Some(path), document_map, ctx);
                                }
                                _ => {
                                    ctx.push(ParserError::Validation {
                                        path: format!("{path}.alertPolicies[{}]", i),
                                        message: format!(
                                            "Alert policy reference `{}` wrong kind.",
                                            policy_ref.alert_policy_ref
                                        ),
                                    });
                                }
                            }
                        } else {
                            ctx.push(ParserError::Validation {
                                path: format!("{path}.alertPolicy[{}]", i),
                                message: format!(
                                    "Alert policy reference `{}` not found.",
                                    policy_ref.alert_policy_ref
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn validate(
        &self,
        path: &str,
        document_map: &HashMap<String, Document>,
        ctx: &mut ValidationContext,
    ) {
        let is_composite = self.is_composite();
        self.validate_service(path, ctx);
        self.validate_budgeting_method(path, ctx);
        self.validate_indicators(path, is_composite, document_map, ctx);
        self.validate_time_window(path, ctx);
        self.validate_objectives(path, is_composite, document_map, ctx);
        self.validate_alert_policy(path, document_map, ctx);
    }
}
#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct TimeWindow {
    pub duration: DurationShorthand,
    pub calendar: Option<CalendarDetails>,
    #[serde(rename = "isRolling")]
    pub is_rolling: bool,
}

impl TimeWindow {
    pub fn validate(&self, path: &str, ctx: &mut ValidationContext) {
        if self.is_rolling && self.calendar.is_some() {
            ctx.push(ParserError::Validation {
                path: format!("{path}.calendar"),
                message:
                    "Calendar details can only be specified for Calendar Aligned time windows."
                        .to_string(),
            });
        }

        if !self.is_rolling && self.calendar.is_none() {
            ctx.push(ParserError::Validation {
                path: format!("{path}.calendar"),
                message: "Calendar details must be specified for Calendar Aligned time windows."
                    .to_string(),
            });
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct CalendarDetails {
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "timeZone")]
    pub time_zone: String,
}

#[cfg(test)]
mod happy_path_tests {
    use crate::parser::api_version::v1::{
        alert_condition::AlertConditionDoc, alert_notification_target::AlertNotificationTargetDoc,
    };

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
        let mut validation_context = ValidationContext::new();

        slo.validate(None, &HashMap::new(), &mut validation_context);

        assert!(
            validation_context.result().is_ok(),
            "Expected valid SLO to pass validation"
        );
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
        let mut document_map = HashMap::new();

        document_map.insert(
            "availability-sli".to_string(),
            Document::Sli(SLIDoc {
                kind: Some(Kind::Sli),
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
                },
            }),
        );

        let mut validation_context = ValidationContext::new();

        slo.validate(None, &document_map, &mut validation_context);

        assert!(
            validation_context.result().is_ok(),
            "Expected valid SLO to pass validation"
        );
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
        let mut document_map = HashMap::new();

        document_map.insert(
            "slo-2".to_string(),
            Document::Sli(SLIDoc {
                kind: Some(Kind::Sli),
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
            }),
        );

        document_map.insert(
            "slo-1".to_string(),
            Document::Sli(SLIDoc {
                kind: Some(Kind::Sli),
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
            }),
        );

        let mut validation_context = ValidationContext::new();

        slo.validate(None, &document_map, &mut validation_context);

        assert!(
            validation_context.result().is_ok(),
            "Expected valid SLO to pass validation"
        );
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
        let mut document_map = HashMap::new();

        document_map.insert(
            "error-rate-sli".to_string(),
            Document::Sli(SLIDoc {
                kind: Some(Kind::Sli),
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
                },
            }),
        );

        document_map.insert(
            "high-error-rate-alert".to_string(),
            Document::AlertPolicy(AlertPolicyDoc {
                kind: Some(Kind::AlertPolicy),
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
                        kind: Some(Kind::AlertCondition),
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
                            kind: Some(Kind::AlertNotificationTarget),
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
            }),
        );

        let mut validation_context = ValidationContext::new();

        slo.validate(None, &document_map, &mut validation_context);

        assert!(
            validation_context.result().is_ok(),
            "Expected valid SLO to pass validation"
        );
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

        let mut validation_context = ValidationContext::new();

        slo.validate(None, &HashMap::new(), &mut validation_context);

        assert!(
            validation_context.result().is_ok(),
            "Expected valid SLO to pass validation"
        );
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

        let mut validation_context = ValidationContext::new();

        slo.validate(None, &HashMap::new(), &mut validation_context);

        assert!(validation_context.result().is_err_and(
            |e| matches!(&e[0], ParserError::Validation { path, message } if
                path == ".spec.indicator, .spec.indicatorRef"
                    && message == "Must define either indicator or indicatorRef."
            )
        ));
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

        let mut validation_context = ValidationContext::new();

        slo.validate(None, &HashMap::new(), &mut validation_context);

        assert!(validation_context.result().is_err_and(
            |e| matches!(&e[0], ParserError::Validation { path, message } if
                path == ".spec.indicator, .spec.indicatorRef"
                    && message == "Cannot define both indicator and indicatorRef."
            )
        ));
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

        let mut validation_context = ValidationContext::new();

        slo.validate(None, &HashMap::new(), &mut validation_context);

        assert!(validation_context.result().is_err_and(
            |e| matches!(&e[0], ParserError::Validation { path, message } if
                path == ".spec.alertPolicy[0]"
                    && message == "Alert policy reference `not-found` not found."
            )
        ));
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

        let mut validation_context = ValidationContext::new();

        slo.validate(None, &HashMap::new(), &mut validation_context);

        assert!(validation_context.result().is_err_and(
            |e| matches!(&e[0], ParserError::Validation { path, message }
                if path == ".spec.indicatorRef"
                    && message == "Indicator reference `not-found` not found."
            )
        ));
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

        let mut validation_context = ValidationContext::new();

        slo.validate(None, &HashMap::new(), &mut validation_context);

        assert!(validation_context.result().is_err_and(
            |e| matches!(&e[0], ParserError::Validation { path, message } if
                path == ".spec.indicator"
                    && message == "indicator is not allowed for composite SLOs."
            )
        ));
    }
}
