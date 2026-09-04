use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fmt;

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
    Pdf,
    #[serde(other)]
    Unknown,
}

/// Feature flags describing what a model can do.
#[derive(Debug, Clone, Default, Deserialize)]
#[non_exhaustive]
pub struct ModelFeatures {
    pub attachment: Option<bool>,
    pub reasoning: Option<bool>,
    pub tool_call: Option<bool>,
    pub structured_output: Option<bool>,
    pub temperature: Option<bool>,
}

/// Exact per-million-token price rates in USD.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct ModelPriceRates {
    #[serde(default, with = "rust_decimal::serde::arbitrary_precision_option")]
    pub input: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::arbitrary_precision_option")]
    pub output: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::arbitrary_precision_option")]
    pub reasoning: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::arbitrary_precision_option")]
    pub cache_read: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::arbitrary_precision_option")]
    pub cache_write: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::arbitrary_precision_option")]
    pub input_audio: Option<Decimal>,
    #[serde(default, with = "rust_decimal::serde::arbitrary_precision_option")]
    pub output_audio: Option<Decimal>,
}

/// The kind of threshold selecting a price tier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ModelPriceTierKind {
    #[default]
    Context,
    #[serde(other)]
    Unknown,
}

/// A threshold selecting a price tier.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct ModelPriceTierThreshold {
    #[serde(rename = "type", default)]
    pub kind: ModelPriceTierKind,
    pub size: u64,
}

/// Price rates that apply at and above a context threshold.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct ModelPriceTier {
    #[serde(flatten)]
    pub rates: ModelPriceRates,
    pub tier: ModelPriceTierThreshold,
}

/// Per-million-token prices in USD for a model.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct ModelPricing {
    #[serde(flatten)]
    pub rates: ModelPriceRates,
    #[serde(default)]
    pub context_over_200k: Option<ModelPriceRates>,
    #[serde(default)]
    pub tiers: Vec<ModelPriceTier>,
}

/// Token limits for a model.
#[derive(Debug, Clone, Default, Deserialize)]
#[non_exhaustive]
pub struct ModelLimit {
    pub context: Option<u64>,
    pub input: Option<u64>,
    pub output: Option<u64>,
}

/// Input and output modalities for a model.
#[derive(Debug, Clone, Default, Deserialize)]
#[non_exhaustive]
pub struct ModelModalities {
    pub input: Option<Vec<ModelModality>>,
    pub output: Option<Vec<ModelModality>>,
}

/// Lifecycle state reported by a model catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ModelStatus {
    Alpha,
    Beta,
    Deprecated,
    #[serde(other)]
    Unknown,
}

/// A supported reasoning effort value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    Default,
    #[serde(other)]
    Unknown,
}

/// A reasoning control supported by a model.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReasoningOption {
    Toggle,
    Effort {
        values: Vec<Option<ReasoningEffort>>,
    },
    BudgetTokens {
        min: Option<i64>,
        max: Option<u64>,
    },
    #[serde(other)]
    Unknown,
}

/// The response field used for interleaved reasoning content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InterleavedReasoningField {
    ReasoningContent,
    ReasoningDetails,
    #[serde(other)]
    Unknown,
}

/// Interleaved reasoning support reported by a model catalog.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum InterleavedReasoning {
    Enabled(bool),
    Field { field: InterleavedReasoningField },
}

/// A provider-qualified model identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelKey {
    pub provider: String,
    pub id: String,
}

impl ModelKey {
    /// Construct a provider-qualified model identity.
    pub fn new(provider: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            id: id.into(),
        }
    }
}

impl fmt::Display for ModelKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.provider, self.id)
    }
}

/// Provider metadata retained from a source catalog.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct ProviderMetadata {
    pub id: String,
    pub name: String,
    pub doc: Option<String>,
    pub website: Option<String>,
    pub api: Option<String>,
    pub npm: Option<String>,
    #[serde(default)]
    pub env: Vec<String>,
}

/// A model record as it appears in JSON data, before flattening.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ModelRecord {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub family: Option<String>,
    #[serde(alias = "knowledge")]
    pub knowledge_cutoff: Option<String>,
    pub release_date: Option<String>,
    pub last_updated: Option<String>,
    pub open_weights: Option<bool>,
    pub features: Option<ModelFeatures>,
    pub attachment: Option<bool>,
    pub reasoning: Option<bool>,
    pub tool_call: Option<bool>,
    pub structured_output: Option<bool>,
    pub temperature: Option<bool>,
    pub reasoning_options: Option<Vec<ReasoningOption>>,
    pub interleaved: Option<InterleavedReasoning>,
    #[serde(alias = "cost")]
    pub pricing: Option<ModelPricing>,
    pub limit: Option<ModelLimit>,
    pub modalities: Option<ModelModalities>,
    pub status: Option<ModelStatus>,
    pub experimental: Option<Value>,
    #[serde(rename = "provider")]
    pub provider_options: Option<Value>,
}

/// A model flattened with its provider identity and metadata attached.
#[derive(Debug, Clone)]
pub struct FlatModel {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub family: Option<String>,
    pub knowledge_cutoff: Option<String>,
    pub release_date: Option<String>,
    pub last_updated: Option<String>,
    pub open_weights: Option<bool>,
    pub features: Option<ModelFeatures>,
    pub reasoning_options: Option<Vec<ReasoningOption>>,
    pub interleaved: Option<InterleavedReasoning>,
    pub pricing: Option<ModelPricing>,
    pub limit: Option<ModelLimit>,
    pub modalities: Option<ModelModalities>,
    pub status: Option<ModelStatus>,
    pub experimental: Option<Value>,
    pub provider_options: Option<Value>,
    pub provider: String,
    pub provider_metadata: Option<ProviderMetadata>,
}

impl FlatModel {
    /// Return this offering's provider-qualified identity.
    pub fn key(&self) -> ModelKey {
        ModelKey::new(self.provider.clone(), self.id.clone())
    }
}

impl From<&FlatModel> for ModelKey {
    fn from(model: &FlatModel) -> Self {
        model.key()
    }
}

/// A provider entry as it appears in JSON data.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ProviderEntry {
    pub id: String,
    pub name: String,
    pub website: Option<String>,
    pub api_base_url: Option<String>,
    pub api: Option<String>,
    pub npm: Option<String>,
    pub doc: Option<String>,
    #[serde(default)]
    pub env: Vec<String>,
    pub models: HashMap<String, ModelRecord>,
}

pub type ModelDirectory = HashMap<String, ProviderEntry>;

/// The origin of a loaded catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CatalogSource {
    AiModelDirectory,
    ModelsDevInline,
    ModelsDevFile,
    ModelsDevBundled,
    ModelsDevLive,
}

/// Provenance and size metadata for a loaded catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CatalogMetadata {
    pub source: CatalogSource,
    pub source_url: Option<String>,
    pub retrieved_at: Option<String>,
    pub sha256: Option<String>,
    pub etag: Option<String>,
    pub source_revision: Option<String>,
    pub byte_count: Option<usize>,
    pub provider_count: usize,
    pub model_count: usize,
}

/// Query parameters for routing models.
#[derive(Debug, Clone, Default)]
pub struct RouteQuery {
    pub provider: Option<String>,
    pub model_id: Option<String>,
    pub family: Option<String>,
    pub status: Option<ModelStatus>,
    pub input_modalities: Option<Vec<ModelModality>>,
    pub output_modalities: Option<Vec<ModelModality>>,
    pub features: Option<ModelFeatures>,
    pub min_context: Option<u64>,
    pub max_input_price: Option<Decimal>,
    pub max_output_price: Option<Decimal>,
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
#[non_exhaustive]
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
    pub context_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub input_audio_tokens: Option<u64>,
    pub output_audio_tokens: Option<u64>,
}

/// Breakdown of exact costs for a single model, in USD.
#[derive(Debug, Clone, PartialEq)]
pub struct CostBreakdown {
    pub input: Decimal,
    pub output: Decimal,
    pub reasoning: Decimal,
    pub cache_read: Decimal,
    pub cache_write: Decimal,
    pub input_audio: Decimal,
    pub output_audio: Decimal,
    pub total: Decimal,
    pub applied_tier: Option<u64>,
}

/// Options controlling fallback chain generation.
#[derive(Debug, Clone)]
pub struct FallbackOptions {
    pub match_features: Option<bool>,
    pub match_modalities: Option<bool>,
    pub max_context_difference: Option<u64>,
    pub max_price_multiplier: Option<Decimal>,
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

/// Result of checking whether a request fits a model's token limits.
#[derive(Debug, Clone)]
pub struct ContextFit {
    pub fits: bool,
    pub context_fits: bool,
    pub input_fits: bool,
    pub output_fits: bool,
    pub model: FlatModel,
    pub available_context: u64,
    pub requested_tokens: u64,
    pub overhead: i128,
    pub should_compact: bool,
    pub better_alternatives: Vec<FlatModel>,
}

/// One field in a model comparison, with values and winners by offering.
#[derive(Debug, Clone)]
pub struct ComparisonField {
    pub field: String,
    pub values: BTreeMap<ModelKey, FieldValue>,
    pub winners: Vec<ModelKey>,
}

/// A typed value in a model comparison field.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FieldValue {
    Text(Option<String>),
    Integer(Option<u64>),
    Decimal(Option<Decimal>),
    Bool(Option<bool>),
}

/// The result of comparing two or more models side by side.
#[derive(Debug, Clone)]
pub struct ModelComparison {
    pub models: Vec<FlatModel>,
    pub fields: Vec<ComparisonField>,
}

/// A priced usage component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CostComponent {
    Input,
    Output,
    Reasoning,
    CacheRead,
    CacheWrite,
    InputAudio,
    OutputAudio,
}

impl fmt::Display for CostComponent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Input => "input",
            Self::Output => "output",
            Self::Reasoning => "reasoning",
            Self::CacheRead => "cache_read",
            Self::CacheWrite => "cache_write",
            Self::InputAudio => "input_audio",
            Self::OutputAudio => "output_audio",
        };
        formatter.write_str(name)
    }
}

/// Errors that can occur while loading, querying, or costing model data.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RouterError {
    #[error("data file not found: {0}")]
    DataFileNotFound(String),
    #[error("failed to parse data: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("invalid catalog identity: {0}")]
    InvalidCatalogIdentity(String),
    #[error("invalid catalog value: {0}")]
    InvalidCatalogValue(String),
    #[error("catalog integrity check failed: {0}")]
    CatalogIntegrityError(String),
    #[error("failed to read data: {0}")]
    IoError(#[from] std::io::Error),
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("model ID {model_id} is ambiguous across providers: {providers:?}")]
    AmbiguousModel {
        model_id: String,
        providers: Vec<String>,
    },
    #[error("model {model} has no {component} price for nonzero usage")]
    MissingPriceComponent {
        model: ModelKey,
        component: CostComponent,
    },
    #[error("model {model} has a negative {component} price: {rate}")]
    InvalidPriceComponent {
        model: ModelKey,
        component: CostComponent,
        rate: Decimal,
    },
    #[error("token count overflow")]
    TokenOverflow,
    #[error("network request failed: {0}")]
    NetworkError(String),
    #[error("failed to fetch {url}: {reason}")]
    FetchError { url: String, reason: String },
    #[error("catalog exceeds the {limit}-byte download limit")]
    CatalogTooLarge { limit: usize },
}
