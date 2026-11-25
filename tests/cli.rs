use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn bin() -> Command {
    Command::cargo_bin("dynamodb-bill").unwrap()
}
fn fix(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn help_lists_flags() {
    let a = bin().arg("--help").assert().success();
    let o = String::from_utf8_lossy(&a.get_output().stdout);
    assert!(o.contains("--group-by"));
    assert!(o.contains("--prices"));
    assert!(o.contains("--json"));
}

#[test]
fn version_ok() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("dynamodb-bill"));
}

#[test]
fn on_demand_fixture() {
    bin()
        .arg(fix("on-demand.json"))
        .assert()
        .success()
        .stdout(predicate::str::contains("0.125"));
}

#[test]
fn provisioned_fixture() {
    bin()
        .arg(fix("provisioned.json"))
        .assert()
        .success()
        .stdout(predicate::str::contains("provisioned"));
}

#[test]
fn json_schema() {
    let out = bin()
        .arg(fix("on-demand.json"))
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["tool"], "dynamodb-bill");
}

#[test]
fn group_by_env() {
    bin()
        .args([
            fix("on-demand.json").to_str().unwrap(),
            fix("provisioned.json").to_str().unwrap(),
            "--group-by",
            "env",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("prod"));
}

#[test]
fn broken_json_exit_2() {
    bin().arg(fix("broken.json")).assert().failure().code(2);
}

#[test]
fn missing_file_exit_2() {
    bin().arg("/no/such/ddb.json").assert().failure().code(2);
}

#[test]
fn directory_with_broken_fails() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    bin().arg(&dir).assert().failure();
}

#[test]
fn prices_file() {
    let prices = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("prices/us-east-1.json");
    bin()
        .arg(fix("on-demand.json"))
        .args(["--prices"])
        .arg(prices)
        .assert()
        .success()
        .stdout(predicate::str::contains("0.25"));
}

#[test]
fn unknown_flag() {
    bin().arg("--nope").assert().failure().code(2);
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ddb-bill-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_json(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn invalid_on_demand_field_exit_2_names_field() {
    let d = scratch("invalid");
    for (i, (field, key)) in [
        (r#""ReadRequestUnits":"abc""#, "ReadRequestUnits"),
        (r#""ReadRequestUnits":-5"#, "ReadRequestUnits"),
        (r#""WriteRequestUnits":1.5"#, "WriteRequestUnits"),
        (r#""WriteRequestUnits":true"#, "WriteRequestUnits"),
    ]
    .iter()
    .enumerate()
    {
        let body = format!(
            r#"{{"Tables":[{{"TableName":"x","BillingMode":"PAY_PER_REQUEST",{field}}}]}}"#
        );
        let f = write_json(&d, &format!("{i}.json"), &body);
        bin()
            .arg(f)
            .assert()
            .failure()
            .code(2)
            .stderr(predicate::str::contains(*key));
    }
}

#[test]
fn missing_on_demand_field_still_ok() {
    let d = scratch("missing");
    let f = write_json(
        &d,
        "a.json",
        r#"{"Tables":[{"TableName":"x","BillingMode":"PAY_PER_REQUEST"}]}"#,
    );
    bin().arg(f).assert().success();
}

#[test]
fn huge_amount_exit_2_not_panic() {
    let d = scratch("huge");
    let f = write_json(
        &d,
        "a.json",
        r#"{"Tables":[{"TableName":"x","BillingMode":"PAY_PER_REQUEST","TableSizeBytes":18446744073709551615,"ReadRequestUnits":18446744073709551615}]}"#,
    );
    let p = write_json(
        &d,
        "p.json",
        r#"{"currency":"USD","region":"r","source":"s","date":"2026-01-01","storage_per_gb_month":"79228162514264337593543950335","on_demand_read_per_million":"79228162514264337593543950335","on_demand_write_per_million":"1","provisioned_rcu_per_hour":"1","provisioned_wcu_per_hour":"1","hours_per_month":"730"}"#,
    );
    let out = bin().arg(f).arg("--prices").arg(p).output().unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("too large"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
}

#[test]
fn control_characters_in_names_do_not_make_rows_or_escapes() {
    let d = scratch("ctrl");
    let f = write_json(
        &d,
        "a.json",
        r#"{"Tables":[{"TableName":"a\nFAKE\tROW\u001b[31m","BillingMode":"PROVISIONED","Tags":{"env":"p\nq\u001b]0;x\u0007"}}]}"#,
    );
    for group in [false, true] {
        let mut c = bin();
        c.arg(&f);
        if group {
            c.args(["--group-by", "env"]);
        }
        let out = c.output().unwrap();
        assert!(out.status.success());
        let t = String::from_utf8(out.stdout).unwrap();
        assert_eq!(t.lines().count(), 3, "{t:?}");
        assert!(!t.contains('\u{1b}') && !t.contains('\u{7}'), "{t:?}");
        for line in t.lines() {
            assert_eq!(line.matches('\t').count(), 6, "{t:?}");
        }
    }
}

fn padded_file(dir: &std::path::Path, name: &str, bytes: usize) -> PathBuf {
    let mut body = String::from(r#"{"Tables":[]}"#);
    body.push_str(&" ".repeat(bytes - body.len()));
    write_json(dir, name, &body)
}

#[test]
fn total_cap_128_mib_exit_2() {
    let d = scratch("total");
    let f: Vec<PathBuf> = (0..5)
        .map(|i| padded_file(&d, &format!("{i}.json"), 30 * 1024 * 1024))
        .collect();
    // 4 x 30 MiB = 120 MiB (four different files) is accepted.
    bin().args(&f[..4]).assert().success();
    // 5 x 30 MiB = 150 MiB is refused with a clear message.
    bin()
        .args(&f)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("128 MiB"));
}

#[test]
fn total_cap_counts_directory_files() {
    let d = scratch("totaldir");
    for i in 0..5 {
        padded_file(&d, &format!("{i}.json"), 30 * 1024 * 1024);
    }
    bin()
        .arg(&d)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("128 MiB"));
}

#[test]
fn single_file_cap_32_mib_still_applies() {
    let d = scratch("single");
    let f = padded_file(&d, "a.json", 32 * 1024 * 1024 + 1);
    bin()
        .arg(f)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("32 MiB"));
}

/// Run the binary through `sh` with standard output closed (`>&-`) or `/dev/null`.
#[cfg(target_os = "linux")]
fn run_with_stdout(redirect: &str, args: &[&str]) -> (Option<i32>, String) {
    let exe = assert_cmd::cargo::cargo_bin("dynamodb-bill");
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("exec \"$0\" \"$@\" {redirect}"))
        .arg(exe)
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[cfg(target_os = "linux")]
#[test]
fn closed_stdout_at_start_is_exit_2() {
    let f = fix("on-demand.json");
    let (code, err) = run_with_stdout(">&-", &[f.to_str().unwrap()]);
    assert_eq!(code, Some(2), "stderr: {err}");
    assert!(err.contains("could not write output"), "stderr: {err}");
}

#[cfg(target_os = "linux")]
#[test]
fn closed_stdout_with_version_is_exit_2() {
    let (code, err) = run_with_stdout(">&-", &["--version"]);
    assert_eq!(code, Some(2), "stderr: {err}");
}

#[cfg(target_os = "linux")]
#[test]
fn dev_null_stdout_stays_exit_0() {
    let f = fix("on-demand.json");
    let (code, err) = run_with_stdout(">/dev/null", &[f.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {err}");
}
