use crate::types::*;
use rust_decimal::Decimal;

fn price_for_tokens(
    rate: Option<Decimal>,
    tokens: u64,
    model: &ModelKey,
    component: CostComponent,
) -> Result<Decimal, RouterError> {
    if tokens == 0 {
        return Ok(Decimal::ZERO);
    }

    let rate = rate.ok_or_else(|| RouterError::MissingPriceComponent {
        model: model.clone(),
        component,
    })?;
    if rate < Decimal::ZERO {
        return Err(RouterError::InvalidPriceComponent {
            model: model.clone(),
            component,
            rate,
        });
    }
    rate.checked_mul(Decimal::from(tokens))
        .and_then(|cost| cost.checked_div(Decimal::from(1_000_000u64)))
        .ok_or(RouterError::TokenOverflow)
}

fn selected_rates(
    pricing: Option<&ModelPricing>,
    context_tokens: u64,
) -> (Option<&ModelPriceRates>, Option<u64>) {
    let Some(pricing) = pricing else {
        return (None, None);
    };

    let has_context_tiers = pricing
        .tiers
        .iter()
        .any(|tier| tier.tier.kind == ModelPriceTierKind::Context);
    if has_context_tiers {
        if let Some(tier) = pricing
            .tiers
            .iter()
            .filter(|tier| {
                tier.tier.kind == ModelPriceTierKind::Context && tier.tier.size <= context_tokens
            })
            .max_by_key(|tier| tier.tier.size)
        {
            return (Some(&tier.rates), Some(tier.tier.size));
        }
    } else if context_tokens > 200_000 {
        if let Some(rates) = pricing.context_over_200k.as_ref() {
            return (Some(rates), Some(200_000));
        }
    }

    (Some(&pricing.rates), None)
}

/// Calculate an exact cost breakdown for a model and token usage.
///
/// Prices are per million tokens. Explicit context tiers take precedence over
/// the legacy `context_over_200k` rates. Nonzero usage without a corresponding
/// rate returns [`RouterError::MissingPriceComponent`].
pub fn calculate_cost_for_model(
    model: &FlatModel,
    request: &CostRequest,
) -> Result<CostBreakdown, RouterError> {
    let context_tokens = request.context_tokens.unwrap_or(request.input_tokens);
    let (rates, applied_tier) = selected_rates(model.pricing.as_ref(), context_tokens);
    let key = model.key();

    let input = price_for_tokens(
        rates.and_then(|rates| rates.input),
        request.input_tokens,
        &key,
        CostComponent::Input,
    )?;
    let output = price_for_tokens(
        rates.and_then(|rates| rates.output),
        request.output_tokens,
        &key,
        CostComponent::Output,
    )?;
    let reasoning = price_for_tokens(
        rates.and_then(|rates| rates.reasoning),
        request.reasoning_tokens.unwrap_or(0),
        &key,
        CostComponent::Reasoning,
    )?;
    let cache_read = price_for_tokens(
        rates.and_then(|rates| rates.cache_read),
        request.cache_read_tokens.unwrap_or(0),
        &key,
        CostComponent::CacheRead,
    )?;
    let cache_write = price_for_tokens(
        rates.and_then(|rates| rates.cache_write),
        request.cache_write_tokens.unwrap_or(0),
        &key,
        CostComponent::CacheWrite,
    )?;
    let input_audio = price_for_tokens(
        rates.and_then(|rates| rates.input_audio),
        request.input_audio_tokens.unwrap_or(0),
        &key,
        CostComponent::InputAudio,
    )?;
    let output_audio = price_for_tokens(
        rates.and_then(|rates| rates.output_audio),
        request.output_audio_tokens.unwrap_or(0),
        &key,
        CostComponent::OutputAudio,
    )?;

    let total = [
        input,
        output,
        reasoning,
        cache_read,
        cache_write,
        input_audio,
        output_audio,
    ]
    .into_iter()
    .try_fold(Decimal::ZERO, |total, cost| total.checked_add(cost))
    .ok_or(RouterError::TokenOverflow)?;

    Ok(CostBreakdown {
        input,
        output,
        reasoning,
        cache_read,
        cache_write,
        input_audio,
        output_audio,
        total,
        applied_tier,
    })
}

/// Estimate exact input and output cost from per-million-token rates.
///
/// Returns `None` when a nonzero token count has no corresponding rate or the
/// decimal calculation overflows.
pub fn estimate_request_cost(
    input_price_per_million: Option<Decimal>,
    output_price_per_million: Option<Decimal>,
    input_tokens: u64,
    output_tokens: u64,
) -> Option<Decimal> {
    fn optional_price(rate: Option<Decimal>, tokens: u64) -> Option<Decimal> {
        if tokens == 0 {
            return Some(Decimal::ZERO);
        }
        let rate = rate?;
        if rate < Decimal::ZERO {
            return None;
        }
        rate.checked_mul(Decimal::from(tokens))?
            .checked_div(Decimal::from(1_000_000u64))
    }

    optional_price(input_price_per_million, input_tokens)?
        .checked_add(optional_price(output_price_per_million, output_tokens)?)
}
