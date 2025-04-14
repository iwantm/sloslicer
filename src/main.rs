use parser::document::Document;
use utils::errors::ParserResult;

mod cli;
mod parser;
mod utils;

fn main() -> ParserResult<()> {
    // let yaml = r#"
    // apiVersion: openslo/v1
    // kind: SLI
    // metadata:
    //     name: http-requests
    // spec:
    //     thresholdMetric:
    //         metricSource:
    //             type: Prometheus
    //             spec:
    //                 query: sum(rate(http_requests_total[5m]))
    // "#;

    // let parsed = Document::parse(yaml);
    // match parsed {
    //     Ok(doc) => println!("{:#?}", doc),
    //     Err(e) => eprintln!("Parse error: {:?}", e),
    // }

    cli::run()
}
