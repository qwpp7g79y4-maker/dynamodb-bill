# Quality

STAND: locally green; GitHub workflow niet uitgevoerd (not run).

- Date: 2026-11-24
- Tested code commit: 5eb0628 (the last commit that is not a pure document; commits after it only touch documents).
- Command: `cargo test --locked --no-fail-fast` (debug)
- Tests: 92 passed, 0 failed, 0 ignored (34 lib, 2 main, 6 claims, 18 cli, 32 strict, 0 doc-tests).
- Red on 09af909 (source of that commit, new tests added): `cargo test` gives `test result: FAILED. 30 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out` for `tests/strict.rs`; failing: `request_units_on_provisioned_table_are_refused`, `request_units_without_mode_are_refused` (the input gave exit 0 and 0 USD, `panicked at tests/strict.rs:44:5`). On 5eb0628 all 32 pass. One older test, `same_table_name_twice_in_one_file_is_refused`, used `ReadRequestUnits` on a table without a mode; its input now uses `TableSizeBytes`, same intent.
- Red on a346597: the 29 tests of `tests/strict.rs` run on that tree give `test result: FAILED. 21 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out`. On 2ebb197 they are 29 passed. (Earlier: red on 27ad119, 14 passed; 6 failed.)
- Version 0.1.0. `rust-version` 1.85.
- Also run on 5eb0628: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`, both clean.

This page records one run. It is not a guarantee and it is not a fuzz report.

## Not run

GitHub Actions workflow: niet uitgevoerd. It was not started (payment block) and nothing was sent to GitHub, so there is no CI result for this tree.

Also not run on this commit: `cargo publish --dry-run`, `cargo test --release`, cargo deny, cargo audit, fuzzing, and mutation testing.

## Known open points

Not fixed in this round (N1 to N13 stay open; the built-in rates are unchanged; indexes, replicas, global tables, Standard-IA and backups are documented as not counted, not refused): rounding rule (banker's, `round_dp(6)`) is not documented, negative prices are accepted, memory use grows with input size (a 32 MiB file of empty tables can take several GB of memory, and no table-count limit exists), mixed Dutch/English messages, and `list-tables` output is not supported.
