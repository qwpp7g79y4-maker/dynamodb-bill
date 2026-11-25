use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prices {
    pub currency: String,
    pub region: String,
    pub source: String,
    pub date: String,
    pub storage_per_gb_month: Decimal,
    pub on_demand_read_per_million: Decimal,
    pub on_demand_write_per_million: Decimal,
    pub provisioned_rcu_per_hour: Decimal,
    pub provisioned_wcu_per_hour: Decimal,
    pub hours_per_month: Decimal,
}

impl Prices {
    pub fn builtin() -> Self {
        Self {
            currency: "USD".into(),
            region: "us-east-1".into(),
            source: "https://aws.amazon.com/dynamodb/pricing/".into(),
            date: "2026-11-09".into(),
            storage_per_gb_month: dec("0.25"),
            on_demand_read_per_million: dec("0.125"),
            on_demand_write_per_million: dec("0.625"),
            provisioned_rcu_per_hour: dec("0.00013"),
            provisioned_wcu_per_hour: dec("0.00065"),
            hours_per_month: dec("730"),
        }
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        crate::parse::check_no_dup(bytes).map_err(|e| {
            Error::msg(format!(
                "price file is not valid JSON or has a duplicate key: {e}"
            ))
        })?;
        let v: Value = serde_json::from_slice(bytes)
            .map_err(|e| Error::msg(format!("price file is not JSON: {e}")))?;
        Ok(Self {
            currency: req_str(&v, "currency")?,
            region: req_str(&v, "region")?,
            source: req_str(&v, "source")?,
            date: req_str(&v, "date")?,
            storage_per_gb_month: req_dec(&v, "storage_per_gb_month")?,
            on_demand_read_per_million: req_dec(&v, "on_demand_read_per_million")?,
            on_demand_write_per_million: req_dec(&v, "on_demand_write_per_million")?,
            provisioned_rcu_per_hour: req_dec(&v, "provisioned_rcu_per_hour")?,
            provisioned_wcu_per_hour: req_dec(&v, "provisioned_wcu_per_hour")?,
            hours_per_month: req_dec(&v, "hours_per_month")?,
        })
    }
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("literal")
}

fn req_str(v: &Value, key: &str) -> Result<String, Error> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Error::msg(format!("price file mist {key}")))
}

fn req_dec(v: &Value, key: &str) -> Result<Decimal, Error> {
    let text = req_str(v, key)?;
    Decimal::from_str(&text)
        .map_err(|_| Error::msg(format!("price file {key} is not a decimal: {text}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rates() {
        let p = Prices::builtin();
        assert_eq!(p.storage_per_gb_month, dec("0.25"));
        assert_eq!(p.on_demand_read_per_million, dec("0.125"));
        assert_eq!(p.on_demand_write_per_million, dec("0.625"));
        assert_eq!(p.provisioned_rcu_per_hour, dec("0.00013"));
        assert_eq!(p.provisioned_wcu_per_hour, dec("0.00065"));
        assert_eq!(p.hours_per_month, dec("730"));
        assert!(p.source.contains("dynamodb/pricing"));
        assert_eq!(p.date, "2026-11-09");
    }

    #[test]
    fn loads_file_shape() {
        let raw = include_bytes!("../prices/us-east-1.json");
        let p = Prices::from_json(raw).unwrap();
        assert_eq!(p.region, "us-east-1");
        assert_eq!(p.storage_per_gb_month, dec("0.25"));
    }

    #[test]
    fn rejects_bad_json() {
        assert!(Prices::from_json(b"{").is_err());
    }

    #[test]
    fn rejects_missing_field() {
        assert!(Prices::from_json(br#"{"currency":"USD"}"#).is_err());
    }
}
