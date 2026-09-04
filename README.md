# ai-model-directory-router

[![crates.io](https://img.shields.io/crates/v/ai-model-directory-router.svg)](https://crates.io/crates/ai-model-directory-router)
[![docs.rs](https://docs.rs/ai-model-directory-router/badge.svg)](https://docs.rs/ai-model-directory-router)
[![CI](https://github.com/OthmanAdi/ai-model-directory-router-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/OthmanAdi/ai-model-directory-router-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Provider-aware Rust routing over a dated bundled snapshot with refreshable
[models.dev](https://models.dev) loaders. The crate provides deterministic
lookup, filtering, exact cost estimation, context checks, comparisons, and
fallback discovery for provider-specific model offerings.

This is a metadata router. It never sends prompts to OpenAI, Alibaba, Anthropic,
DeepSeek, or any other model provider. No provider API key is required to use a
bundled, local-file, inline, or live models.dev catalog. Provider authentication
and model inference are explicitly outside this crate's scope.

## Installation

Version 0.3 requires Rust 1.85 or newer. The default `bundled` feature includes
the dated models.dev snapshot used by `RouterStore::bundled()`.

```toml
[dependencies]
ai-model-directory-router = "0.3"
```

For a minimal file or inline JSON build without the bundled snapshot:

```toml
[dependencies]
ai-model-directory-router = { version = "0.3", default-features = false }
```

For HTTPS access to the fixed public `https://models.dev/api.json` endpoint,
enable `online`. This example omits the bundled snapshot:

```toml
[dependencies]
ai-model-directory-router = { version = "0.3", default-features = false, features = ["online"] }
```

## Quick Start

Use a provider-qualified lookup whenever you know the offering you want:

```rust
use ai_model_directory_router::RouterStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = RouterStore::bundled()?;
    if let Some(model) = store.find_model_in("alibaba", "qwen3.8-flash") {
        println!("{}", model.key());
        println!("context: {:?}", model.limit.as_ref().and_then(|limit| limit.context));
    } else {
        println!("the offering is not present in this dated snapshot");
    }
    Ok(())
}
```

The lookup reads metadata only. It does not test whether your provider account
can invoke the model.

## Provider-Qualified Identity

A model ID is not a globally unique commercial offering. The same ID may be
served by several providers with different prices, limits, lifecycle states,
and regions. `ModelKey` represents the stable pair:

```rust
use ai_model_directory_router::ModelKey;

let key = ModelKey::new("alibaba", "qwen3.8-flash");
assert_eq!(key.to_string(), "alibaba/qwen3.8-flash");
```

Use these lookup rules:

- `find_model_in(provider, model_id)` selects one provider offering.
- `find_models_by_id(model_id)` returns every provider offering with that ID.
- `resolve_model(model_id)` succeeds only when the bare ID is unique. It returns
  `RouterError::AmbiguousModel` with the matching providers when it is not.
- `FlatModel::key()` returns the offering's `ModelKey`.

High-level APIs that accept only a bare model ID use the same unique-only
resolution rule. Prefer their provider-qualified variants for production code.

## Catalog Sources

```rust
use ai_model_directory_router::RouterStore;
use std::path::Path;

fn load(json: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bundled = RouterStore::bundled()?;
    let inline = RouterStore::from_models_dev_json(json)?;
    let file = RouterStore::from_models_dev_file(Path::new("models-dev-api.json"))?;
    println!(
        "{} {} {}",
        bundled.flat_models().len(),
        inline.flat_models().len(),
        file.flat_models().len()
    );
    Ok(())
}
```

With the optional `online` feature:

```rust,no_run
use ai_model_directory_router::RouterStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let live = RouterStore::from_models_dev_live()?;
    println!("loaded {} offerings", live.flat_models().len());
    Ok(())
}
```

`from_models_dev_live()` downloads only the fixed public models.dev catalog.
It does not contact model providers and does not read provider credentials.

models.dev publishes three related JSON views:

- [`api.json`](https://models.dev/api.json) describes provider-specific
  offerings. It contains the pricing, limits, lifecycle state, and provider
  metadata used by this crate.
- [`models.json`](https://models.dev/models.json) describes underlying models
  independently of where they are served.
- [`catalog.json`](https://models.dev/catalog.json) combines the provider and
  underlying-model collections.

Pass the provider-shaped `api.json` document to the
`from_models_dev_*` constructors. `models.json` alone is not a provider routing
catalog.

## Routing

`route` supports provider, model ID, family, lifecycle status, modality,
feature, context, exact price, and open-weight filters. Pagination uses numeric
`offset` and `limit`, not a cursor.

```rust
use ai_model_directory_router::{
    route, ModelFeatures, RouteQuery, RouterStore, SortField, SortOrder,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = RouterStore::bundled()?;
    let result = route(
        &store,
        &RouteQuery {
            provider: Some("alibaba".into()),
            min_context: Some(128_000),
            max_input_price: Some("0.20".parse()?),
            features: Some(ModelFeatures {
                tool_call: Some(true),
                ..ModelFeatures::default()
            }),
            sort: Some(SortField::InputPrice),
            order: Some(SortOrder::Asc),
            limit: Some(10),
            offset: Some(0),
            ..RouteQuery::default()
        },
    );

    println!("{} total matches, more: {}", result.total, result.has_more);
    Ok(())
}
```

An unset `status` filter includes records with any lifecycle state, including
records whose status is unknown. Setting `status` selects that exact catalog
value: `Alpha`, `Beta`, or `Deprecated`. Feature filters distinguish `false`
from missing metadata. Missing prices sort after known prices in both sort
directions.

## Exact Cost Estimation

Catalog rates, price filters, fallback multipliers, and calculated costs use
`rust_decimal::Decimal`. JSON prices are deserialized directly into decimals,
without adding a binary floating-point conversion. This preserves the numeric
lexeme supplied by the catalog, including any source-side artifacts. It does
not normalize or independently verify provider prices. Rates are USD per
million tokens. `CostBreakdown` values are already scaled to the requested
usage.

```rust
use ai_model_directory_router::{
    calculate_cost_for_model, CostRequest, RouterStore,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = RouterStore::bundled()?;
    let model = store
        .find_model_in("alibaba", "qwen3.8-flash")
        .expect("model should exist in this snapshot");
    let cost = calculate_cost_for_model(
        model,
        &CostRequest {
            input_tokens: 1_000_000,
            output_tokens: 250_000,
            context_tokens: Some(300_000),
            ..CostRequest::default()
        },
    )?;

    println!("input: ${}, total: ${}", cost.input, cost.total);
    Ok(())
}
```

The estimator supports input, output, reasoning, cache-read, cache-write,
audio-input, and audio-output token components. `context_tokens` selects the
highest applicable explicit context tier. If a record has no explicit tiers,
the models.dev `context_over_200k` rates apply above 200,000 context tokens.
When `context_tokens` is omitted, the estimator uses `input_tokens` for tier
selection. `applied_tier` reports the selected threshold.

A nonzero usage component without a matching catalog rate returns
`RouterError::MissingPriceComponent`. Negative rates are rejected by catalog
loading and guarded again during model cost calculation. Missing price data is
never interpreted as free. Token arithmetic that cannot be represented returns
`RouterError::TokenOverflow`.

## Context Checks

Context, input, and requested output are checked independently:

```rust
use ai_model_directory_router::{check_context_fit_for_provider, RouterStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = RouterStore::bundled()?;
    let fit = check_context_fit_for_provider(
        &store,
        "alibaba",
        "qwen3.8-flash",
        100_000,
        Some(20_000),
    )?;

    println!(
        "all: {}, context: {}, input: {}, output: {}",
        fit.fits, fit.context_fits, fit.input_fits, fit.output_fits
    );
    Ok(())
}
```

`fits` is true only when `context_fits`, `input_fits`, and `output_fits` are all
true. The context check uses input plus requested output. Input and output are
also compared with their own provider limits. Checked addition prevents token
count overflow.

## Retained Catalog Schema

The models.dev loader retains the fields needed to inspect and route current
offerings:

| Area | Fields |
|------|--------|
| Identity and text | provider, id, name, description, family |
| Dates and weights | knowledge cutoff, release date, last updated, open weights |
| Capabilities | attachment, reasoning, tool call, structured output, temperature |
| Reasoning | reasoning options, interleaved reasoning field |
| Lifecycle | alpha, beta, or deprecated status; experimental metadata |
| Modalities | text, image, audio, video, file, and PDF input or output |
| Limits | context, input, and output token limits |
| Pricing | input, output, reasoning, cache read/write, audio input/output, context-over-200k rates, and context tiers |
| Provider | name, documentation URL, website, API base, npm package, and declared environment variable names |
| Provenance | catalog source, URL, retrieval time, SHA-256, ETag, observed source revision, byte count, and record counts |

The known provider-specific and experimental objects are retained as
`serde_json::Value`. Unknown future top-level fields are accepted by serde but
are not retained. New string enum values map to an `Unknown` variant so one new
upstream value does not reject the whole provider catalog.

`source_revision` records the models.dev source revision observed when the
snapshot was retrieved. The endpoint does not prove which source commit
generated a particular response, so this value is not a payload commit ID.

## Bundled Snapshot and Data Limits

The 0.3.0 bundle was retrieved from `https://models.dev/api.json` at
`2026-09-04T07:53:34Z`. Its HTTP ETag is
`"c68bb1a66b2260432a30862fa0c759ce"`, and its SHA-256 is
`c68bb1a66b2260432a30862fa0c759ce08bde20322e2c949dee9c615c3cc4c8f`.
It contains 4,465,310 bytes, 213 providers, and 7,526 provider-model
offerings. The models.dev `dev` revision observed at retrieval was
`ba816a069564b9dbf7c61cc117a79301b3f9ecdc`.

The snapshot is dated source data. It is not an authority for provider billing,
regional access, account entitlements, rate limits, deprecations, or real-time
availability. Prices can be promotional, regional, tiered, or time-dependent.
Verify production limits and prices against the provider's current official
documentation before making operational or financial decisions.

The models.dev snapshot is redistributed under the MIT license. Its copyright
and license notice is included in
[`THIRD_PARTY_LICENSES/models.dev-MIT.txt`](THIRD_PARTY_LICENSES/models.dev-MIT.txt).
The Rust crate itself is Apache-2.0 licensed.

## Migration from 0.2

Version 0.3 contains breaking correctness changes:

- Raise the Rust toolchain to 1.85 or newer.
- Replace runtime dependence on `data/all.min.json` with `bundled()` or a
  `from_models_dev_*` constructor when using models.dev data.
- Replace bare `find_model` assumptions with `find_model_in`,
  `find_models_by_id`, or the unique-only `resolve_model` method.
- Handle `AmbiguousModel` from bare-ID resolution, context, fallback, and
  comparison operations, or use provider-qualified variants.
- Replace `f64` price values and multipliers with `Decimal`.
- Handle the `Result` returned by cost calculation. Unknown nonzero price
  components now produce an explicit error instead of a zero cost.
- Supply `context_tokens` when tiered pricing may apply.
- Read `ContextFit::context_fits`, `input_fits`, and `output_fits` when you need
  to distinguish which provider limit rejected a request.

Legacy AI Model Directory JSON and overlay methods remain available for callers
that still own those data files, but they do not gain the bundled or live
models.dev provenance guarantees.

## More Examples

See [`examples/`](examples/) for bundled, live, and legacy-overlay programs:

```bash
cargo run --example bundled_lookup
cargo run --example online_catalog --features online
cargo run --example overlay_impact -- data/all.min.json data/models-dev-api.json
```

## Contributing and Security

See [CONTRIBUTING.md](CONTRIBUTING.md) for development and catalog-update
guidance. Report vulnerabilities privately as described in
[SECURITY.md](SECURITY.md). Community participation is governed by
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Acknowledgments

The bundled catalog comes from models.dev and its contributors. Earlier releases
were inspired by AI Model Directory. See [THANKS.md](THANKS.md) for attribution
and project history.

## License

The crate is licensed under [Apache License 2.0](LICENSE). The bundled
models.dev snapshot retains its upstream MIT notice.
