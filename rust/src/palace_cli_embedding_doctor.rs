use anyhow::Result;
use mempalace_rs::model::DoctorSummary;
use serde_json::json;

use crate::palace_cli_embedding_support::{
    create_embedding_app, print_embedding_json, resolve_embedding_config,
};

pub async fn handle_doctor(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    warm_embedding: bool,
    human: bool,
) -> Result<()> {
    let config = resolve_embedding_config(
        palace,
        hf_endpoint,
        human,
        print_doctor_error_human,
        print_doctor_error_json,
    )?;
    let app = create_embedding_app(
        config,
        human,
        print_doctor_error_human,
        print_doctor_error_json,
    )?;
    let summary = match app.doctor(warm_embedding).await {
        Ok(summary) => summary,
        Err(err) if human => {
            print_doctor_error_human(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_doctor_error_json(&err.to_string())?;
            std::process::exit(1);
        }
    };
    if human {
        print_doctor_human(&summary);
    } else {
        print_embedding_json(&summary)?;
    }
    Ok(())
}

fn print_doctor_human(summary: &DoctorSummary) {
    print!("{}", render_doctor_human(summary));
}

fn print_doctor_error_human(message: &str) {
    println!("\n  Doctor error: {message}");
    println!("  Check the embedding provider and local runtime, then rerun `mempalace-rs doctor`.");
}

fn print_doctor_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Doctor error: {message}"),
        "hint": "Check the embedding provider and local runtime, then rerun `mempalace-rs doctor`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

pub(crate) fn render_doctor_human(summary: &DoctorSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!("\n{}\n", "=".repeat(55)));
    out.push_str("  MemPalace Doctor\n");
    out.push_str(&format!("{}\n\n", "=".repeat(55)));
    out.push_str(&format!("  Palace:     {}\n", summary.palace_path));
    out.push_str(&format!("  SQLite:     {}\n", summary.sqlite_path));
    out.push_str(&format!("  LanceDB:    {}\n", summary.lance_path));
    out.push_str(&format!("  Provider:   {}\n", summary.provider));
    out.push_str(&format!("  Model:      {}\n", summary.model));
    out.push_str(&format!("  Dimension:  {}\n", summary.dimension));
    if let Some(path) = &summary.cache_dir {
        out.push_str(&format!("  Cache dir:  {path}\n"));
    }
    if let Some(path) = &summary.model_cache_dir {
        out.push_str(&format!("  Model dir:  {path}\n"));
    }
    if let Some(path) = &summary.expected_model_file {
        out.push_str(&format!("  Model file: {path}\n"));
    }
    out.push_str(&format!(
        "  Cache hit:  {}\n",
        if summary.model_cache_present {
            "yes"
        } else {
            "no"
        }
    ));
    out.push_str(&format!(
        "  Model file present: {}\n",
        if summary.expected_model_file_present {
            "yes"
        } else {
            "no"
        }
    ));
    if let Some(path) = &summary.ort_dylib_path {
        out.push_str(&format!("  ORT dylib:  {path}\n"));
    }
    if let Some(endpoint) = &summary.hf_endpoint {
        out.push_str(&format!("  HF endpoint: {endpoint}\n"));
    }
    if !summary.model_cache_present {
        out.push_str("  Cache state: model cache directory not populated yet\n");
    } else if !summary.expected_model_file_present {
        out.push_str("  Cache state: model snapshot exists but onnx/model.onnx is missing\n");
    } else {
        out.push_str("  Cache state: model snapshot looks ready\n");
    }
    if summary.warmup_attempted {
        out.push_str(&format!(
            "  Warmup:     {}\n",
            if summary.warmup_ok { "ok" } else { "failed" }
        ));
        if let Some(error) = &summary.warmup_error {
            out.push_str(&format!("  Warmup err: {error}\n"));
        }
        if !summary.warmup_ok {
            out.push_str("\n  Suggested next step:\n");
            if summary.hf_endpoint.is_none() {
                out.push_str(
                    "    Retry with --hf-endpoint https://hf-mirror.com if the default HuggingFace route is blocked.\n",
                );
            } else {
                out.push_str(
                    "    Retry prepare-embedding after verifying the configured HuggingFace mirror and local network access.\n",
                );
            }
        }
    }
    out.push_str(&format!("\n{}\n\n", "=".repeat(55)));
    out
}

#[cfg(test)]
mod tests {
    use super::render_doctor_human;
    use mempalace_rs::model::DoctorSummary;

    fn failed_doctor_summary(hf_endpoint: Option<&str>) -> DoctorSummary {
        DoctorSummary {
            kind: "doctor".to_string(),
            palace_path: "/tmp/palace".to_string(),
            sqlite_path: "/tmp/palace/palace.sqlite3".to_string(),
            lance_path: "/tmp/palace/lance".to_string(),
            version: "0.1.0".to_string(),
            provider: "fastembed".to_string(),
            model: "MultilingualE5Small".to_string(),
            dimension: 384,
            cache_dir: Some("/tmp/cache".to_string()),
            model_cache_dir: Some("/tmp/cache/model".to_string()),
            model_cache_present: false,
            expected_model_file: Some("/tmp/cache/model/onnx/model.onnx".to_string()),
            expected_model_file_present: false,
            hf_endpoint: hf_endpoint.map(ToOwned::to_owned),
            ort_dylib_path: Some(
                "/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib".to_string(),
            ),
            warmup_attempted: true,
            warmup_ok: false,
            warmup_error: Some("Failed to retrieve onnx/model.onnx".to_string()),
        }
    }

    #[test]
    fn doctor_human_failure_suggests_mirror_when_default_endpoint_fails() {
        let output = render_doctor_human(&failed_doctor_summary(None));
        assert!(output.contains("Cache state: model cache directory not populated yet"));
        assert!(output.contains("Warmup:     failed"));
        assert!(output.contains("Suggested next step:"));
        assert!(output.contains("--hf-endpoint https://hf-mirror.com"));
    }
}
