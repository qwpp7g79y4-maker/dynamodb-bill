# dynamodb-bill

[![crates.io](https://img.shields.io/crates/v/dynamodb-bill.svg)](https://crates.io/crates/dynamodb-bill) [![downloads](https://img.shields.io/crates/d/dynamodb-bill.svg)](https://crates.io/crates/dynamodb-bill) [![CI](https://github.com/qwpp7g79y4-maker/dynamodb-bill/actions/workflows/ci.yml/badge.svg)](https://github.com/qwpp7g79y4-maker/dynamodb-bill/actions/workflows/ci.yml) [![PEPAI checked](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fqwpp7g79y4-maker%2Fdynamodb-bill%2Fmain%2F.github%2Fbadge.json)](QUALITY.md) [![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://github.com/qwpp7g79y4-maker/dynamodb-bill/blob/main/Cargo.toml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/qwpp7g79y4-maker/dynamodb-bill#license)

## Why

Estimate monthly **AWS DynamoDB** cost (storage, on-demand request units, and
provisioned RCU/WCU) from a local JSON export, without calling AWS.

It reads files only and uses no network. The tests and the measured numbers
are in `QUALITY.md`. A Dutch version is in `README.nl.md`.

## Installation

Version 0.1.1. From crates.io:

```sh
cargo install dynamodb-bill
```

Or build from source.

Rust 1.85+. From a checkout:

```sh
cargo install --path .
```

## Quick start

```sh
dynamodb-bill examples/on-demand.json
dynamodb-bill examples/provisioned.json --json
dynamodb-bill examples/on-demand.json examples/provisioned.json --group-by env
```

## Quickstart in 5 minutes

1. Build: `cargo build --release` (Rust 1.85+).
2. Run the sample: `target/release/dynamodb-bill examples/on-demand.json`.
3. Put your own table export (the shape is described under "How it counts")
   in a file and pass it as the argument.
4. Add `--json` for machine-readable output or `--group-by KEY` to sum by tag.

Nothing is written to disk and nothing is sent anywhere.

## Usage

```text
dynamodb-bill [OPTIONS] <PATH>...
```

- `<PATH>...`: JSON export files or a directory of `*.json` files. Only the
  lowercase extension `.json` counts. A directory with no `.json` file, and a
  file such as `X.JSON` in other capitals, exit 2 (the message names the file);
  rename the file.
- `--prices <FILE>`: price file. Default: built-in us-east-1 rates.
- `--group-by <KEY>`: group rows by this `Tags`/`Metadata` key.
- `--json`: print JSON (`schema_version` 1) instead of the text table.
- `-h`, `--help` and `-V`, `--version`.

## Example

```sh
dynamodb-bill examples/on-demand.json
```

```text
table	mode	size_bytes	storage_cost	read_cost	write_cost	total
orders	on-demand	1073741824	0.25 USD	0.125 USD	0.625 USD	1.000 USD
total			0.25 USD	0.125 USD	0.625 USD	1.000 USD
```

## How it counts

- Input: one or more JSON files, or a directory of `*.json`, in the shape
  `{ "Tables": [ { "TableName", "BillingModeSummary"|"BillingMode",
  "TableSizeBytes", "ProvisionedThroughput"?, "ReadRequestUnits"?,
  "WriteRequestUnits"?, "Tags"? } ] }` or a single `Table` object. Field names
  follow
  [DescribeTable](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_DescribeTable.html)
  (geraadpleegd 2026-11-09).
- **Storage:** `TableSizeBytes / 1024³ × storage_per_gb_month`. Built-in
  us-east-1 rate from
  [Amazon DynamoDB pricing](https://aws.amazon.com/dynamodb/pricing/)
  (geraadpleegd 2026-11-09): **$0.25** / GB-month.
- **On-demand:** `ReadRequestUnits / 1e6 × $0.125` and
  `WriteRequestUnits / 1e6 × $0.625` (same page, 2026-11-09). Units may also
  come from `Metrics`.
- **Provisioned:** `RCU × $0.00013/hour × 730` and
  `WCU × $0.00065/hour × 730` (same page, 2026-11-09).
- `--group-by KEY` aggregates by `Tags` or `Metadata`.

## Exit codes

- `0`: ok (also for `--help` and `--version`).
- `2`: input or usage error, for example an unreadable or broken JSON file, an
  field that is present but has the wrong type (the message names the field), a
  duplicate JSON key, `Metrics` together with `ReadRequestUnits`, `Metrics` on a
  provisioned table, the same table name twice (in two files or in one file),
  the same file given twice (in any spelling of the path), an amount too large
  to compute, or input over the size limits. Also exit 2: `Tags` or `Metadata`
  that is not an object or a list of `{"Key","Value"}` objects, a tag key twice
  in one list, `Tags` and `Metadata` giving one key two values, `Tables` together
  with `Table`, `TableName` or `BillingMode`, `BillingModeSummary` and
  `BillingMode` naming different modes, a directory with no `.json` file, and a
  duplicate key in the price file.

Apart from running out of memory (see Limitations), there is no other exit code
for a bad input, and the tool does not panic.

## Prices and responsibility

The built-in rates are the us-east-1 rates in `prices/us-east-1.json`, taken
from the [Amazon DynamoDB pricing](https://aws.amazon.com/dynamodb/pricing/)
page on 2026-11-09. AWS can change prices at any time, and other regions and
discounts differ. The output is an estimate, not an invoice. You are
responsible for checking it against your AWS bill. Pass `--prices FILE` to use
your own rates.

## Limitations

- Offline only: no AWS network client in v0.1.
- Free tier is not subtracted.
- GlobalSecondaryIndexes, LocalSecondaryIndexes, replicas, global tables, Standard-IA and backups are not counted, so real costs can be higher.
- On-demand request counts must be present in the export when you want request
  cost; size alone does not invent traffic.
- Each file is at most 32 MiB, and all input files together are at most
  128 MiB. Over either limit the tool exits 2 before reading the files.
- These limits do not cap memory. Parsing takes far more memory than the file
  size: a 32 MiB file of empty tables (about 11 million) was measured at about
  6.4 GB, and several files add up. On a machine with less memory the process
  can be killed by the system, and then the exit code is not 2.
- Control characters in table names and group labels are shown as `?` in the
  text table; `--json` escapes them.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The suite has 92 tests. See `QUALITY.md`.

## Maintenance

The prices are a snapshot from 2026-11-09 and are not updated automatically.
Patches come in through Issues and should include a test. See
`CONTRIBUTING.md`; report security problems as described in `SECURITY.md`.

After a release the tool is dormant: a new version appears only for a reported bug, a request, or a security problem. Security always comes first.

## License

MIT OR Apache-2.0.

The licenses of the third-party crates built into the binary are listed in [THIRD-PARTY-LICENSES.html](THIRD-PARTY-LICENSES.html).
