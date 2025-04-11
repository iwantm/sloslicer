use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DataSourceSpec {
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "connectionDetails")]
    pub connection_details: serde_yaml::Value,
}
