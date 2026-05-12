//! AI Model Directory Router
//!
//! A Rust library for routing, filtering, cost calculation, fallback chain
//! generation, and context window management for over 7,000 AI models across
//! 50+ providers.
//!
//! # Quick Start
//!
//! ```no_run
//! use ai_model_directory_router::{RouterStore, route, RouteQuery};
//!
//! let store = RouterStore::from_file(std::path::Path::new("data/all.min.json")).unwrap();
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
//! ```
//!
//! # Modules
//!
//! - [`store`] loads and flattens model data into [`RouterStore`]
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

pub use compare::compare;
pub use context::{check_context_fit, find_best_context_model};
pub use cost::{calculate_cost_for_model, estimate_request_cost};
pub use fallback::fallback_chain;
pub use overlay::{
    apply_overlay, load_models_dev_from_file, parse_models_dev, ModelsDevCost, ModelsDevIndex,
    ModelsDevLimit, ModelsDevModalities, ModelsDevModel, ModelsDevProvider, OverlayMode,
    OverlayReport,
};
pub use router::route;
pub use store::RouterStore;
pub use types::*;

#[cfg(test)]
mod tests;
