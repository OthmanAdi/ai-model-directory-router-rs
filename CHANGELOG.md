# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-09-04

### Added

- Added a default `bundled` feature and `RouterStore::bundled()` for a
  consumer-visible models.dev snapshot.
- Added `from_models_dev_json`, `from_models_dev_file`, and the optional
  `online` feature with `from_models_dev_live` for the fixed public
  `https://models.dev/api.json` endpoint.
- Added `ModelKey`, provider-qualified lookup, all-provider lookup by model ID,
  and unique-only bare-ID resolution.
- Added catalog provenance, provider metadata, family, lifecycle status,
  reasoning controls, interleaved reasoning, experimental fields, PDF modality,
  per-axis limits, and typed pricing tiers.
- Added provider-qualified context and fallback entry points.
- Added CI coverage on Linux, Windows, and macOS, a Rust 1.85 MSRV gate,
  feature-combination checks, and monthly Dependabot updates.
- Added a security policy and runnable bundled and online catalog examples.

### Changed

- Changed prices, price filters, cost results, and fallback price multipliers
  from binary floating-point values to `rust_decimal::Decimal`.
- Changed cost calculation to return `Result<CostBreakdown, RouterError>` and
  select context-based rates from `context_over_200k` and `tiers`.
- Changed bare model resolution to succeed only for IDs offered by exactly one
  provider. Ambiguous IDs now report their matching providers.
- Changed routing to deterministic provider and model tie-breaking, exact
  status selection, explicit `false` feature matching, and numeric offset
  pagination.
- Changed context results to report total-context, input, and output fit
  independently.
- Raised the minimum supported Rust version from 1.70 to 1.85.

### Fixed

- Fixed nondeterministic results when multiple providers publish the same bare
  model ID.
- Fixed prices losing decimal precision during JSON deserialization.
- Fixed missing price components being treated as free for nonzero usage.
- Fixed descending price sorts placing unknown prices before known prices.
- Fixed feature filters ignoring requested `false` values.
- Fixed unchecked token and pagination arithmetic.
- Fixed context checks that considered only the total context window while
  ignoring provider input and output limits.
- Fixed fallback generation excluding the same model ID from other providers.
- Removed the stale TypeScript companion link and claims of API parity.

### Breaking

- `ModelPricing` now exposes exact base rates through `rates` and separate
  context-dependent rates through `context_over_200k` and `tiers`.
- Public price and comparison value types now use `Decimal` where the value is
  monetary.
- Model cost calculation now returns explicit errors for missing nonzero price
  components and token overflow.
- Bare-ID operations may return `RouterError::AmbiguousModel`. Callers that know
  the provider should use provider-qualified APIs.
- `ContextFit` now requires all three limit checks to pass and exposes each
  result separately.
- The declared MSRV is now Rust 1.85.

### Data

- Replaced the May 2026 models.dev development snapshot with the public
  `api.json` snapshot retrieved at `2026-09-04T07:53:34Z`.
- The bundled snapshot contains 4,465,310 bytes, 213 providers, and 7,526
  provider-model offerings. Its HTTP ETag is
  `"c68bb1a66b2260432a30862fa0c759ce"`.
- Snapshot SHA-256:
  `c68bb1a66b2260432a30862fa0c759ce08bde20322e2c949dee9c615c3cc4c8f`.
- The models.dev `dev` revision observed at retrieval was
  `ba816a069564b9dbf7c61cc117a79301b3f9ecdc`.
- Included the models.dev MIT notice in
  `THIRD_PARTY_LICENSES/models.dev-MIT.txt`.
- Documented that the bundled catalog is dated metadata, not an authority for
  provider billing, regional access, account entitlements, or availability.

## [0.2.0] - 2026-05-12

- Added models.dev overlay enrichment with fill-only and prefer-overlay modes.
- Added overlay impact reporting and tests.

## [0.1.0] - 2026-05-12

- Initial release with routing, cost estimation, fallback chains, context
  checks, comparisons, and local AI Model Directory JSON loading.
