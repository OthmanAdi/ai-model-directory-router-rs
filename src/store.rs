use crate::overlay::{
    apply_overlay, load_models_dev_from_file, parse_models_dev, ModelsDevIndex, OverlayMode,
    OverlayReport,
};
use crate::types::*;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static STORE: OnceLock<RouterStore> = OnceLock::new();

/// The primary entry point for all operations. Holds flattened model data
/// loaded from the AI Model Directory JSON file.
///
/// # Construction
///
/// - [`RouterStore::from_json`] parses a JSON string directly.
/// - [`RouterStore::from_file`] reads and parses a JSON file from disk.
/// - [`RouterStore::global`] returns a process-wide singleton, reading from
///   `data/all.min.json` or `data/all.json`.
///
/// # Example
///
/// ```no_run
/// use ai_model_directory_router::RouterStore;
/// use std::path::Path;
///
/// let store = RouterStore::from_file(Path::new("data/all.min.json")).unwrap();
/// println!("Loaded {} models", store.flat_models().len());
/// ```
pub struct RouterStore {
    flat_models: Vec<FlatModel>,
}

impl RouterStore {
    /// Parse model data from a JSON string.
    ///
    /// The JSON must be a map of provider ID to [`ProviderEntry`], matching
    /// the format of `all.min.json` from the AI Model Directory.
    pub fn from_json(json: &str) -> Result<Self, RouterError> {
        let directory: ModelDirectory = serde_json::from_str(json)?;
        let mut flat_models = Vec::new();

        for provider in directory.values() {
            for model in provider.models.values() {
                flat_models.push(FlatModel {
                    id: model.id.clone(),
                    name: model.name.clone(),
                    knowledge_cutoff: model.knowledge_cutoff.clone(),
                    release_date: model.release_date.clone(),
                    last_updated: model.last_updated.clone(),
                    open_weights: model.open_weights,
                    features: model.features.clone(),
                    pricing: model.pricing.clone(),
                    limit: model.limit.clone(),
                    modalities: model.modalities.clone(),
                    provider: provider.id.clone(),
                });
            }
        }

        Ok(Self { flat_models })
    }

    /// Load model data from a JSON file on disk.
    pub fn from_file(path: &std::path::Path) -> Result<Self, RouterError> {
        let json = fs::read_to_string(path)?;
        Self::from_json(&json)
    }

    /// Returns a process-wide singleton [`RouterStore`].
    ///
    /// Searches for `data/all.min.json` then `data/all.json` relative to the
    /// current working directory. **Panics** if neither file exists or if
    /// parsing fails. This matches the TypeScript version's behavior.
    pub fn global() -> &'static RouterStore {
        STORE.get_or_init(|| {
            let candidates = [
                PathBuf::from("data/all.min.json"),
                PathBuf::from("data/all.json"),
            ];

            for candidate in &candidates {
                if candidate.exists() {
                    match RouterStore::from_file(candidate) {
                        Ok(store) => return store,
                        Err(e) => panic!("Failed to load model data: {}", e),
                    }
                }
            }

            panic!("Cannot find data/all.min.json or data/all.json")
        })
    }

    /// All models, flattened with their provider ID attached.
    pub fn flat_models(&self) -> &[FlatModel] {
        &self.flat_models
    }

    /// Look up a model by its ID (e.g. `"gpt-4o"`).
    pub fn find_model(&self, model_id: &str) -> Option<&FlatModel> {
        self.flat_models.iter().find(|m| m.id == model_id)
    }

    /// All models belonging to a given provider. Case insensitive.
    pub fn find_models_by_provider(&self, provider_id: &str) -> Vec<&FlatModel> {
        let lower = provider_id.to_lowercase();
        self.flat_models
            .iter()
            .filter(|m| m.provider.to_lowercase() == lower)
            .collect()
    }

    /// Enrich models in place from a parsed models.dev catalog.
    ///
    /// Returns an [`OverlayReport`] describing how many models were touched
    /// and how many fields were written. See [`crate::overlay`] for the
    /// schema mapping and merge semantics.
    pub fn apply_overlay(
        &mut self,
        overlay: &ModelsDevIndex,
        mode: OverlayMode,
    ) -> OverlayReport {
        apply_overlay(&mut self.flat_models, overlay, mode)
    }

    /// Enrich models in place from a models.dev catalog JSON string.
    pub fn apply_overlay_from_json(
        &mut self,
        json: &str,
        mode: OverlayMode,
    ) -> Result<OverlayReport, RouterError> {
        let overlay = parse_models_dev(json)?;
        Ok(apply_overlay(&mut self.flat_models, &overlay, mode))
    }

    /// Enrich models in place from a models.dev catalog file on disk.
    pub fn apply_overlay_from_file(
        &mut self,
        path: &std::path::Path,
        mode: OverlayMode,
    ) -> Result<OverlayReport, RouterError> {
        let overlay = load_models_dev_from_file(path)?;
        Ok(apply_overlay(&mut self.flat_models, &overlay, mode))
    }
}
