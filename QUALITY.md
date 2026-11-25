# Quality

STATUS: green locally on this commit; the GitHub workflow is not run.

- Date: 2026-11-25
- Commit measured: ec43a2c
- Toolchain: cargo 1.99.0, rustc 1.99.0. `rust-version` 1.85. Version 0.1.0.

| Command | Result |
|---|---|
| `cargo fmt --check` | exit 0, clean |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, no warnings |
| `cargo test --locked` (debug) | 92 passed, 0 failed, 0 ignored |
| `cargo test --release --locked` | 92 passed, 0 failed, 0 ignored |

Tests: 92 passed, 0 failed (34 lib, 2 main, 6 claims, 18 cli, 32 strict, 0 doc-tests), in both the debug and the release run.

This page records one run. It is not a guarantee and it is not a fuzz report.

## Not run

- GitHub Actions workflow: not run
- `cargo publish --dry-run`: not run
- cargo deny: not run
- cargo audit: not run
- fuzzing: not run
- mutation testing: not run
- red-run comparison against earlier commits: not run
