use std::collections::{BTreeMap, BTreeSet};

use crate::entity_detector::detect_entities_for_registry;
use crate::error::{MempalaceError, Result};
use crate::registry::{COMMON_ENGLISH_WORDS, SeedPerson};

use crate::onboarding::OnboardingRequest;

const DEFAULT_WORK_WINGS: &[&str] = &["projects", "clients", "team", "decisions", "research"];
const DEFAULT_PERSONAL_WINGS: &[&str] = &[
    "family",
    "health",
    "creative",
    "reflections",
    "relationships",
];
const DEFAULT_COMBO_WINGS: &[&str] = &[
    "family",
    "work",
    "health",
    "creative",
    "projects",
    "reflections",
];

pub fn should_prompt(request: &OnboardingRequest) -> bool {
    request.mode.is_none()
        && request.people.is_empty()
        && request.projects.is_empty()
        && request.aliases.is_empty()
        && request.wings.is_empty()
        && request.scan.is_none()
        && !request.auto_accept_detected
}

pub fn normalize_mode(value: &str) -> Result<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "work" => Ok("work".to_string()),
        "personal" => Ok("personal".to_string()),
        "combo" | "both" => Ok("combo".to_string()),
        other => Err(MempalaceError::InvalidArgument(format!(
            "Unsupported onboarding mode: {other}"
        ))),
    }
}

pub fn default_wings(mode: &str) -> Vec<String> {
    match mode {
        "work" => DEFAULT_WORK_WINGS,
        "personal" => DEFAULT_PERSONAL_WINGS,
        _ => DEFAULT_COMBO_WINGS,
    }
    .iter()
    .map(|value| (*value).to_string())
    .collect()
}

pub fn default_person_context(mode: &str) -> &'static str {
    match mode {
        "personal" => "personal",
        _ => "work",
    }
}

pub fn dedupe_list(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_lowercase();
        if seen.insert(key) {
            output.push(trimmed.to_string());
        }
    }
    output
}

pub fn dedupe_people(values: &[SeedPerson], default_context: &str) -> Vec<SeedPerson> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for value in values {
        let name = value.name.trim();
        if name.is_empty() {
            continue;
        }
        let key = name.to_ascii_lowercase();
        if seen.insert(key) {
            output.push(SeedPerson {
                name: name.to_string(),
                relationship: value.relationship.trim().to_string(),
                context: if value.context.trim().is_empty() {
                    default_context.to_string()
                } else {
                    value.context.trim().to_ascii_lowercase()
                },
            });
        }
    }
    output
}

pub fn merge_detected_people(
    people: &mut Vec<SeedPerson>,
    detected_people: &[String],
    default_context: &str,
    auto_accept: bool,
    interactive: bool,
    mut confirm: impl FnMut(&str) -> Result<bool>,
) -> Result<Vec<String>> {
    let existing = people
        .iter()
        .map(|person| person.name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut accepted = Vec::new();
    for name in detected_people {
        if existing.contains(&name.to_ascii_lowercase()) {
            continue;
        }
        let add = if auto_accept {
            true
        } else if interactive {
            confirm(name)?
        } else {
            false
        };
        if add {
            accepted.push(name.to_string());
            people.push(SeedPerson {
                name: name.to_string(),
                relationship: String::new(),
                context: default_context.to_string(),
            });
        }
    }
    Ok(accepted)
}

pub fn merge_detected_projects(
    projects: &mut Vec<String>,
    detected_projects: &[String],
    auto_accept: bool,
    interactive: bool,
    mut confirm: impl FnMut(&str) -> Result<bool>,
) -> Result<Vec<String>> {
    let existing = projects
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut accepted = Vec::new();
    for name in detected_projects {
        if existing.contains(&name.to_ascii_lowercase()) {
            continue;
        }
        let add = if auto_accept {
            true
        } else if interactive {
            confirm(name)?
        } else {
            false
        };
        if add {
            accepted.push(name.to_string());
            projects.push(name.to_string());
        }
    }
    Ok(accepted)
}

pub fn auto_detect_entities(project_dir: &std::path::Path) -> Result<(Vec<String>, Vec<String>)> {
    detect_entities_for_registry(project_dir)
}

pub fn split_name_relationship(input: &str) -> (&str, &str) {
    let mut parts = input.splitn(2, ',').map(str::trim);
    let name = parts.next().unwrap_or_default();
    let relationship = parts.next().unwrap_or_default();
    (name, relationship)
}

pub fn parse_person_arg(value: &str) -> Result<SeedPerson> {
    let parts = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return Err(MempalaceError::InvalidArgument(
            "Person must include at least a name".to_string(),
        ));
    }
    Ok(SeedPerson {
        name: parts[0].to_string(),
        relationship: parts.get(1).copied().unwrap_or_default().to_string(),
        context: parts.get(2).copied().unwrap_or_default().to_string(),
    })
}

pub fn parse_alias_arg(value: &str) -> Result<(String, String)> {
    let Some((alias, canonical)) = value.split_once('=') else {
        return Err(MempalaceError::InvalidArgument(
            "Alias must use alias=canonical format".to_string(),
        ));
    };
    let alias = alias.trim();
    let canonical = canonical.trim();
    if alias.is_empty() || canonical.is_empty() {
        return Err(MempalaceError::InvalidArgument(
            "Alias must use alias=canonical format".to_string(),
        ));
    }
    Ok((alias.to_string(), canonical.to_string()))
}

pub fn ambiguous_names(people: &[String]) -> Vec<String> {
    let common = COMMON_ENGLISH_WORDS
        .iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    people
        .iter()
        .filter(|name| common.contains(&name.to_ascii_lowercase()))
        .cloned()
        .collect()
}

pub fn dedupe_aliases(aliases: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    aliases
        .iter()
        .filter_map(|(alias, canonical)| {
            let alias = alias.trim();
            let canonical = canonical.trim();
            if alias.is_empty() || canonical.is_empty() {
                None
            } else {
                Some((alias.to_string(), canonical.to_string()))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ambiguous_names, parse_alias_arg, parse_person_arg};

    #[test]
    fn parse_person_arg_supports_name_relationship_and_context() {
        let person = parse_person_arg("Riley,daughter,personal").unwrap();
        assert_eq!(person.name, "Riley");
        assert_eq!(person.relationship, "daughter");
        assert_eq!(person.context, "personal");
    }

    #[test]
    fn parse_alias_arg_requires_alias_equals_canonical() {
        let (alias, canonical) = parse_alias_arg("Ry=Riley").unwrap();
        assert_eq!(alias, "Ry");
        assert_eq!(canonical, "Riley");
    }

    #[test]
    fn ambiguous_names_flags_common_english_words() {
        let names = ambiguous_names(&["Ever".to_string(), "Riley".to_string()]);
        assert_eq!(names, vec!["Ever".to_string()]);
    }
}
