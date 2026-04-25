//! Transcript splitting utilities for oversized session logs.
//!
//! This subsystem identifies multi-session "mega files", writes one output per
//! detected session, and optionally renames the original file into a backup.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::Result;

const MAX_SCAN_SIZE: u64 = 500 * 1024 * 1024;
const FALLBACK_KNOWN_PEOPLE: [&str; 7] = ["Alice", "Ben", "Riley", "Max", "Sam", "Devon", "Jordan"];

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum KnownNamesConfig {
    Names(Vec<String>),
    Object {
        names: Option<Vec<String>>,
        username_map: Option<BTreeMap<String, String>>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Per-file result for a split operation.
pub struct SplitFileResult {
    pub source_file: String,
    pub detected_sessions: usize,
    pub output_files: Vec<String>,
    pub renamed_backup: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Summary returned by directory or single-file split commands.
pub struct SplitSummary {
    pub kind: String,
    pub source_dir: String,
    pub output_dir: String,
    pub dry_run: bool,
    pub mega_files: usize,
    pub files_created: usize,
    pub files: Vec<SplitFileResult>,
}

/// Splits every oversized transcript file in a directory.
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
        let contents = read_text_lossy(&path)?;
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

/// Splits one transcript file when it contains at least `min_sessions`.
pub fn split_single_file(
    path: &Path,
    output_dir: Option<&Path>,
    min_sessions: usize,
    dry_run: bool,
) -> Result<SplitSummary> {
    let mut files = Vec::new();
    let mut files_created = 0usize;
    let source_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let output_dir = output_dir.unwrap_or(source_dir);

    if path.metadata()?.len() <= MAX_SCAN_SIZE {
        let contents = read_text_lossy(path)?;
        let lines = contents
            .lines()
            .map(|line| format!("{line}\n"))
            .collect::<Vec<_>>();
        let boundaries = find_session_boundaries(&lines);
        if boundaries.len() >= min_sessions {
            let result = split_file(path, Some(output_dir), boundaries.len(), dry_run)?;
            files_created += result.output_files.len();
            files.push(result);
        }
    }

    Ok(SplitSummary {
        kind: "split".to_string(),
        source_dir: source_dir.display().to_string(),
        output_dir: output_dir.display().to_string(),
        dry_run,
        mega_files: files.len(),
        files_created,
        files,
    })
}

fn split_file(
    path: &Path,
    output_dir: Option<&Path>,
    detected_sessions: usize,
    dry_run: bool,
) -> Result<SplitFileResult> {
    let contents = read_text_lossy(path)?;
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
            people.iter().take(3).cloned().collect::<Vec<_>>().join("-")
        };
        let subject = extract_subject(chunk);
        let src_stem = source_stem_part(path);
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

fn read_text_lossy(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn sanitize_filename(value: &str) -> String {
    let sanitized = Regex::new(r"[^\w\.-]")
        .unwrap()
        .replace_all(value, "_")
        .to_string();
    Regex::new(r"_+")
        .unwrap()
        .replace_all(&sanitized, "_")
        .to_string()
}

fn source_stem_part(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("session");
    Regex::new(r"[^\w-]")
        .unwrap()
        .replace_all(stem, "_")
        .chars()
        .take(40)
        .collect()
}

fn is_true_session_start(lines: &[String], idx: usize) -> bool {
    let nearby = lines.iter().skip(idx).take(6).cloned().collect::<String>();
    !nearby.contains("Ctrl+E") && !nearby.contains("previous messages")
}

/// Finds session boundary indices inside a transcript file.
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
            let time = caps.get(1)?.as_str().replace([':', ' '], "");
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
    let config = load_known_names_config();
    extract_people_with_config(&text, config.as_ref())
}

fn extract_people_with_config(text: &str, config: Option<&KnownNamesConfig>) -> Vec<String> {
    let (known_people, username_map) = known_people_and_username_map(config);
    let mut people = BTreeSet::new();
    for person in known_people {
        let pattern = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(person))).unwrap();
        if pattern.is_match(text) {
            people.insert(person.to_string());
        }
    }

    if let Some(caps) = Regex::new(r"/Users/(\w+)/").unwrap().captures(text)
        && let Some(name) = username_map.get(caps.get(1).unwrap().as_str())
    {
        people.insert(name.clone());
    }

    people.into_iter().collect()
}

fn known_people_and_username_map(
    config: Option<&KnownNamesConfig>,
) -> (Vec<&str>, BTreeMap<String, String>) {
    match config {
        Some(KnownNamesConfig::Names(names)) => {
            (names.iter().map(String::as_str).collect(), BTreeMap::new())
        }
        Some(KnownNamesConfig::Object {
            names,
            username_map,
        }) => (
            names
                .as_ref()
                .map(|names| names.iter().map(String::as_str).collect())
                .unwrap_or_default(),
            username_map.clone().unwrap_or_default(),
        ),
        None => (FALLBACK_KNOWN_PEOPLE.to_vec(), BTreeMap::new()),
    }
}

fn load_known_names_config() -> Option<KnownNamesConfig> {
    let path = known_names_path()?;
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn known_names_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".mempalace/known_names.json"))
}

fn extract_subject(lines: &[String]) -> String {
    let skip = Regex::new(r"^(\./|cd |ls |python|bash|git |cat |source |export |claude)").unwrap();
    for line in lines {
        if let Some(prompt) = line.strip_prefix("> ") {
            let prompt = prompt.trim();
            if prompt.len() > 5 && !skip.is_match(prompt) {
                return subject_part(prompt);
            }
        }
    }
    "session".to_string()
}

fn subject_part(prompt: &str) -> String {
    let without_punctuation = Regex::new(r"[^\w\s-]")
        .unwrap()
        .replace_all(prompt, "")
        .to_string();
    Regex::new(r"\s+")
        .unwrap()
        .replace_all(without_punctuation.trim(), "-")
        .chars()
        .take(60)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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

    #[test]
    fn people_detection_uses_python_fallback_names() {
        let lines = vec![
            "Claude Code v1\n".to_string(),
            "⏺ 9:30 AM Monday, April 1, 2026\n".to_string(),
            "> Riley and Ben reviewed the Lantern design\n".to_string(),
            "Project text mentions Claude, Code, Monday, and April.\n".to_string(),
        ];

        assert_eq!(extract_people(&lines), vec!["Ben", "Riley"]);
    }

    #[test]
    fn people_detection_uses_known_names_list_config() {
        let config = KnownNamesConfig::Names(vec!["Morgan".to_string(), "Riley".to_string()]);

        assert_eq!(
            extract_people_with_config("Morgan mentioned Alice and Riley.", Some(&config)),
            vec!["Morgan", "Riley"]
        );
    }

    #[test]
    fn people_detection_uses_known_names_object_and_username_map() {
        let config = KnownNamesConfig::Object {
            names: Some(vec!["Morgan".to_string()]),
            username_map: Some(BTreeMap::from([(
                "devuser".to_string(),
                "Devon".to_string(),
            )])),
        };

        assert_eq!(
            extract_people_with_config(
                "cwd: /Users/devuser/project\nMorgan joined.",
                Some(&config)
            ),
            vec!["Devon", "Morgan"]
        );
    }

    #[test]
    fn people_detection_object_without_names_matches_python_empty_names() {
        let config = KnownNamesConfig::Object {
            names: None,
            username_map: None,
        };

        assert!(extract_people_with_config("Alice and Riley joined.", Some(&config)).is_empty());
    }

    #[test]
    fn source_stem_part_matches_python_split_prefix() {
        let path = Path::new("very.long source name with punctuation!! and extra suffix.txt");

        assert_eq!(
            source_stem_part(path),
            "very_long_source_name_with_punctuation__"
        );
    }

    #[test]
    fn subject_part_matches_python_split_prompt_cleanup() {
        assert_eq!(
            subject_part("Review: split naming, punctuation & spacing now"),
            "Review-split-naming-punctuation-spacing-now"
        );
    }

    #[test]
    fn sanitize_filename_collapses_underscores_like_python_split() {
        assert_eq!(
            sanitize_filename("source__2026-04-01_930AM_Ben-Riley_Review split.txt"),
            "source_2026-04-01_930AM_Ben-Riley_Review_split.txt"
        );
    }

    #[test]
    fn split_directory_tolerates_invalid_utf8_like_python() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("mega.txt");
        fs::write(
            &path,
            [
                b"Claude Code v1\n".as_slice(),
                b"\xff\n".as_slice(),
                b"> first prompt about lossy read\n".as_slice(),
                b"line1\nline2\nline3\nline4\nline5\nline6\nline7\n".as_slice(),
                b"Claude Code v1\n".as_slice(),
                b"> second prompt about lossy read\n".as_slice(),
                b"line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n".as_slice(),
            ]
            .concat(),
        )
        .unwrap();

        let summary = split_directory(tmp.path(), None, 2, true).unwrap();

        assert_eq!(summary.mega_files, 1);
        assert_eq!(summary.files_created, 2);
    }

    #[test]
    fn split_single_file_limits_scan_to_requested_file() {
        let tmp = tempdir().unwrap();
        let source = tmp.path().join("source");
        fs::create_dir_all(&source).unwrap();
        let mega = source.join("target.txt");
        fs::write(
            &mega,
            concat!(
                "Claude Code v1\n",
                "> first prompt about file mode\n",
                "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n",
                "Claude Code v1\n",
                "> second prompt about file mode\n",
                "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n",
            ),
        )
        .unwrap();
        fs::write(
            source.join("ignored.txt"),
            concat!(
                "Claude Code v1\n",
                "> ignored first session\n",
                "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n",
                "Claude Code v1\n",
                "> ignored second session\n",
                "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n",
            ),
        )
        .unwrap();

        let summary = split_single_file(&mega, None, 2, true).unwrap();

        assert_eq!(summary.source_dir, source.display().to_string());
        assert_eq!(summary.mega_files, 1);
        assert_eq!(summary.files_created, 2);
        assert_eq!(summary.files[0].source_file, mega.display().to_string());
    }
}
