use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::normalize_json::try_normalize_json;
use crate::normalize_transcript::{count_quote_lines, normalize_quote_transcript};
use crate::spellcheck::known_names_for_path;

pub fn normalize_conversation_file(path: &Path) -> Result<Option<String>> {
    let known_names = known_names_for_path(path);
    let raw = match fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(err) => return Err(err.into()),
    };
    normalize_conversation(path, &raw, &known_names)
}

pub fn normalize_conversation(
    path: &Path,
    raw: &str,
    known_names: &HashSet<String>,
) -> Result<Option<String>> {
    let content = raw.trim();
    if content.is_empty() {
        return Ok(Some(raw.to_string()));
    }

    if count_quote_lines(content) >= 3 {
        return Ok(Some(normalize_quote_transcript(raw, known_names)));
    }

    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    if matches!(ext.as_str(), "json" | "jsonl")
        || matches!(content.chars().next(), Some('{') | Some('['))
    {
        if let Some(normalized) = try_normalize_json(content, known_names) {
            return Ok(Some(normalized));
        }
        if matches!(ext.as_str(), "json" | "jsonl") {
            return Ok(None);
        }
    }

    Ok(Some(raw.to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{normalize_conversation, normalize_conversation_file};

    #[test]
    fn normalize_json_transcript_spellchecks_user_turns() {
        let known_names = HashSet::from(["riley".to_string()]);
        let normalized = normalize_conversation(
            Path::new("demo.jsonl"),
            r#"{"type":"session_meta","payload":{"id":"demo"}}
{"type":"event_msg","payload":{"type":"user_message","message":"Riley knoe the deploy befor lunch"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"We fixed it."}}
"#,
            &known_names,
        )
        .unwrap()
        .unwrap();

        assert!(normalized.contains("> Riley know the deploy before lunch"));
        assert!(normalized.contains("We fixed it."));
    }

    #[test]
    fn normalize_chatgpt_export_to_transcript() {
        let known_names = HashSet::new();
        let normalized = normalize_conversation(
            Path::new("chatgpt.json"),
            r#"{
  "mapping": {
    "root": {"id":"root","parent":null,"message":null,"children":["u1"]},
    "u1": {"id":"u1","parent":"root","message":{"author":{"role":"user"},"content":{"parts":["How do we ship this?"]}},"children":["a1"]},
    "a1": {"id":"a1","parent":"u1","message":{"author":{"role":"assistant"},"content":{"parts":["Run tests first."]}},"children":[]}
  }
}"#,
            &known_names,
        )
        .unwrap()
        .unwrap();

        assert!(normalized.contains("> How do we ship this?"));
        assert!(normalized.contains("Run tests first."));
    }

    #[test]
    fn normalize_file_tolerates_invalid_utf8_like_python() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("notes.txt");
        fs::write(
            &path,
            [
                b"plain transcript before bad byte\n".as_slice(),
                b"\xff\n".as_slice(),
                b"plain transcript after bad byte\n".as_slice(),
            ]
            .concat(),
        )
        .unwrap();

        let normalized = normalize_conversation_file(&path).unwrap().unwrap();

        assert!(normalized.contains("plain transcript before bad byte"));
        assert!(normalized.contains('\u{fffd}'));
        assert!(normalized.contains("plain transcript after bad byte"));
    }

    #[test]
    fn normalize_blank_content_passes_through_like_python() {
        let known_names = HashSet::new();
        let raw = " \n\t\n";

        let normalized = normalize_conversation(Path::new("blank.txt"), raw, &known_names)
            .unwrap()
            .unwrap();

        assert_eq!(normalized, raw);
    }
}
