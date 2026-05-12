use serde::Deserialize;
use std::collections::HashMap;

/// Input or output modality supported by a model.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ModelModality {
    Text,
    Image,
    Audio,
    Video,
    File,
}

/// Feature flags describing what a model can do.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelFeatures {
    pub attachment: Option<bool>,
    pub reasoning: Option<bool>,
    pub tool_call: Option<bool>,
    pub structured_output: Option<bool>,
    pub temperature: Option<bool>,
}

/// Per-token prices (USD per million tokens) for a model.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelPricing {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub reasoning: Option<f64>,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
    pub input_audio: Option<f64>,
    pub output_audio: Option<f64>,
}

/// Token limits for a model.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelLimit {
    pub context: Option<u64>,
    pub input: Option<u64>,
    pub output: Option<u64>,
}

/// Input and output modalities for a model.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelModalities {
    pub input: Option<Vec<ModelModality>>,
    pub output: Option<Vec<ModelModality>>,
}

/// A model record as it appears in the JSON data, before flattening.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelRecord {
    pub id: String,
    pub name: Option<String>,
    pub knowledge_cutoff: Option<String>,
    pub release_date: Option<String>,
    pub last_updated: Option<String>,
    pub open_weights: Option<bool>,
    pub features: Option<ModelFeatures>,
    pub pricing: Option<ModelPricing>,
    pub limit: Option<ModelLimit>,
    pub modalities: Option<ModelModalities>,
}

/// A model flattened with its provider ID attached.
///
/// This is the primary data type used throughout the library. Each
/// [`FlatModel`] corresponds to exactly one model from one provider.
#[derive(Debug, Clone)]
pub struct FlatModel {
    pub id: String,
    pub name: Option<String>,
    pub knowledge_cutoff: Option<String>,
    pub release_date: Option<String>,
    pub last_updated: Option<String>,
    pub open_weights: Option<bool>,
    pub features: Option<ModelFeatures>,
    pub pricing: Option<ModelPricing>,
    pub limit: Option<ModelLimit>,
    pub modalities: Option<ModelModalities>,
    pub provider: String,
}

/// A provider entry as it appears in the JSON data.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderEntry {
    pub id: String,
    pub name: String,
    pub website: Option<String>,
    pub api_base_url: Option<String>,
    pub models: HashMap<String, ModelRecord>,
}

pub type ModelDirectory = HashMap<String, ProviderEntry>;

/// Query parameters for routing models.
///
/// All fields are optional; unset fields are not used as filters.
/// Use [`RouteQuery::default`] to start with no filters and set only what you need.
///
/// # Example
///
/// ```no_run
/// use ai_model_directory_router::RouteQuery;
///
/// let query = RouteQuery {
///     provider: Some("anthropic".to_string()),
///     min_context: Some(100_000),
///     limit: Some(10),
///     ..RouteQuery::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct RouteQuery {
    pub provider: Option<String>,
    pub input_modalities: Option<Vec<ModelModality>>,
    pub output_modalities: Option<Vec<ModelModality>>,
    pub features: Option<ModelFeatures>,
    pub min_context: Option<u64>,
    pub max_input_price: Option<f64>,
    pub max_output_price: Option<f64>,
    pub open_weights: Option<bool>,
    pub sort: Option<SortField>,
    pub order: Option<SortOrder>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Field to sort route results by.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum SortField {
    #[default]
    Id,
    Context,
    InputPrice,
    OutputPrice,
}

/// Sort direction for route results.
#[derive(Debug, Clone, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Paginated result from a route query.
#[derive(Debug, Clone)]
pub struct RouteResult {
    pub models: Vec<FlatModel>,
    pub total: usize,
    pub has_more: bool,
}

/// Token counts for a cost calculation request.
#[derive(Debug, Clone, Default)]
pub struct CostRequest {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub input_audio_tokens: Option<u64>,
    pub output_audio_tokens: Option<u64>,
}

/// Breakdown of costs for a single model, in USD.
///
/// All fields use [`rust_decimal::Decimal`] for exact arithmetic.
/// Prices are in USD (not per million tokens; already scaled).
#[derive(Debug, Clone)]
pub struct CostBreakdown {
    pub input: rust_decimal::Decimal,
    pub output: rust_decimal::Decimal,
    pub reasoning: rust_decimal::Decimal,
    pub cache_read: rust_decimal::Decimal,
    pub cache_write: rust_decimal::Decimal,
    pub input_audio: rust_decimal::Decimal,
    pub output_audio: rust_decimal::Decimal,
    pub total: rust_decimal::Decimal,
}

/// Options controlling fallback chain generation.
#[derive(Debug, Clone)]
pub struct FallbackOptions {
    pub match_features: Option<bool>,
    pub match_modalities: Option<bool>,
    pub max_context_difference: Option<u64>,
    pub max_price_multiplier: Option<f64>,
    pub limit: Option<usize>,
}

impl Default for FallbackOptions {
    fn default() -> Self {
        Self {
            match_features: None,
            match_modalities: None,
            max_context_difference: None,
            max_price_multiplier: None,
            limit: Some(10),
        }
    }
}

/// A scored list of fallback models, ordered by similarity to the original.
#[derive(Debug, Clone)]
pub struct FallbackChain {
    pub models: Vec<FlatModel>,
    pub original: FlatModel,
}

/// Result of checking whether a prompt fits within a model's context window.
#[derive(Debug, Clone)]
pub struct ContextFit {
    pub fits: bool,
    pub model: FlatModel,
    pub available_context: u64,
    pub requested_tokens: u64,
    pub overhead: i64,
    pub should_compact: bool,
    pub better_alternatives: Vec<FlatModel>,
}

/// One field in a model comparison, with values per model and an optional winner.
#[derive(Debug, Clone)]
pub struct ComparisonField {
    pub field: String,
    pub values: HashMap<String, FieldValue>,
    pub winner: Option<String>,
}

/// A value in a comparison field. Can be text, a number, or a boolean.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FieldValue {
    Text(Option<String>),
    Number(Option<f64>),
    Bool(Option<bool>),
}

/// The result of comparing two or more models side by side.
#[derive(Debug, Clone)]
pub struct ModelComparison {
    pub models: Vec<FlatModel>,
    pub fields: Vec<ComparisonField>,
}

/// Errors that can occur when loading model data or looking up models.
#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("data file not found: {0}")]
    DataFileNotFound(String),
    #[error("failed to parse data: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("failed to read data: {0}")]
    IoError(#[from] std::io::Error),
    #[error("model not found: {0}")]
    ModelNotFound(String),
}
