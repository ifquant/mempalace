use std::sync::Arc;

use crate::config::{EmbeddingBackend, EmbeddingSettings};
use crate::error::Result;
use crate::model::DoctorSummary;

#[path = "embed_fastembed.rs"]
mod embed_fastembed;
#[path = "embed_hash.rs"]
mod embed_hash;
#[path = "embed_runtime_env.rs"]
mod embed_runtime_env;

pub use embed_fastembed::FastEmbedder;
pub use embed_hash::HashEmbedder;
pub(crate) use embed_runtime_env::{
    configure_hf_endpoint, configure_ort_dylib_path, detect_ort_dylib_path, expected_model_file,
    model_cache_ready,
};

pub const HASH_EMBEDDING_DIM: usize = 64;
pub const HASH_PROVIDER_NAME: &str = "hash";
pub const HASH_MODEL_NAME: &str = "hash-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddingProfile {
    pub provider: String,
    pub model: String,
    pub dimension: usize,
}

impl EmbeddingProfile {
    pub fn legacy_hash() -> Self {
        Self {
            provider: HASH_PROVIDER_NAME.to_string(),
            model: HASH_MODEL_NAME.to_string(),
            dimension: HASH_EMBEDDING_DIM,
        }
    }
}

pub trait EmbeddingProvider: Send + Sync {
    fn profile(&self) -> &EmbeddingProfile;
    fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f32>>>;
    fn embed_query(&self, query: &str) -> Result<Vec<f32>>;
    fn doctor(&self, palace_path: &str, warmup: bool) -> DoctorSummary;
}

pub fn build_embedder(settings: &EmbeddingSettings) -> Result<Arc<dyn EmbeddingProvider>> {
    match settings.backend {
        EmbeddingBackend::Hash => Ok(Arc::new(HashEmbedder::new())),
        EmbeddingBackend::Fastembed => Ok(Arc::new(FastEmbedder::new(
            settings.model.clone(),
            settings.cache_dir.clone(),
            settings.hf_endpoint.clone(),
            settings.show_download_progress,
        )?)),
    }
}
