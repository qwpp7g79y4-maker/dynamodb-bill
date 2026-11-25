# Changelog

## Unreleased

- `ReadRequestUnits`, `WriteRequestUnits`, `ConsumedReadRequestUnits` or `ConsumedWriteRequestUnits` on a provisioned table, or on a table with no billing mode (which counts as provisioned), exit 2 and name the field. Before, they were ignored: exit 0 and 0 USD. An on-demand table (`PAY_PER_REQUEST`) with these fields stays valid.
- README Limitations: indexes, replicas, global tables, Standard-IA and backups are not counted. Documentation only; indexes are not refused.
- `Tags` or `Metadata` of the wrong type, a tag key twice in one list, and `Tags` and `Metadata` giving one key two values exit 2 and name the field.
- A directory with no `.json` file, an empty directory, and a `.JSON`-style extension in other capitals exit 2. Only lowercase `.json` counts; this is now documented.
- `Tables` together with `Table`, `TableName` or `BillingMode`, and `BillingModeSummary` against `BillingMode` naming different modes, exit 2.
- A duplicate key in the price file exits 2 (same check as the export files).
- `stderr_path_is_cleaned` only runs on unix, because Windows refuses a control character in a file name.
- The same table name twice (also inside one file) and the same file twice, in any spelling of the path, exit 2.
- A wrong type for `ProvisionedThroughput`, `TableName` or a tag value, and `Metrics` on a provisioned table, exit 2.
- README states that 32 MiB of empty tables can take several GB of memory.
- A wrong type for `BillingMode`, `BillingModeSummary` or an element of `Tables` exits 2 and names the field.
- A duplicate JSON key, `Metrics` together with `ReadRequestUnits`/`WriteRequestUnits`, and the same table name in two files exit 2.
- Every error message and path on stderr goes through the same control-character cleaning as the text table.
- A present but invalid on-demand field now exits 2 and names the field.
- Too large amounts exit 2 instead of panicking.
- Control characters in names and labels are replaced in the text table.
- All input files together are capped at 128 MiB (each file still 32 MiB).

## [0.1.0] - 2026-11-09

- First release: offline `dynamodb-bill` from local DynamoDB JSON exports.
- On-demand and provisioned capacity (us-east-1 built-in).
- Storage GB-month line.
- `--group-by`, `--prices`, `--json`.
