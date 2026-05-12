use crate::store::RouterStore;
use crate::types::*;

/// Check whether a prompt fits within a model's context window.
///
/// Returns a [`ContextFit`] describing the fit, the available context,
/// requested tokens, and up to 5 better alternatives sorted by context size.
///
/// # Errors
///
/// Returns [`RouterError::ModelNotFound`] if the model ID does not exist.
///
/// # Example
///
/// ```no_run
/// use ai_model_directory_router::{RouterStore, check_context_fit};
/// use std::path::Path;
///
/// let store = RouterStore::from_file(Path::new("data/all.min.json")).unwrap();
/// let fit = check_context_fit(&store, "gpt-4o", 50_000, Some(10_000)).unwrap();
/// if fit.fits {
///     println!("Fits! {} tokens of overhead", fit.overhead);
/// } else {
///     println!("Does not fit. Alternatives:");
///     for alt in &fit.better_alternatives {
///         println!("  {} ({} context)", alt.id,
///             alt.limit.as_ref().and_then(|l| l.context).unwrap_or(0));
///     }
/// }
/// ```
pub fn check_context_fit(
    store: &RouterStore,
    model_id: &str,
    input_tokens: u64,
    output_tokens: Option<u64>,
) -> Result<ContextFit, RouterError> {
    let model = store
        .find_model(model_id)
        .ok_or_else(|| RouterError::ModelNotFound(model_id.to_string()))?
        .clone();

    let available_context = model.limit.as_ref().and_then(|l| l.context).unwrap_or(0);
    let requested_tokens = input_tokens + output_tokens.unwrap_or(0);
    let overhead = available_context as i64 - requested_tokens as i64;
    let fits = overhead >= 0;

    let alternatives: Vec<FlatModel> = store
        .flat_models()
        .iter()
        .filter(|m| m.id != model.id)
        .filter(|m| {
            m.limit
                .as_ref()
                .and_then(|l| l.context)
                .unwrap_or(0)
                > available_context
        })
        .filter(|m| {
            let m_price = m.pricing.as_ref().and_then(|p| p.input).unwrap_or(0.0);
            let orig_price = model.pricing.as_ref().and_then(|p| p.input).unwrap_or(0.0);
            if orig_price > 0.0 {
                m_price <= orig_price * 2.0
            } else {
                true
            }
        })
        .cloned()
        .collect();

    let mut sorted_alternatives = alternatives;
    sorted_alternatives.sort_by(|a, b| {
        let ca = a.limit.as_ref().and_then(|l| l.context).unwrap_or(0);
        let cb = b.limit.as_ref().and_then(|l| l.context).unwrap_or(0);
        ca.cmp(&cb)
    });
    sorted_alternatives.truncate(5);

    Ok(ContextFit {
        fits,
        model,
        available_context,
        requested_tokens,
        overhead,
        should_compact: !fits && available_context > 0,
        better_alternatives: sorted_alternatives,
    })
}

/// Find the cheapest model that fits a given token count.
///
/// Optionally filter by provider. Returns the model with the lowest input
/// price per million tokens that has a context window large enough for
/// `tokens`.
///
/// Returns `None` if no model has a large enough context window.
pub fn find_best_context_model(
    store: &RouterStore,
    tokens: u64,
    provider: Option<&str>,
) -> Option<FlatModel> {
    let mut candidates: Vec<&FlatModel> = store
        .flat_models()
        .iter()
        .filter(|m| m.limit.as_ref().and_then(|l| l.context).unwrap_or(0) >= tokens)
        .filter(|m| match provider {
            Some(p) => m.provider.to_lowercase() == p.to_lowercase(),
            None => true,
        })
        .collect();

    candidates.sort_by(|a, b| {
        let pa = a.pricing.as_ref().and_then(|p| p.input).unwrap_or(f64::MAX);
        let pb = b.pricing.as_ref().and_then(|p| p.input).unwrap_or(f64::MAX);
        pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
    });

    candidates.first().cloned().cloned()
}
