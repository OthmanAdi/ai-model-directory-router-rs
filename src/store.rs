use crate::overlay::{
    apply_overlay, load_models_dev_from_file, parse_models_dev, parse_models_dev_directory,
    validate_provider_identity, ModelsDevIndex, OverlayMode, OverlayReport,
};
use crate::types::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
#[cfg(feature = "bundled")]
use std::sync::OnceLock;
#[cfg(feature = "online")]
use std::time::Duration;

/// The fixed public provider-catalog endpoint used by the online loader.
pub const MODELS_DEV_API_URL: &str = "https://models.dev/api.json";

/// Maximum accepted response body for [`RouterStore::from_models_dev_live`].
#[cfg(feature = "online")]
pub const MODELS_DEV_BODY_LIMIT: usize = 16 * 1024 * 1024;

// These constants describe the exact bytes embedded by the `bundled` feature.
// Refresh all of them together when `data/models-dev-api.json` changes.
#[cfg(feature = "bundled")]
pub const BUNDLED_MODELS_DEV_RETRIEVED_AT: &str = "2026-09-04T07:53:34Z";
#[cfg(feature = "bundled")]
pub const BUNDLED_MODELS_DEV_SHA256: &str =
    "c68bb1a66b2260432a30862fa0c759ce08bde20322e2c949dee9c615c3cc4c8f";
#[cfg(feature = "bundled")]
pub const BUNDLED_MODELS_DEV_ETAG: &str = "\"c68bb1a66b2260432a30862fa0c759ce\"";
#[cfg(feature = "bundled")]
pub const BUNDLED_MODELS_DEV_SOURCE_REVISION: &str = "ba816a069564b9dbf7c61cc117a79301b3f9ecdc";
#[cfg(feature = "bundled")]
pub const BUNDLED_MODELS_DEV_BYTE_COUNT: usize = 4_465_310;
#[cfg(feature = "bundled")]
pub const BUNDLED_MODELS_DEV_PROVIDER_COUNT: usize = 213;
#[cfg(feature = "bundled")]
pub const BUNDLED_MODELS_DEV_MODEL_COUNT: usize = 7_526;

#[cfg(feature = "bundled")]
static STORE: OnceLock<RouterStore> = OnceLock::new();

/// A deterministic, provider-aware model catalog.
pub struct RouterStore {
    flat_models: Vec<FlatModel>,
    providers: Vec<ProviderMetadata>,
    models_by_key: BTreeMap<ModelKey, usize>,
    models_by_id: BTreeMap<String, Vec<usize>>,
    catalog_metadata: CatalogMetadata,
}

impl RouterStore {
    /// Parse the legacy AI Model Directory JSON format.
    pub fn from_json(json: &str) -> Result<Self, RouterError> {
        let directory: ModelDirectory = serde_json::from_str(json)?;
        for (provider_key, provider) in &directory {
            validate_provider_identity(provider_key, provider)?;
        }
        Ok(Self::from_directory(
            directory,
            CatalogSource::AiModelDirectory,
            None,
            None,
            None,
            None,
            Some(sha256(json.as_bytes())),
            Some(json.len()),
        ))
    }

    /// Load the legacy AI Model Directory JSON format from disk.
    pub fn from_file(path: &Path) -> Result<Self, RouterError> {
        let json = read_data_file(path)?;
        Self::from_json(&json)
    }

    /// Parse the provider-scoped models.dev `api.json` format directly.
    ///
    /// This constructor includes every provider offering in the source. Unlike
    /// an overlay, it does not require a separate base catalog.
    pub fn from_models_dev_json(json: &str) -> Result<Self, RouterError> {
        let directory = parse_models_dev_directory(json)?;
        Ok(Self::from_directory(
            directory,
            CatalogSource::ModelsDevInline,
            None,
            None,
            None,
            None,
            Some(sha256(json.as_bytes())),
            Some(json.len()),
        ))
    }

    /// Load a provider-scoped models.dev `api.json` file from disk.
    pub fn from_models_dev_file(path: &Path) -> Result<Self, RouterError> {
        let json = read_data_file(path)?;
        let directory = parse_models_dev_directory(&json)?;
        Ok(Self::from_directory(
            directory,
            CatalogSource::ModelsDevFile,
            None,
            None,
            None,
            None,
            Some(sha256(json.as_bytes())),
            Some(json.len()),
        ))
    }

    /// Parse the exact models.dev snapshot embedded in this crate.
    #[cfg(feature = "bundled")]
    pub fn bundled() -> Result<Self, RouterError> {
        let json = include_str!("../data/models-dev-api.json");
        let directory = parse_models_dev_directory(json)?;
        let actual_hash = sha256(json.as_bytes());
        if json.len() != BUNDLED_MODELS_DEV_BYTE_COUNT {
            return Err(RouterError::CatalogIntegrityError(format!(
                "bundled models.dev byte count: expected {BUNDLED_MODELS_DEV_BYTE_COUNT}, got {}",
                json.len()
            )));
        }
        if actual_hash != BUNDLED_MODELS_DEV_SHA256 {
            return Err(RouterError::CatalogIntegrityError(format!(
                "bundled models.dev SHA-256: expected {BUNDLED_MODELS_DEV_SHA256}, got {actual_hash}"
            )));
        }

        let store = Self::from_directory(
            directory,
            CatalogSource::ModelsDevBundled,
            Some(MODELS_DEV_API_URL.to_owned()),
            Some(BUNDLED_MODELS_DEV_RETRIEVED_AT.to_owned()),
            Some(BUNDLED_MODELS_DEV_ETAG.to_owned()),
            Some(BUNDLED_MODELS_DEV_SOURCE_REVISION.to_owned()),
            Some(actual_hash),
            Some(json.len()),
        );
        let metadata = store.catalog_metadata();
        if metadata.provider_count != BUNDLED_MODELS_DEV_PROVIDER_COUNT
            || metadata.model_count != BUNDLED_MODELS_DEV_MODEL_COUNT
        {
            return Err(RouterError::CatalogIntegrityError(format!(
                "bundled models.dev record counts: expected {BUNDLED_MODELS_DEV_PROVIDER_COUNT} providers and {BUNDLED_MODELS_DEV_MODEL_COUNT} offerings, got {} providers and {} offerings",
                metadata.provider_count, metadata.model_count
            )));
        }
        Ok(store)
    }

    /// Return the process-wide embedded catalog.
    #[cfg(feature = "bundled")]
    pub fn global() -> &'static RouterStore {
        STORE.get_or_init(|| {
            Self::bundled().expect("embedded models.dev catalog must be valid and match its hash")
        })
    }

    /// Fetch and parse the current public models.dev provider catalog.
    ///
    /// The request is always sent to [`MODELS_DEV_API_URL`], accepts at most
    /// 16 MiB, and reads no provider credential or API-key environment
    /// variables.
    #[cfg(feature = "online")]
    pub fn from_models_dev_live() -> Result<Self, RouterError> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .max_redirects(0)
            .build()
            .into();
        let mut response =
            agent
                .get(MODELS_DEV_API_URL)
                .call()
                .map_err(|error| RouterError::FetchError {
                    url: MODELS_DEV_API_URL.to_owned(),
                    reason: error.to_string(),
                })?;
        if !response.status().is_success() {
            return Err(RouterError::FetchError {
                url: MODELS_DEV_API_URL.to_owned(),
                reason: format!("unexpected HTTP status {}", response.status()),
            });
        }

        let headers = response.headers();
        let etag = header_value(headers, "etag");
        let retrieved_at = header_value(headers, "date");
        if let Some(content_length) =
            header_value(headers, "content-length").and_then(|value| value.parse::<usize>().ok())
        {
            if content_length > MODELS_DEV_BODY_LIMIT {
                return Err(RouterError::CatalogTooLarge {
                    limit: MODELS_DEV_BODY_LIMIT,
                });
            }
        }

        let json = response
            .body_mut()
            .with_config()
            .limit((MODELS_DEV_BODY_LIMIT + 1) as u64)
            .read_to_string()
            .map_err(|error| {
                let message = error.to_string();
                if message.to_ascii_lowercase().contains("limit") {
                    RouterError::CatalogTooLarge {
                        limit: MODELS_DEV_BODY_LIMIT,
                    }
                } else {
                    RouterError::NetworkError(message)
                }
            })?;
        if json.len() > MODELS_DEV_BODY_LIMIT {
            return Err(RouterError::CatalogTooLarge {
                limit: MODELS_DEV_BODY_LIMIT,
            });
        }

        let hash = sha256(json.as_bytes());
        let directory = parse_models_dev_directory(&json)?;
        Ok(Self::from_directory(
            directory,
            CatalogSource::ModelsDevLive,
            Some(MODELS_DEV_API_URL.to_owned()),
            retrieved_at,
            etag,
            None,
            Some(hash),
            Some(json.len()),
        ))
    }

    /// All model offerings, sorted by provider and then model ID.
    pub fn flat_models(&self) -> &[FlatModel] {
        &self.flat_models
    }

    /// Look up one exact provider offering.
    pub fn find_model_in(&self, provider_id: &str, model_id: &str) -> Option<&FlatModel> {
        let key = ModelKey::new(provider_id, model_id);
        self.models_by_key
            .get(&key)
            .map(|index| &self.flat_models[*index])
    }

    /// Return every provider offering with this exact model ID.
    pub fn find_models_by_id(&self, model_id: &str) -> Vec<&FlatModel> {
        self.models_by_id
            .get(model_id)
            .map(|indices| {
                indices
                    .iter()
                    .map(|index| &self.flat_models[*index])
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Resolve a bare model ID only when it identifies exactly one offering.
    pub fn resolve_model(&self, model_id: &str) -> Result<&FlatModel, RouterError> {
        let matches = self.find_models_by_id(model_id);
        match matches.as_slice() {
            [] => Err(RouterError::ModelNotFound(model_id.to_owned())),
            [model] => Ok(*model),
            models => Err(RouterError::AmbiguousModel {
                model_id: model_id.to_owned(),
                providers: models.iter().map(|model| model.provider.clone()).collect(),
            }),
        }
    }

    /// Legacy bare-ID lookup that returns `Some` only for a unique offering.
    /// Use [`RouterStore::resolve_model`] to distinguish missing and ambiguous.
    pub fn find_model(&self, model_id: &str) -> Option<&FlatModel> {
        self.resolve_model(model_id).ok()
    }

    /// All models belonging to a provider, preserving the v0.2
    /// case-insensitive convenience behavior.
    pub fn find_models_by_provider(&self, provider_id: &str) -> Vec<&FlatModel> {
        self.flat_models
            .iter()
            .filter(|model| model.provider.eq_ignore_ascii_case(provider_id))
            .collect()
    }

    /// Provider records, sorted by provider ID.
    pub fn providers(&self) -> &[ProviderMetadata] {
        &self.providers
    }

    /// Look up provider metadata by exact provider ID.
    pub fn find_provider(&self, provider_id: &str) -> Option<&ProviderMetadata> {
        self.providers
            .binary_search_by(|provider| provider.id.as_str().cmp(provider_id))
            .ok()
            .map(|index| &self.providers[index])
    }

    /// Provenance and size information for this catalog.
    pub fn catalog_metadata(&self) -> &CatalogMetadata {
        &self.catalog_metadata
    }

    /// Enrich only existing offerings from an exact provider-qualified index.
    /// Overlays never add models; use `from_models_dev_*` for a current source.
    pub fn apply_overlay(&mut self, overlay: &ModelsDevIndex, mode: OverlayMode) -> OverlayReport {
        apply_overlay(&mut self.flat_models, overlay, mode)
    }

    /// Parse and apply a models.dev overlay to existing exact offerings.
    pub fn apply_overlay_from_json(
        &mut self,
        json: &str,
        mode: OverlayMode,
    ) -> Result<OverlayReport, RouterError> {
        let overlay = parse_models_dev(json)?;
        Ok(apply_overlay(&mut self.flat_models, &overlay, mode))
    }

    /// Load and apply a models.dev overlay to existing exact offerings.
    pub fn apply_overlay_from_file(
        &mut self,
        path: &Path,
        mode: OverlayMode,
    ) -> Result<OverlayReport, RouterError> {
        let overlay = load_models_dev_from_file(path)?;
        Ok(apply_overlay(&mut self.flat_models, &overlay, mode))
    }

    #[allow(clippy::too_many_arguments)]
    fn from_directory(
        directory: ModelDirectory,
        source: CatalogSource,
        source_url: Option<String>,
        retrieved_at: Option<String>,
        etag: Option<String>,
        source_revision: Option<String>,
        sha256: Option<String>,
        byte_count: Option<usize>,
    ) -> Self {
        let mut providers = Vec::with_capacity(directory.len());
        let mut flat_models = Vec::new();

        for provider in directory.into_values() {
            let metadata = provider_metadata(&provider);
            for model in provider.models.into_values() {
                flat_models.push(flatten_model(model, &metadata));
            }
            providers.push(metadata);
        }

        providers.sort_by(|left, right| left.id.cmp(&right.id));
        flat_models.sort_by(|left, right| {
            left.provider
                .cmp(&right.provider)
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut models_by_key = BTreeMap::new();
        let mut models_by_id: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, model) in flat_models.iter().enumerate() {
            models_by_key.insert(model.key(), index);
            models_by_id
                .entry(model.id.clone())
                .or_default()
                .push(index);
        }

        let catalog_metadata = CatalogMetadata {
            source,
            source_url,
            retrieved_at,
            sha256,
            etag,
            source_revision,
            byte_count,
            provider_count: providers.len(),
            model_count: flat_models.len(),
        };

        Self {
            flat_models,
            providers,
            models_by_key,
            models_by_id,
            catalog_metadata,
        }
    }
}

fn provider_metadata(provider: &ProviderEntry) -> ProviderMetadata {
    ProviderMetadata {
        id: provider.id.clone(),
        name: provider.name.clone(),
        doc: provider.doc.clone(),
        website: provider.website.clone(),
        api: provider
            .api
            .clone()
            .or_else(|| provider.api_base_url.clone()),
        npm: provider.npm.clone(),
        env: provider.env.clone(),
    }
}

fn flatten_model(model: ModelRecord, provider: &ProviderMetadata) -> FlatModel {
    let features = merged_features(&model);
    FlatModel {
        id: model.id,
        name: model.name,
        description: model.description,
        family: model.family,
        knowledge_cutoff: model.knowledge_cutoff,
        release_date: model.release_date,
        last_updated: model.last_updated,
        open_weights: model.open_weights,
        features,
        reasoning_options: model.reasoning_options,
        interleaved: model.interleaved,
        pricing: model.pricing,
        limit: model.limit,
        modalities: model.modalities,
        status: model.status,
        experimental: model.experimental,
        provider_options: model.provider_options,
        provider: provider.id.clone(),
        provider_metadata: Some(provider.clone()),
    }
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

fn read_data_file(path: &Path) -> Result<String, RouterError> {
    if !path.is_file() {
        return Err(RouterError::DataFileNotFound(
            path.to_string_lossy().into_owned(),
        ));
    }
    Ok(fs::read_to_string(path)?)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(feature = "online")]
fn header_value(headers: &ureq::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}
