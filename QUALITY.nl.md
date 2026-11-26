# Kwaliteit

pep: 8.6

STAND: lokaal groen op deze commit; de GitHub-workflow is not run.

- Datum: 2026-11-25
- Gemeten commit: aa2e34e
- Toolchain: cargo 1.99.0, rustc 1.99.0. `rust-version` 1.85. Versie 0.1.0.

| Opdracht | Uitslag |
|---|---|
| `cargo fmt --check` | exit 0, schoon |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, geen waarschuwingen |
| `cargo test --locked` (debug) | 95 geslaagd, 0 mislukt, 0 genegeerd |
| `cargo test --release --locked` | 95 geslaagd, 0 mislukt, 0 genegeerd |

Beide testruns: 34 lib, 2 main, 6 claims, 21 cli, 32 strict, 0 doc-tests.

Fix aa2e34e: gesloten stdout bij de start gaf exit 0, nu exit 2 met een melding (alleen Linux; `/dev/null` blijft exit 0). Vóór de fix faalden de twee nieuwe tests (rood); `--output` en `build.rs` bestaan niet in deze crate, dus daar viel niets te controleren.

## Run met Rust 1.85

- Datum: 2026-11-25
- Gemeten commit: 220a489 (losse HEAD; de eerste commit hierboven, aa2e34e, blijft de eerste omdat er geen code is gewijzigd)
- Toolchain: cargo 1.85.1, rustc 1.85.1 (de `rust-version`)

| Opdracht | Uitslag |
|---|---|
| `cargo +1.85 fmt --check` | exit 0, schoon |
| `cargo +1.85 clippy --all-targets --locked -- -D warnings` | exit 0, geen waarschuwingen |
| `cargo +1.85 test --locked` (debug) | 95 geslaagd, 0 mislukt, 0 genegeerd |
| `cargo +1.85 test --release --locked` | 95 geslaagd, 0 mislukt, 0 genegeerd (2026-11-25) |

Tests op 1.85 (debug): 95 geslaagd, 0 mislukt (34 lib, 2 main, 6 claims, 21 cli, 32 strict, 0 doc-tests).

Deze pagina legt één run vast. Het is geen garantie en geen fuzzrapport.

## Niet uitgevoerd

- GitHub-workflow: not run
- `cargo publish --dry-run`: not run
- cargo deny: not run
- cargo audit: not run
- fuzzing: not run
- mutatietests: not run
- rood-vergelijking met eerdere commits: not run
