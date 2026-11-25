# Kwaliteit

STAND: lokaal groen op deze commit; de GitHub-workflow is not run.

- Datum: 2026-11-25
- Gemeten commit: ec43a2c
- Toolchain: cargo 1.99.0, rustc 1.99.0. `rust-version` 1.85. Versie 0.1.0.

| Opdracht | Uitslag |
|---|---|
| `cargo fmt --check` | exit 0, schoon |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, geen waarschuwingen |
| `cargo test --locked` (debug) | 92 geslaagd, 0 mislukt, 0 genegeerd |
| `cargo test --release --locked` | 92 geslaagd, 0 mislukt, 0 genegeerd |

Beide testruns: 34 lib, 2 main, 6 claims, 18 cli, 32 strict, 0 doc-tests.

Deze pagina legt één run vast. Het is geen garantie en geen fuzzrapport.

## Niet uitgevoerd

- GitHub-workflow: not run
- `cargo publish --dry-run`: not run
- cargo deny: not run
- cargo audit: not run
- fuzzing: not run
- mutatietests: not run
- rood-vergelijking met eerdere commits: not run
