use crate::store::RouterStore;
use crate::types::*;

struct FeatureProfile {
    has_tool_call: bool,
    has_reasoning: bool,
    has_structured_output: bool,
    has_attachment: bool,
}

fn get_feature_profile(model: &FlatModel) -> FeatureProfile {
    match &model.features {
        Some(f) => FeatureProfile {
            has_tool_call: f.tool_call == Some(true),
            has_reasoning: f.reasoning == Some(true),
            has_structured_output: f.structured_output == Some(true),
            has_attachment: f.attachment == Some(true),
        },
        None => FeatureProfile {
            has_tool_call: false,
            has_reasoning: false,
            has_structured_output: false,
            has_attachment: false,
        },
    }
}

fn features_match(a: &FeatureProfile, b: &FeatureProfile) -> bool {
    a.has_tool_call == b.has_tool_call
        && a.has_reasoning == b.has_reasoning
        && a.has_structured_output == b.has_structured_output
        && a.has_attachment == b.has_attachment
}

fn modality_match(a: &FlatModel, b: &FlatModel) -> bool {
    let a_in: Vec<String> = a
        .modalities
        .as_ref()
        .and_then(|m| m.input.as_ref())
        .map(|v| v.iter().map(|m| format!("{:?}", m)).collect())
        .unwrap_or_default();
    let b_in: Vec<String> = b
        .modalities
        .as_ref()
        .and_then(|m| m.input.as_ref())
        .map(|v| v.iter().map(|m| format!("{:?}", m)).collect())
        .unwrap_or_default();
    let a_out: Vec<String> = a
        .modalities
        .as_ref()
        .and_then(|m| m.output.as_ref())
        .map(|v| v.iter().map(|m| format!("{:?}", m)).collect())
        .unwrap_or_default();
    let b_out: Vec<String> = b
        .modalities
        .as_ref()
        .and_then(|m| m.output.as_ref())
        .map(|v| v.iter().map(|m| format!("{:?}", m)).collect())
        .unwrap_or_default();
    b_in.iter().all(|m| a_in.contains(m)) && b_out.iter().all(|m| a_out.contains(m))
}

pub fn fallback_chain(
    store: &RouterStore,
    model_id: &str,
    options: &FallbackOptions,
) -> Result<FallbackChain, RouterError> {
    let original = store
        .find_model(model_id)
        .ok_or_else(|| RouterError::ModelNotFound(model_id.to_string()))?
        .clone();

    let original_profile = get_feature_profile(&original);
    let original_context = original.limit.as_ref().and_then(|l| l.context).unwrap_or(0) as f64;
    let original_input_price = original.pricing.as_ref().and_then(|p| p.input).unwrap_or(0.0);

    let max_context_diff = options.max_context_difference.unwrap_or(u64::MAX) as f64;
    let max_price_mult = options.max_price_multiplier.unwrap_or(5.0);
    let limit = options.limit.unwrap_or(10);

    let mut scored: Vec<(FlatModel, i32)> = Vec::new();

    for candidate in store.flat_models() {
        if candidate.id == original.id {
            continue;
        }

        let mut score: i32 = 0;
        let candidate_profile = get_feature_profile(candidate);

        if options.match_features.unwrap_or(false) {
            if !features_match(&candidate_profile, &original_profile) {
                continue;
            }
        } else {
            if candidate_profile.has_tool_call && !original_profile.has_tool_call {
                score -= 1;
            }
            if candidate_profile.has_tool_call == original_profile.has_tool_call {
                score += 2;
            }
            if candidate_profile.has_reasoning == original_profile.has_reasoning {
                score += 1;
            }
        }

        if options.match_modalities.unwrap_or(false)
            && !modality_match(candidate, &original)
        {
            continue;
        }

        let candidate_context =
            candidate.limit.as_ref().and_then(|l| l.context).unwrap_or(0) as f64;
        let context_diff = (candidate_context - original_context).abs();
        if context_diff > max_context_diff {
            continue;
        }
        if candidate_context >= original_context {
            score += 2;
        }

        if let Some(candidate_price) = candidate.pricing.as_ref().and_then(|p| p.input) {
            if original_input_price > 0.0 {
                let ratio = candidate_price / original_input_price;
                if ratio > max_price_mult {
                    continue;
                }
                if ratio <= 1.0 {
                    score += 3;
                } else if ratio <= 1.5 {
                    score += 1;
                }
            }
        }

        if candidate.provider == original.provider {
            score += 1;
        }

        scored.push((candidate.clone(), score));
    }

    scored.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(FallbackChain {
        models: scored.into_iter().take(limit).map(|(m, _)| m).collect(),
        original,
    })
}
