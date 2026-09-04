use crate::store::RouterStore;
use crate::types::*;
use std::cmp::Ordering;

fn matches_input_modality(model: &FlatModel, modalities: &[ModelModality]) -> bool {
    if modalities.is_empty() {
        return true;
    }
    model
        .modalities
        .as_ref()
        .and_then(|m| m.input.as_ref())
        .map(|available| {
            modalities
                .iter()
                .all(|required| available.contains(required))
        })
        .unwrap_or(false)
}

fn matches_output_modality(model: &FlatModel, modalities: &[ModelModality]) -> bool {
    if modalities.is_empty() {
        return true;
    }
    model
        .modalities
        .as_ref()
        .and_then(|m| m.output.as_ref())
        .map(|available| {
            modalities
                .iter()
                .all(|required| available.contains(required))
        })
        .unwrap_or(false)
}

fn matches_features(model: &FlatModel, requested: &ModelFeatures) -> bool {
    requested.attachment.is_none_or(|value| {
        model.features.as_ref().and_then(|actual| actual.attachment) == Some(value)
    }) && requested.reasoning.is_none_or(|value| {
        model.features.as_ref().and_then(|actual| actual.reasoning) == Some(value)
    }) && requested.tool_call.is_none_or(|value| {
        model.features.as_ref().and_then(|actual| actual.tool_call) == Some(value)
    }) && requested.structured_output.is_none_or(|value| {
        model
            .features
            .as_ref()
            .and_then(|actual| actual.structured_output)
            == Some(value)
    }) && requested.temperature.is_none_or(|value| {
        model
            .features
            .as_ref()
            .and_then(|actual| actual.temperature)
            == Some(value)
    })
}

fn identity_cmp(a: &FlatModel, b: &FlatModel) -> Ordering {
    a.provider.cmp(&b.provider).then_with(|| a.id.cmp(&b.id))
}

fn compare_known_first<T: Ord>(a: Option<T>, b: Option<T>, descending: bool) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) if descending => b.cmp(&a),
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Route models through configurable filters, sorting, and pagination.
///
/// Every result is deterministic. Missing numeric values sort after known
/// values in both ascending and descending order, and ties are resolved by
/// provider and model ID. Feature filters distinguish `false` from unknown.
///
/// Pagination uses saturating arithmetic. A zero limit always returns an empty
/// page with `has_more` set to `false`.
pub fn route(store: &RouterStore, query: &RouteQuery) -> RouteResult {
    let mut models: Vec<FlatModel> = store.flat_models().to_vec();

    if let Some(provider) = &query.provider {
        models.retain(|model| model.provider.eq_ignore_ascii_case(provider));
    }

    if let Some(model_id) = &query.model_id {
        models.retain(|model| model.id == *model_id);
    }

    if let Some(family) = &query.family {
        models.retain(|model| {
            model
                .family
                .as_deref()
                .is_some_and(|actual| actual.eq_ignore_ascii_case(family))
        });
    }

    if let Some(status) = &query.status {
        models.retain(|model| model.status.as_ref() == Some(status));
    }

    if let Some(modalities) = &query.input_modalities {
        models.retain(|model| matches_input_modality(model, modalities));
    }

    if let Some(modalities) = &query.output_modalities {
        models.retain(|model| matches_output_modality(model, modalities));
    }

    if let Some(features) = &query.features {
        models.retain(|model| matches_features(model, features));
    }

    if let Some(minimum) = query.min_context {
        models.retain(|model| {
            model
                .limit
                .as_ref()
                .and_then(|limit| limit.context)
                .is_some_and(|context| context >= minimum)
        });
    }

    if let Some(maximum) = query.max_input_price {
        models.retain(|model| {
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.input)
                .is_some_and(|price| price <= maximum)
        });
    }

    if let Some(maximum) = query.max_output_price {
        models.retain(|model| {
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.output)
                .is_some_and(|price| price <= maximum)
        });
    }

    if let Some(open) = query.open_weights {
        models.retain(|model| model.open_weights == Some(open));
    }

    let descending = matches!(query.order, Some(SortOrder::Desc));
    models.sort_by(|a, b| {
        let Some(sort_field) = query.sort.as_ref() else {
            return identity_cmp(a, b);
        };
        let primary = match sort_field {
            SortField::Id if descending => b.id.cmp(&a.id),
            SortField::Id => a.id.cmp(&b.id),
            SortField::Context => compare_known_first(
                a.limit.as_ref().and_then(|limit| limit.context),
                b.limit.as_ref().and_then(|limit| limit.context),
                descending,
            ),
            SortField::InputPrice => compare_known_first(
                a.pricing.as_ref().and_then(|pricing| pricing.rates.input),
                b.pricing.as_ref().and_then(|pricing| pricing.rates.input),
                descending,
            ),
            SortField::OutputPrice => compare_known_first(
                a.pricing.as_ref().and_then(|pricing| pricing.rates.output),
                b.pricing.as_ref().and_then(|pricing| pricing.rates.output),
                descending,
            ),
        };

        primary.then_with(|| identity_cmp(a, b))
    });

    let total = models.len();
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    if limit == 0 || offset >= total {
        return RouteResult {
            models: Vec::new(),
            total,
            has_more: false,
        };
    }

    let end = offset.saturating_add(limit).min(total);
    RouteResult {
        models: models[offset..end].to_vec(),
        total,
        has_more: end < total,
    }
}
