# Security

## Reporting a vulnerability

Please report security problems by e-mail to pep_ai@icloud.com. Do not open a
public Issue for them. Include the version (`dynamodb-bill --version`), the
input that triggers the problem and what you expected.

## Scope

`dynamodb-bill` reads local JSON files and prints an estimate. It has no
network client. Reports about crashes or unbounded memory use on crafted input
are in scope. The correctness of the AWS prices is not a security matter; see
"Prices and responsibility" in `README.md`.

## Supported versions

Only the latest released version gets fixes.
