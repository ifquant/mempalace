use std::fs;
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::Result;

const MAX_SCAN_SIZE: u64 = 500 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SplitFileResult {
    pub source_file: String,
    pub detected_sessions: usize,
    pub output_files: Vec<String>,
    pub renamed_backup: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SplitSummary {
    pub kind: String,
    pub source_dir: String,
    pub output_dir: String,
    pub dry_run: bool,
    pub mega_files: usize,
    pub files_created: usize,
    pub files: Vec<SplitFileResult>,
}

pub fn split_directory(
    source_dir: &Path,
    output_dir: Option<&Path>,
    min_sessions: usize,
    dry_run: bool,
) -> Result<SplitSummary> {
    let mut mega_files = Vec::new();
    for entry in fs::read_dir(source_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("txt") {
            continue;
        }
        let metadata = path.metadata()?;
        if metadata.len() > MAX_SCAN_SIZE {
            continue;
        }
        let contents = fs::read_to_string(&path)?;
        let lines = contents
            .lines()
            .map(|line| format!("{line}\n"))
            .collect::<Vec<_>>();
        let boundaries = find_session_boundaries(&lines);
        if boundaries.len() >= min_sessions {
            mega_files.push((path, boundaries.len()));
        }
    }

    let mut results = Vec::new();
    let mut files_created = 0usize;
    for (path, session_count) in mega_files {
        let result = split_file(&path, output_dir, session_count, dry_run)?;
        files_created += result.output_files.len();
        results.push(result);
    }

    Ok(SplitSummary {
        kind: "split".to_string(),
        source_dir: source_dir.display().to_string(),
        output_dir: output_dir.unwrap_or(source_dir).display().to_string(),
        dry_run,
        mega_files: results.len(),
        files_created,
        files: results,
    })
}

fn split_file(
    path: &Path,
    output_dir: Option<&Path>,
    detected_sessions: usize,
    dry_run: bool,
) -> Result<SplitFileResult> {
    let contents = fs::read_to_string(path)?;
    let lines = contents
        .lines()
        .map(|line| format!("{line}\n"))
        .collect::<Vec<_>>();
    let mut boundaries = find_session_boundaries(&lines);
    boundaries.push(lines.len());

    let mut output_files = Vec::new();
    let out_dir = output_dir.unwrap_or_else(|| path.parent().unwrap_or_else(|| Path::new(".")));

    for (index, window) in boundaries.windows(2).enumerate() {
        let start = window[0];
        let end = window[1];
        let chunk = &lines[start..end];
        if chunk.len() < 10 {
            continue;
        }

        let timestamp = extract_timestamp(chunk).unwrap_or_else(|| format!("part{:02}", index + 1));
        let people = extract_people(chunk);
        let people_part = if people.is_empty() {
            "unknown".to_string()
        } else {
            people.join("-")
        };
        let subject = extract_subject(chunk);
        let src_stem = sanitize_filename(
            path.file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("session"),
        );
        let file_name = sanitize_filename(&format!(
            "{src_stem}__{timestamp}_{people_part}_{subject}.txt"
        ));
        let out_path = out_dir.join(file_name);
        if !dry_run {
            fs::write(&out_path, chunk.concat())?;
        }
        output_files.push(out_path.display().to_string());
    }

    let renamed_backup = if !dry_run && !output_files.is_empty() {
        let backup = path.with_extension("mega_backup");
        fs::rename(path, &backup)?;
        Some(backup.display().to_string())
    } else {
        None
    };

    Ok(SplitFileResult {
        source_file: path.display().to_string(),
        detected_sessions,
        output_files,
        renamed_backup,
    })
}

fn sanitize_filename(value: &str) -> String {
    Regex::new(r"[^\w\.-]+")
        .unwrap()
        .replace_all(value, "_")
        .to_string()
}

fn is_true_session_start(lines: &[String], idx: usize) -> bool {
    let nearby = lines.iter().skip(idx).take(6).cloned().collect::<String>();
    !nearby.contains("Ctrl+E") && !nearby.contains("previous messages")
}

pub fn find_session_boundaries(lines: &[String]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            if line.contains("Claude Code v") && is_true_session_start(lines, idx) {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}

fn extract_timestamp(lines: &[String]) -> Option<String> {
    let pattern =
        Regex::new(r"⏺\s+(\d{1,2}:\d{2}\s+[AP]M)\s+\w+,\s+(\w+)\s+(\d{1,2}),\s+(\d{4})").unwrap();
    let months = [
        ("January", "01"),
        ("February", "02"),
        ("March", "03"),
        ("April", "04"),
        ("May", "05"),
        ("June", "06"),
        ("July", "07"),
        ("August", "08"),
        ("September", "09"),
        ("October", "10"),
        ("November", "11"),
        ("December", "12"),
    ]
    .into_iter()
    .collect::<std::collections::BTreeMap<_, _>>();

    for line in lines.iter().take(50) {
        if let Some(caps) = pattern.captures(line) {
            let time = caps.get(1)?.as_str().replace(':', "").replace(' ', "");
            let month = months.get(caps.get(2)?.as_str()).copied().unwrap_or("00");
            let day = format!("{:0>2}", caps.get(3)?.as_str());
            let year = caps.get(4)?.as_str();
            return Some(format!("{year}-{month}-{day}_{time}"));
        }
    }
    None
}

fn extract_people(lines: &[String]) -> Vec<String> {
    let text = lines.iter().take(100).cloned().collect::<String>();
    let pattern = Regex::new(r"\b([A-Z][a-z]+)\b").unwrap();
    let mut people = Vec::new();
    for caps in pattern.captures_iter(&text) {
        let name = caps.get(1).unwrap().as_str().to_string();
        if !people.iter().any(|existing| existing == &name) {
            people.push(name);
        }
        if people.len() >= 3 {
            break;
        }
    }
    people
}

fn extract_subject(lines: &[String]) -> String {
    let skip = Regex::new(r"^(\./|cd |ls |python|bash|git |cat |source |export |claude)").unwrap();
    for line in lines {
        if let Some(prompt) = line.strip_prefix("> ") {
            let prompt = prompt.trim();
            if prompt.len() > 5 && !skip.is_match(prompt) {
                return sanitize_filename(prompt).chars().take(60).collect();
            }
        }
    }
    "session".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn true_session_start_ignores_context_restore_headers() {
        let lines = vec![
            "Claude Code v1\n".to_string(),
            "Ctrl+E to show 20 previous messages\n".to_string(),
            "Claude Code v1\n".to_string(),
            "fresh session\n".to_string(),
        ];
        assert_eq!(find_session_boundaries(&lines), vec![2]);
    }
}
