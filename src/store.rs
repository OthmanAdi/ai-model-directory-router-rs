use crate::types::*;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static STORE: OnceLock<RouterStore> = OnceLock::new();

pub struct RouterStore {
    flat_models: Vec<FlatModel>,
}

impl RouterStore {
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

    pub fn from_file(path: &std::path::Path) -> Result<Self, RouterError> {
        let json = fs::read_to_string(path)?;
        Self::from_json(&json)
    }

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

    pub fn flat_models(&self) -> &[FlatModel] {
        &self.flat_models
    }

    pub fn find_model(&self, model_id: &str) -> Option<&FlatModel> {
        self.flat_models.iter().find(|m| m.id == model_id)
    }

    pub fn find_models_by_provider(&self, provider_id: &str) -> Vec<&FlatModel> {
        let lower = provider_id.to_lowercase();
        self.flat_models
            .iter()
            .filter(|m| m.provider.to_lowercase() == lower)
            .collect()
    }
}
