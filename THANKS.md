# Thanks and Attribution

## models.dev

Version 0.3 uses the provider-scoped catalog published by
[models.dev](https://models.dev), an open-source database maintained by
[Anomaly](https://github.com/anomalyco/models.dev) and its contributors.

The bundled `data/models-dev-api.json` file is a dated snapshot of the public
`https://models.dev/api.json` endpoint. It is redistributed under the models.dev
MIT license. The required copyright and license notice is included at
`THIRD_PARTY_LICENSES/models.dev-MIT.txt`.

models.dev distinguishes three public JSON views:

- `api.json` contains provider-specific offerings, including provider pricing,
  limits, lifecycle state, and API metadata.
- `models.json` contains facts about underlying models that are independent of
  where they are served.
- `catalog.json` combines the provider and underlying-model collections.

This crate ingests `api.json` because routing, pricing, limits, and availability
belong to a provider offering. The upstream catalog remains a metadata source,
not an authority for provider billing or production availability.

## AI Model Directory

Earlier releases were inspired by the
[AI Model Directory](https://github.com/The-Best-Codes/ai-model-directory) and
its TypeScript routing work. Version 0.3 no longer claims API parity with a
TypeScript companion package. The former `packages/router` link is not present
on the upstream `main` branch.

## Rust Dependencies

The crate also relies on the work of the maintainers and contributors of
[`rust_decimal`](https://crates.io/crates/rust_decimal),
[`serde`](https://crates.io/crates/serde),
[`serde_json`](https://crates.io/crates/serde_json),
[`sha2`](https://crates.io/crates/sha2),
[`thiserror`](https://crates.io/crates/thiserror), and the optional
[`ureq`](https://crates.io/crates/ureq) HTTP client.
