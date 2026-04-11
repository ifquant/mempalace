use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use blake3::Hasher;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use regex::Regex;

use crate::config::{EmbeddingBackend, EmbeddingSettings};
use crate::error::{MempalaceError, Result};

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
}

pub fn build_embedder(settings: &EmbeddingSettings) -> Result<Arc<dyn EmbeddingProvider>> {
    match settings.backend {
        EmbeddingBackend::Hash => Ok(Arc::new(HashEmbedder::new())),
        EmbeddingBackend::Fastembed => Ok(Arc::new(FastEmbedder::new(
            settings.model.clone(),
            settings.cache_dir.clone(),
            settings.show_download_progress,
        )?)),
    }
}

pub struct HashEmbedder {
    profile: EmbeddingProfile,
}

impl Default for HashEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

impl HashEmbedder {
    pub fn new() -> Self {
        Self {
            profile: EmbeddingProfile::legacy_hash(),
        }
    }

    fn embed_text(text: &str) -> Vec<f32> {
        let token_re = Regex::new(r"[A-Za-z0-9_]+").expect("token regex");
        let mut vector = vec![0.0_f32; HASH_EMBEDDING_DIM];
        let mut count = 0_u32;

        for token in token_re.find_iter(text) {
            let normalized = token.as_str().to_ascii_lowercase();
            if normalized.is_empty() {
                continue;
            }
            accumulate(&mut vector, &normalized);
            count += 1;
        }

        if count == 0 {
            for ch in text.chars().take(HASH_EMBEDDING_DIM) {
                accumulate(&mut vector, &ch.to_string());
                count += 1;
            }
        }

        if count == 0 {
            vector[0] = 1.0;
            return vector;
        }

        let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm == 0.0 {
            vector[0] = 1.0;
            return vector;
        }

        for value in &mut vector {
            *value /= norm;
        }

        vector
    }
}

impl EmbeddingProvider for HashEmbedder {
    fn profile(&self) -> &EmbeddingProfile {
        &self.profile
    }

    fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(documents
            .iter()
            .map(|document| Self::embed_text(document))
            .collect())
    }

    fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        Ok(Self::embed_text(query))
    }
}

pub struct FastEmbedder {
    cache_dir: PathBuf,
    model_name: EmbeddingModel,
    profile: EmbeddingProfile,
    runtime: Mutex<Option<TextEmbedding>>,
    show_download_progress: bool,
    use_e5_prefixes: bool,
}

impl FastEmbedder {
    pub fn new(model: String, cache_dir: PathBuf, show_download_progress: bool) -> Result<Self> {
        let model_name = EmbeddingModel::from_str(&model).map_err(|err| {
            MempalaceError::InvalidArgument(format!("Unknown fastembed model `{model}`: {err}"))
        })?;
        let dimension = TextEmbedding::get_model_info(&model_name)?.dim;

        Ok(Self {
            cache_dir,
            use_e5_prefixes: matches!(model_name, EmbeddingModel::MultilingualE5Small),
            model_name,
            profile: EmbeddingProfile {
                provider: "fastembed".to_string(),
                model,
                dimension,
            },
            runtime: Mutex::new(None),
            show_download_progress,
        })
    }

    fn with_model<T>(&self, f: impl FnOnce(&mut TextEmbedding) -> Result<T>) -> Result<T> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| MempalaceError::Mcp("embedding model lock poisoned".to_string()))?;

        if runtime.is_none() {
            configure_ort_dylib_path();
            let options = InitOptions::new(self.model_name.clone())
                .with_cache_dir(self.cache_dir.clone())
                .with_show_download_progress(self.show_download_progress);
            *runtime = Some(TextEmbedding::try_new(options)?);
        }

        let model = runtime.as_mut().ok_or_else(|| {
            MempalaceError::InvalidArgument("embedding model failed to initialize".to_string())
        })?;
        f(model)
    }

    fn normalize_document(&self, text: &str) -> String {
        if self.use_e5_prefixes {
            format!("passage: {text}")
        } else {
            text.to_string()
        }
    }

    fn normalize_query(&self, text: &str) -> String {
        if self.use_e5_prefixes {
            format!("query: {text}")
        } else {
            text.to_string()
        }
    }
}

fn configure_ort_dylib_path() {
    if std::env::var_os("ORT_DYLIB_PATH").is_some() {
        return;
    }

    for candidate in [
        "/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib",
        "/usr/local/opt/onnxruntime/lib/libonnxruntime.dylib",
    ] {
        if std::path::Path::new(candidate).exists() {
            // Prefer a local system runtime over flaky prebuilt downloads.
            unsafe {
                std::env::set_var("ORT_DYLIB_PATH", candidate);
            }
            break;
        }
    }
}

impl EmbeddingProvider for FastEmbedder {
    fn profile(&self) -> &EmbeddingProfile {
        &self.profile
    }

    fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f32>>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let prefixed: Vec<String> = documents
            .iter()
            .map(|document| self.normalize_document(document))
            .collect();
        self.with_model(|model| model.embed(prefixed, None).map_err(Into::into))
    }

    fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        let prefixed = vec![self.normalize_query(query)];
        self.with_model(|model| {
            let mut embeddings = model.embed(prefixed, None)?;
            embeddings.pop().ok_or_else(|| {
                MempalaceError::InvalidArgument(
                    "embedding provider returned no query vector".to_string(),
                )
            })
        })
    }
}

fn accumulate(vector: &mut [f32], token: &str) {
    let mut hasher = Hasher::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    let bytes = digest.as_bytes();

    for chunk in bytes.chunks_exact(4).take(HASH_EMBEDDING_DIM / 4) {
        let bucket = (chunk[0] as usize) % HASH_EMBEDDING_DIM;
        let sign = if chunk[1] & 1 == 0 { 1.0 } else { -1.0 };
        let weight = 1.0 + (chunk[2] as f32 / 255.0);
        vector[bucket] += sign * weight;
    }
}
