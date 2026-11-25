use rust_decimal::Decimal;
use std::collections::BTreeMap;

use crate::parse::{BillingMode, Table};
use crate::price::Prices;
use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCost {
    pub name: String,
    pub mode: String,
    pub size_bytes: u64,
    pub storage_cost: Decimal,
    pub read_cost: Decimal,
    pub write_cost: Decimal,
    pub total: Decimal,
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bill {
    pub rows: Vec<TableCost>,
    pub total_storage_cost: Decimal,
    pub total_read_cost: Decimal,
    pub total_write_cost: Decimal,
    pub total: Decimal,
    pub currency: String,
    pub group_by: Option<String>,
}

fn too_large() -> Error {
    Error::msg("amount is too large to compute (decimal overflow)")
}

fn mul(a: Decimal, b: Decimal) -> Result<Decimal, Error> {
    a.checked_mul(b).ok_or_else(too_large)
}

fn add(a: Decimal, b: Decimal) -> Result<Decimal, Error> {
    a.checked_add(b).ok_or_else(too_large)
}

fn div(a: Decimal, b: Decimal) -> Result<Decimal, Error> {
    a.checked_div(b).ok_or_else(too_large)
}

pub fn bill_tables(
    tables: &[Table],
    prices: &Prices,
    group_by: Option<&str>,
) -> Result<Bill, Error> {
    let mut rows = Vec::with_capacity(tables.len());
    let mut total_storage_cost = Decimal::ZERO;
    let mut total_read_cost = Decimal::ZERO;
    let mut total_write_cost = Decimal::ZERO;
    for t in tables {
        let gib = div(Decimal::from(t.size_bytes), Decimal::from(1024u64.pow(3)))?;
        let storage_cost = mul(gib, prices.storage_per_gb_month)?.round_dp(6);
        let million = Decimal::from(1_000_000u64);
        let (read_cost, write_cost, mode) = match t.mode {
            BillingMode::OnDemand => {
                let r = mul(
                    div(Decimal::from(t.read_units), million)?,
                    prices.on_demand_read_per_million,
                )?
                .round_dp(6);
                let w = mul(
                    div(Decimal::from(t.write_units), million)?,
                    prices.on_demand_write_per_million,
                )?
                .round_dp(6);
                (r, w, "on-demand")
            }
            BillingMode::Provisioned => {
                let r = mul(
                    mul(Decimal::from(t.rcu), prices.provisioned_rcu_per_hour)?,
                    prices.hours_per_month,
                )?
                .round_dp(6);
                let w = mul(
                    mul(Decimal::from(t.wcu), prices.provisioned_wcu_per_hour)?,
                    prices.hours_per_month,
                )?
                .round_dp(6);
                (r, w, "provisioned")
            }
        };
        let total = add(add(storage_cost, read_cost)?, write_cost)?;
        let group = group_by.and_then(|key| t.tags.get(key).cloned());
        total_storage_cost = add(total_storage_cost, storage_cost)?;
        total_read_cost = add(total_read_cost, read_cost)?;
        total_write_cost = add(total_write_cost, write_cost)?;
        rows.push(TableCost {
            name: t.name.clone(),
            mode: mode.into(),
            size_bytes: t.size_bytes,
            storage_cost,
            read_cost,
            write_cost,
            total,
            group,
        });
    }
    if let Some(key) = group_by {
        let mut grouped: BTreeMap<String, TableCost> = BTreeMap::new();
        for row in rows {
            let label = row.group.clone().unwrap_or_else(|| format!("(no {key})"));
            let entry = grouped.entry(label.clone()).or_insert(TableCost {
                name: label,
                mode: "group".into(),
                size_bytes: 0,
                storage_cost: Decimal::ZERO,
                read_cost: Decimal::ZERO,
                write_cost: Decimal::ZERO,
                total: Decimal::ZERO,
                group: row.group.clone(),
            });
            entry.size_bytes = entry.size_bytes.saturating_add(row.size_bytes);
            entry.storage_cost = add(entry.storage_cost, row.storage_cost)?;
            entry.read_cost = add(entry.read_cost, row.read_cost)?;
            entry.write_cost = add(entry.write_cost, row.write_cost)?;
            entry.total = add(entry.total, row.total)?;
        }
        rows = grouped.into_values().collect();
    } else {
        rows.sort_by(|a, b| a.name.cmp(&b.name));
    }
    Ok(Bill {
        rows,
        total_storage_cost: total_storage_cost.round_dp(6),
        total_read_cost: total_read_cost.round_dp(6),
        total_write_cost: total_write_cost.round_dp(6),
        total: add(add(total_storage_cost, total_read_cost)?, total_write_cost)?.round_dp(6),
        currency: prices.currency.clone(),
        group_by: group_by.map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{BillingMode, Table};
    use rust_decimal::Decimal;
    use std::collections::BTreeMap;
    use std::str::FromStr;

    fn on_demand(name: &str, bytes: u64, r: u64, w: u64) -> Table {
        Table {
            name: name.into(),
            mode: BillingMode::OnDemand,
            size_bytes: bytes,
            read_units: r,
            write_units: w,
            rcu: 0,
            wcu: 0,
            tags: BTreeMap::new(),
        }
    }

    fn provisioned(name: &str, bytes: u64, rcu: u64, wcu: u64) -> Table {
        Table {
            name: name.into(),
            mode: BillingMode::Provisioned,
            size_bytes: bytes,
            read_units: 0,
            write_units: 0,
            rcu,
            wcu,
            tags: BTreeMap::new(),
        }
    }

    #[test]
    fn on_demand_million_units() {
        let bill = bill_tables(
            &[on_demand("a", 0, 1_000_000, 1_000_000)],
            &Prices::builtin(),
            None,
        )
        .unwrap();
        // 0.125 + 0.625
        assert_eq!(bill.total, Decimal::from_str("0.750000").unwrap());
    }

    #[test]
    fn storage_one_gib() {
        let bill = bill_tables(
            &[on_demand("a", 1024u64.pow(3), 0, 0)],
            &Prices::builtin(),
            None,
        )
        .unwrap();
        assert_eq!(
            bill.total_storage_cost,
            Decimal::from_str("0.250000").unwrap()
        );
    }

    #[test]
    fn provisioned_one_rcu_one_wcu() {
        let bill = bill_tables(&[provisioned("a", 0, 1, 1)], &Prices::builtin(), None).unwrap();
        // 0.00013*730 + 0.00065*730 = 0.0949 + 0.4745 = 0.5694
        assert_eq!(bill.total, Decimal::from_str("0.569400").unwrap());
    }

    #[test]
    fn group_by_tag() {
        let mut a = on_demand("a", 0, 1_000_000, 0);
        a.tags.insert("env".into(), "prod".into());
        let mut b = on_demand("b", 0, 1_000_000, 0);
        b.tags.insert("env".into(), "prod".into());
        let bill = bill_tables(&[a, b], &Prices::builtin(), Some("env")).unwrap();
        assert_eq!(bill.rows.len(), 1);
        assert_eq!(bill.rows[0].name, "prod");
        assert_eq!(
            bill.rows[0].read_cost,
            Decimal::from_str("0.250000").unwrap()
        );
    }

    #[test]
    fn missing_group_bucket() {
        let bill =
            bill_tables(&[on_demand("a", 0, 0, 0)], &Prices::builtin(), Some("env")).unwrap();
        assert_eq!(bill.rows[0].name, "(no env)");
    }

    #[test]
    fn sorts_by_name() {
        let bill = bill_tables(
            &[on_demand("b", 0, 0, 0), on_demand("a", 0, 0, 0)],
            &Prices::builtin(),
            None,
        )
        .unwrap();
        assert_eq!(bill.rows[0].name, "a");
    }

    #[test]
    fn empty_input() {
        let bill = bill_tables(&[], &Prices::builtin(), None).unwrap();
        assert!(bill.rows.is_empty());
        assert_eq!(bill.total, Decimal::ZERO);
    }

    #[test]
    fn currency_propagates() {
        assert_eq!(
            bill_tables(&[on_demand("a", 0, 0, 0)], &Prices::builtin(), None)
                .unwrap()
                .currency,
            "USD"
        );
    }

    #[test]
    fn huge_amounts_are_error_not_panic() {
        let p = Prices::builtin();
        let mut big = p.clone();
        big.storage_per_gb_month = Decimal::MAX;
        big.on_demand_read_per_million = Decimal::MAX;
        big.provisioned_rcu_per_hour = Decimal::MAX;
        for t in [
            on_demand("a", u64::MAX, 0, 0),
            on_demand("a", 0, u64::MAX, 0),
            provisioned("a", 0, u64::MAX, 0),
        ] {
            let err = bill_tables(&[t], &big, None).unwrap_err();
            assert!(err.message().contains("too large"), "{}", err.message());
        }
    }

    #[test]
    fn summing_overflow_is_error_not_panic() {
        let mut big = Prices::builtin();
        big.storage_per_gb_month = Decimal::MAX / Decimal::from(2);
        let t = on_demand("a", 1024u64.pow(3), 0, 0);
        let err = bill_tables(&[t.clone(), t.clone(), t], &big, None).unwrap_err();
        assert!(err.message().contains("too large"));
    }

    #[test]
    fn formats_builtin() {
        assert_eq!(Prices::builtin().storage_per_gb_month.to_string(), "0.25");
        assert_eq!(
            Prices::builtin().on_demand_write_per_million.to_string(),
            "0.625"
        );
    }
}
