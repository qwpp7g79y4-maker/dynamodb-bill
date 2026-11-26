# Quality

pep: 8.6

STATUS: green locally on this commit; the GitHub workflow is not run.

- Date: 2026-11-25
- Commit measured: aa2e34e
- Toolchain: cargo 1.99.0, rustc 1.99.0. `rust-version` 1.85. Version 0.1.0.

| Command | Result |
|---|---|
| `cargo fmt --check` | exit 0, clean |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, no warnings |
| `cargo test --locked` (debug) | 95 passed, 0 failed, 0 ignored |
| `cargo test --release --locked` | 95 passed, 0 failed, 0 ignored |

Tests: 95 passed, 0 failed (34 lib, 2 main, 6 claims, 21 cli, 32 strict, 0 doc-tests), in both the debug and the release run.

Fix aa2e34e: closed standard output at start gave exit 0, now exit 2 with a message (Linux only; `/dev/null` stays exit 0). Before the fix the two new closed-stdout tests failed (red); `--output` and `build.rs` do not exist in this crate, so nothing to check there.

## Rust 1.85 run

- Date: 2026-11-25
- Commit measured: 220a489 (detached HEAD; the first commit above, aa2e34e, stays the first because no code changed)
- Toolchain: cargo 1.85.1, rustc 1.85.1 (the `rust-version`)

| Command | Result |
|---|---|
| `cargo +1.85 fmt --check` | exit 0, clean |
| `cargo +1.85 clippy --all-targets --locked -- -D warnings` | exit 0, no warnings |
| `cargo +1.85 test --locked` (debug) | 95 passed, 0 failed, 0 ignored |
| `cargo +1.85 test --release --locked` | 95 passed, 0 failed, 0 ignored (2026-11-25) |

Tests on 1.85 (debug): 95 passed, 0 failed (34 lib, 2 main, 6 claims, 21 cli, 32 strict, 0 doc-tests).

This page records one run. It is not a guarantee and it is not a fuzz report.

## Not run

- GitHub Actions workflow: not run
- `cargo publish --dry-run`: not run
- cargo deny: not run
- cargo audit: not run
- fuzzing: not run
- mutation testing: not run
- red-run comparison against earlier commits: not run
