use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelModality {
    Text,
    Image,
    Audio,
    Video,
    File,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelFeatures {
    pub attachment: Option<bool>,
    pub reasoning: Option<bool>,
    pub tool_call: Option<bool>,
    pub structured_output: Option<bool>,
    pub temperature: Option<bool>,
}

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

#[derive(Debug, Clone, Deserialize)]
pub struct ModelLimit {
    pub context: Option<u64>,
    pub input: Option<u64>,
    pub output: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelModalities {
    pub input: Option<Vec<ModelModality>>,
    pub output: Option<Vec<ModelModality>>,
}

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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum SortField {
    Context,
    InputPrice,
    OutputPrice,
    Id,
}

#[derive(Debug, Clone)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct RouteResult {
    pub models: Vec<FlatModel>,
    pub total: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
pub struct CostRequest {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub input_audio_tokens: Option<u64>,
    pub output_audio_tokens: Option<u64>,
}

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

#[derive(Debug, Clone)]
pub struct FallbackChain {
    pub models: Vec<FlatModel>,
    pub original: FlatModel,
}

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

#[derive(Debug, Clone)]
pub struct ComparisonField {
    pub field: String,
    pub values: HashMap<String, FieldValue>,
    pub winner: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Text(Option<String>),
    Number(Option<f64>),
    Bool(Option<bool>),
}

#[derive(Debug, Clone)]
pub struct ModelComparison {
    pub models: Vec<FlatModel>,
    pub fields: Vec<ComparisonField>,
}

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
