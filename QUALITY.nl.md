# Kwaliteit

STAND: lokaal groen; GitHub-workflow niet uitgevoerd.

- Datum: 2026-11-24
- Geteste codecommit: 5eb0628 (de laatste commit die geen puur document is; commits daarna raken alleen documenten).
- Commando: `cargo test --locked --no-fail-fast` (debug)
- Tests: 92 geslaagd, 0 mislukt, 0 genegeerd (34 lib, 2 main, 6 claims, 18 cli, 32 strict, 0 doc-tests).
- Rood op 09af909 (bron van die commit, nieuwe tests erbij): `cargo test` geeft `test result: FAILED. 30 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out` voor `tests/strict.rs`; mislukt: `request_units_on_provisioned_table_are_refused`, `request_units_without_mode_are_refused` (de invoer gaf exit 0 en 0 USD, `panicked at tests/strict.rs:44:5`). Op 5eb0628 zijn alle 32 geslaagd. Een oudere test, `same_table_name_twice_in_one_file_is_refused`, gebruikte `ReadRequestUnits` op een tabel zonder modus; de invoer gebruikt nu `TableSizeBytes`, zelfde bedoeling.
- Rood op a346597: de 29 tests van `tests/strict.rs` op die boom geven `test result: FAILED. 21 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out`. Op 2ebb197 zijn het er 29 geslaagd. (Eerder: rood op 27ad119, 14 geslaagd; 6 mislukt.)
- Versie 0.1.0. `rust-version` 1.85.
- Ook gedraaid op 5eb0628: `cargo fmt --check` en `cargo clippy --all-targets -- -D warnings`, beide schoon.

Deze pagina legt één run vast. Het is geen garantie en geen fuzzrapport.

## Niet uitgevoerd

GitHub-workflow: niet uitgevoerd. Hij is niet gestart (betaalblokkade) en er is niets naar GitHub gestuurd, dus er is geen CI-uitslag voor deze boom.

Ook niet uitgevoerd op deze commit: `cargo publish --dry-run`, `cargo test --release`, cargo deny, cargo audit, fuzzing en mutatietests.

## Bekende open punten

Niet gerepareerd in deze ronde (N1 tot en met N13 blijven open; de ingebouwde tarieven zijn ongewijzigd; indexen, replica's, global tables, Standard-IA en back-ups staan als niet geteld beschreven, ze worden niet geweigerd): de afrondingsregel (bankiers, `round_dp(6)`) is niet beschreven, negatieve prijzen worden aanvaard, het geheugengebruik groeit met de invoer, de meldingen zijn deels Nederlands en deels Engels, en `list-tables`-uitvoer wordt niet ondersteund.
