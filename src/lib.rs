//! AI Model Directory Router
//!
//! Provider-aware AI model routing, filtering, exact cost calculation,
//! fallback selection, and context-window management. The default `bundled`
//! feature ships a dated models.dev provider catalog for offline use.
//!
//! # Quick Start
//!
//! ```
//! use ai_model_directory_router::{route, RouteQuery, RouterStore};
//!
//! # #[cfg(feature = "bundled")]
//! # {
//! let store = RouterStore::bundled().unwrap();
//!
//! let query = RouteQuery {
//!     provider: Some("openai".to_string()),
//!     min_context: Some(128_000),
//!     ..RouteQuery::default()
//! };
//!
//! let result = route(&store, &query);
//! for model in &result.models {
//!     println!("{} (via {})", model.id, model.provider);
//! }
//! # }
//! ```
//!
//! # Modules
//!
//! - [`store`] loads bundled, local, inline, or live provider catalogs
//! - [`cost`] calculates per-token costs using exact decimal arithmetic
//! - [`router`] filters, sorts, and paginates models by provider, price, context, features, and modalities
//! - [`fallback`] generates scored alternative model chains
//! - [`context`] checks if prompts fit within context windows
//! - [`compare()`] produces field-by-field model comparisons with winners

pub mod compare;
pub mod context;
pub mod cost;
pub mod fallback;
pub mod overlay;
pub mod router;
pub mod store;
pub mod types;

pub use compare::{compare, compare_models};
pub use context::{check_context_fit, check_context_fit_for_provider, find_best_context_model};
pub use cost::{calculate_cost_for_model, estimate_request_cost};
pub use fallback::{fallback_chain, fallback_chain_for_provider};
pub use overlay::{
    apply_overlay, load_models_dev_from_file, parse_models_dev, ModelsDevCost, ModelsDevIndex,
    ModelsDevLimit, ModelsDevModalities, ModelsDevModel, ModelsDevProvider, OverlayMode,
    OverlayReport,
};
pub use router::route;
#[cfg(feature = "online")]
pub use store::MODELS_DEV_BODY_LIMIT;
pub use store::{RouterStore, MODELS_DEV_API_URL};
#[cfg(feature = "bundled")]
pub use store::{
    BUNDLED_MODELS_DEV_BYTE_COUNT, BUNDLED_MODELS_DEV_ETAG, BUNDLED_MODELS_DEV_MODEL_COUNT,
    BUNDLED_MODELS_DEV_PROVIDER_COUNT, BUNDLED_MODELS_DEV_RETRIEVED_AT, BUNDLED_MODELS_DEV_SHA256,
    BUNDLED_MODELS_DEV_SOURCE_REVISION,
};
pub use types::*;

#[cfg(test)]
mod tests;
