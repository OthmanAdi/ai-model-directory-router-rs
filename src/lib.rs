pub mod compare;
pub mod context;
pub mod cost;
pub mod fallback;
pub mod router;
pub mod store;
pub mod types;

pub use compare::compare;
pub use context::{check_context_fit, find_best_context_model};
pub use cost::{calculate_cost_for_model, estimate_request_cost};
pub use fallback::fallback_chain;
pub use router::route;
pub use store::RouterStore;
pub use types::*;

#[cfg(test)]
mod tests;
