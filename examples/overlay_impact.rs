//! Measure an exact models.dev overlay against a caller-supplied legacy
//! AI Model Directory dataset.
//!
//! Run from the workspace root and pass the legacy base file explicitly:
//!
//! ```bash
//! cargo run --release --example overlay_impact -- data/all.min.json data/models-dev-api.json
//! ```

use ai_model_directory_router::{OverlayMode, RouterStore};
use std::path::Path;

fn count_missing(store: &RouterStore) -> (usize, [usize; 8]) {
    let mut with_any = 0usize;
    let mut counts = [0usize; 8];
    for model in store.flat_models() {
        let missing = [
            model.name.is_none(),
            model
                .limit
                .as_ref()
                .and_then(|limit| limit.context)
                .is_none(),
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.input)
                .is_none(),
            model
                .pricing
                .as_ref()
                .and_then(|pricing| pricing.rates.output)
                .is_none(),
            model
                .features
                .as_ref()
                .and_then(|features| features.attachment)
                .is_none(),
            model
                .features
                .as_ref()
                .and_then(|features| features.tool_call)
                .is_none(),
            model
                .modalities
                .as_ref()
                .and_then(|modalities| modalities.input.as_ref())
                .is_none(),
            model
                .modalities
                .as_ref()
                .and_then(|modalities| modalities.output.as_ref())
                .is_none(),
        ];
        for (index, is_missing) in missing.iter().enumerate() {
            if *is_missing {
                counts[index] += 1;
            }
        }
        if missing.iter().any(|is_missing| *is_missing) {
            with_any += 1;
        }
    }
    (with_any, counts)
}

fn print_row(label: &str, before: &[usize; 8], after: &[usize; 8]) {
    let names = [
        "name",
        "limit.context",
        "pricing.input",
        "pricing.output",
        "features.attachment",
        "features.tool_call",
        "modalities.input",
        "modalities.output",
    ];
    println!("{:─^60}", format!(" {label} "));
    for (index, name) in names.iter().enumerate() {
        let delta = after[index] as i64 - before[index] as i64;
        let marker = if delta < 0 {
            "↓"
        } else if delta > 0 {
            "↑"
        } else {
            "·"
        };
        println!(
            "  {name:<22} {:>6} → {:>6}  {marker} {:>5}",
            before[index],
            after[index],
            delta.abs()
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let primary_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/all.min.json".to_owned());
    let overlay_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "data/models-dev-api.json".to_owned());

    if !Path::new(&primary_path).is_file() {
        eprintln!("Primary data file not found: {primary_path}");
        eprintln!("Usage: cargo run --example overlay_impact -- [primary] [overlay]");
        std::process::exit(1);
    }
    if !Path::new(&overlay_path).is_file() {
        eprintln!("Overlay data file not found: {overlay_path}");
        std::process::exit(1);
    }

    println!("Loading {primary_path}");
    let mut store = RouterStore::from_file(Path::new(&primary_path))?;
    let total = store.flat_models().len();
    println!("Loaded {total} models");

    let (before_with_any, before_counts) = count_missing(&store);
    println!(
        "Before overlay: {before_with_any}/{total} models have at least one missing required field"
    );

    println!("\nApplying overlay (FillOnly) from {overlay_path}");
    let report = store.apply_overlay_from_file(Path::new(&overlay_path), OverlayMode::FillOnly)?;
    println!(
        "Overlay touched {} models, wrote {} fields, {} models unmatched",
        report.models_touched, report.fields_written, report.models_unmatched
    );

    let (after_with_any, after_counts) = count_missing(&store);
    println!(
        "\nAfter FillOnly: {after_with_any}/{total} models have at least one missing required field (delta: {})",
        after_with_any as i64 - before_with_any as i64
    );
    print_row("FillOnly impact per field", &before_counts, &after_counts);

    let mut store = RouterStore::from_file(Path::new(&primary_path))?;
    let report =
        store.apply_overlay_from_file(Path::new(&overlay_path), OverlayMode::PreferOverlay)?;
    let (preferred_with_any, preferred_counts) = count_missing(&store);
    println!(
        "\nWith PreferOverlay: {} touched, {} writes, {preferred_with_any}/{total} still have any missing (delta: {})",
        report.models_touched,
        report.fields_written,
        preferred_with_any as i64 - before_with_any as i64
    );
    print_row(
        "PreferOverlay impact per field",
        &before_counts,
        &preferred_counts,
    );

    println!("\n{:─^60}", " Provider-qualified canary offerings ");
    let canaries = [
        "gpt-4o",
        "gpt-5.6-sol",
        "claude-sonnet-5",
        "deepseek-v4-flash",
        "qwen3.8-flash",
        "glm-5.3",
    ];
    for id in canaries {
        let offerings = store.find_models_by_id(id);
        if offerings.is_empty() {
            println!("  {id:<22} NOT FOUND");
        }
        for model in offerings {
            let context = model
                .limit
                .as_ref()
                .and_then(|limit| limit.context)
                .map(|value| value as i128)
                .unwrap_or(-1);
            let tool_call = model
                .features
                .as_ref()
                .and_then(|features| features.tool_call);
            println!(
                "  {:<32} context={context:>10} tool_call={tool_call:?}",
                model.key()
            );
        }
    }

    Ok(())
}
