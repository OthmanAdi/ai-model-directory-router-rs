//! Real-world measurement of the models.dev overlay against the
//! ai-model-directory dataset.
//!
//! Run from the workspace root:
//!
//! ```bash
//! cargo run --release --example overlay_impact
//! ```

use ai_model_directory_router::{OverlayMode, RouterStore};
use std::path::Path;

fn count_missing(store: &RouterStore) -> (usize, [usize; 8]) {
    let mut with_any = 0usize;
    let mut counts = [0usize; 8];
    for m in store.flat_models() {
        let missing = [
            m.name.is_none(),
            m.limit.as_ref().and_then(|l| l.context).is_none(),
            m.pricing.as_ref().and_then(|p| p.input).is_none(),
            m.pricing.as_ref().and_then(|p| p.output).is_none(),
            m.features.as_ref().and_then(|f| f.attachment).is_none(),
            m.features.as_ref().and_then(|f| f.tool_call).is_none(),
            m.modalities
                .as_ref()
                .and_then(|x| x.input.as_ref())
                .is_none(),
            m.modalities
                .as_ref()
                .and_then(|x| x.output.as_ref())
                .is_none(),
        ];
        for (i, miss) in missing.iter().enumerate() {
            if *miss {
                counts[i] += 1;
            }
        }
        if missing.iter().any(|x| *x) {
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
    println!("{:─^60}", format!(" {} ", label));
    for (i, name) in names.iter().enumerate() {
        let delta = after[i] as i64 - before[i] as i64;
        let arrow = if delta < 0 { "↓" } else if delta > 0 { "↑" } else { "·" };
        println!(
            "  {:<22} {:>6} → {:>6}  {} {:>5}",
            name, before[i], after[i], arrow, delta.abs()
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let primary_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/all.min.json".to_string());
    let overlay_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "data/models-dev-api.json".to_string());

    if !Path::new(&primary_path).exists() {
        eprintln!("Primary data file not found: {}", primary_path);
        eprintln!("Usage: cargo run --example overlay_impact -- [primary] [overlay]");
        std::process::exit(1);
    }
    if !Path::new(&overlay_path).exists() {
        eprintln!("Overlay data file not found: {}", overlay_path);
        std::process::exit(1);
    }

    println!("Loading {}", primary_path);
    let mut store = RouterStore::from_file(Path::new(&primary_path))?;
    let total = store.flat_models().len();
    println!("Loaded {} models", total);

    let (before_with_any, before_counts) = count_missing(&store);
    println!(
        "Before overlay: {}/{} models have at least one missing required field",
        before_with_any, total
    );

    println!("\nApplying overlay (FillOnly) from {}", overlay_path);
    let report = store.apply_overlay_from_file(Path::new(&overlay_path), OverlayMode::FillOnly)?;
    println!(
        "Overlay touched {} models, wrote {} fields, {} models unmatched",
        report.models_touched, report.fields_written, report.models_unmatched
    );

    let (after_with_any, after_counts) = count_missing(&store);
    println!(
        "\nAfter FillOnly: {}/{} models have at least one missing required field (delta: {})",
        after_with_any,
        total,
        after_with_any as i64 - before_with_any as i64
    );
    print_row("FillOnly impact per field", &before_counts, &after_counts);

    // Reset and re-test with PreferOverlay
    let mut store = RouterStore::from_file(Path::new(&primary_path))?;
    let report = store.apply_overlay_from_file(Path::new(&overlay_path), OverlayMode::PreferOverlay)?;
    let (po_with_any, po_counts) = count_missing(&store);
    println!(
        "\nWith PreferOverlay: {} touched, {} writes, {}/{} still have any missing (delta: {})",
        report.models_touched,
        report.fields_written,
        po_with_any,
        total,
        po_with_any as i64 - before_with_any as i64
    );
    print_row("PreferOverlay impact per field", &before_counts, &po_counts);

    // Critical canary models — these are the ones migRaven cares about most.
    println!("\n{:─^60}", " Critical canary models ");
    let canaries = [
        "gpt-4o",
        "gpt-5",
        "gpt-5.4-pro",
        "gpt-5.5",
        "claude-opus-4-5",
        "claude-sonnet-4-6",
        "claude-haiku-4-5",
        "deepseek-r1",
        "deepseek-v3",
        "glm-4.6",
    ];
    for id in canaries {
        if let Some(m) = store.find_model(id) {
            let ctx = m.limit.as_ref().and_then(|l| l.context).map(|v| v as i64).unwrap_or(-1);
            let tool = m.features.as_ref().and_then(|f| f.tool_call);
            let temp = m.features.as_ref().and_then(|f| f.temperature);
            println!(
                "  {:<22} provider={:<14} ctx={:>10} tool_call={:?} temperature={:?}",
                id, m.provider, ctx, tool, temp
            );
        } else {
            println!("  {:<22} NOT FOUND", id);
        }
    }

    Ok(())
}
