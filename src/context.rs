use crate::store::RouterStore;
use crate::types::*;
use rust_decimal::Decimal;
use std::cmp::Ordering;

fn limits(model: &FlatModel) -> (Option<u64>, Option<u64>, Option<u64>) {
    model
        .limit
        .as_ref()
        .map(|limit| (limit.context, limit.input, limit.output))
        .unwrap_or((None, None, None))
}

fn input_price(model: &FlatModel) -> Option<Decimal> {
    model
        .pricing
        .as_ref()
        .and_then(|pricing| pricing.rates.input)
}

fn price_is_compatible(original: &FlatModel, candidate: &FlatModel) -> bool {
    match (input_price(original), input_price(candidate)) {
        (Some(original), Some(candidate)) => original
            .checked_mul(Decimal::from(2u64))
            .is_some_and(|maximum| candidate <= maximum),
        (Some(_), None) => false,
        (None, _) => true,
    }
}

fn compare_optional_price(a: Option<Decimal>, b: Option<Decimal>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn check_context_fit_for_model(
    store: &RouterStore,
    model: &FlatModel,
    input_tokens: u64,
    output_tokens: Option<u64>,
) -> Result<ContextFit, RouterError> {
    let requested_output = output_tokens.unwrap_or(0);
    let requested_tokens = input_tokens
        .checked_add(requested_output)
        .ok_or(RouterError::TokenOverflow)?;
    let (context_limit, input_limit, output_limit) = limits(model);
    let available_context = context_limit.unwrap_or(0);
    let context_fits = context_limit.is_some_and(|limit| requested_tokens <= limit);
    let input_fits = input_limit.is_none_or(|limit| input_tokens <= limit);
    let output_fits = output_limit.is_none_or(|limit| requested_output <= limit);
    let fits = context_fits && input_fits && output_fits;

    let mut better_alternatives: Vec<FlatModel> = store
        .flat_models()
        .iter()
        .filter(|candidate| candidate.key() != model.key())
        .filter(|candidate| {
            let (context, input, output) = limits(candidate);
            context.is_some_and(|limit| requested_tokens <= limit)
                && context_limit.is_none_or(|current| context.is_some_and(|limit| limit > current))
                && input.is_none_or(|limit| input_tokens <= limit)
                && output.is_none_or(|limit| requested_output <= limit)
        })
        .filter(|candidate| price_is_compatible(model, candidate))
        .cloned()
        .collect();

    better_alternatives.sort_by(|a, b| {
        let (a_context, _, _) = limits(a);
        let (b_context, _, _) = limits(b);
        a_context
            .cmp(&b_context)
            .then_with(|| compare_optional_price(input_price(a), input_price(b)))
            .then_with(|| a.provider.cmp(&b.provider))
            .then_with(|| a.id.cmp(&b.id))
    });
    better_alternatives.truncate(5);

    Ok(ContextFit {
        fits,
        context_fits,
        input_fits,
        output_fits,
        model: model.clone(),
        available_context,
        requested_tokens,
        overhead: i128::from(available_context) - i128::from(requested_tokens),
        should_compact: output_fits && ((!context_fits && context_limit.is_some()) || !input_fits),
        better_alternatives,
    })
}

/// Check whether token counts fit a uniquely identified bare model ID.
///
/// Returns AmbiguousModel when more than one provider offers the model ID.
/// Use check_context_fit_for_provider to select an offering explicitly.
/// Token-count addition is checked and can return TokenOverflow.
pub fn check_context_fit(
    store: &RouterStore,
    model_id: &str,
    input_tokens: u64,
    output_tokens: Option<u64>,
) -> Result<ContextFit, RouterError> {
    let model = store.resolve_model(model_id)?;
    check_context_fit_for_model(store, model, input_tokens, output_tokens)
}

/// Check whether token counts fit one provider-qualified model offering.
pub fn check_context_fit_for_provider(
    store: &RouterStore,
    provider: &str,
    model_id: &str,
    input_tokens: u64,
    output_tokens: Option<u64>,
) -> Result<ContextFit, RouterError> {
    let model = store
        .find_model_in(provider, model_id)
        .ok_or_else(|| RouterError::ModelNotFound(format!("{provider}/{model_id}")))?;
    check_context_fit_for_model(store, model, input_tokens, output_tokens)
}

/// Find the cheapest model that fits a context token count.
///
/// Models with unknown input prices remain eligible but sort after all models
/// with known prices. Provider and model ID provide deterministic tie-breaks.
pub fn find_best_context_model(
    store: &RouterStore,
    tokens: u64,
    provider: Option<&str>,
) -> Option<FlatModel> {
    let mut candidates: Vec<&FlatModel> = store
        .flat_models()
        .iter()
        .filter(|model| {
            model.limit.as_ref().is_some_and(|limit| {
                limit.context.is_some_and(|context| context >= tokens)
                    && limit.input.is_none_or(|input| input >= tokens)
            })
        })
        .filter(|model| {
            provider.is_none_or(|provider| model.provider.eq_ignore_ascii_case(provider))
        })
        .collect();

    candidates.sort_by(|a, b| {
        compare_optional_price(input_price(a), input_price(b))
            .then_with(|| a.provider.cmp(&b.provider))
            .then_with(|| a.id.cmp(&b.id))
    });

    candidates.first().map(|model| (*model).clone())
}
