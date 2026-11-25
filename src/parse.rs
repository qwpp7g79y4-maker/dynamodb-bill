//! Parse local DynamoDB JSON exports (`list-tables` / `describe-table` style).
//!
//! Fields follow
//! <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_DescribeTable.html>
//! (geraadpleegd 2026-11-09).

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingMode {
    OnDemand,
    Provisioned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub mode: BillingMode,
    pub size_bytes: u64,
    pub read_units: u64,
    pub write_units: u64,
    pub rcu: u64,
    pub wcu: u64,
    pub tags: BTreeMap<String, String>,
}

/// Walks the JSON once and fails on a repeated key in the same object.
/// `serde_json::Value` would silently keep the last one.
struct NoDup;

/// The one duplicate-key check, used for export files and the price file.
pub(crate) fn check_no_dup(bytes: &[u8]) -> Result<(), serde_json::Error> {
    serde_json::from_slice::<NoDup>(bytes).map(|_| ())
}

impl<'de> Deserialize<'de> for NoDup {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = NoDup;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_bool<E>(self, _: bool) -> Result<NoDup, E> {
                Ok(NoDup)
            }
            fn visit_i64<E>(self, _: i64) -> Result<NoDup, E> {
                Ok(NoDup)
            }
            fn visit_u64<E>(self, _: u64) -> Result<NoDup, E> {
                Ok(NoDup)
            }
            fn visit_f64<E>(self, _: f64) -> Result<NoDup, E> {
                Ok(NoDup)
            }
            fn visit_str<E>(self, _: &str) -> Result<NoDup, E> {
                Ok(NoDup)
            }
            fn visit_unit<E>(self) -> Result<NoDup, E> {
                Ok(NoDup)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<NoDup, A::Error> {
                while a.next_element::<NoDup>()?.is_some() {}
                Ok(NoDup)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<NoDup, A::Error> {
                let mut keys = BTreeSet::new();
                while let Some(k) = a.next_key::<String>()? {
                    if !keys.insert(k.clone()) {
                        return Err(de::Error::custom(format!("duplicate key {k}")));
                    }
                    a.next_value::<NoDup>()?;
                }
                Ok(NoDup)
            }
        }
        d.deserialize_any(V)
    }
}

pub fn parse_export(bytes: &[u8]) -> Result<Vec<Table>, Error> {
    check_no_dup(bytes).map_err(|e| {
        Error::msg(format!(
            "input is not valid JSON or has a duplicate key: {e}. Pass a DynamoDB export file"
        ))
    })?;
    let v: Value = serde_json::from_slice(bytes).map_err(|e| {
        Error::msg(format!(
            "input is not JSON: {e}. Pass a DynamoDB export file"
        ))
    })?;
    if let Some(arr) = v.get("Tables").and_then(Value::as_array) {
        for k in ["Table", "TableName", "BillingMode"] {
            if v.get(k).is_some() {
                return Err(Error::msg(format!(
                    "Tables and {k} are both present; give a list of tables or a single table, not both"
                )));
            }
        }
        return arr
            .iter()
            .enumerate()
            .map(|(i, t)| parse_table(t, &format!("Tables[{i}]")))
            .collect();
    }
    if v.get("TableName").is_some() || v.get("Table").is_some() {
        let t = v.get("Table").unwrap_or(&v);
        return Ok(vec![parse_table(t, "Table")?]);
    }
    Err(Error::msg(
        "JSON mist Tables[] of een enkele tabel met TableName; geef een dynamodb-bill-export",
    ))
}

fn parse_table(v: &Value, label: &str) -> Result<Table, Error> {
    if !v.is_object() {
        return Err(Error::msg(format!(
            "{label} has an invalid type (expected an object)"
        )));
    }
    let name = match v.get("TableName") {
        Some(Value::String(s)) => s.clone(),
        Some(_) => {
            return Err(Error::msg(format!(
                "{label}.TableName has an invalid type (expected a string)"
            )))
        }
        None => v
            .get("TableArn")
            .and_then(Value::as_str)
            .and_then(|arn| arn.rsplit('/').next())
            .unwrap_or("table")
            .to_string(),
    };
    let mode = billing_mode(v)?;
    let size_bytes = num_u64(v, "TableSizeBytes")?.unwrap_or(0);
    let (read_units, write_units, rcu, wcu) = match mode {
        BillingMode::OnDemand => {
            let r = first_u64(v, &["ReadRequestUnits", "ConsumedReadRequestUnits"])?.unwrap_or(0);
            let w = first_u64(v, &["WriteRequestUnits", "ConsumedWriteRequestUnits"])?.unwrap_or(0);
            if v.get("Metrics").is_some()
                && [
                    "ReadRequestUnits",
                    "ConsumedReadRequestUnits",
                    "WriteRequestUnits",
                    "ConsumedWriteRequestUnits",
                ]
                .iter()
                .any(|k| v.get(*k).is_some())
            {
                return Err(Error::msg(
                    "Metrics and ReadRequestUnits/WriteRequestUnits are both present; give one source, not both",
                ));
            }
            let (mr, mw) = metrics_units(v)?;
            let r = r
                .checked_add(mr)
                .ok_or_else(|| Error::msg("read units are too large (sum overflows)"))?;
            let w = w
                .checked_add(mw)
                .ok_or_else(|| Error::msg("write units are too large (sum overflows)"))?;
            (r, w, 0, 0)
        }
        BillingMode::Provisioned => {
            if v.get("Metrics").is_some() {
                return Err(Error::msg(
                    "Metrics is only valid on an on-demand table; this table is provisioned",
                ));
            }
            for k in [
                "ReadRequestUnits",
                "WriteRequestUnits",
                "ConsumedReadRequestUnits",
                "ConsumedWriteRequestUnits",
            ] {
                if v.get(k).is_some() {
                    return Err(Error::msg(format!(
                        "{k} is only valid on an on-demand table; this table is provisioned (set BillingMode to PAY_PER_REQUEST or remove the field)"
                    )));
                }
            }
            let pt = match v.get("ProvisionedThroughput") {
                None => Value::Object(Default::default()),
                Some(o @ Value::Object(_)) => o.clone(),
                Some(_) => {
                    return Err(Error::msg(
                        "ProvisionedThroughput has an invalid type (expected an object)",
                    ))
                }
            };
            let rcu = num_u64(&pt, "ReadCapacityUnits")?.unwrap_or(0);
            let wcu = num_u64(&pt, "WriteCapacityUnits")?.unwrap_or(0);
            (0, 0, rcu, wcu)
        }
    };
    let tags = collect_tags(v)?;
    Ok(Table {
        name,
        mode,
        size_bytes,
        read_units,
        write_units,
        rcu,
        wcu,
        tags,
    })
}

fn billing_mode(v: &Value) -> Result<BillingMode, Error> {
    let from_summary = match v.get("BillingModeSummary") {
        None => None,
        Some(Value::Object(o)) => match o.get("BillingMode") {
            None => None,
            Some(Value::String(s)) => Some(s.as_str()),
            Some(_) => {
                return Err(Error::msg(
                    "BillingModeSummary.BillingMode has an invalid type (expected a string)",
                ))
            }
        },
        Some(_) => {
            return Err(Error::msg(
                "BillingModeSummary has an invalid type (expected an object)",
            ))
        }
    };
    let top = match v.get("BillingMode") {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(_) => {
            return Err(Error::msg(
                "BillingMode has an invalid type (expected a string)",
            ))
        }
    };
    let mode_of = |raw: &str| match raw {
        "PAY_PER_REQUEST" | "ON_DEMAND" | "OnDemand" => Ok(BillingMode::OnDemand),
        "PROVISIONED" | "Provisioned" => Ok(BillingMode::Provisioned),
        other => Err(Error::msg(format!("onbekende BillingMode: {other}"))),
    };
    let a = from_summary.map(mode_of).transpose()?;
    let b = top.map(mode_of).transpose()?;
    match (a, b) {
        (Some(a), Some(b)) if a != b => Err(Error::msg(
            "BillingModeSummary.BillingMode and BillingMode contradict each other; give one mode",
        )),
        (Some(m), _) | (None, Some(m)) => Ok(m),
        (None, None) => Ok(BillingMode::Provisioned),
    }
}

fn metrics_units(v: &Value) -> Result<(u64, u64), Error> {
    let Some(m) = v.get("Metrics") else {
        return Ok((0, 0));
    };
    if !m.is_object() {
        return Err(Error::msg(
            "Metrics has an invalid type (expected an object)",
        ));
    }
    let r = first_u64(m, &["ConsumedReadCapacityUnits", "ReadRequestUnits"])?.unwrap_or(0);
    let w = first_u64(m, &["ConsumedWriteCapacityUnits", "WriteRequestUnits"])?.unwrap_or(0);
    Ok((r, w))
}

fn tag_str<'a>(v: &'a Value, what: &str) -> Result<&'a str, Error> {
    v.as_str()
        .ok_or_else(|| Error::msg(format!("{what} has an invalid type (expected a string)")))
}

fn put_tag(
    tags: &mut BTreeMap<String, String>,
    field: &str,
    key: &str,
    value: &str,
) -> Result<(), Error> {
    if tags.insert(key.to_string(), value.to_string()).is_some() {
        return Err(Error::msg(format!("{field} has the key {key} twice")));
    }
    Ok(())
}

/// One tag container (`Tags` or `Metadata`): an object or a list of
/// `{"Key","Value"}` objects. Any other shape is an error that names the field.
fn read_tags(field: &str, v: &Value) -> Result<BTreeMap<String, String>, Error> {
    let mut tags = BTreeMap::new();
    let pair = |item: &Value, what: &str| -> Result<(String, String), Error> {
        let (Some(k), Some(val)) = (item.get("Key"), item.get("Value")) else {
            return Err(Error::msg(format!(
                "{field} {what} has an invalid type (expected an object with Key and Value)"
            )));
        };
        Ok((
            tag_str(k, &format!("{field} tag Key"))?.to_string(),
            tag_str(val, &format!("{field} tag Value"))?.to_string(),
        ))
    };
    match v {
        Value::Object(obj) => {
            for (k, val) in obj {
                if val.is_object() {
                    // {"env": {"Key": "env", "Value": "x"}}
                    let (key, value) = pair(val, "entry")?;
                    put_tag(&mut tags, field, &key, &value)?;
                } else {
                    let s = tag_str(val, &format!("{field} tag value"))?;
                    put_tag(&mut tags, field, k, s)?;
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                let (key, value) = pair(item, "element")?;
                put_tag(&mut tags, field, &key, &value)?;
            }
        }
        _ => {
            return Err(Error::msg(format!(
                "{field} has an invalid type (expected an object or a list)"
            )))
        }
    }
    Ok(tags)
}

fn collect_tags(v: &Value) -> Result<BTreeMap<String, String>, Error> {
    let mut tags = match v.get("Tags") {
        Some(t) => read_tags("Tags", t)?,
        None => BTreeMap::new(),
    };
    if let Some(m) = v.get("Metadata") {
        for (k, val) in read_tags("Metadata", m)? {
            match tags.get(&k) {
                Some(old) if *old != val => {
                    return Err(Error::msg(format!(
                        "Tags and Metadata give the key {k} different values"
                    )))
                }
                _ => {
                    tags.insert(k, val);
                }
            }
        }
    }
    Ok(tags)
}

/// First key that is present. A present but invalid field is an error;
/// only a missing field falls through to the next key.
fn first_u64(v: &Value, keys: &[&str]) -> Result<Option<u64>, Error> {
    for key in keys {
        if let Some(n) = num_u64(v, key)? {
            return Ok(Some(n));
        }
    }
    Ok(None)
}

fn num_u64(v: &Value, key: &str) -> Result<Option<u64>, Error> {
    match v.get(key) {
        None => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .map(Some)
            .ok_or_else(|| Error::msg(format!("{key} is geen niet-negatief geheel getal"))),
        Some(Value::String(s)) => s
            .parse::<u64>()
            .map(Some)
            .map_err(|_| Error::msg(format!("{key} is geen niet-negatief geheel getal: {s}"))),
        Some(_) => Err(Error::msg(format!("{key} heeft een ongeldig type"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_on_demand() {
        let t = parse_export(
            br#"{"Tables":[{"TableName":"orders","BillingModeSummary":{"BillingMode":"PAY_PER_REQUEST"},"TableSizeBytes":1073741824,"ReadRequestUnits":1000000,"WriteRequestUnits":1000000}]}"#,
        )
        .unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].mode, BillingMode::OnDemand);
        assert_eq!(t[0].size_bytes, 1073741824);
        assert_eq!(t[0].read_units, 1_000_000);
        assert_eq!(t[0].write_units, 1_000_000);
    }

    #[test]
    fn parses_provisioned() {
        let t = parse_export(
            br#"{"Tables":[{"TableName":"users","BillingMode":"PROVISIONED","ProvisionedThroughput":{"ReadCapacityUnits":10,"WriteCapacityUnits":5},"TableSizeBytes":0}]}"#,
        )
        .unwrap();
        assert_eq!(t[0].mode, BillingMode::Provisioned);
        assert_eq!(t[0].rcu, 10);
        assert_eq!(t[0].wcu, 5);
    }

    #[test]
    fn parses_single_table_wrapper() {
        let t = parse_export(
            br#"{"Table":{"TableName":"solo","BillingModeSummary":{"BillingMode":"PAY_PER_REQUEST"},"TableSizeBytes":0}}"#,
        )
        .unwrap();
        assert_eq!(t[0].name, "solo");
    }

    #[test]
    fn reads_metrics_and_tags() {
        let t = parse_export(
            br#"{"Tables":[{"TableName":"m","BillingModeSummary":{"BillingMode":"PAY_PER_REQUEST"},"TableSizeBytes":0,"Metrics":{"ConsumedReadCapacityUnits":3,"ConsumedWriteCapacityUnits":4},"Tags":{"env":"prod"},"Metadata":{"team":"data"}}]}"#,
        )
        .unwrap();
        assert_eq!(t[0].read_units, 3);
        assert_eq!(t[0].write_units, 4);
        assert_eq!(t[0].tags.get("env").map(String::as_str), Some("prod"));
        assert_eq!(t[0].tags.get("team").map(String::as_str), Some("data"));
    }

    #[test]
    fn rejects_broken_json() {
        assert!(parse_export(b"{").is_err());
    }

    #[test]
    fn rejects_wrong_shape() {
        assert!(parse_export(br#"{"hello":1}"#).is_err());
    }

    #[test]
    fn bad_size() {
        assert!(parse_export(
            br#"{"Tables":[{"TableName":"x","BillingMode":"PROVISIONED","TableSizeBytes":"nope"}]}"#
        )
        .is_err());
    }

    #[test]
    fn unknown_mode() {
        assert!(parse_export(br#"{"Tables":[{"TableName":"x","BillingMode":"WEIRD"}]}"#).is_err());
    }

    fn on_demand_with(field: &str) -> Vec<u8> {
        format!(r#"{{"Tables":[{{"TableName":"x","BillingMode":"PAY_PER_REQUEST",{field}}}]}}"#)
            .into_bytes()
    }

    #[test]
    fn present_invalid_on_demand_field_is_error_naming_field() {
        for (field, key) in [
            (r#""ReadRequestUnits":"abc""#, "ReadRequestUnits"),
            (r#""ReadRequestUnits":-5"#, "ReadRequestUnits"),
            (r#""ReadRequestUnits":1.5"#, "ReadRequestUnits"),
            (r#""ReadRequestUnits":true"#, "ReadRequestUnits"),
            (r#""ReadRequestUnits":null"#, "ReadRequestUnits"),
            (r#""ReadRequestUnits":[1]"#, "ReadRequestUnits"),
            (r#""ReadRequestUnits":"-1""#, "ReadRequestUnits"),
            (
                r#""ConsumedReadRequestUnits":"x""#,
                "ConsumedReadRequestUnits",
            ),
            (r#""WriteRequestUnits":-1"#, "WriteRequestUnits"),
            (
                r#""ConsumedWriteRequestUnits":0.5"#,
                "ConsumedWriteRequestUnits",
            ),
            (
                r#""Metrics":{"ConsumedReadCapacityUnits":-3}"#,
                "ConsumedReadCapacityUnits",
            ),
            (
                r#""Metrics":{"WriteRequestUnits":"zz"}"#,
                "WriteRequestUnits",
            ),
            (r#""Metrics":"oops""#, "Metrics"),
        ] {
            let err = parse_export(&on_demand_with(field)).expect_err(field);
            assert!(err.message().contains(key), "{field}: {}", err.message());
        }
    }

    #[test]
    fn missing_on_demand_field_falls_back() {
        let t = parse_export(&on_demand_with(r#""ConsumedReadRequestUnits":7"#)).unwrap();
        assert_eq!(t[0].read_units, 7);
        assert_eq!(t[0].write_units, 0);
    }

    #[test]
    fn units_sum_overflow_is_error() {
        let b = on_demand_with(
            r#""ReadRequestUnits":18446744073709551615,"Metrics":{"ConsumedReadCapacityUnits":1}"#,
        );
        assert!(parse_export(&b).is_err());
    }

    #[test]
    fn tag_list_shape() {
        let t = parse_export(
            br#"{"Tables":[{"TableName":"x","BillingMode":"PROVISIONED","ProvisionedThroughput":{"ReadCapacityUnits":1,"WriteCapacityUnits":1},"TableSizeBytes":0,"Tags":[{"Key":"env","Value":"dev"}]}]}"#,
        )
        .unwrap();
        assert_eq!(t[0].tags.get("env").map(String::as_str), Some("dev"));
    }
}
