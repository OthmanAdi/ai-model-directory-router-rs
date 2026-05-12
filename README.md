# ai-model-directory-router

[![crates.io](https://img.shields.io/crates/v/ai-model-directory-router.svg)](https://crates.io/crates/ai-model-directory-router)
[![docs.rs](https://docs.rs/ai-model-directory-router/badge.svg)](https://docs.rs/ai-model-directory-router)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Rust library for routing, cost calculation, fallback chain generation, and
context window management for over 7,000 AI models across 50+ providers.

This is the Rust companion to the TypeScript
[`@ai-model-directory/router`](https://github.com/The-Best-Codes/ai-model-directory/tree/feat/router-package/packages/router)
package, mirroring its API surface with idiomatic Rust and exact decimal
arithmetic via [`rust_decimal`](https://crates.io/crates/rust_decimal).

## Features

- **Model routing** with filters for provider, price, context window, features,
  modalities, and open weights. Sorting and cursor-style pagination built in.
- **Exact cost calculation** using `Decimal` (no float drift) across input,
  output, reasoning, cache read/write, and audio token types.
- **Fallback chains** scored by feature overlap, modality compatibility, context
  window size, price ratio, and provider affinity.
- **Context window checks** with automatic suggestions for better-fitting models.
- **Side-by-side comparisons** across any number of models, with winners
  computed for every numeric field.

## Installation

```toml
[dependencies]
ai-model-directory-router = "0.1"
```

Requires Rust 1.70+.

### Data file

This crate reads model data at runtime from the AI Model Directory's
`all.min.json` file. You need either:

- A copy of [`data/all.min.json`](https://github.com/The-Best-Codes/ai-model-directory/blob/main/data/all.min.json)
  in your working directory, or
- An explicit path passed to [`RouterStore::from_file`].

## Usage

```rust
use ai_model_directory_router::{
    RouterStore, route, calculate_cost_for_model, fallback_chain,
    check_context_fit, compare, CostRequest, RouteQuery, SortField, SortOrder,
};
use std::path::Path;

// Load model data
let store = RouterStore::from_file(Path::new("data/all.min.json")).unwrap();
println!("Loaded {} models", store.flat_models().len());

// Route: find cheap OpenAI models with large context
let query = RouteQuery {
    provider: Some("openai".to_string()),
    min_context: Some(128_000),
    sort: Some(SortField::InputPrice),
    order: Some(SortOrder::Asc),
    limit: Some(5),
    ..RouteQuery::default()
};
let result = route(&store, &query);
for model in &result.models {
    println!("{} at ${}/M input", model.id,
        model.pricing.as_ref().and_then(|p| p.input).unwrap_or(0.0));
}

// Cost: calculate exact pricing
let model = store.find_model("gpt-4o").unwrap();
let cost = calculate_cost_for_model(model, &CostRequest {
    input_tokens: 1_000_000,
    output_tokens: 500_000,
    ..Default::default()
});
println!("Total cost: ${}", cost.total);

// Fallback: find alternatives for a model
let chain = fallback_chain(&store, "gpt-4o", &Default::default()).unwrap();
for model in &chain.models {
    println!("Fallback: {}", model.id);
}

// Context: check if a prompt fits
let fit = check_context_fit(&store, "gpt-4o", 50_000, Some(10_000)).unwrap();
println!("Fits: {}, overhead: {} tokens", fit.fits, fit.overhead);

// Compare: side-by-side model comparison
let comp = compare(&store, &["gpt-4o", "claude-sonnet-4-20250514"]);
for field in &comp.fields {
    if let Some(winner) = &field.winner {
        println!("{}: winner = {}", field.field, winner);
    }
}
```

## API Overview

| Function | Description |
|----------|-------------|
| [`RouterStore::from_file`] | Load model data from a JSON file |
| [`RouterStore::from_json`] | Parse model data from a string |
| [`RouterStore::global`] | Process-wide singleton (reads from `data/`) |
| [`route`] | Filter, sort, and paginate models |
| [`calculate_cost_for_model`] | Full cost breakdown for token usage |
| [`estimate_request_cost`] | Quick input+output cost estimate |
| [`fallback_chain`] | Scored list of alternative models |
| [`check_context_fit`] | Check if a prompt fits a context window |
| [`find_best_context_model`] | Cheapest model that fits a token count |
| [`compare`] | Side-by-side comparison with winners |

## Contributing

Contributions are welcome. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for
guidelines and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for our community
standards.

## Acknowledgments

This crate is built on top of the
[AI Model Directory](https://github.com/The-Best-Codes/ai-model-directory) by
[The-Best-Codes](https://github.com/The-Best-Codes), which maintains the
comprehensive model dataset and the original TypeScript router package. See
[THANKS.md](THANKS.md) for full attribution.

## License

Licensed under the [Apache License 2.0](LICENSE).
