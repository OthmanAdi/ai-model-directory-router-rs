# Contributing

Thank you for improving `ai-model-directory-router`. Contributions should keep
the crate focused on deterministic metadata routing. Provider authentication,
API request construction, and model inference belong in downstream clients.

## Development Setup

Install Rust 1.85 or newer, then run:

```bash
git clone https://github.com/OthmanAdi/ai-model-directory-router-rs.git
cd ai-model-directory-router-rs
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

The default `bundled` feature includes the checked-in models.dev snapshot. Most
tests should use small inline JSON fixtures so failures remain focused and
repeatable.

## Project Layout

```text
src/
  lib.rs        Crate root and public exports
  types.rs      Public catalog, routing, pricing, and result types
  store.rs      Catalog loading and provider-aware lookup
  cost.rs       Exact cost calculation and pricing tiers
  router.rs     Filtering, sorting, and offset pagination
  fallback.rs   Fallback chain generation
  context.rs    Context, input, and output limit checks
  compare.rs    Model comparison
  overlay.rs    Legacy AI Model Directory overlay support
  tests.rs      Test suite
data/
  models-dev-api.json  Bundled, dated models.dev snapshot
examples/       Runnable catalog and routing examples
```

## Coding Guidelines

- Keep changes surgical and do not reformat unrelated code.
- Use `rust_decimal::Decimal` for every monetary value and calculation. Do not
  convert prices through `f32` or `f64`.
- Preserve provider-qualified model identity. A bare model ID may identify more
  than one commercial offering.
- Represent unknown metadata as unknown. Missing prices must not become zero,
  and absent limits must not become invented limits.
- Keep input, output, and total context checks independent.
- Add documentation and focused tests for public behavior.
- Add `#[non_exhaustive]` where callers should be able to tolerate future
  variants or fields.

## Updating the Bundled Catalog

The bundled file is a snapshot of the public
[`https://models.dev/api.json`](https://models.dev/api.json) endpoint. It does
not require a provider API key.

For a catalog update:

1. Download the public endpoint without modifying its JSON values.
2. Update the bundled retrieval time, ETag, observed source revision, SHA-256,
   provider count, and offering count in `src/store.rs`, `README.md`, and
   `CHANGELOG.md`.
3. Confirm the models.dev MIT notice remains in
   `THIRD_PARTY_LICENSES/models.dev-MIT.txt`.
4. Run the full validation suite, including minimal and online feature builds.
5. Review additions, removals, lifecycle changes, prices, and limits. Treat the
   snapshot as source data, not as provider-verified production truth.

Do not add provider-specific corrections without cited primary evidence and a
clear provenance boundary. Promotional, regional, time-varying, and tiered
prices require particular care.

## Validation

Run the gates used by CI:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features --locked
cargo test --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
cargo check --all-targets --no-default-features --locked
cargo test --all-targets --no-default-features --locked
cargo check --all-targets --no-default-features --features online --locked
cargo test --all-targets --no-default-features --features online --locked
cargo package --locked
cargo publish --dry-run --locked
```

Also verify the declared minimum toolchain:

```bash
cargo +1.85 check --all-targets --all-features --locked
cargo +1.85 test --all-targets --all-features --locked
```

## Pull Requests

Before opening a pull request:

- Add tests for behavior changes and regression fixes.
- Update `README.md` and `CHANGELOG.md` when the public contract changes.
- Run `cargo fmt`, tests, Clippy, documentation, and relevant feature checks.
- Confirm examples compile with the feature combinations they describe.
- Do not commit credentials, provider API keys, or unpublished security reports.

Open bug reports through
[GitHub Issues](https://github.com/OthmanAdi/ai-model-directory-router-rs/issues)
with the crate version, `rustc --version`, and a minimal reproducer. Report
security issues privately as described in [SECURITY.md](SECURITY.md).
