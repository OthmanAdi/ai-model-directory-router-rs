# Contributing

Thanks for your interest in improving `ai-model-directory-router`. This document
covers everything you need to make a useful contribution.

## Quick Start

```bash
git clone https://github.com/OthmanAdi/ai-model-directory-router-rs.git
cd ai-model-directory-router-rs
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

You will need a copy of the AI Model Directory data file for integration
testing. Download
[`all.min.json`](https://github.com/The-Best-Codes/ai-model-directory/blob/main/data/all.min.json)
and place it at `data/all.min.json` relative to your working directory, or use
`RouterStore::from_json` with inline test data (see `src/tests.rs` for the
existing test fixture).

## Project Layout

```
src/
  lib.rs        Crate root, re-exports
  types.rs      All public types
  store.rs      RouterStore, data loading
  cost.rs       Cost calculation
  router.rs     Filtering, sorting, pagination
  fallback.rs   Fallback chain generation
  context.rs    Context window checks
  compare.rs    Model comparison
  tests.rs      Test suite
```

## Coding Guidelines

- Run `cargo fmt` before every commit.
- Run `cargo clippy --all-targets --all-features -- -D warnings` and fix all
  warnings.
- Keep code self-documenting. Prefer descriptive names over comments.
- Use `rust_decimal::Decimal` for all cost and price arithmetic. Never use
  `f64` for monetary values in new code.
- Make surgical edits. Do not reformat or refactor unrelated code.
- All public functions and types must have `///` doc comments with usage
  examples where practical.
- Add `#[non_exhaustive]` to enums and structs that may grow in future
  releases.

## Adding a New Feature

1. Add any new types to `types.rs`.
2. Implement the feature in the appropriate module (or a new one).
3. Re-export public items from `lib.rs`.
4. Add tests to `tests.rs`.
5. Add a doc comment with a `no_run` example.
6. Run `cargo test`, `cargo clippy`, and `cargo doc --no-deps` (verify docs
   render correctly).

## Pull Request Checklist

Before opening a PR:

- [ ] `cargo test` passes (all existing and new tests).
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean.
- [ ] `cargo fmt` has been run.
- [ ] New public items have doc comments.
- [ ] No secrets, API keys, or large data files committed.

## Reporting Issues

Open a [GitHub issue](https://github.com/OthmanAdi/ai-model-directory-router-rs/issues)
with:

- Rust version (`rustc --version`)
- Crate version
- A minimal reproducer if applicable

## Relationship to the Upstream Project

This crate is a Rust implementation of the TypeScript router package in the
[AI Model Directory](https://github.com/The-Best-Codes/ai-model-directory)
monorepo. API changes that break parity with the TypeScript version should be
discussed in an issue first.

Thank you for contributing!
