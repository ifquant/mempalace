use blake3::Hasher;
use regex::Regex;

pub const EMBEDDING_DIM: usize = 64;

pub fn embed_text(text: &str) -> Vec<f32> {
    let token_re = Regex::new(r"[A-Za-z0-9_]+").expect("token regex");
    let mut vector = vec![0.0_f32; EMBEDDING_DIM];
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
        for ch in text.chars().take(64) {
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

fn accumulate(vector: &mut [f32], token: &str) {
    let mut hasher = Hasher::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    let bytes = digest.as_bytes();

    for chunk in bytes.chunks_exact(4).take(EMBEDDING_DIM / 4) {
        let bucket = (chunk[0] as usize) % EMBEDDING_DIM;
        let sign = if chunk[1] & 1 == 0 { 1.0 } else { -1.0 };
        let weight = 1.0 + (chunk[2] as f32 / 255.0);
        vector[bucket] += sign * weight;
    }
}
