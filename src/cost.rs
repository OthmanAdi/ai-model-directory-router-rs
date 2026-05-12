use crate::types::*;
use rust_decimal::Decimal;

fn price_for_tokens(rate: Option<f64>, tokens: u64) -> Decimal {
    match rate {
        Some(r) if tokens > 0 => {
            Decimal::from_f64_retain(r).unwrap_or(Decimal::ZERO)
                * Decimal::from(tokens)
                / Decimal::from(1_000_000u64)
        }
        _ => Decimal::ZERO,
    }
}

/// Calculate a full cost breakdown for a model given token usage.
///
/// Prices in the model data are per million tokens. The returned
/// [`CostBreakdown`] contains values already scaled to the requested
/// token counts, using exact [`Decimal`] arithmetic (no float drift).
///
/// # Example
///
/// ```no_run
/// use ai_model_directory_router::{RouterStore, calculate_cost_for_model, CostRequest};
/// use std::path::Path;
///
/// let store = RouterStore::from_file(Path::new("data/all.min.json")).unwrap();
/// let model = store.find_model("gpt-4o").unwrap();
/// let req = CostRequest {
///     input_tokens: 1_000_000,
///     output_tokens: 500_000,
///     ..Default::default()
/// };
/// let cost = calculate_cost_for_model(model, &req);
/// println!("Total: ${}", cost.total);
/// ```
pub fn calculate_cost_for_model(
    model: &FlatModel,
    request: &CostRequest,
) -> CostBreakdown {
    let p = model.pricing.as_ref();

    let input = price_for_tokens(p.and_then(|x| x.input), request.input_tokens);
    let output = price_for_tokens(p.and_then(|x| x.output), request.output_tokens);
    let reasoning = price_for_tokens(
        p.and_then(|x| x.reasoning),
        request.reasoning_tokens.unwrap_or(0),
    );
    let cache_read = price_for_tokens(
        p.and_then(|x| x.cache_read),
        request.cache_read_tokens.unwrap_or(0),
    );
    let cache_write = price_for_tokens(
        p.and_then(|x| x.cache_write),
        request.cache_write_tokens.unwrap_or(0),
    );
    let input_audio = price_for_tokens(
        p.and_then(|x| x.input_audio),
        request.input_audio_tokens.unwrap_or(0),
    );
    let output_audio = price_for_tokens(
        p.and_then(|x| x.output_audio),
        request.output_audio_tokens.unwrap_or(0),
    );

    let total = input + output + reasoning + cache_read + cache_write + input_audio + output_audio;

    CostBreakdown {
        input,
        output,
        reasoning,
        cache_read,
        cache_write,
        input_audio,
        output_audio,
        total,
    }
}

/// Quick cost estimate given per-million-token prices and token counts.
///
/// Returns the combined input + output cost as a [`Decimal`].
pub fn estimate_request_cost(
    input_price_per_million: Option<f64>,
    output_price_per_million: Option<f64>,
    input_tokens: u64,
    output_tokens: u64,
) -> Decimal {
    price_for_tokens(input_price_per_million, input_tokens)
        + price_for_tokens(output_price_per_million, output_tokens)
}
