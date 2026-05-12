use crate::store::RouterStore;
use crate::types::*;
use std::collections::HashMap;

fn string_field(
    models: &[FlatModel],
    name: &str,
    extract: impl Fn(&FlatModel) -> Option<String>,
) -> ComparisonField {
    let mut values = HashMap::new();
    for m in models {
        values.insert(m.id.clone(), FieldValue::Text(extract(m)));
    }
    ComparisonField {
        field: name.to_string(),
        values,
        winner: None,
    }
}

fn number_field(
    models: &[FlatModel],
    name: &str,
    extract: impl Fn(&FlatModel) -> Option<f64>,
    higher_is_better: bool,
) -> ComparisonField {
    let mut values = HashMap::new();
    let mut best_id: Option<String> = None;
    let mut best_val: Option<f64> = None;

    for m in models {
        let v = extract(m);
        values.insert(m.id.clone(), FieldValue::Number(v));
        if let Some(val) = v {
            match best_val {
                None => {
                    best_val = Some(val);
                    best_id = Some(m.id.clone());
                }
                Some(bv) => {
                    let is_better = if higher_is_better { val > bv } else { val < bv };
                    if is_better {
                        best_val = Some(val);
                        best_id = Some(m.id.clone());
                    }
                }
            }
        }
    }

    ComparisonField {
        field: name.to_string(),
        values,
        winner: best_id,
    }
}

fn bool_field(
    models: &[FlatModel],
    name: &str,
    extract: impl Fn(&FlatModel) -> Option<bool>,
) -> ComparisonField {
    let mut values = HashMap::new();
    for m in models {
        values.insert(m.id.clone(), FieldValue::Bool(extract(m)));
    }
    ComparisonField {
        field: name.to_string(),
        values,
        winner: None,
    }
}

/// Compare two or more models side by side across all key dimensions.
///
/// Returns a [`ModelComparison`] with fields for context, pricing, features,
/// modalities, and more. Numeric fields include a `winner` pointing to the
/// best model ID.
///
/// Unknown model IDs are silently skipped. If fewer than two valid models
/// are found, the `fields` vector will be empty.
///
/// # Example
///
/// ```no_run
/// use ai_model_directory_router::{RouterStore, compare};
/// use std::path::Path;
///
/// let store = RouterStore::from_file(Path::new("data/all.min.json")).unwrap();
/// let comp = compare(&store, &["gpt-4o", "claude-sonnet-4-20250514"]);
/// for field in &comp.fields {
///     println!("{}: winner={:?}", field.field, field.winner);
/// }
/// ```
pub fn compare(store: &RouterStore, model_ids: &[&str]) -> ModelComparison {
    let models: Vec<FlatModel> = model_ids
        .iter()
        .filter_map(|id| store.find_model(id).cloned())
        .collect();

    let mut fields: Vec<ComparisonField> = Vec::new();

    if models.len() < 2 {
        return ModelComparison { models, fields };
    }

    fields.push(string_field(&models, "provider", |m| Some(m.provider.clone())));
    fields.push(number_field(&models, "context", |m| m.limit.as_ref().and_then(|l| l.context).map(|v| v as f64), true));
    fields.push(number_field(&models, "input_price", |m| m.pricing.as_ref().and_then(|p| p.input), false));
    fields.push(number_field(&models, "output_price", |m| m.pricing.as_ref().and_then(|p| p.output), false));
    fields.push(number_field(&models, "reasoning_price", |m| m.pricing.as_ref().and_then(|p| p.reasoning), false));
    fields.push(number_field(&models, "cache_read_price", |m| m.pricing.as_ref().and_then(|p| p.cache_read), false));
    fields.push(number_field(&models, "cache_write_price", |m| m.pricing.as_ref().and_then(|p| p.cache_write), false));
    fields.push(number_field(&models, "output_limit", |m| m.limit.as_ref().and_then(|l| l.output).map(|v| v as f64), true));

    fields.push(string_field(&models, "input_modalities", |m| {
        m.modalities.as_ref().and_then(|mod_| mod_.input.as_ref()).map(|v| {
            v.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>().join(", ")
        })
    }));
    fields.push(string_field(&models, "output_modalities", |m| {
        m.modalities.as_ref().and_then(|mod_| mod_.output.as_ref()).map(|v| {
            v.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>().join(", ")
        })
    }));

    fields.push(bool_field(&models, "tool_call", |m| m.features.as_ref().and_then(|f| f.tool_call)));
    fields.push(bool_field(&models, "reasoning", |m| m.features.as_ref().and_then(|f| f.reasoning)));
    fields.push(bool_field(&models, "structured_output", |m| m.features.as_ref().and_then(|f| f.structured_output)));
    fields.push(bool_field(&models, "attachment", |m| m.features.as_ref().and_then(|f| f.attachment)));
    fields.push(bool_field(&models, "open_weights", |m| m.open_weights));

    ModelComparison { models, fields }
}
