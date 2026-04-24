use std::path::{Path, PathBuf};

pub(crate) fn configure_ort_dylib_path() {
    if std::env::var_os("ORT_DYLIB_PATH").is_some() {
        return;
    }

    if let Some(path) = detect_ort_dylib_path() {
        unsafe {
            std::env::set_var("ORT_DYLIB_PATH", path);
        }
    }
}

pub(crate) fn detect_ort_dylib_path() -> Option<PathBuf> {
    for candidate in [
        "/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib",
        "/usr/local/opt/onnxruntime/lib/libonnxruntime.dylib",
    ] {
        if Path::new(candidate).exists() {
            return Some(PathBuf::from(candidate));
        }
    }

    std::env::var("ORT_DYLIB_PATH").ok().map(PathBuf::from)
}

pub(crate) fn configure_hf_endpoint(endpoint: Option<&str>) {
    let Some(endpoint) = endpoint else {
        return;
    };

    if std::env::var("HF_ENDPOINT").ok().as_deref() == Some(endpoint) {
        return;
    }

    unsafe {
        std::env::set_var("HF_ENDPOINT", endpoint);
    }
}

pub(crate) fn model_cache_ready(path: &Path) -> bool {
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

pub(crate) fn expected_model_file(cache_dir: &Path, model_file: &str) -> Option<PathBuf> {
    let refs_main = cache_dir.join("refs").join("main");
    let snapshot = std::fs::read_to_string(refs_main).ok()?;
    let snapshot = snapshot.trim();
    if snapshot.is_empty() {
        return None;
    }

    Some(cache_dir.join("snapshots").join(snapshot).join(model_file))
}
