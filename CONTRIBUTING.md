# Contributing

Thanks for helping. Please keep changes small and focused.

## Patches

- A patch should come with a test. A bug fix needs a test that fails before the
  fix and passes after it.
- Patches come in through Issues: open an Issue that describes the problem and
  attach or link the patch there.
- Before you send it, run `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings` and `cargo test`, and say which
  you ran.
- Do not invent prices. A price change needs the official AWS pricing page and
  the date you read it.

## Security

Do not report security problems in a public Issue. See `SECURITY.md`.

## Licence

By contributing you agree that your work is licensed under MIT OR Apache-2.0.
