use crate::store::RouterStore;
use crate::types::*;
use rust_decimal::Decimal;
use std::collections::BTreeSet;

fn string_field(
    models: &[FlatModel],
    name: &str,
    extract: impl Fn(&FlatModel) -> Option<String>,
) -> ComparisonField {
    let values = models
        .iter()
        .map(|model| (model.key(), FieldValue::Text(extract(model))))
        .collect();
    ComparisonField {
        field: name.to_string(),
        values,
        winners: Vec::new(),
    }
}

fn integer_field(
    models: &[FlatModel],
    name: &str,
    extract: impl Fn(&FlatModel) -> Option<u64>,
    higher_is_better: bool,
) -> ComparisonField {
    let extracted: Vec<(ModelKey, Option<u64>)> = models
        .iter()
        .map(|model| (model.key(), extract(model)))
        .collect();
    let best = extracted
        .iter()
        .filter_map(|(_, value)| *value)
        .reduce(|best, value| {
            if (higher_is_better && value > best) || (!higher_is_better && value < best) {
                value
            } else {
                best
            }
        });
    let mut winners: Vec<ModelKey> = best
        .map(|best| {
            extracted
                .iter()
                .filter(|(_, value)| *value == Some(best))
                .map(|(key, _)| key.clone())
                .collect()
        })
        .unwrap_or_default();
    winners.sort();
    let values = extracted
        .into_iter()
        .map(|(key, value)| (key, FieldValue::Integer(value)))
        .collect();

    ComparisonField {
        field: name.to_string(),
        values,
        winners,
    }
}

fn decimal_field(
    models: &[FlatModel],
    name: &str,
    extract: impl Fn(&FlatModel) -> Option<Decimal>,
    higher_is_better: bool,
) -> ComparisonField {
    let extracted: Vec<(ModelKey, Option<Decimal>)> = models
        .iter()
        .map(|model| (model.key(), extract(model)))
        .collect();
    let best = extracted
        .iter()
        .filter_map(|(_, value)| *value)
        .reduce(|best, value| {
            if (higher_is_better && value > best) || (!higher_is_better && value < best) {
                value
            } else {
                best
            }
        });
    let mut winners: Vec<ModelKey> = best
        .map(|best| {
            extracted
                .iter()
                .filter(|(_, value)| *value == Some(best))
                .map(|(key, _)| key.clone())
                .collect()
        })
        .unwrap_or_default();
    winners.sort();
    let values = extracted
        .into_iter()
        .map(|(key, value)| (key, FieldValue::Decimal(value)))
        .collect();

    ComparisonField {
        field: name.to_string(),
        values,
        winners,
    }
}

fn bool_field(
    models: &[FlatModel],
    name: &str,
    extract: impl Fn(&FlatModel) -> Option<bool>,
) -> ComparisonField {
    let values = models
        .iter()
        .map(|model| (model.key(), FieldValue::Bool(extract(model))))
        .collect();
    ComparisonField {
        field: name.to_string(),
        values,
        winners: Vec::new(),
    }
}

fn modality_list(modalities: Option<&Vec<ModelModality>>) -> Option<String> {
    modalities.map(|values| {
        values
            .iter()
            .map(|value| format!("{value:?}").to_lowercase())
            .collect::<Vec<_>>()
            .join(", ")
    })
}

fn build_comparison(models: Vec<FlatModel>) -> ModelComparison {
    let mut fields = Vec::new();
    if models.len() < 2 {
        return ModelComparison { models, fields };
    }

    fields.push(string_field(&models, "provider", |model| {
        Some(model.provider.clone())
    }));
    fields.push(string_field(&models, "name", |model| model.name.clone()));
    fields.push(string_field(&models, "family", |model| {
        model.family.clone()
    }));
    fields.push(string_field(&models, "status", |model| {
        model
            .status
            .as_ref()
            .map(|status| format!("{status:?}").to_lowercase())
    }));
    fields.push(integer_field(
        &models,
        "context",
        |model| model.limit.as_ref().and_then(|limit| limit.context),
        true,
    ));
    fields.push(integer_field(
        &models,
        "input_limit",
        |model| model.limit.as_ref().and_then(|limit| limit.input),
        true,
    ));
    fields.push(integer_field(
        &models,
        "output_limit",
        |model| model.limit.as_ref().and_then(|limit| limit.output),
        true,
    ));
    fields.push(decimal_field(
        &models,
        "input_price",
        |model| {
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.input)
        },
        false,
    ));
    fields.push(decimal_field(
        &models,
        "output_price",
        |model| {
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.output)
        },
        false,
    ));
    fields.push(decimal_field(
        &models,
        "reasoning_price",
        |model| {
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.reasoning)
        },
        false,
    ));
    fields.push(decimal_field(
        &models,
        "cache_read_price",
        |model| {
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.cache_read)
        },
        false,
    ));
    fields.push(decimal_field(
        &models,
        "cache_write_price",
        |model| {
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.cache_write)
        },
        false,
    ));
    fields.push(decimal_field(
        &models,
        "input_audio_price",
        |model| {
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.input_audio)
        },
        false,
    ));
    fields.push(decimal_field(
        &models,
        "output_audio_price",
        |model| {
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.output_audio)
        },
        false,
    ));
    fields.push(string_field(&models, "input_modalities", |model| {
        modality_list(
            model
                .modalities
                .as_ref()
                .and_then(|modalities| modalities.input.as_ref()),
        )
    }));
    fields.push(string_field(&models, "output_modalities", |model| {
        modality_list(
            model
                .modalities
                .as_ref()
                .and_then(|modalities| modalities.output.as_ref()),
        )
    }));
    fields.push(bool_field(&models, "tool_call", |model| {
        model
            .features
            .as_ref()
            .and_then(|features| features.tool_call)
    }));
    fields.push(bool_field(&models, "reasoning", |model| {
        model
            .features
            .as_ref()
            .and_then(|features| features.reasoning)
    }));
    fields.push(bool_field(&models, "structured_output", |model| {
        model
            .features
            .as_ref()
            .and_then(|features| features.structured_output)
    }));
    fields.push(bool_field(&models, "attachment", |model| {
        model
            .features
            .as_ref()
            .and_then(|features| features.attachment)
    }));
    fields.push(bool_field(&models, "open_weights", |model| {
        model.open_weights
    }));

    ModelComparison { models, fields }
}

fn unique_models(models: impl IntoIterator<Item = FlatModel>) -> Vec<FlatModel> {
    let mut seen = BTreeSet::new();
    models
        .into_iter()
        .filter(|model| seen.insert(model.key()))
        .collect()
}

/// Compare unique bare model IDs across limits, prices, and capabilities.
///
/// Missing and ambiguous IDs are returned as errors instead of being silently
/// skipped. Numeric fields retain exact integer or decimal values, and every
/// model tied for the best known value is included in the field's winners.
pub fn compare(store: &RouterStore, model_ids: &[&str]) -> Result<ModelComparison, RouterError> {
    let models = model_ids
        .iter()
        .map(|model_id| store.resolve_model(model_id).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(build_comparison(unique_models(models)))
}

/// Compare provider-qualified model offerings.
///
/// Values are keyed by ModelKey, so offerings that share a bare ID never
/// overwrite one another.
pub fn compare_models(
    store: &RouterStore,
    model_keys: &[ModelKey],
) -> Result<ModelComparison, RouterError> {
    let models = model_keys
        .iter()
        .map(|key| {
            store
                .find_model_in(&key.provider, &key.id)
                .cloned()
                .ok_or_else(|| RouterError::ModelNotFound(format!("{}/{}", key.provider, key.id)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(build_comparison(unique_models(models)))
}
