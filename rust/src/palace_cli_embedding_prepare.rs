use anyhow::Result;
use mempalace_rs::model::PrepareEmbeddingSummary;
use serde_json::json;

use crate::palace_cli_embedding_support::{
    create_embedding_app, print_embedding_json, resolve_embedding_config,
};

pub async fn handle_prepare_embedding(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    attempts: usize,
    wait_ms: u64,
    human: bool,
) -> Result<()> {
    let config = resolve_embedding_config(
        palace,
        hf_endpoint,
        human,
        print_prepare_embedding_error_human,
        print_prepare_embedding_error_json,
    )?;
    let app = create_embedding_app(
        config,
        human,
        print_prepare_embedding_error_human,
        print_prepare_embedding_error_json,
    )?;
    let summary = match app.prepare_embedding(attempts, wait_ms).await {
        Ok(summary) => summary,
        Err(err) if human => {
            print_prepare_embedding_error_human(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_prepare_embedding_error_json(&err.to_string())?;
            std::process::exit(1);
        }
    };
    if human {
        print_prepare_embedding_human(&summary);
    } else {
        print_embedding_json(&summary)?;
    }
    Ok(())
}

fn print_prepare_embedding_human(summary: &PrepareEmbeddingSummary) {
    print!("{}", render_prepare_embedding_human(summary));
}

pub(crate) fn render_prepare_embedding_human(summary: &PrepareEmbeddingSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!("\n{}\n", "=".repeat(55)));
    out.push_str("  MemPalace Prepare Embedding\n");
    out.push_str(&format!("{}\n\n", "=".repeat(55)));
    out.push_str(&format!("  Palace:    {}\n", summary.palace_path));
    out.push_str(&format!("  Provider:  {}\n", summary.provider));
    out.push_str(&format!("  Model:     {}\n", summary.model));
    out.push_str(&format!("  Attempts:  {}\n", summary.attempts));
    out.push_str(&format!(
        "  Result:    {}\n",
        if summary.success { "ok" } else { "failed" }
    ));
    if let Some(error) = &summary.last_error {
        out.push_str(&format!("  Last err:  {error}\n"));
    }
    out.push_str(&format!(
        "  Warmup:    {}\n",
        if summary.doctor.warmup_ok {
            "ok"
        } else {
            "failed"
        }
    ));
    if let Some(path) = &summary.doctor.model_cache_dir {
        out.push_str(&format!("  Model dir: {path}\n"));
    }
    if let Some(path) = &summary.doctor.expected_model_file {
        out.push_str(&format!("  Model file: {path}\n"));
    }
    out.push_str(&format!(
        "  Model file present: {}\n",
        if summary.doctor.expected_model_file_present {
            "yes"
        } else {
            "no"
        }
    ));
    if !summary.success {
        out.push_str("\n  Suggested next step:\n");
        if summary.doctor.hf_endpoint.is_none() {
            out.push_str(
                "    Retry with --hf-endpoint https://hf-mirror.com if model download cannot reach HuggingFace.\n",
            );
        } else {
            out.push_str(
                "    Verify the configured HuggingFace mirror and rerun prepare-embedding once model download works.\n",
            );
        }
    }
    out.push_str(&format!("\n{}\n\n", "=".repeat(55)));
    out
}

fn print_prepare_embedding_error_human(message: &str) {
    println!("\n  Prepare embedding error: {message}");
    println!(
        "  Check the palace files and embedding runtime, then rerun `mempalace-rs prepare-embedding`."
    );
}

fn print_prepare_embedding_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Prepare embedding error: {message}"),
        "hint": "Check the palace files and embedding runtime, then rerun `mempalace-rs prepare-embedding`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::render_prepare_embedding_human;
    use mempalace_rs::model::{DoctorSummary, PrepareEmbeddingSummary};

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
    fn prepare_embedding_human_failure_mentions_configured_mirror_when_present() {
        let doctor = failed_doctor_summary(Some("https://hf-mirror.example"));
        let output = render_prepare_embedding_human(&PrepareEmbeddingSummary {
            kind: "prepare_embedding".to_string(),
            palace_path: "/tmp/palace".to_string(),
            sqlite_path: "/tmp/palace/palace.sqlite3".to_string(),
            lance_path: "/tmp/palace/lance".to_string(),
            version: "0.1.0".to_string(),
            provider: "fastembed".to_string(),
            model: "MultilingualE5Small".to_string(),
            attempts: 1,
            success: false,
            last_error: Some("Failed to retrieve onnx/model.onnx".to_string()),
            doctor,
        });
        assert!(output.contains("Result:    failed"));
        assert!(output.contains("Last err:  Failed to retrieve onnx/model.onnx"));
        assert!(output.contains("Suggested next step:"));
        assert!(output.contains("Verify the configured HuggingFace mirror"));
    }
}
