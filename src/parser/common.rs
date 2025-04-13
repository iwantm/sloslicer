use regex::Regex;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize, PartialEq)]
pub enum Operator {
    #[serde(rename = "lte")]
    Lte,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "gt")]
    Gt,
    #[serde(other)]
    Invalid,
}

#[derive(Debug, PartialEq)]
pub struct DurationShorthand(pub String);

impl<'de> Deserialize<'de> for DurationShorthand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let re = Regex::new(r"^\d+(m|h|d|w|M|Q|Y)$").unwrap();
        if !re.is_match(&s) {
            return Err(serde::de::Error::custom(format!(
                "Invalid duration format: `{}`. Must be a positive integer followed by m/h/d/w/M/Q/Y.",
                s
            )));
        }
        Ok(DurationShorthand(s))
    }
}

impl DurationShorthand {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum BudgetingMethod {
    #[serde(rename = "Occurrences")]
    Occurrences,
    #[serde(rename = "Timeslices")]
    Timeslices,
    #[serde(rename = "RatioTimeslices")]
    RatioTimeslices,
    #[serde(other)]
    Unknown,
}
