use crate::store::RouterStore;
use crate::types::*;

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
