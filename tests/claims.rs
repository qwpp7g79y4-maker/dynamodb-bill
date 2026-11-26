use assert_cmd::Command;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn readme_sections() {
    let t = std::fs::read_to_string(root().join("README.md")).unwrap();
    for n in [
        "Why",
        "Install",
        "Quick start",
        "How it counts",
        "Limitations",
        "https://aws.amazon.com/dynamodb/pricing/",
        "2026-11-09",
        "--group-by",
        "on-demand",
        "provisioned",
        "RCU",
        "WCU",
    ] {
        assert!(t.contains(n), "missing {n}");
    }
    assert!(!t.contains("PLACEHOLDER"));
}

#[test]
fn changelog_0_1_0() {
    assert!(std::fs::read_to_string(root().join("CHANGELOG.md"))
        .unwrap()
        .contains("0.1.1"));
}

#[test]
fn licenses() {
    assert!(root().join("LICENSE-MIT").is_file());
    assert!(root().join("LICENSE-APACHE").is_file());
}

#[test]
fn examples() {
    Command::cargo_bin("dynamodb-bill")
        .unwrap()
        .arg(root().join("examples/on-demand.json"))
        .assert()
        .success();
    Command::cargo_bin("dynamodb-bill")
        .unwrap()
        .arg(root().join("examples/provisioned.json"))
        .assert()
        .success()
        .stdout(predicates::str::contains("provisioned"));
}

#[test]
fn quality() {
    let t = std::fs::read_to_string(root().join("QUALITY.md")).unwrap();
    assert!(t.contains("Tests:"));
    assert!(t.contains("clippy"));
}

#[test]
fn describe_table_url_in_readme_or_lib() {
    let readme = std::fs::read_to_string(root().join("README.md")).unwrap();
    let lib = std::fs::read_to_string(root().join("src/lib.rs")).unwrap();
    assert!(
        readme.contains("API_DescribeTable") || lib.contains("API_DescribeTable"),
        "DescribeTable API URL missing"
    );
}
