# Thanks and Attribution

## AI Model Directory

This crate is built on the [AI Model
Directory](https://github.com/The-Best-Codes/ai-model-directory) by
[The-Best-Codes](https://github.com/The-Best-Codes).

The AI Model Directory is the most comprehensive, automatically updated
directory of AI models and their metadata. It maintains pricing, context
windows, features, modalities, and provider information for over 7,000 models
across 50+ providers.

Specifically, this crate depends on:

- **The model dataset** (`data/all.min.json`), which is the runtime data source
  loaded by `RouterStore`. Without the ongoing work of the upstream project to
  keep this data current and accurate, this crate would not exist.
- **The TypeScript router package**
  ([`packages/router`](https://github.com/The-Best-Codes/ai-model-directory/tree/feat/router-package/packages/router)),
  which defined the API surface and business logic that this Rust crate mirrors.
  Every function, type, and scoring algorithm follows the original TypeScript
  implementation.

## How to Support the Upstream Project

- Star [the repository](https://github.com/The-Best-Codes/ai-model-directory).
- Contribute new providers, fix metadata errors, or report missing models by
  following their
  [CONTRIBUTING.md](https://github.com/The-Best-Codes/ai-model-directory/blob/main/CONTRIBUTING.md).
- Visit [models.agent-one.dev](https://models.agent-one.dev/) for the web
  directory.

## Open Source Dependencies

This crate relies on excellent Rust libraries:

- [serde](https://crates.io/crates/serde) and
  [serde_json](https://crates.io/crates/serde_json) for JSON deserialization.
- [rust_decimal](https://crates.io/crates/rust_decimal) for exact decimal
  arithmetic in cost calculations.
- [thiserror](https://crates.io/crates/thiserror) for ergonomic error types.
