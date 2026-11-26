# dynamodb-bill

[![crates.io](https://img.shields.io/crates/v/dynamodb-bill.svg)](https://crates.io/crates/dynamodb-bill) [![downloads](https://img.shields.io/crates/d/dynamodb-bill.svg)](https://crates.io/crates/dynamodb-bill) [![CI](https://github.com/qwpp7g79y4-maker/dynamodb-bill/actions/workflows/ci.yml/badge.svg)](https://github.com/qwpp7g79y4-maker/dynamodb-bill/actions/workflows/ci.yml) [![PEPAI checked](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fqwpp7g79y4-maker%2Fdynamodb-bill%2Fmain%2F.github%2Fbadge.json)](QUALITY.md) [![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://github.com/qwpp7g79y4-maker/dynamodb-bill/blob/main/Cargo.toml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/qwpp7g79y4-maker/dynamodb-bill#license)

## Waarom

Schat de maandelijkse kosten van **AWS DynamoDB** (opslag, on-demand
request units en provisioned RCU/WCU) uit een lokale JSON-export, zonder AWS
aan te roepen.

Het leest alleen bestanden en gebruikt geen netwerk. De tests en de gemeten
aantallen staan in `QUALITY.nl.md`. De Engelse versie is `README.md`.

## Installatie

Versie 0.1.1. Van crates.io, zodra 0.1.1 daar staat:

```sh
cargo install dynamodb-bill
```

Of bouw uit de broncode.

Rust 1.85+. Vanuit een checkout:

```sh
cargo install --path .
```

## Aan de slag in 5 minuten

1. Bouw: `cargo build --release` (Rust 1.85+).
2. Draai het voorbeeld: `target/release/dynamodb-bill examples/on-demand.json`.
3. Zet je eigen tabelexport (de vorm staat onder "Hoe het telt") in een
   bestand en geef dat als argument mee.
4. Voeg `--json` toe voor machineleesbare uitvoer of `--group-by KEY` om op tag
   op te tellen.

Er wordt niets naar schijf geschreven en niets verstuurd.

## Gebruik

```text
dynamodb-bill [OPTIONS] <PATH>...
```

- `<PATH>...`: JSON-exportbestanden of een map met `*.json`. Alleen de kleine
  letters `.json` tellen mee. Een map zonder `.json`-bestand en een bestand
  zoals `X.JSON` in andere hoofdletters geven exit 2 (de melding noemt het
  bestand); hernoem het bestand.
- `--prices <FILE>`: prijsbestand. Standaard: ingebouwde us-east-1-tarieven.
- `--group-by <KEY>`: groepeer rijen op deze `Tags`/`Metadata`-sleutel.
- `--json`: JSON (`schema_version` 1) in plaats van de teksttabel.
- `-h`, `--help` en `-V`, `--version`.

## Voorbeeld

```sh
dynamodb-bill examples/on-demand.json
```

```text
table	mode	size_bytes	storage_cost	read_cost	write_cost	total
orders	on-demand	1073741824	0.25 USD	0.125 USD	0.625 USD	1.000 USD
total			0.25 USD	0.125 USD	0.625 USD	1.000 USD
```

## Hoe het telt

- Invoer: een of meer JSON-bestanden, of een map met `*.json`, in de vorm
  `{ "Tables": [ { "TableName", "BillingModeSummary"|"BillingMode",
  "TableSizeBytes", "ProvisionedThroughput"?, "ReadRequestUnits"?,
  "WriteRequestUnits"?, "Tags"? } ] }` of een enkel `Table`-object. De
  veldnamen volgen
  [DescribeTable](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_DescribeTable.html)
  (geraadpleegd 2026-11-09).
- **Opslag:** `TableSizeBytes / 1024³ × storage_per_gb_month`. Ingebouwd
  us-east-1-tarief van
  [Amazon DynamoDB pricing](https://aws.amazon.com/dynamodb/pricing/)
  (geraadpleegd 2026-11-09): **$0.25** / GB-maand.
- **On-demand:** `ReadRequestUnits / 1e6 × $0.125` en
  `WriteRequestUnits / 1e6 × $0.625` (zelfde pagina, 2026-11-09). De units
  mogen ook uit `Metrics` komen.
- **Provisioned:** `RCU × $0.00013/uur × 730` en
  `WCU × $0.00065/uur × 730` (zelfde pagina, 2026-11-09).
- `--group-by KEY` telt op per `Tags` of `Metadata`.

## Afsluitcodes

- `0`: goed (ook bij `--help` en `--version`).
- `2`: fout in invoer of gebruik, bijvoorbeeld een onleesbaar of kapot
  JSON-bestand, een aanwezig veld met het verkeerde type of een ongeldige waarde
  (de melding noemt het veld), `Metrics` bij een provisioned tabel, dezelfde
  tabelnaam twee keer (in twee bestanden of in één bestand), hetzelfde bestand
  twee keer (in elke schrijfwijze van het pad), een bedrag dat te groot is om
  te rekenen, of invoer boven de groottegrenzen. Ook exit 2: `Tags` of
  `Metadata` dat geen object is en geen lijst van `{"Key","Value"}`-objecten,
  een tagsleutel twee keer in één lijst, `Tags` en `Metadata` die één sleutel
  anders vullen, `Tables` samen met `Table`, `TableName` of `BillingMode`,
  `BillingModeSummary` en `BillingMode` die een andere modus noemen, een map
  zonder `.json`-bestand, en een dubbele sleutel in het prijsbestand.

Behalve bij een geheugentekort (zie Beperkingen) is er voor foute invoer geen
andere exitcode, en het programma raakt niet in paniek.

## Prijzen en verantwoordelijkheid

De ingebouwde tarieven zijn de us-east-1-tarieven in `prices/us-east-1.json`,
overgenomen van de pagina
[Amazon DynamoDB pricing](https://aws.amazon.com/dynamodb/pricing/) op
2026-11-09. AWS kan prijzen op elk moment wijzigen en andere regio's en
kortingen wijken af. De uitvoer is een schatting, geen factuur. Jij bent
verantwoordelijk voor het naast je AWS-factuur leggen. Gebruik `--prices FILE`
voor je eigen tarieven.

## Beperkingen

- Alleen offline: geen AWS-netwerkclient in v0.1.
- GlobalSecondaryIndexes, LocalSecondaryIndexes, replica's, global tables, Standard-IA en back-ups worden niet geteld, dus echte kosten kunnen hoger zijn.
- On-demand aantallen moeten in de export staan als je requestkosten wilt;
  grootte alleen verzint geen verkeer.
- Elk bestand is hoogstens 32 MiB, en alle invoerbestanden samen zijn
  hoogstens 128 MiB. Daarboven stopt het met exit 2 voordat de bestanden
  gelezen worden.
- Die grenzen begrenzen het geheugen niet. Het verwerken kost veel meer geheugen
  dan de bestandsgrootte: een bestand van 32 MiB met lege tabellen (ongeveer
  11 miljoen) mat ongeveer 6,4 GB, en meerdere bestanden tellen op. Op een
  machine met minder geheugen kan het systeem het proces afbreken, en dan is de
  exitcode niet 2.
- Besturingstekens in tabelnamen en groepslabels staan als `?` in de
  teksttabel; `--json` escapet ze.

## Ontwikkeling

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

De suite heeft 92 tests. Zie `QUALITY.nl.md`.

## Onderhoud

De prijzen zijn een momentopname van 2026-11-09 en worden niet automatisch
bijgewerkt. Patches komen binnen via Issues en horen een test te hebben. Zie
`CONTRIBUTING.md`; meld beveiligingsproblemen zoals in `SECURITY.md`.

Na een release staat de tool in sluimermodus (dormant): een nieuwe versie komt er alleen bij een gemelde bug, een verzoek of een beveiligingsprobleem. Beveiliging gaat altijd voor.

## Licentie

MIT OR Apache-2.0.

De licenties van de externe crates in het programma staan in [THIRD-PARTY-LICENSES.html](THIRD-PARTY-LICENSES.html).
