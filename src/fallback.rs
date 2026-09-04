use crate::store::RouterStore;
use crate::types::*;
use rust_decimal::Decimal;

#[derive(Clone, Copy)]
struct FeatureProfile {
    tool_call: Option<bool>,
    reasoning: Option<bool>,
    structured_output: Option<bool>,
    attachment: Option<bool>,
    temperature: Option<bool>,
}

fn feature_profile(model: &FlatModel) -> FeatureProfile {
    model
        .features
        .as_ref()
        .map(|features| FeatureProfile {
            tool_call: features.tool_call,
            reasoning: features.reasoning,
            structured_output: features.structured_output,
            attachment: features.attachment,
            temperature: features.temperature,
        })
        .unwrap_or(FeatureProfile {
            tool_call: None,
            reasoning: None,
            structured_output: None,
            attachment: None,
            temperature: None,
        })
}

fn features_match(a: FeatureProfile, b: FeatureProfile) -> bool {
    a.tool_call == b.tool_call
        && a.reasoning == b.reasoning
        && a.structured_output == b.structured_output
        && a.attachment == b.attachment
        && a.temperature == b.temperature
}

fn contains_modalities(
    available: Option<&[ModelModality]>,
    required: Option<&[ModelModality]>,
) -> bool {
    match required {
        None => true,
        Some([]) => true,
        Some(required) => available
            .is_some_and(|available| required.iter().all(|modality| available.contains(modality))),
    }
}

fn modalities_match(candidate: &FlatModel, original: &FlatModel) -> bool {
    let candidate = candidate.modalities.as_ref();
    let original = original.modalities.as_ref();
    contains_modalities(
        candidate.and_then(|modalities| modalities.input.as_deref()),
        original.and_then(|modalities| modalities.input.as_deref()),
    ) && contains_modalities(
        candidate.and_then(|modalities| modalities.output.as_deref()),
        original.and_then(|modalities| modalities.output.as_deref()),
    )
}

fn input_price(model: &FlatModel) -> Option<Decimal> {
    model
        .pricing
        .as_ref()
        .and_then(|pricing| pricing.rates.input)
}

fn fallback_chain_for_model(
    store: &RouterStore,
    original: &FlatModel,
    options: &FallbackOptions,
) -> FallbackChain {
    let original_profile = feature_profile(original);
    let original_context = original.limit.as_ref().and_then(|limit| limit.context);
    let original_input_price = input_price(original);
    let max_context_difference = options.max_context_difference.unwrap_or(u64::MAX);
    let max_price_multiplier = options.max_price_multiplier.unwrap_or(Decimal::from(5u64));
    let limit = options.limit.unwrap_or(10);
    let mut scored: Vec<(FlatModel, i32)> = Vec::new();

    for candidate in store.flat_models() {
        if candidate.key() == original.key() {
            continue;
        }

        let mut score = 0;
        let candidate_profile = feature_profile(candidate);
        if options.match_features.unwrap_or(false) {
            if !features_match(candidate_profile, original_profile) {
                continue;
            }
        } else {
            if candidate_profile.tool_call == original_profile.tool_call {
                score += 2;
            } else if candidate_profile.tool_call == Some(true)
                && original_profile.tool_call != Some(true)
            {
                score -= 1;
            }
            if candidate_profile.reasoning == original_profile.reasoning {
                score += 1;
            }
            if candidate_profile.structured_output == original_profile.structured_output {
                score += 1;
            }
            if candidate_profile.attachment == original_profile.attachment {
                score += 1;
            }
            if candidate_profile.temperature == original_profile.temperature {
                score += 1;
            }
        }

        if options.match_modalities.unwrap_or(false) && !modalities_match(candidate, original) {
            continue;
        }

        let candidate_context = candidate.limit.as_ref().and_then(|limit| limit.context);
        match (original_context, candidate_context) {
            (Some(original_context), Some(candidate_context)) => {
                if candidate_context.abs_diff(original_context) > max_context_difference {
                    continue;
                }
                if candidate_context >= original_context {
                    score += 2;
                }
            }
            (Some(_), None) if options.max_context_difference.is_some() => continue,
            _ => {}
        }

        match (original_input_price, input_price(candidate)) {
            (Some(original_price), Some(candidate_price)) => {
                let Some(maximum_price) = original_price.checked_mul(max_price_multiplier) else {
                    continue;
                };
                if candidate_price > maximum_price {
                    continue;
                }
                if candidate_price <= original_price {
                    score += 3;
                } else if candidate_price
                    .checked_mul(Decimal::from(2u64))
                    .zip(original_price.checked_mul(Decimal::from(3u64)))
                    .is_some_and(|(candidate, original)| candidate <= original)
                {
                    score += 1;
                }
            }
            (Some(_), None) => continue,
            (None, _) => {}
        }

        if candidate.provider == original.provider {
            score += 1;
        }

        scored.push((candidate.clone(), score));
    }

    scored.sort_by(|(a_model, a_score), (b_model, b_score)| {
        b_score
            .cmp(a_score)
            .then_with(|| a_model.provider.cmp(&b_model.provider))
            .then_with(|| a_model.id.cmp(&b_model.id))
    });

    FallbackChain {
        models: scored
            .into_iter()
            .take(limit)
            .map(|(model, _)| model)
            .collect(),
        original: original.clone(),
    }
}

/// Generate a deterministic fallback chain for a unique bare model ID.
///
/// Returns AmbiguousModel if multiple providers offer the ID. Use
/// fallback_chain_for_provider to select an offering explicitly.
pub fn fallback_chain(
    store: &RouterStore,
    model_id: &str,
    options: &FallbackOptions,
) -> Result<FallbackChain, RouterError> {
    let original = store.resolve_model(model_id)?;
    Ok(fallback_chain_for_model(store, original, options))
}

/// Generate a fallback chain for one provider-qualified model offering.
pub fn fallback_chain_for_provider(
    store: &RouterStore,
    provider: &str,
    model_id: &str,
    options: &FallbackOptions,
) -> Result<FallbackChain, RouterError> {
    let original = store
        .find_model_in(provider, model_id)
        .ok_or_else(|| RouterError::ModelNotFound(format!("{provider}/{model_id}")))?;
    Ok(fallback_chain_for_model(store, original, options))
}
