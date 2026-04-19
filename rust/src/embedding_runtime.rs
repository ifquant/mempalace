use crate::VERSION;
use crate::config::AppConfig;
use std::sync::Arc;
use std::time::Duration;

use crate::embed::EmbeddingProvider;
use crate::error::Result;
use crate::model::{DoctorSummary, PrepareEmbeddingSummary};

pub struct EmbeddingRuntime {
    pub config: AppConfig,
    pub embedder: Arc<dyn EmbeddingProvider>,
}

pub struct EmbeddingRuntimeContext {
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub provider: String,
    pub model: String,
}

pub fn finalize_doctor_summary(
    mut summary: DoctorSummary,
    context: &EmbeddingRuntimeContext,
) -> DoctorSummary {
    summary.sqlite_path = context.sqlite_path.clone();
    summary.lance_path = context.lance_path.clone();
    summary.version = context.version.clone();
    summary
}

impl EmbeddingRuntime {
    fn context(&self) -> Result<EmbeddingRuntimeContext> {
        self.config.ensure_dirs()?;
        Ok(EmbeddingRuntimeContext {
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            provider: self.embedder.profile().provider.clone(),
            model: self.embedder.profile().model.clone(),
        })
    }

    pub fn doctor(&self, warm_embedding: bool) -> Result<DoctorSummary> {
        let context = self.context()?;
        let summary = self.embedder.doctor(&context.palace_path, warm_embedding);
        Ok(finalize_doctor_summary(summary, &context))
    }

    pub async fn prepare_embedding(
        &self,
        attempts: usize,
        wait_ms: u64,
    ) -> Result<PrepareEmbeddingSummary> {
        let context = self.context()?;
        let run = prepare_embedding_run(
            self.embedder.clone(),
            &context.palace_path,
            attempts,
            wait_ms,
        )
        .await?;
        Ok(run.into_summary(&context))
    }
}

pub struct PrepareEmbeddingRun {
    pub attempts: usize,
    pub success: bool,
    pub last_error: Option<String>,
    pub doctor: DoctorSummary,
}

impl PrepareEmbeddingRun {
    pub fn into_summary(self, context: &EmbeddingRuntimeContext) -> PrepareEmbeddingSummary {
        PrepareEmbeddingSummary {
            kind: "prepare_embedding".to_string(),
            palace_path: context.palace_path.clone(),
            sqlite_path: context.sqlite_path.clone(),
            lance_path: context.lance_path.clone(),
            version: context.version.clone(),
            provider: context.provider.clone(),
            model: context.model.clone(),
            attempts: self.attempts,
            success: self.success,
            last_error: self.last_error,
            doctor: finalize_doctor_summary(self.doctor, context),
        }
    }
}

pub async fn prepare_embedding_run(
    embedder: Arc<dyn EmbeddingProvider>,
    palace_path: &str,
    attempts: usize,
    wait_ms: u64,
) -> Result<PrepareEmbeddingRun> {
    let total_attempts = attempts.max(1);
    let mut last_error = None;
    let mut last_doctor = embedder.doctor(palace_path, false);

    for attempt in 0..total_attempts {
        let doctor = embedder.doctor(palace_path, true);
        let success = doctor.warmup_ok;
        last_error = doctor.warmup_error.clone();
        last_doctor = doctor;

        if success {
            return Ok(PrepareEmbeddingRun {
                attempts: attempt + 1,
                success: true,
                last_error: None,
                doctor: last_doctor,
            });
        }

        if attempt + 1 < total_attempts && wait_ms > 0 {
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        }
    }

    Ok(PrepareEmbeddingRun {
        attempts: total_attempts,
        success: false,
        last_error,
        doctor: last_doctor,
    })
}

#[cfg(test)]
mod tests {
    use crate::config::{EmbeddingBackend, EmbeddingSettings};
    use crate::embed::build_embedder;

    use super::*;

    #[tokio::test]
    async fn prepare_embedding_run_succeeds_with_hash_embedder() {
        let embedder = build_embedder(&EmbeddingSettings {
            backend: EmbeddingBackend::Hash,
            model: "hash-v1".to_string(),
            cache_dir: std::env::temp_dir(),
            hf_endpoint: None,
            show_download_progress: false,
        })
        .unwrap();

        let run = prepare_embedding_run(embedder, "/tmp/palace", 2, 0)
            .await
            .unwrap();
        assert!(run.success);
        assert_eq!(run.attempts, 1);
        assert!(run.doctor.warmup_ok);
    }

    #[test]
    fn finalize_doctor_summary_fills_runtime_paths() {
        let context = EmbeddingRuntimeContext {
            palace_path: "/tmp/palace".to_string(),
            sqlite_path: "/tmp/palace/palace.sqlite3".to_string(),
            lance_path: "/tmp/palace/lance".to_string(),
            version: "0.1.0".to_string(),
            provider: "hash".to_string(),
            model: "hash-v1".to_string(),
        };
        let summary = DoctorSummary {
            kind: "doctor".to_string(),
            palace_path: "/tmp/palace".to_string(),
            sqlite_path: String::new(),
            lance_path: String::new(),
            version: String::new(),
            provider: "hash".to_string(),
            model: "hash-v1".to_string(),
            dimension: 64,
            cache_dir: None,
            model_cache_dir: None,
            model_cache_present: false,
            expected_model_file: None,
            expected_model_file_present: false,
            hf_endpoint: None,
            ort_dylib_path: None,
            warmup_attempted: false,
            warmup_ok: true,
            warmup_error: None,
        };

        let finalized = finalize_doctor_summary(summary, &context);
        assert_eq!(finalized.sqlite_path, context.sqlite_path);
        assert_eq!(finalized.lance_path, context.lance_path);
        assert_eq!(finalized.version, context.version);
    }
}
