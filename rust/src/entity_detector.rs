use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use regex::Regex;

use crate::error::Result;

#[path = "entity_detector_scan.rs"]
mod scan;
#[path = "entity_detector_score.rs"]
mod score;

pub use scan::scan_for_detection;

#[derive(Clone, Debug, PartialEq)]
pub struct DetectedEntities {
    pub people: Vec<String>,
    pub projects: Vec<String>,
    pub files_scanned: usize,
}

pub fn detect_entities(project_dir: &Path) -> Result<DetectedEntities> {
    let files = scan_for_detection(project_dir)?;
    if files.is_empty() {
        return Ok(DetectedEntities {
            people: Vec::new(),
            projects: Vec::new(),
            files_scanned: 0,
        });
    }

    let mut all_text = String::new();
    let mut all_lines = Vec::new();
    let max_bytes = 5_000usize;
    let files_scanned = files.len().min(10);

    for path in files.into_iter().take(10) {
        let mut content = fs::read_to_string(&path).unwrap_or_default();
        if content.len() > max_bytes {
            content.truncate(max_bytes);
        }
        all_lines.extend(content.lines().map(|line| line.to_string()));
        all_text.push_str(&content);
        all_text.push('\n');
    }

    let candidate_re = Regex::new(r"\b[A-Z][a-z]{1,19}\b").unwrap();
    let mut counts = BTreeMap::<String, usize>::new();
    for cap in candidate_re.captures_iter(&all_text) {
        let name = cap.get(0).unwrap().as_str();
        if score::is_stopword(name) {
            continue;
        }
        *counts.entry(name.to_string()).or_insert(0) += 1;
    }
    let multi_candidate_re = Regex::new(r"\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)+\b").unwrap();
    for cap in multi_candidate_re.captures_iter(&all_text) {
        let phrase = cap.get(0).unwrap().as_str();
        if phrase.split_whitespace().any(score::is_stopword) {
            continue;
        }
        *counts.entry(phrase.to_string()).or_insert(0) += 1;
    }

    let mut people = Vec::new();
    let mut projects = Vec::new();
    for (name, frequency) in counts {
        if frequency < 3 {
            continue;
        }
        let person_score = score::score_person(&name, &all_text, &all_lines);
        let project_score = score::score_project(&name, &all_text, &all_lines);
        let person_signal_categories =
            score::person_signal_category_count(&name, &all_text, &all_lines);
        let total_score = person_score + project_score;
        let has_person_ratio = total_score > 0 && person_score * 10 >= total_score * 7;
        if person_score >= 5 && person_signal_categories >= 2 && has_person_ratio {
            people.push((name, person_score, frequency));
        } else if project_score >= 2 {
            projects.push((name, project_score, frequency));
        }
    }

    people.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then(right.2.cmp(&left.2))
            .then(left.0.cmp(&right.0))
    });
    projects.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then(right.2.cmp(&left.2))
            .then(left.0.cmp(&right.0))
    });

    Ok(DetectedEntities {
        people: people
            .into_iter()
            .map(|(name, _, _)| name)
            .take(15)
            .collect(),
        projects: projects
            .into_iter()
            .map(|(name, _, _)| name)
            .take(10)
            .collect(),
        files_scanned,
    })
}

pub fn detect_entities_for_registry(project_dir: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let detected = detect_entities(project_dir)?;
    Ok((detected.people, detected.projects))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{detect_entities, scan_for_detection};

    #[test]
    fn entity_detector_prefers_prose_files_and_detects_people_projects() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Jordan said the Atlas service should launch next week.\nJordan wrote the Atlas architecture guide.\nhey Jordan, should Atlas ship?",
        )
        .unwrap();
        fs::write(
            project.join("README.txt"),
            "Jordan asked whether Atlas should deploy on Friday.\nJordan pushed the Atlas repo.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();
        assert_eq!(detected.files_scanned, 2);
        assert!(detected.people.iter().any(|name| name == "Jordan"));
        assert!(detected.projects.iter().any(|name| name == "Atlas"));
    }

    #[test]
    fn scan_for_detection_skips_noise_directories() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("target")).unwrap();
        fs::create_dir_all(project.join("notes")).unwrap();
        fs::write(
            project.join("target").join("generated.md"),
            "Jordan said Atlas.",
        )
        .unwrap();
        fs::write(project.join("notes").join("real.md"), "Jordan said Atlas.").unwrap();

        let files = scan_for_detection(&project).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("real.md"));
    }

    #[test]
    fn scan_for_detection_treats_json_as_readable_fallback_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(project.join("docs").join("a.md"), "alpha").unwrap();
        fs::write(project.join("docs").join("b.txt"), "beta").unwrap();
        fs::write(project.join("docs").join("c.csv"), "gamma").unwrap();
        fs::write(
            project.join("docs").join("export.json"),
            r#"{"name":"Atlas"}"#,
        )
        .unwrap();

        let files = scan_for_detection(&project).unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|path| path.ends_with("a.md")));
        assert!(files.iter().any(|path| path.ends_with("b.txt")));
        assert!(files.iter().any(|path| path.ends_with("c.csv")));
        assert!(!files.iter().any(|path| path.ends_with("export.json")));
    }

    #[test]
    fn entity_detector_extracts_multi_word_projects_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "building Atlas Core will help recall.\nAtlas Core architecture is local-first.\nAtlas Core repo should launch soon.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(detected.projects.iter().any(|name| name == "Atlas Core"));
    }

    #[test]
    fn entity_detector_filters_python_stopwords() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Click said the plan changed.\nShe documented the decision.\nClick wrote careful notes.\nHer follow-up was clear.\nClick asked for review.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(!detected.people.iter().any(|name| name == "Click"));
    }

    #[test]
    fn entity_detector_filters_multi_word_phrases_with_python_stopwords() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "building Memory Palace will help recall.\nMemory Palace architecture is local-first.\nMemory Palace repo should launch soon.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(!detected.projects.iter().any(|name| name == "Memory Palace"));
    }

    #[test]
    fn entity_detector_detects_project_install_and_import_markers_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "pip install Atlas for local setup.\nimport Atlas in the harness.\nAtlas.py stores the CLI shim.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(detected.projects.iter().any(|name| name == "Atlas"));
    }

    #[test]
    fn entity_detector_detects_project_version_and_local_markers_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Atlas v2 changed the runtime.\nAtlas-local keeps offline data.\nWe shipped Atlas yesterday.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(detected.projects.iter().any(|name| name == "Atlas"));
    }

    #[test]
    fn entity_detector_requires_python_candidate_frequency() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Jordan said the Atlas service should launch next week.\nJordan wrote the Atlas architecture guide.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(!detected.people.iter().any(|name| name == "Jordan"));
        assert!(!detected.projects.iter().any(|name| name == "Atlas"));
    }

    #[test]
    fn entity_detector_does_not_accept_action_only_people_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Jordan said Atlas should launch soon.\nJordan wrote Atlas notes.\nJordan pushed Atlas repo.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(!detected.people.iter().any(|name| name == "Jordan"));
    }

    #[test]
    fn entity_detector_requires_python_person_ratio() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Morgan said Morgan service should launch soon.\nMorgan wrote Morgan architecture notes.\nhey Morgan, should Morgan repo ship?",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(!detected.people.iter().any(|name| name == "Morgan"));
    }

    #[test]
    fn entity_detector_accepts_action_plus_pronoun_person_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Riley said the plan changed.\nShe documented the decision.\nRiley wrote careful notes.\nHer follow-up was clear.\nRiley joined the review.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(detected.people.iter().any(|name| name == "Riley"));
    }

    #[test]
    fn entity_detector_does_not_accept_pronoun_only_people_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Riley joined review.\nShe shared context.\nRiley joined pairing.\nHer notes were useful.\nRiley joined retro.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(!detected.people.iter().any(|name| name == "Riley"));
    }

    #[test]
    fn entity_detector_accepts_dear_direct_address_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Dear Avery, please review this.\nAvery said the plan changed.\nAvery wrote careful notes.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(detected.people.iter().any(|name| name == "Avery"));
    }

    #[test]
    fn entity_detector_accepts_bracket_dialogue_marker_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "[Avery] Please review this.\nAvery said the plan changed.\nAvery wrote careful notes.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(detected.people.iter().any(|name| name == "Avery"));
    }

    #[test]
    fn entity_detector_accepts_quoted_said_dialogue_marker_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "\"Avery said the plan changed,\" the note says.\nAvery wrote careful notes.\nAvery asked for review.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(detected.people.iter().any(|name| name == "Avery"));
    }

    #[test]
    fn entity_detector_accepts_two_letter_names_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Jo said the plan changed.\nShe documented the decision.\nJo wrote careful notes.\nHer follow-up was clear.\nJo asked for review.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(detected.people.iter().any(|name| name == "Jo"));
    }

    #[test]
    fn entity_detector_ignores_overlong_single_word_candidates_like_python() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Supercalifragilisticexpialidocious said the plan changed.\nShe documented the decision.\nSupercalifragilisticexpialidocious wrote careful notes.\nHer follow-up was clear.\nSupercalifragilisticexpialidocious asked for review.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();

        assert!(
            !detected
                .people
                .iter()
                .any(|name| name == "Supercalifragilisticexpialidocious")
        );
    }
}
