use regex::Regex;

use crate::embed::{
    EmbeddingProfile, EmbeddingProvider, HASH_EMBEDDING_DIM, HASH_MODEL_NAME, HASH_PROVIDER_NAME,
};
use crate::error::Result;
use crate::model::DoctorSummary;

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
            kind: "doctor".to_string(),
            palace_path: palace_path.to_string(),
            sqlite_path: String::new(),
            lance_path: String::new(),
            version: crate::VERSION.to_string(),
            provider: HASH_PROVIDER_NAME.to_string(),
            model: HASH_MODEL_NAME.to_string(),
            dimension: HASH_EMBEDDING_DIM,
            cache_dir: None,
            model_cache_dir: None,
            model_cache_present: false,
            expected_model_file: None,
            expected_model_file_present: false,
            hf_endpoint: None,
            ort_dylib_path: None,
            warmup_attempted: warmup,
            warmup_ok: true,
            warmup_error: None,
        }
    }
}

fn accumulate(vector: &mut [f32], token: &str) {
    let mut hasher = blake3::Hasher::new();
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
