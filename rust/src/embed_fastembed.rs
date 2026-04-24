use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::embed::{
    EmbeddingProfile, EmbeddingProvider, configure_hf_endpoint, configure_ort_dylib_path,
    detect_ort_dylib_path, expected_model_file, model_cache_ready,
};
use crate::error::{MempalaceError, Result};
use crate::model::DoctorSummary;

pub struct FastEmbedder {
    cache_dir: PathBuf,
    hf_endpoint: Option<String>,
    model_name: EmbeddingModel,
    model_code: String,
    model_file: String,
    profile: EmbeddingProfile,
    runtime: Mutex<Option<TextEmbedding>>,
    show_download_progress: bool,
    use_e5_prefixes: bool,
}

impl FastEmbedder {
    pub fn new(
        model: String,
        cache_dir: PathBuf,
        hf_endpoint: Option<String>,
        show_download_progress: bool,
    ) -> Result<Self> {
        let model_name = EmbeddingModel::from_str(&model).map_err(|err| {
            MempalaceError::InvalidArgument(format!("Unknown fastembed model `{model}`: {err}"))
        })?;
        let model_info = TextEmbedding::get_model_info(&model_name)?;
        let dimension = model_info.dim;

        Ok(Self {
            cache_dir,
            hf_endpoint,
            model_code: model_info.model_code.clone(),
            model_file: model_info.model_file.clone(),
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
            configure_hf_endpoint(self.hf_endpoint.as_deref());
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
        let expected_model_file = model_cache_dir
            .as_ref()
            .and_then(|path| expected_model_file(path, &self.model_file));
        let expected_model_file_present = expected_model_file
            .as_ref()
            .map(|path| path.exists())
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
            kind: "doctor".to_string(),
            palace_path: palace_path.to_string(),
            sqlite_path: String::new(),
            lance_path: String::new(),
            version: crate::VERSION.to_string(),
            provider: self.profile.provider.clone(),
            model: self.profile.model.clone(),
            dimension: self.profile.dimension,
            cache_dir: Some(self.cache_dir.display().to_string()),
            model_cache_dir: model_cache_dir.map(|path| path.display().to_string()),
            model_cache_present,
            expected_model_file: expected_model_file.map(|path| path.display().to_string()),
            expected_model_file_present,
            hf_endpoint: self
                .hf_endpoint
                .clone()
                .or_else(|| std::env::var("HF_ENDPOINT").ok()),
            ort_dylib_path: detect_ort_dylib_path().map(|path| path.display().to_string()),
            warmup_attempted: warmup,
            warmup_ok,
            warmup_error,
        }
    }
}
