use crate::store::RouterStore;
use crate::types::*;

fn matches_input_modality(model: &FlatModel, modalities: &[ModelModality]) -> bool {
    model
        .modalities
        .as_ref()
        .and_then(|m| m.input.as_ref())
        .map(|m_input| modalities.iter().all(|req| m_input.contains(req)))
        .unwrap_or(false)
}

fn matches_output_modality(model: &FlatModel, modalities: &[ModelModality]) -> bool {
    model
        .modalities
        .as_ref()
        .and_then(|m| m.output.as_ref())
        .map(|m_output| modalities.iter().all(|req| m_output.contains(req)))
        .unwrap_or(false)
}

fn matches_features(model: &FlatModel, features: &ModelFeatures) -> bool {
    let mf = match &model.features {
        Some(f) => f,
        None => return false,
    };

    if features.tool_call == Some(true) && mf.tool_call != Some(true) {
        return false;
    }
    if features.reasoning == Some(true) && mf.reasoning != Some(true) {
        return false;
    }
    if features.structured_output == Some(true) && mf.structured_output != Some(true) {
        return false;
    }
    if features.attachment == Some(true) && mf.attachment != Some(true) {
        return false;
    }
    if features.temperature == Some(true) && mf.temperature != Some(true) {
        return false;
    }
    true
}

pub fn route(store: &RouterStore, query: &RouteQuery) -> RouteResult {
    let mut models: Vec<FlatModel> = store.flat_models().to_vec();

    if let Some(ref provider) = query.provider {
        let lower = provider.to_lowercase();
        models.retain(|m| m.provider.to_lowercase() == lower);
    }

    if let Some(ref modalities) = query.input_modalities {
        models.retain(|m| matches_input_modality(m, modalities));
    }

    if let Some(ref modalities) = query.output_modalities {
        models.retain(|m| matches_output_modality(m, modalities));
    }

    if let Some(ref features) = query.features {
        models.retain(|m| matches_features(m, features));
    }

    if let Some(min_ctx) = query.min_context {
        models.retain(|m| m.limit.as_ref().and_then(|l| l.context).unwrap_or(0) >= min_ctx);
    }

    if let Some(max_price) = query.max_input_price {
        models.retain(|m| {
            m.pricing
                .as_ref()
                .and_then(|p| p.input)
                .map(|price| price <= max_price)
                .unwrap_or(false)
        });
    }

    if let Some(max_price) = query.max_output_price {
        models.retain(|m| {
            m.pricing
                .as_ref()
                .and_then(|p| p.output)
                .map(|price| price <= max_price)
                .unwrap_or(false)
        });
    }

    if let Some(open) = query.open_weights {
        models.retain(|m| m.open_weights == Some(open));
    }

    if let Some(ref sort_field) = query.sort {
        let descending = matches!(query.order, Some(SortOrder::Desc));
        models.sort_by(|a, b| {
            let cmp = match sort_field {
                SortField::Id => a.id.cmp(&b.id),
                SortField::Context => {
                    let va = a.limit.as_ref().and_then(|l| l.context).unwrap_or(0);
                    let vb = b.limit.as_ref().and_then(|l| l.context).unwrap_or(0);
                    va.cmp(&vb)
                }
                SortField::InputPrice => {
                    let va = a.pricing.as_ref().and_then(|p| p.input).unwrap_or(f64::MAX);
                    let vb = b.pricing.as_ref().and_then(|p| p.input).unwrap_or(f64::MAX);
                    va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortField::OutputPrice => {
                    let va = a.pricing.as_ref().and_then(|p| p.output).unwrap_or(f64::MAX);
                    let vb = b.pricing.as_ref().and_then(|p| p.output).unwrap_or(f64::MAX);
                    va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                }
            };
            if descending { cmp.reverse() } else { cmp }
        });
    }

    let total = models.len();
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    if offset >= total {
        return RouteResult {
            models: vec![],
            total,
            has_more: false,
        };
    }

    let end = std::cmp::min(offset + limit, total);
    let paged: Vec<FlatModel> = models[offset..end].to_vec();
    let has_more = offset + limit < total;

    RouteResult {
        models: paged,
        total,
        has_more,
    }
}
