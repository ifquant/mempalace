use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use blake3::Hasher;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use regex::Regex;

use crate::config::{EmbeddingBackend, EmbeddingSettings};
use crate::error::{MempalaceError, Result};
use crate::model::DoctorSummary;

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

    fn doctor(&self, palace_path: &str, warmup: bool) -> DoctorSummary {
        DoctorSummary {
            palace_path: palace_path.to_string(),
            provider: self.profile.provider.clone(),
            model: self.profile.model.clone(),
            dimension: self.profile.dimension,
            cache_dir: None,
            model_cache_dir: None,
            model_cache_present: false,
            ort_dylib_path: None,
            warmup_attempted: warmup,
            warmup_ok: true,
            warmup_error: None,
        }
    }
}

pub struct FastEmbedder {
    cache_dir: PathBuf,
    model_name: EmbeddingModel,
    model_code: String,
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
        let model_info = TextEmbedding::get_model_info(&model_name)?;
        let dimension = model_info.dim;

        Ok(Self {
            cache_dir,
            model_code: model_info.model_code.clone(),
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

    if let Some(path) = detect_ort_dylib_path() {
        // Prefer a local system runtime over flaky prebuilt downloads.
        unsafe {
            std::env::set_var("ORT_DYLIB_PATH", path);
        }
    }
}

fn detect_ort_dylib_path() -> Option<PathBuf> {
    for candidate in [
        "/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib",
        "/usr/local/opt/onnxruntime/lib/libonnxruntime.dylib",
    ] {
        if std::path::Path::new(candidate).exists() {
            return Some(PathBuf::from(candidate));
        }
    }

    std::env::var("ORT_DYLIB_PATH").ok().map(PathBuf::from)
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

    fn doctor(&self, palace_path: &str, warmup: bool) -> DoctorSummary {
        let model_cache_dir = Some(
            self.cache_dir
                .join(format!("models--{}", self.model_code.replace('/', "--"))),
        );
        let model_cache_present = model_cache_dir
            .as_ref()
            .map(|path| model_cache_ready(path))
            .unwrap_or(false);
        let (warmup_ok, warmup_error) = if warmup {
            match self.embed_query("health check") {
                Ok(_) => (true, None),
                Err(err) => (false, Some(err.to_string())),
            }
        } else {
            (false, None)
        };

        DoctorSummary {
            palace_path: palace_path.to_string(),
            provider: self.profile.provider.clone(),
            model: self.profile.model.clone(),
            dimension: self.profile.dimension,
            cache_dir: Some(self.cache_dir.display().to_string()),
            model_cache_dir: model_cache_dir.map(|path| path.display().to_string()),
            model_cache_present,
            ort_dylib_path: detect_ort_dylib_path().map(|path| path.display().to_string()),
            warmup_attempted: warmup,
            warmup_ok,
            warmup_error,
        }
    }
}

fn model_cache_ready(path: &std::path::Path) -> bool {
    if !path.exists() {
        return false;
    }

    if let Ok(entries) = std::fs::read_dir(path.join("blobs")) {
        for entry in entries.flatten() {
            let candidate = entry.path();
            if candidate
                .extension()
                .is_some_and(|ext| ext == "part" || ext == "lock")
            {
                continue;
            }
            if candidate.is_file() {
                return true;
            }
        }
    }

    path.join("refs").exists()
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
