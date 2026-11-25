//! Wrong types, duplicate keys, double sources and raw bytes on stderr.
use std::path::PathBuf;
use std::process::{Command, Output};

struct Dir(PathBuf);
impl Dir {
    fn new(name: &str) -> Self {
        let d = std::env::temp_dir().join(format!("ddb-strict-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Dir(d)
    }
    fn file(&self, name: &str, body: &str) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, body).unwrap();
        p
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&std::path::Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dynamodb-bill"))
        .args(args)
        .output()
        .unwrap()
}

fn run_json(name: &str, body: &str) -> Output {
    let d = Dir::new(name);
    let f = d.file("in.json", body);
    run(&[&f])
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn refuses(name: &str, body: &str, needle: &str) {
    let o = run_json(name, body);
    assert_eq!(o.status.code(), Some(2), "{body}: {}", stderr(&o));
    assert!(o.stdout.is_empty(), "{body}: stdout not empty");
    assert!(stderr(&o).contains(needle), "{body}: {}", stderr(&o));
}

#[test]
fn billing_mode_summary_wrong_type() {
    refuses(
        "bms-str",
        r#"{"Tables":[{"TableName":"a","BillingModeSummary":"PAY_PER_REQUEST","ReadRequestUnits":1000000}]}"#,
        "BillingModeSummary",
    );
    refuses(
        "bms-inner",
        r#"{"Tables":[{"TableName":"a","BillingModeSummary":{"BillingMode":5}}]}"#,
        "BillingModeSummary.BillingMode",
    );
}

#[test]
fn billing_mode_wrong_type() {
    for v in ["5", "null", "true", "[1]", "{}"] {
        refuses(
            "bm",
            &format!(
                r#"{{"Tables":[{{"TableName":"a","BillingMode":{v},"ReadRequestUnits":9}}]}}"#
            ),
            "BillingMode",
        );
    }
}

#[test]
fn table_element_not_an_object() {
    refuses("el", r#"{"Tables":[1,"x",null]}"#, "Tables[0]");
    refuses("el2", r#"{"Tables":[{"TableName":"a"},null]}"#, "Tables[1]");
    refuses("el3", r#"{"Table":5}"#, "Table");
}

#[test]
fn duplicate_keys_refused() {
    refuses(
        "dup1",
        r#"{"Tables":[{"TableName":"a","TableName":"b"}]}"#,
        "duplicate key TableName",
    );
    refuses(
        "dup2",
        r#"{"Tables":[{"TableName":"a","BillingMode":"PROVISIONED","BillingMode":"PAY_PER_REQUEST"}]}"#,
        "duplicate key BillingMode",
    );
    // escaped spelling of the same key, nested object, top level
    refuses(
        "dup3",
        r#"{"Tables":[{"TableName":"a","Tags":{"k":"1","k":"2"}}]}"#,
        "duplicate key k",
    );
    refuses(
        "dup4",
        r#"{"Tables":[],"Tables":[]}"#,
        "duplicate key Tables",
    );
}

#[test]
fn same_key_in_different_objects_is_fine() {
    let o = run_json(
        "dup-ok",
        r#"{"Tables":[{"TableName":"a","Tags":{"env":"x"}},{"TableName":"b","Tags":{"env":"y"}}]}"#,
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
}

#[test]
fn metrics_and_request_units_together_refused() {
    refuses(
        "both",
        r#"{"Tables":[{"TableName":"a","BillingMode":"PAY_PER_REQUEST","ReadRequestUnits":1000000,"Metrics":{"ConsumedReadCapacityUnits":1000000}}]}"#,
        "Metrics",
    );
    refuses(
        "both-w",
        r#"{"Tables":[{"TableName":"a","BillingMode":"PAY_PER_REQUEST","ConsumedWriteRequestUnits":1,"Metrics":{"ConsumedReadCapacityUnits":1}}]}"#,
        "Metrics",
    );
}

#[test]
fn one_source_still_works() {
    let o = run_json(
        "one",
        r#"{"Tables":[{"TableName":"a","BillingMode":"PAY_PER_REQUEST","Metrics":{"ConsumedReadCapacityUnits":1000000}}]}"#,
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(String::from_utf8_lossy(&o.stdout).contains("0.125"));
}

#[test]
fn same_table_in_two_files_refused() {
    let d = Dir::new("twice");
    let body = r#"{"Tables":[{"TableName":"orders","BillingMode":"PAY_PER_REQUEST","ReadRequestUnits":1000000}]}"#;
    let a = d.file("a.json", body);
    let b = d.file("b.json", body);
    let o = run(&[&a, &b]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(o.stdout.is_empty());
    assert!(stderr(&o).contains("orders"), "{}", stderr(&o));
    // different tables in two files stay fine
    let c = d.file(
        "c.json",
        r#"{"Tables":[{"TableName":"other","BillingMode":"PAY_PER_REQUEST"}]}"#,
    );
    assert_eq!(run(&[&a, &c]).status.code(), Some(0));
}

fn assert_clean(o: &Output) {
    assert_eq!(o.status.code(), Some(2));
    let e = &o.stderr;
    assert!(
        e.iter().all(|&b| b == b'\n' || (b >= 0x20 && b != 0x7f)),
        "raw control byte on stderr: {:?}",
        String::from_utf8_lossy(e)
    );
    assert!(!stderr(o).contains("FAKE: exit 0\n"));
    // one line: only the final newline
    assert_eq!(e.iter().filter(|&&b| b == b'\n').count(), 1);
}

#[test]
fn stderr_number_is_cleaned() {
    let o = run_json(
        "esc-num",
        r#"{"Tables":[{"TableName":"a","TableSizeBytes":"\u001b]0;PWNED\u0007\u001b[2J"}]}"#,
    );
    assert_clean(&o);
    assert!(stderr(&o).contains("TableSizeBytes"));
}

#[test]
fn stderr_mode_is_cleaned() {
    let o = run_json(
        "esc-mode",
        r#"{"Tables":[{"TableName":"a","BillingMode":"X\nFAKE: exit 0\u001b[31m\u0085"}]}"#,
    );
    assert_clean(&o);
}

// Windows refuses control characters in file names, so this file cannot be made there.
#[cfg(unix)]
#[test]
fn stderr_path_is_cleaned() {
    let d = Dir::new("esc-path");
    let o = run(&[&d.0.join("no\u{1b}[2Jfile\n.json")]);
    assert_clean(&o);
    let bad = d.file("bad\u{1b}[31m.json", "{");
    assert_clean(&run(&[&bad]));
}

#[test]
fn stderr_price_file_is_cleaned() {
    let d = Dir::new("esc-price");
    let input = d.file("in.json", r#"{"Tables":[]}"#);
    let prices = d.file(
        "p.json",
        r#"{"currency":"USD","region":"r","source":"s","date":"d","storage_per_gb_month":"\u001b[31m\u0007x","on_demand_read_per_million":"1","on_demand_write_per_million":"1","provisioned_rcu_per_hour":"1","provisioned_wcu_per_hour":"1","hours_per_month":"1"}"#,
    );
    let o = Command::new(env!("CARGO_BIN_EXE_dynamodb-bill"))
        .arg(&input)
        .arg("--prices")
        .arg(&prices)
        .output()
        .unwrap();
    assert_clean(&o);
    assert!(stderr(&o).contains("storage_per_gb_month"));
}

#[test]
fn stderr_unknown_argument_is_cleaned() {
    let o = Command::new(env!("CARGO_BIN_EXE_dynamodb-bill"))
        .arg("--x\u{1b}[31m\u{7}")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(o
        .stderr
        .iter()
        .all(|&b| b == b'\n' || (b >= 0x20 && b != 0x7f)));
}

fn refuses_files(name: &str, files: &[(&str, &str)], args: &[&str], needle: &str) {
    let d = Dir::new(name);
    for (n, body) in files {
        if let Some((sub, _)) = n.split_once('/') {
            std::fs::create_dir_all(d.0.join(sub)).unwrap();
        }
        d.file(n, body);
    }
    let paths: Vec<PathBuf> = args.iter().map(|a| d.0.join(a)).collect();
    let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
    let o = run(&refs);
    assert_eq!(o.status.code(), Some(2), "{args:?}: {}", stderr(&o));
    assert!(o.stdout.is_empty(), "{args:?}: stdout not empty");
    assert!(stderr(&o).contains(needle), "{args:?}: {}", stderr(&o));
}

const ONE: &str =
    r#"{"Tables":[{"TableName":"a","BillingMode":"PAY_PER_REQUEST","ReadRequestUnits":1000000}]}"#;

#[test]
fn same_path_twice_is_refused() {
    refuses_files(
        "sp",
        &[("one.json", ONE)],
        &["one.json", "one.json"],
        "counted twice",
    );
}

#[test]
fn same_path_other_spelling_is_refused() {
    refuses_files(
        "sp2",
        &[("one.json", ONE)],
        &["one.json", "./one.json"],
        "counted twice",
    );
    refuses_files(
        "sp3",
        &[("one.json", ONE), ("sub/x.json", "{}")],
        &["one.json", "sub/../one.json"],
        "counted twice",
    );
}

#[test]
fn directory_plus_file_in_it_is_refused() {
    refuses_files(
        "dirfile",
        &[("d/one.json", ONE)],
        &["d", "d/one.json"],
        "counted twice",
    );
}

#[test]
fn same_table_name_twice_in_one_file_is_refused() {
    refuses(
        "dupname",
        r#"{"Tables":[{"TableName":"a"},{"TableName":"a","TableSizeBytes":1}]}"#,
        "counted twice",
    );
}

#[test]
fn same_table_name_in_two_files_is_refused() {
    refuses_files(
        "two",
        &[("a.json", ONE), ("b.json", ONE)],
        &["a.json", "b.json"],
        "counted twice",
    );
}

#[test]
fn field_wrong_type_is_refused() {
    refuses(
        "pt1",
        r#"{"Tables":[{"TableName":"a","ProvisionedThroughput":5}]}"#,
        "ProvisionedThroughput",
    );
    refuses(
        "pt2",
        r#"{"Tables":[{"TableName":"a","ProvisionedThroughput":null}]}"#,
        "ProvisionedThroughput",
    );
    refuses("tn1", r#"{"Tables":[{"TableName":5}]}"#, "TableName");
    refuses("tn2", r#"{"Tables":[{"TableName":null}]}"#, "TableName");
    refuses(
        "tg1",
        r#"{"Tables":[{"TableName":"a","Tags":{"env":5}}]}"#,
        "tag",
    );
    refuses(
        "tg2",
        r#"{"Tables":[{"TableName":"a","Metadata":{"env":null}}]}"#,
        "tag",
    );
    refuses(
        "tg3",
        r#"{"Tables":[{"TableName":"a","Tags":[{"Key":"env","Value":5}]}]}"#,
        "tag",
    );
}

#[test]
fn metrics_on_provisioned_table_is_refused() {
    refuses(
        "mprov",
        r#"{"Tables":[{"TableName":"a","BillingMode":"PROVISIONED","Metrics":{"ConsumedReadCapacityUnits":1}}]}"#,
        "Metrics",
    );
    refuses(
        "mprov2",
        r#"{"Tables":[{"TableName":"a","Metrics":{}}]}"#,
        "Metrics",
    );
}

#[test]
fn request_units_without_mode_are_refused() {
    refuses(
        "ruNomode",
        r#"{"Tables":[{"TableName":"a","ReadRequestUnits":1000000,"WriteRequestUnits":1000000}]}"#,
        "ReadRequestUnits",
    );
}

#[test]
fn request_units_on_provisioned_table_are_refused() {
    refuses(
        "ruProv",
        r#"{"Tables":[{"TableName":"a","BillingMode":"PROVISIONED","ReadRequestUnits":1000000}]}"#,
        "ReadRequestUnits",
    );
    for f in [
        "ReadRequestUnits",
        "WriteRequestUnits",
        "ConsumedReadRequestUnits",
        "ConsumedWriteRequestUnits",
    ] {
        let body =
            format!(r#"{{"Tables":[{{"TableName":"a","BillingMode":"PROVISIONED","{f}":5}}]}}"#);
        refuses("ruProvEach", &body, f);
    }
}

#[test]
fn request_units_on_pay_per_request_table_still_work() {
    let o = run_json(
        "ruOnDemand",
        r#"{"Tables":[{"TableName":"a","BillingMode":"PAY_PER_REQUEST","ReadRequestUnits":1000000,"WriteRequestUnits":1000000}]}"#,
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
}

fn tags_table(tags: &str) -> String {
    format!(r#"{{"Tables":[{{"TableName":"a",{tags}}}]}}"#)
}

#[test]
fn tags_container_wrong_type_is_refused() {
    for (i, t) in [
        r#""Tags":5"#,
        r#""Tags":"env=prod""#,
        r#""Tags":null"#,
        r#""Tags":true"#,
        r#""Tags":[1,"x",null]"#,
        r#""Tags":[{"Key":"env"}]"#,
        r#""Tags":[{"Value":"prod"}]"#,
        r#""Tags":{"env":{"foo":"bar"}}"#,
        r#""Tags":{"env":5}"#,
    ]
    .iter()
    .enumerate()
    {
        refuses(&format!("tags{i}"), &tags_table(t), "Tags");
    }
    for (i, t) in [
        r#""Metadata":5"#,
        r#""Metadata":"x""#,
        r#""Metadata":[null]"#,
        r#""Metadata":{"env":[]}"#,
    ]
    .iter()
    .enumerate()
    {
        refuses(&format!("meta{i}"), &tags_table(t), "Metadata");
    }
}

#[test]
fn tag_key_twice_in_one_list_is_refused() {
    refuses(
        "tagdup",
        &tags_table(r#""Tags":[{"Key":"env","Value":"prod"},{"Key":"env","Value":"dev"}]"#),
        "env",
    );
    refuses(
        "tagdup-same",
        &tags_table(r#""Tags":[{"Key":"env","Value":"prod"},{"Key":"env","Value":"prod"}]"#),
        "twice",
    );
    refuses(
        "tagdup-obj",
        &tags_table(r#""Tags":{"a":{"Key":"env","Value":"x"},"b":{"Key":"env","Value":"y"}}"#),
        "env",
    );
}

#[test]
fn tags_and_metadata_disagree_is_refused() {
    refuses(
        "tm",
        &tags_table(r#""Tags":{"env":"prod"},"Metadata":{"env":"dev"}"#),
        "Tags and Metadata",
    );
    // same value in both, and different keys, stay fine
    let o = run_json(
        "tm-ok",
        &tags_table(r#""Tags":{"env":"prod"},"Metadata":{"env":"prod","team":"x"}"#),
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
}

#[test]
fn directory_without_json_is_refused() {
    let d = Dir::new("emptydir");
    let o = run(&[&d.0]);
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
    assert!(stderr(&o).contains("no .json file"), "{}", stderr(&o));
    refuses_files(
        "txtdir",
        &[("notes.txt", "x"), ("a.json.bak", "{}")],
        &["."],
        "no .json file",
    );
    refuses_files(
        "subdir-only",
        &[("sub/a.json", ONE)],
        &["."],
        "no .json file",
    );
}

#[test]
fn json_extension_in_other_capitals_is_refused() {
    for name in ["X.JSON", "x.Json"] {
        refuses_files("caps", &[(name, ONE)], &["."], name);
        // also next to a usable file
        refuses_files(
            "caps2",
            &[(name, ONE), ("ok.json", "{\"Tables\":[]}")],
            &["."],
            name,
        );
    }
}

#[test]
fn lowercase_json_in_directory_still_works() {
    let d = Dir::new("okdir");
    d.file("a.json", ONE);
    d.file("readme.txt", "ignored");
    let o = run(&[&d.0]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
}

#[test]
fn tables_and_table_together_is_refused() {
    refuses(
        "tt",
        r#"{"Tables":[{"TableName":"a"}],"Table":{"TableName":"b","ReadRequestUnits":9000000000}}"#,
        "Tables and Table",
    );
    refuses(
        "tt2",
        r#"{"Table":{"TableName":"b"},"Tables":[{"TableName":"a"}]}"#,
        "Tables and Table",
    );
    refuses(
        "tn",
        r#"{"Tables":[{"TableName":"a"}],"TableName":"b"}"#,
        "Tables and TableName",
    );
    refuses(
        "tbm",
        r#"{"Tables":[{"TableName":"a"}],"BillingMode":"PAY_PER_REQUEST"}"#,
        "Tables and BillingMode",
    );
}

#[test]
fn billing_mode_contradiction_is_refused() {
    let t = |s: &str, m: &str| {
        format!(
            r#"{{"Tables":[{{"TableName":"a","BillingModeSummary":{{"BillingMode":"{s}"}},"BillingMode":"{m}","ProvisionedThroughput":{{"ReadCapacityUnits":100,"WriteCapacityUnits":100}}}}]}}"#
        )
    };
    refuses("bmc", &t("PAY_PER_REQUEST", "PROVISIONED"), "contradict");
    refuses("bmc2", &t("PROVISIONED", "ON_DEMAND"), "contradict");
    // the same mode in two spellings is not a contradiction
    let o = run_json("bmc-ok", &t("PAY_PER_REQUEST", "ON_DEMAND"));
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
}

const PRICES: &str = r#"{"currency":"USD","region":"r","source":"s","date":"d","storage_per_gb_month":"2","on_demand_read_per_million":"1","on_demand_write_per_million":"1","provisioned_rcu_per_hour":"1","provisioned_wcu_per_hour":"1","hours_per_month":"1"}"#;

#[test]
fn price_file_duplicate_key_is_refused() {
    let d = Dir::new("pdup");
    let input = d.file("in.json", ONE);
    let dup = PRICES.replacen(
        r#""storage_per_gb_month":"2""#,
        r#""storage_per_gb_month":"2","storage_per_gb_month":"9999""#,
        1,
    );
    let prices = d.file("p.json", &dup);
    let o = Command::new(env!("CARGO_BIN_EXE_dynamodb-bill"))
        .arg(&input)
        .arg("--prices")
        .arg(&prices)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(o.stdout.is_empty());
    assert!(
        stderr(&o).contains("storage_per_gb_month"),
        "{}",
        stderr(&o)
    );
    // the same file without the duplicate is accepted
    let ok = d.file("ok.json", PRICES);
    let o = Command::new(env!("CARGO_BIN_EXE_dynamodb-bill"))
        .arg(&input)
        .arg("--prices")
        .arg(&ok)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
}
