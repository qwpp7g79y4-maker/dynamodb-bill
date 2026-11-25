use crate::bill::Bill;
use crate::error::clean;
use serde_json::json;

pub fn render_text(bill: &Bill) -> String {
    let mut out = String::new();
    let label = clean(bill.group_by.as_deref().unwrap_or("table"));
    let currency = clean(&bill.currency);
    let bill = &Bill {
        currency,
        ..bill.clone()
    };
    out.push_str(&format!(
        "{label}\tmode\tsize_bytes\tstorage_cost\tread_cost\twrite_cost\ttotal\n"
    ));
    for row in &bill.rows {
        out.push_str(&format!(
            "{}\t{}\t{}\t{} {}\t{} {}\t{} {}\t{} {}\n",
            clean(&row.name),
            row.mode,
            row.size_bytes,
            row.storage_cost,
            bill.currency,
            row.read_cost,
            bill.currency,
            row.write_cost,
            bill.currency,
            row.total,
            bill.currency
        ));
    }
    out.push_str(&format!(
        "total\t\t\t{} {}\t{} {}\t{} {}\t{} {}\n",
        bill.total_storage_cost,
        bill.currency,
        bill.total_read_cost,
        bill.currency,
        bill.total_write_cost,
        bill.currency,
        bill.total,
        bill.currency
    ));
    out
}

pub fn render_json(bill: &Bill) -> String {
    let rows: Vec<_> = bill
        .rows
        .iter()
        .map(|r| {
            json!({
                "name": r.name,
                "mode": r.mode,
                "size_bytes": r.size_bytes,
                "storage_cost": r.storage_cost.to_string(),
                "read_cost": r.read_cost.to_string(),
                "write_cost": r.write_cost.to_string(),
                "total": r.total.to_string(),
                "group": r.group,
            })
        })
        .collect();
    let doc = json!({
        "schema_version": 1,
        "tool": "dynamodb-bill",
        "currency": bill.currency,
        "group_by": bill.group_by,
        "total_storage_cost": bill.total_storage_cost.to_string(),
        "total_read_cost": bill.total_read_cost.to_string(),
        "total_write_cost": bill.total_write_cost.to_string(),
        "total": bill.total.to_string(),
        "tables": rows,
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&doc).unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bill::bill_tables;
    use crate::parse::{BillingMode, Table};
    use crate::price::Prices;
    use std::collections::BTreeMap;

    fn sample() -> Bill {
        bill_tables(
            &[Table {
                name: "orders".into(),
                mode: BillingMode::OnDemand,
                size_bytes: 0,
                read_units: 1_000_000,
                write_units: 0,
                rcu: 0,
                wcu: 0,
                tags: BTreeMap::new(),
            }],
            &Prices::builtin(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn text_has_header_and_total() {
        let t = render_text(&sample());
        assert!(t.contains("table\t"));
        assert!(t.contains("total\t"));
        assert!(t.contains("0.125"));
    }

    #[test]
    fn text_replaces_control_characters() {
        let mut bill = sample();
        bill.rows[0].name = "a\nfake\trow\x1b[31m\r\u{85}\u{2028}end".into();
        bill.group_by = Some("g\nx\x1b".into());
        bill.currency = "U\nSD".into();
        let t = render_text(&bill);
        assert_eq!(t.lines().count(), 3, "{t:?}");
        for line in t.lines() {
            assert!(!line.contains(['\x1b', '\r', '\u{85}', '\u{2028}']));
        }
        assert!(t
            .lines()
            .nth(1)
            .unwrap()
            .starts_with("a?fake?row?[31m???end\t"));
        assert_eq!(t.matches('\x1b').count(), 0);
    }

    #[test]
    fn json_still_escapes() {
        let mut bill = sample();
        bill.rows[0].name = "a\nb\x1b".into();
        let j = render_json(&bill);
        assert!(j.contains("a\\nb\\u001b"), "{j}");
    }

    #[test]
    fn json_schema() {
        let v: serde_json::Value = serde_json::from_str(&render_json(&sample())).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["tool"], "dynamodb-bill");
    }
}
