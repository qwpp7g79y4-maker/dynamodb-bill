//! Offline estimate of AWS DynamoDB storage / RCU / WCU cost from local JSON.
//!
//! Pricing: <https://aws.amazon.com/dynamodb/pricing/> (geraadpleegd 2026-11-09).
//! DescribeTable: <https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_DescribeTable.html>
//! (geraadpleegd 2026-11-09).

#![forbid(unsafe_code)]

mod bill;
mod error;
mod parse;
mod price;
mod report;

pub use bill::{bill_tables, Bill, TableCost};
pub use error::{clean_with, Error};
pub use parse::{parse_export, BillingMode, Table};
pub use price::Prices;
pub use report::{render_json, render_text};

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Largest single file (input or price file): 32 MiB.
pub const FILE_CAP: u64 = 32 * 1024 * 1024;
/// Largest total of all accepted input files together: 128 MiB.
pub const TOTAL_CAP: u64 = 128 * 1024 * 1024;

pub struct Options {
    pub inputs: Vec<PathBuf>,
    pub prices: Option<PathBuf>,
    pub group_by: Option<String>,
    pub json: bool,
}

pub fn run(options: &Options) -> Result<String, Error> {
    let prices = match &options.prices {
        Some(path) => Prices::from_json(&read_capped(path, "prices")?)?,
        None => Prices::builtin(),
    };
    let mut files: Vec<PathBuf> = Vec::new();
    for path in &options.inputs {
        if path.is_dir() {
            let mut found: Vec<PathBuf> = Vec::new();
            let entries = fs::read_dir(path).map_err(|e| {
                Error::msg(format!("cannot read directory {}: {e}", path.display()))
            })?;
            for p in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
                match p.extension().and_then(|s| s.to_str()) {
                    Some("json") => found.push(p),
                    Some(x) if x.eq_ignore_ascii_case("json") => {
                        return Err(Error::msg(format!(
                            "{} has a JSON extension in other capitals; only lowercase .json is read. Rename it",
                            p.display()
                        )))
                    }
                    _ => {}
                }
            }
            if found.is_empty() {
                return Err(Error::msg(format!(
                    "directory {} has no .json file",
                    path.display()
                )));
            }
            found.sort();
            files.extend(found);
        } else {
            files.push(path.clone());
        }
    }
    // Check every size first, so the total cap triggers before any memory is used.
    let mut total: u64 = 0;
    for file in &files {
        let len = file_len(file, "input")?;
        total = total.saturating_add(len);
        if total > TOTAL_CAP {
            return Err(Error::msg(format!(
                "input files together are larger than 128 MiB (stopped at {})",
                file.display()
            )));
        }
    }
    let mut tables: Vec<Table> = Vec::new();
    // Same file in any spelling (./x, symlink, directory plus file in it).
    let mut seen_files: std::collections::BTreeMap<PathBuf, PathBuf> = Default::default();
    // Table name -> the file that first had it.
    let mut seen: std::collections::BTreeMap<String, PathBuf> = Default::default();
    for file in &files {
        let canon = fs::canonicalize(file).unwrap_or_else(|_| file.clone());
        if let Some(first) = seen_files.get(&canon) {
            return Err(Error::msg(format!(
                "file {} is the same as {}; it would be counted twice",
                file.display(),
                first.display()
            )));
        }
        seen_files.insert(canon, file.clone());
        let parsed = parse_export(&read_capped(file, "input")?)?;
        for t in &parsed {
            // The same table twice would be counted twice.
            if let Some(first) = seen.get(&t.name) {
                return Err(Error::msg(format!(
                    "table {} is in both {} and {}; it would be counted twice",
                    t.name,
                    first.display(),
                    file.display()
                )));
            }
            seen.insert(t.name.clone(), file.clone());
        }
        tables.extend(parsed);
    }
    let bill = bill_tables(&tables, &prices, options.group_by.as_deref())?;
    if options.json {
        Ok(render_json(&bill))
    } else {
        Ok(render_text(&bill))
    }
}

fn file_len(path: &Path, label: &str) -> Result<u64, Error> {
    let meta = fs::metadata(path)
        .map_err(|e| Error::msg(format!("cannot read {label} {}: {e}", path.display())))?;
    if !meta.is_file() {
        return Err(Error::msg(format!(
            "{label} {} is not a regular file",
            path.display()
        )));
    }
    if meta.len() > FILE_CAP {
        return Err(Error::msg(format!(
            "{label} {} is larger than 32 MiB",
            path.display()
        )));
    }
    Ok(meta.len())
}

fn read_capped(path: &Path, label: &str) -> Result<Vec<u8>, Error> {
    file_len(path, label)?;
    let f = fs::File::open(path)
        .map_err(|e| Error::msg(format!("cannot read {label} {}: {e}", path.display())))?;
    let mut buf = Vec::new();
    f.take(FILE_CAP + 1)
        .read_to_end(&mut buf)
        .map_err(|e| Error::msg(format!("cannot read {label} {}: {e}", path.display())))?;
    if buf.len() as u64 > FILE_CAP {
        return Err(Error::msg(format!(
            "{label} {} is larger than 32 MiB",
            path.display()
        )));
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_on_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/on-demand.json");
        let out = run(&Options {
            inputs: vec![path],
            prices: None,
            group_by: None,
            json: false,
        })
        .unwrap();
        assert!(
            out.contains("0.125") || out.contains("0.75") || out.contains("0.250"),
            "{out}"
        );
    }

    #[test]
    fn run_json() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/on-demand.json");
        let out = run(&Options {
            inputs: vec![path],
            prices: None,
            group_by: None,
            json: true,
        })
        .unwrap();
        assert!(out.contains("\"schema_version\": 1"));
        assert!(out.contains("dynamodb-bill"));
    }

    #[test]
    fn missing_file() {
        let err = run(&Options {
            inputs: vec![PathBuf::from("/no/such/dynamodb.json")],
            prices: None,
            group_by: None,
            json: false,
        })
        .unwrap_err();
        assert!(err.message().contains("cannot read"));
    }
}
