//! Exact provider-qualified enrichment from a models.dev provider catalog.
//!
//! An overlay only updates offerings already present in a store. It never
//! inserts models. Use [`crate::RouterStore::from_models_dev_json`],
//! [`crate::RouterStore::from_models_dev_file`], or the bundled/live
//! constructors when models.dev should be the complete catalog source.
//!
//! Matching is deliberately strict: both provider ID and model ID must be an
//! exact match. Provider aliases, model-family guesses, and cross-region
//! Alibaba matching are not used because capabilities, lifecycle state, and
//! commercial terms can differ between provider offerings.

use crate::types::*;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// How models.dev values are merged into an existing exact offering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayMode {
    /// Fill individual fields that are currently missing.
    #[default]
    FillOnly,

    /// Prefer fields reported by this overlay for the same exact offering.
    ///
    /// This is a caller-selected preference, not a claim that models.dev is
    /// universally authoritative for provider availability, limits, or bills.
    PreferOverlay,
}

/// One model record from the current models.dev provider API.
pub type ModelsDevModel = ModelRecord;
/// Current models.dev pricing, including audio/reasoning rates and tiers.
pub type ModelsDevCost = ModelPricing;
/// Current models.dev modality metadata.
pub type ModelsDevModalities = ModelModalities;
/// Current models.dev token-limit metadata.
pub type ModelsDevLimit = ModelLimit;
/// One provider record from the current models.dev provider API.
pub type ModelsDevProvider = ProviderEntry;

/// Exact `(provider ID, model ID)` lookup index.
pub type ModelsDevIndex = BTreeMap<(String, String), ModelsDevModel>;

/// Parse a models.dev provider catalog into an exact lookup index.
pub fn parse_models_dev(json: &str) -> Result<ModelsDevIndex, RouterError> {
    let directory = parse_models_dev_btree(json)?;
    let mut index = ModelsDevIndex::new();
    for provider in directory.into_values() {
        for model in provider.models.into_values() {
            index.insert((provider.id.clone(), model.id.clone()), model);
        }
    }
    Ok(index)
}

/// Load a models.dev provider catalog from disk.
pub fn load_models_dev_from_file(path: &Path) -> Result<ModelsDevIndex, RouterError> {
    if !path.is_file() {
        return Err(RouterError::DataFileNotFound(
            path.to_string_lossy().into_owned(),
        ));
    }
    let json = fs::read_to_string(path)?;
    parse_models_dev(&json)
}

pub(crate) fn parse_models_dev_directory(json: &str) -> Result<ModelDirectory, RouterError> {
    Ok(parse_models_dev_btree(json)?.into_iter().collect())
}

fn parse_models_dev_btree(json: &str) -> Result<BTreeMap<String, ModelsDevProvider>, RouterError> {
    let directory: BTreeMap<String, ModelsDevProvider> = serde_json::from_str(json)?;
    for (provider_key, provider) in &directory {
        validate_provider_identity(provider_key, provider)?;
    }
    Ok(directory)
}

pub(crate) fn validate_provider_identity(
    provider_key: &str,
    provider: &ProviderEntry,
) -> Result<(), RouterError> {
    if provider_key != provider.id {
        return Err(RouterError::InvalidCatalogIdentity(format!(
            "provider map key {provider_key:?} does not match embedded id {:?}",
            provider.id
        )));
    }
    for (model_key, model) in &provider.models {
        if model_key != &model.id {
            return Err(RouterError::InvalidCatalogIdentity(format!(
                "model map key {model_key:?} under provider {provider_key:?} does not match embedded id {:?}",
                model.id
            )));
        }
        if let Some(pricing) = &model.pricing {
            validate_price_rates(provider_key, model_key, "base", &pricing.rates)?;
            if let Some(rates) = &pricing.context_over_200k {
                validate_price_rates(provider_key, model_key, "context_over_200k", rates)?;
            }
            for tier in &pricing.tiers {
                validate_price_rates(provider_key, model_key, "tier", &tier.rates)?;
            }
        }
    }
    Ok(())
}

fn validate_price_rates(
    provider: &str,
    model: &str,
    band: &str,
    rates: &ModelPriceRates,
) -> Result<(), RouterError> {
    let components = [
        (CostComponent::Input, rates.input),
        (CostComponent::Output, rates.output),
        (CostComponent::Reasoning, rates.reasoning),
        (CostComponent::CacheRead, rates.cache_read),
        (CostComponent::CacheWrite, rates.cache_write),
        (CostComponent::InputAudio, rates.input_audio),
        (CostComponent::OutputAudio, rates.output_audio),
    ];
    for (component, rate) in components {
        if rate.is_some_and(|value| value < Decimal::ZERO) {
            return Err(RouterError::InvalidCatalogValue(format!(
                "negative {component} price in {band} rates for {provider}/{model}"
            )));
        }
    }
    Ok(())
}

/// Result of applying an overlay to a collection of offerings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverlayReport {
    /// Number of models that received at least one field.
    pub models_touched: usize,
    /// Number of logical model fields written.
    pub fields_written: usize,
    /// Number of input offerings with no exact overlay match.
    pub models_unmatched: usize,
}

/// Apply an overlay only where provider ID and model ID both match exactly.
pub fn apply_overlay(
    models: &mut [FlatModel],
    overlay: &ModelsDevIndex,
    mode: OverlayMode,
) -> OverlayReport {
    let mut report = OverlayReport::default();
    for model in models {
        let key = (model.provider.clone(), model.id.clone());
        let Some(source) = overlay.get(&key) else {
            report.models_unmatched += 1;
            continue;
        };

        let mut writes = 0;
        writes += usize::from(apply_option(&mut model.name, &source.name, mode));
        writes += usize::from(apply_option(
            &mut model.description,
            &source.description,
            mode,
        ));
        writes += usize::from(apply_option(&mut model.family, &source.family, mode));
        writes += usize::from(apply_option(
            &mut model.knowledge_cutoff,
            &source.knowledge_cutoff,
            mode,
        ));
        writes += usize::from(apply_option(
            &mut model.release_date,
            &source.release_date,
            mode,
        ));
        writes += usize::from(apply_option(
            &mut model.last_updated,
            &source.last_updated,
            mode,
        ));
        writes += usize::from(apply_copy(
            &mut model.open_weights,
            source.open_weights,
            mode,
        ));

        let source_features = merged_features(source);
        writes += usize::from(apply_features(&mut model.features, &source_features, mode));
        writes += usize::from(apply_option(
            &mut model.reasoning_options,
            &source.reasoning_options,
            mode,
        ));
        writes += usize::from(apply_option(
            &mut model.interleaved,
            &source.interleaved,
            mode,
        ));
        writes += usize::from(apply_pricing(&mut model.pricing, &source.pricing, mode));
        writes += usize::from(apply_limit(&mut model.limit, &source.limit, mode));
        writes += usize::from(apply_modalities(
            &mut model.modalities,
            &source.modalities,
            mode,
        ));
        writes += usize::from(apply_copy(&mut model.status, source.status, mode));
        writes += usize::from(apply_option(
            &mut model.experimental,
            &source.experimental,
            mode,
        ));
        writes += usize::from(apply_option(
            &mut model.provider_options,
            &source.provider_options,
            mode,
        ));

        if writes > 0 {
            report.models_touched += 1;
            report.fields_written += writes;
        }
    }
    report
}

fn merged_features(model: &ModelRecord) -> Option<ModelFeatures> {
    let legacy = model.features.as_ref();
    let features = ModelFeatures {
        attachment: model
            .attachment
            .or_else(|| legacy.and_then(|value| value.attachment)),
        reasoning: model
            .reasoning
            .or_else(|| legacy.and_then(|value| value.reasoning)),
        tool_call: model
            .tool_call
            .or_else(|| legacy.and_then(|value| value.tool_call)),
        structured_output: model
            .structured_output
            .or_else(|| legacy.and_then(|value| value.structured_output)),
        temperature: model
            .temperature
            .or_else(|| legacy.and_then(|value| value.temperature)),
    };
    if features.attachment.is_none()
        && features.reasoning.is_none()
        && features.tool_call.is_none()
        && features.structured_output.is_none()
        && features.temperature.is_none()
    {
        None
    } else {
        Some(features)
    }
}

fn apply_option<T: Clone + PartialEq>(
    target: &mut Option<T>,
    source: &Option<T>,
    mode: OverlayMode,
) -> bool {
    let Some(value) = source else {
        return false;
    };
    let should_write = match mode {
        OverlayMode::FillOnly => target.is_none(),
        OverlayMode::PreferOverlay => target.as_ref() != Some(value),
    };
    if should_write {
        *target = Some(value.clone());
    }
    should_write
}

fn apply_copy<T: Copy + PartialEq>(
    target: &mut Option<T>,
    source: Option<T>,
    mode: OverlayMode,
) -> bool {
    let Some(value) = source else {
        return false;
    };
    let should_write = match mode {
        OverlayMode::FillOnly => target.is_none(),
        OverlayMode::PreferOverlay => *target != Some(value),
    };
    if should_write {
        *target = Some(value);
    }
    should_write
}

fn apply_features(
    target: &mut Option<ModelFeatures>,
    source: &Option<ModelFeatures>,
    mode: OverlayMode,
) -> bool {
    let Some(source) = source else {
        return false;
    };
    let mut merged = target.clone().unwrap_or_default();
    let mut changed = false;
    changed |= apply_copy(&mut merged.attachment, source.attachment, mode);
    changed |= apply_copy(&mut merged.reasoning, source.reasoning, mode);
    changed |= apply_copy(&mut merged.tool_call, source.tool_call, mode);
    changed |= apply_copy(
        &mut merged.structured_output,
        source.structured_output,
        mode,
    );
    changed |= apply_copy(&mut merged.temperature, source.temperature, mode);
    if changed {
        *target = Some(merged);
    }
    changed
}

fn apply_pricing(
    target: &mut Option<ModelPricing>,
    source: &Option<ModelPricing>,
    mode: OverlayMode,
) -> bool {
    let Some(source) = source else {
        return false;
    };
    let mut merged = target.clone().unwrap_or_default();
    let mut changed = apply_rates(&mut merged.rates, &source.rates, mode);
    changed |= apply_option(
        &mut merged.context_over_200k,
        &source.context_over_200k,
        mode,
    );
    if !source.tiers.is_empty() {
        let should_write = match mode {
            OverlayMode::FillOnly => merged.tiers.is_empty(),
            OverlayMode::PreferOverlay => merged.tiers != source.tiers,
        };
        if should_write {
            merged.tiers.clone_from(&source.tiers);
            changed = true;
        }
    }
    if changed {
        *target = Some(merged);
    }
    changed
}

fn apply_rates(target: &mut ModelPriceRates, source: &ModelPriceRates, mode: OverlayMode) -> bool {
    let mut changed = false;
    changed |= apply_copy(&mut target.input, source.input, mode);
    changed |= apply_copy(&mut target.output, source.output, mode);
    changed |= apply_copy(&mut target.reasoning, source.reasoning, mode);
    changed |= apply_copy(&mut target.cache_read, source.cache_read, mode);
    changed |= apply_copy(&mut target.cache_write, source.cache_write, mode);
    changed |= apply_copy(&mut target.input_audio, source.input_audio, mode);
    changed |= apply_copy(&mut target.output_audio, source.output_audio, mode);
    changed
}

fn apply_limit(
    target: &mut Option<ModelLimit>,
    source: &Option<ModelLimit>,
    mode: OverlayMode,
) -> bool {
    let Some(source) = source else {
        return false;
    };
    let mut merged = target.clone().unwrap_or_default();
    let mut changed = false;
    changed |= apply_copy(&mut merged.context, source.context, mode);
    changed |= apply_copy(&mut merged.input, source.input, mode);
    changed |= apply_copy(&mut merged.output, source.output, mode);
    if changed {
        *target = Some(merged);
    }
    changed
}

fn apply_modalities(
    target: &mut Option<ModelModalities>,
    source: &Option<ModelModalities>,
    mode: OverlayMode,
) -> bool {
    let Some(source) = source else {
        return false;
    };
    let mut merged = target.clone().unwrap_or_default();
    let mut changed = false;
    changed |= apply_option(&mut merged.input, &source.input, mode);
    changed |= apply_option(&mut merged.output, &source.output, mode);
    if changed {
        *target = Some(merged);
    }
    changed
}
