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

pub fn estimate_request_cost(
    input_price_per_million: Option<f64>,
    output_price_per_million: Option<f64>,
    input_tokens: u64,
    output_tokens: u64,
) -> Decimal {
    price_for_tokens(input_price_per_million, input_tokens)
        + price_for_tokens(output_price_per_million, output_tokens)
}
