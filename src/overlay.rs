//! Overlay enrichment from the models.dev catalog.
//!
//! The primary [`crate::RouterStore`] is loaded from an ai-model-directory
//! JSON file whose schema mirrors `data/all.min.json`. That dataset has
//! known gaps — many models lack `features.tool_call`, `limit.context`, or
//! pricing fields, and some flags are demonstrably wrong (e.g. `gpt-4o`
//! ships with `tool_call: false`).
//!
//! This module loads a second, parallel catalog from
//! [models.dev](https://models.dev/api.json) and merges it into the store.
//! models.dev is hand-curated and tends to be more complete and accurate
//! for canonical-provider models. The merge is configurable: by default
//! the overlay only fills fields that are currently `None` in the store
//! ([`OverlayMode::FillOnly`]), but you can opt into
//! [`OverlayMode::PreferOverlay`] to let models.dev override fields the
//! store already holds.
//!
//! # Example
//!
//! ```no_run
//! use ai_model_directory_router::{RouterStore, OverlayMode};
//! use std::path::Path;
//!
//! let mut store = RouterStore::from_file(Path::new("data/all.min.json")).unwrap();
//! store
//!     .apply_overlay_from_file(Path::new("data/models-dev-api.json"), OverlayMode::FillOnly)
//!     .unwrap();
//! ```
//!
//! # Schema mapping
//!
//! | models.dev field | router-rs field |
//! |------------------|-----------------|
//! | top-level `attachment` (bool) | `features.attachment` |
//! | top-level `reasoning` (bool) | `features.reasoning` |
//! | top-level `tool_call` (bool) | `features.tool_call` |
//! | top-level `structured_output` (bool) | `features.structured_output` |
//! | top-level `temperature` (bool) | `features.temperature` |
//! | top-level `open_weights` (bool) | `open_weights` |
//! | top-level `knowledge` (string) | `knowledge_cutoff` |
//! | top-level `release_date` (string) | `release_date` |
//! | top-level `last_updated` (string) | `last_updated` |
//! | `cost.input` (number) | `pricing.input` |
//! | `cost.output` (number) | `pricing.output` |
//! | `cost.cache_read` (number) | `pricing.cache_read` |
//! | `cost.cache_write` (number) | `pricing.cache_write` |
//! | `limit.context` (number) | `limit.context` |
//! | `limit.input` (number) | `limit.input` |
//! | `limit.output` (number) | `limit.output` |
//! | `modalities.input` (strings) | `modalities.input` (`Vec<ModelModality>`) |
//! | `modalities.output` (strings) | `modalities.output` |
//!
//! Modality strings are normalized: `"pdf"` is mapped to
//! [`ModelModality::File`] since the ai-model-directory enum does not
//! distinguish PDF from generic file attachments. Unknown modality
//! strings are silently dropped.

use crate::types::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// How the overlay merges with the existing store data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayMode {
    /// Only fill fields that are currently `None`. Existing values in the
    /// store are never overwritten. This is the safest default.
    #[default]
    FillOnly,

    /// Whenever the overlay provides a value, use it — even if the store
    /// already has a value. Useful when you trust models.dev as the
    /// authoritative source (e.g. correcting known data errors).
    PreferOverlay,
}

/// One model record as it appears in the models.dev JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelsDevModel {
    pub id: String,
    pub name: Option<String>,
    pub family: Option<String>,
    pub attachment: Option<bool>,
    pub reasoning: Option<bool>,
    pub tool_call: Option<bool>,
    pub structured_output: Option<bool>,
    pub temperature: Option<bool>,
    pub knowledge: Option<String>,
    pub release_date: Option<String>,
    pub last_updated: Option<String>,
    pub open_weights: Option<bool>,
    pub modalities: Option<ModelsDevModalities>,
    pub cost: Option<ModelsDevCost>,
    pub limit: Option<ModelsDevLimit>,
}

/// Input and output modalities in the models.dev schema.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelsDevModalities {
    pub input: Option<Vec<String>>,
    pub output: Option<Vec<String>>,
}

/// Pricing block in the models.dev schema (per million tokens).
#[derive(Debug, Clone, Deserialize)]
pub struct ModelsDevCost {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
}

/// Token limits block in the models.dev schema.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelsDevLimit {
    pub context: Option<u64>,
    pub input: Option<u64>,
    pub output: Option<u64>,
}

/// One provider entry in the models.dev JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelsDevProvider {
    pub id: String,
    pub name: Option<String>,
    pub doc: Option<String>,
    pub models: HashMap<String, ModelsDevModel>,
}

/// Lookup index built from a parsed models.dev catalog.
///
/// Keys are `(normalized_provider, normalized_model_id)` where
/// normalization is lowercase + non-alphanumeric collapsed to `-`.
pub type ModelsDevIndex = HashMap<(String, String), ModelsDevModel>;

fn normalize(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Map a provider ID from the ai-model-directory catalog to candidate
/// provider IDs in the models.dev catalog. Returns the input first, then
/// any known aliases.
fn provider_aliases(provider: &str) -> Vec<String> {
    let base = normalize(provider);
    let mut aliases = vec![base.clone()];
    let extra: &[&str] = match base.as_str() {
        "alibaba-cn" => &["alibaba", "qwen", "dashscope"],
        "alibaba" => &["alibaba-cn", "qwen", "dashscope"],
        "qwen" => &["alibaba", "alibaba-cn", "dashscope"],
        "anthropic" => &["claude"],
        "claude" => &["anthropic"],
        "fireworks-ai" => &["fireworks"],
        "fireworks" => &["fireworks-ai"],
        "github-models" => &["github"],
        "github-copilot" => &["github"],
        "z-ai" => &["zhipuai", "zhipu"],
        "zhipu" => &["z-ai", "zhipuai"],
        _ => &[],
    };
    for alias in extra {
        aliases.push((*alias).to_string());
    }
    aliases
}

fn modality_from_str(value: &str) -> Option<ModelModality> {
    match value.to_ascii_lowercase().as_str() {
        "text" => Some(ModelModality::Text),
        "image" => Some(ModelModality::Image),
        "audio" => Some(ModelModality::Audio),
        "video" => Some(ModelModality::Video),
        "file" | "pdf" => Some(ModelModality::File),
        _ => None,
    }
}

fn convert_modalities(values: &Option<Vec<String>>) -> Option<Vec<ModelModality>> {
    let raw = values.as_ref()?;
    let converted: Vec<ModelModality> = raw.iter().filter_map(|m| modality_from_str(m)).collect();
    if converted.is_empty() {
        None
    } else {
        Some(converted)
    }
}

/// Parse a models.dev catalog JSON string and build a lookup index.
pub fn parse_models_dev(json: &str) -> Result<ModelsDevIndex, RouterError> {
    let raw: HashMap<String, ModelsDevProvider> = serde_json::from_str(json)?;
    let mut index = ModelsDevIndex::new();
    for (provider_id, provider) in raw {
        let provider_norm = normalize(&provider_id);
        for (model_key, model) in provider.models {
            let model_norm = normalize(&model_key);
            index.insert((provider_norm.clone(), model_norm), model);
        }
    }
    Ok(index)
}

/// Load a models.dev catalog from disk and build a lookup index.
pub fn load_models_dev_from_file(
    path: &std::path::Path,
) -> Result<ModelsDevIndex, RouterError> {
    let json = fs::read_to_string(path)?;
    parse_models_dev(&json)
}

/// Guess the canonical provider for a model based on a well-known
/// naming convention (e.g. `gpt-*` → openai, `claude-*` → anthropic).
///
/// Returns `None` if the prefix doesn't match any known family. This
/// lets the overlay match aggregator-served models (`302ai/deepseek-r1`,
/// `aihubmix/claude-haiku-4-5`) against canonical-provider data when no
/// direct provider match exists.
fn canonical_provider_for_model(model_id: &str) -> Option<&'static str> {
    let norm = model_id.to_ascii_lowercase();
    let needle = norm.strip_prefix("openai/").unwrap_or(&norm);
    let needle = needle.strip_prefix("anthropic/").unwrap_or(needle);
    let needle = needle.strip_prefix("google/").unwrap_or(needle);
    if needle.starts_with("gpt-")
        || needle.starts_with("gpt5")
        || needle.starts_with("o1")
        || needle.starts_with("o3")
        || needle.starts_with("o4")
        || needle.starts_with("text-embedding-")
        || needle.starts_with("dall-e")
        || needle.starts_with("babbage")
        || needle.starts_with("davinci")
        || needle.starts_with("whisper")
        || needle.starts_with("omni-moderation")
        || needle.starts_with("codex")
    {
        return Some("openai");
    }
    if needle.starts_with("claude-") {
        return Some("anthropic");
    }
    if needle.starts_with("gemini-") || needle.starts_with("gemma-") {
        return Some("google");
    }
    if needle.starts_with("glm-") || needle.starts_with("chatglm") {
        return Some("zhipu");
    }
    if needle.starts_with("qwen-") || needle.starts_with("qwen2") || needle.starts_with("qwen3") {
        return Some("alibaba");
    }
    if needle.starts_with("deepseek-") {
        return Some("deepseek");
    }
    if needle.starts_with("grok-") {
        return Some("xai");
    }
    if needle.starts_with("llama-") || needle.starts_with("llama3") || needle.starts_with("llama4") {
        return Some("meta");
    }
    if needle.starts_with("mistral-") || needle.starts_with("codestral-") || needle.starts_with("mixtral-") {
        return Some("mistral");
    }
    if needle.starts_with("command-") || needle.starts_with("embed-") {
        return Some("cohere");
    }
    if needle.starts_with("phi-") || needle.starts_with("phi4") {
        return Some("microsoft");
    }
    None
}

fn find_overlay<'a>(
    index: &'a ModelsDevIndex,
    provider: &str,
    model_id: &str,
) -> Option<&'a ModelsDevModel> {
    let model_norm = normalize(model_id);
    for alias in provider_aliases(provider) {
        if let Some(found) = index.get(&(alias, model_norm.clone())) {
            return Some(found);
        }
    }
    // Some models in the catalog carry a vendor prefix (e.g. `openai/gpt-4o`
    // in an aggregator provider). Try stripping the leading segment so we
    // can match the canonical provider's entry.
    if let Some((prefix, rest)) = model_id.split_once('/') {
        let rest_norm = normalize(rest);
        for alias in provider_aliases(provider) {
            if let Some(found) = index.get(&(alias, rest_norm.clone())) {
                return Some(found);
            }
        }
        for alias in provider_aliases(prefix) {
            if let Some(found) = index.get(&(alias, rest_norm.clone())) {
                return Some(found);
            }
        }
        // Canonical provider fallback using the stripped model name.
        if let Some(canonical) = canonical_provider_for_model(rest) {
            for alias in provider_aliases(canonical) {
                if let Some(found) = index.get(&(alias, rest_norm.clone())) {
                    return Some(found);
                }
            }
        }
    }
    // Canonical provider fallback using the unprefixed model id. This is
    // what enables `302ai/deepseek-r1` to match `deepseek/deepseek-r1` in
    // models.dev when the local provider has no direct alias.
    if let Some(canonical) = canonical_provider_for_model(model_id) {
        for alias in provider_aliases(canonical) {
            if let Some(found) = index.get(&(alias, model_norm.clone())) {
                return Some(found);
            }
        }
    }
    None
}

fn apply_to_features(
    target: &mut Option<ModelFeatures>,
    overlay: &ModelsDevModel,
    mode: OverlayMode,
) -> bool {
    let mut changed = false;
    let needs_init = target.is_none()
        && (overlay.attachment.is_some()
            || overlay.reasoning.is_some()
            || overlay.tool_call.is_some()
            || overlay.structured_output.is_some()
            || overlay.temperature.is_some());
    if needs_init {
        *target = Some(ModelFeatures::default());
    }
    let features = match target.as_mut() {
        Some(f) => f,
        None => return false,
    };
    macro_rules! apply_bool {
        ($field:ident) => {
            if let Some(v) = overlay.$field {
                let should_write = match mode {
                    OverlayMode::FillOnly => features.$field.is_none(),
                    OverlayMode::PreferOverlay => features.$field != Some(v),
                };
                if should_write {
                    features.$field = Some(v);
                    changed = true;
                }
            }
        };
    }
    apply_bool!(attachment);
    apply_bool!(reasoning);
    apply_bool!(tool_call);
    apply_bool!(structured_output);
    apply_bool!(temperature);
    changed
}

fn apply_to_pricing(
    target: &mut Option<ModelPricing>,
    overlay: &Option<ModelsDevCost>,
    mode: OverlayMode,
) -> bool {
    let cost = match overlay {
        Some(c) => c,
        None => return false,
    };
    let any_value = cost.input.is_some()
        || cost.output.is_some()
        || cost.cache_read.is_some()
        || cost.cache_write.is_some();
    if !any_value {
        return false;
    }
    if target.is_none() {
        *target = Some(ModelPricing {
            input: None,
            output: None,
            reasoning: None,
            cache_read: None,
            cache_write: None,
            input_audio: None,
            output_audio: None,
        });
    }
    let pricing = target.as_mut().unwrap();
    let mut changed = false;
    macro_rules! apply_price {
        ($src:ident, $dst:ident) => {
            if let Some(v) = cost.$src {
                let should_write = match mode {
                    OverlayMode::FillOnly => pricing.$dst.is_none(),
                    OverlayMode::PreferOverlay => pricing.$dst != Some(v),
                };
                if should_write {
                    pricing.$dst = Some(v);
                    changed = true;
                }
            }
        };
    }
    apply_price!(input, input);
    apply_price!(output, output);
    apply_price!(cache_read, cache_read);
    apply_price!(cache_write, cache_write);
    changed
}

fn apply_to_limit(
    target: &mut Option<ModelLimit>,
    overlay: &Option<ModelsDevLimit>,
    mode: OverlayMode,
) -> bool {
    let limit_src = match overlay {
        Some(l) => l,
        None => return false,
    };
    let any_value =
        limit_src.context.is_some() || limit_src.input.is_some() || limit_src.output.is_some();
    if !any_value {
        return false;
    }
    if target.is_none() {
        *target = Some(ModelLimit {
            context: None,
            input: None,
            output: None,
        });
    }
    let limit = target.as_mut().unwrap();
    let mut changed = false;
    macro_rules! apply_lim {
        ($field:ident) => {
            if let Some(v) = limit_src.$field {
                let should_write = match mode {
                    OverlayMode::FillOnly => limit.$field.is_none(),
                    OverlayMode::PreferOverlay => limit.$field != Some(v),
                };
                if should_write {
                    limit.$field = Some(v);
                    changed = true;
                }
            }
        };
    }
    apply_lim!(context);
    apply_lim!(input);
    apply_lim!(output);
    changed
}

fn apply_to_modalities(
    target: &mut Option<ModelModalities>,
    overlay: &Option<ModelsDevModalities>,
    mode: OverlayMode,
) -> bool {
    let modalities_src = match overlay {
        Some(m) => m,
        None => return false,
    };
    let input_overlay = convert_modalities(&modalities_src.input);
    let output_overlay = convert_modalities(&modalities_src.output);
    if input_overlay.is_none() && output_overlay.is_none() {
        return false;
    }
    if target.is_none() {
        *target = Some(ModelModalities {
            input: None,
            output: None,
        });
    }
    let modalities = target.as_mut().unwrap();
    let mut changed = false;
    if let Some(v) = input_overlay {
        let should_write = match mode {
            OverlayMode::FillOnly => modalities.input.is_none(),
            OverlayMode::PreferOverlay => modalities.input.as_ref() != Some(&v),
        };
        if should_write {
            modalities.input = Some(v);
            changed = true;
        }
    }
    if let Some(v) = output_overlay {
        let should_write = match mode {
            OverlayMode::FillOnly => modalities.output.is_none(),
            OverlayMode::PreferOverlay => modalities.output.as_ref() != Some(&v),
        };
        if should_write {
            modalities.output = Some(v);
            changed = true;
        }
    }
    changed
}

fn apply_scalar_str(
    target: &mut Option<String>,
    overlay: &Option<String>,
    mode: OverlayMode,
) -> bool {
    let value = match overlay {
        Some(v) if !v.is_empty() => v,
        _ => return false,
    };
    let should_write = match mode {
        OverlayMode::FillOnly => target.is_none(),
        OverlayMode::PreferOverlay => target.as_deref() != Some(value),
    };
    if should_write {
        *target = Some(value.clone());
        true
    } else {
        false
    }
}

fn apply_scalar_bool(
    target: &mut Option<bool>,
    overlay: Option<bool>,
    mode: OverlayMode,
) -> bool {
    let value = match overlay {
        Some(v) => v,
        None => return false,
    };
    let should_write = match mode {
        OverlayMode::FillOnly => target.is_none(),
        OverlayMode::PreferOverlay => *target != Some(value),
    };
    if should_write {
        *target = Some(value);
        true
    } else {
        false
    }
}

/// Result of running an overlay merge across a collection of models.
#[derive(Debug, Clone, Default)]
pub struct OverlayReport {
    /// Number of models that received at least one field from the overlay.
    pub models_touched: usize,
    /// Number of (model, field) writes performed.
    pub fields_written: usize,
    /// Number of input models with no matching overlay entry.
    pub models_unmatched: usize,
}

/// Apply a models.dev overlay to a slice of [`FlatModel`]s in place.
///
/// Returns an [`OverlayReport`] summarizing how many models were touched
/// and how many field writes occurred.
pub fn apply_overlay(
    models: &mut [FlatModel],
    overlay: &ModelsDevIndex,
    mode: OverlayMode,
) -> OverlayReport {
    let mut report = OverlayReport::default();
    for model in models.iter_mut() {
        let source = match find_overlay(overlay, &model.provider, &model.id) {
            Some(s) => s,
            None => {
                report.models_unmatched += 1;
                continue;
            }
        };
        let mut writes = 0usize;
        if apply_scalar_str(&mut model.name, &source.name, mode) {
            writes += 1;
        }
        if apply_scalar_str(&mut model.knowledge_cutoff, &source.knowledge, mode) {
            writes += 1;
        }
        if apply_scalar_str(&mut model.release_date, &source.release_date, mode) {
            writes += 1;
        }
        if apply_scalar_str(&mut model.last_updated, &source.last_updated, mode) {
            writes += 1;
        }
        if apply_scalar_bool(&mut model.open_weights, source.open_weights, mode) {
            writes += 1;
        }
        if apply_to_features(&mut model.features, source, mode) {
            writes += 1;
        }
        if apply_to_pricing(&mut model.pricing, &source.cost, mode) {
            writes += 1;
        }
        if apply_to_limit(&mut model.limit, &source.limit, mode) {
            writes += 1;
        }
        if apply_to_modalities(&mut model.modalities, &source.modalities, mode) {
            writes += 1;
        }
        if writes > 0 {
            report.models_touched += 1;
            report.fields_written += writes;
        }
    }
    report
}
