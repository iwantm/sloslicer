mod parser;
use parser::document::{Document, parse_kind};

fn main() {
    let yaml = r#"
    apiVersion: openslo/v1
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

    let parsed = Document::parse(yaml).unwrap();

    println!("{:?}", parsed);

    let test = parsed.validate("test", None, None, None, None);
    println!("{:?}", test);

    println!("Hello, world!");
}
