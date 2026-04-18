use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use crate::VERSION;
use crate::bootstrap::{
    default_wing, write_aaak_entities, write_critical_facts, write_entities,
    write_project_config_from_names,
};
use crate::entity_detector::detect_entities_for_registry;
use crate::error::{MempalaceError, Result};
use crate::model::OnboardingSummary;
use crate::registry::{COMMON_ENGLISH_WORDS, EntityRegistry, SeedPerson};

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OnboardingRequest {
    pub mode: Option<String>,
    pub people: Vec<SeedPerson>,
    pub projects: Vec<String>,
    pub aliases: BTreeMap<String, String>,
    pub wings: Vec<String>,
    pub scan: Option<bool>,
    pub auto_accept_detected: bool,
}

pub fn run_onboarding(project_dir: &Path, request: OnboardingRequest) -> Result<OnboardingSummary> {
    let project_dir = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());
    if !project_dir.exists() {
        return Err(MempalaceError::InvalidArgument(format!(
            "Project directory does not exist: {}",
            project_dir.display()
        )));
    }

    let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
    let mut request = if should_prompt(&request) && interactive {
        prompt_for_request(&project_dir, request)?
    } else {
        request
    };

    let mode = normalize_mode(request.mode.as_deref().unwrap_or("work"))?;
    let wing = default_wing(&project_dir);
    let context_default = default_person_context(&mode);

    for person in &mut request.people {
        if person.context.trim().is_empty() {
            person.context = context_default.to_string();
        }
    }

    if request.wings.is_empty() {
        request.wings = default_wings(&mode);
    }
    request.wings = dedupe_list(&request.wings);
    request.projects = dedupe_list(&request.projects);
    request.people = dedupe_people(&request.people, context_default);

    let should_scan = request.scan.unwrap_or(false);
    let mut auto_detected_people = Vec::new();
    let mut auto_detected_projects = Vec::new();

    if should_scan {
        let (detected_people, detected_projects) = detect_entities_for_registry(&project_dir)?;
        auto_detected_people = merge_detected_people(
            &mut request.people,
            &detected_people,
            context_default,
            request.auto_accept_detected,
            interactive,
        )?;
        auto_detected_projects = merge_detected_projects(
            &mut request.projects,
            &detected_projects,
            request.auto_accept_detected,
            interactive,
        )?;
    }

    let people_names = request
        .people
        .iter()
        .filter_map(|person| {
            let trimmed = person.name.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect::<Vec<_>>();

    let config_path = project_dir.join("mempalace.yaml");
    let config_written = if config_path.exists() {
        false
    } else {
        write_project_config_from_names(&config_path, &wing, &request.wings)?;
        true
    };

    let entities_path = project_dir.join("entities.json");
    write_entities(&entities_path, &people_names, &request.projects)?;

    let entity_registry_path = project_dir.join("entity_registry.json");
    let mut registry = if entity_registry_path.exists() {
        EntityRegistry::load(&entity_registry_path)?
    } else {
        EntityRegistry::empty(&mode)
    };
    registry.seed(&mode, &request.people, &request.projects, &request.aliases);
    registry.save(&entity_registry_path)?;

    let aaak_entities_path = project_dir.join("aaak_entities.md");
    write_aaak_entities(&aaak_entities_path, &people_names, &request.projects, &mode)?;

    let critical_facts_path = project_dir.join("critical_facts.md");
    write_critical_facts(
        &critical_facts_path,
        &people_names,
        &request.projects,
        &request.wings,
        &wing,
        &mode,
    )?;

    Ok(OnboardingSummary {
        kind: "onboarding".to_string(),
        project_path: project_dir.display().to_string(),
        mode,
        wing,
        wings: request.wings,
        people: people_names,
        projects: request.projects,
        aliases: request.aliases,
        ambiguous_flags: registry.ambiguous_flags,
        auto_detected_people,
        auto_detected_projects,
        config_path: Some(config_path.display().to_string()),
        config_written,
        entities_path: Some(entities_path.display().to_string()),
        entities_written: true,
        entity_registry_path: entity_registry_path.display().to_string(),
        entity_registry_written: true,
        aaak_entities_path: aaak_entities_path.display().to_string(),
        aaak_entities_written: true,
        critical_facts_path: critical_facts_path.display().to_string(),
        critical_facts_written: true,
        version: VERSION.to_string(),
    })
}

fn should_prompt(request: &OnboardingRequest) -> bool {
    request.mode.is_none()
        && request.people.is_empty()
        && request.projects.is_empty()
        && request.aliases.is_empty()
        && request.wings.is_empty()
        && request.scan.is_none()
        && !request.auto_accept_detected
}

fn normalize_mode(value: &str) -> Result<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "work" => Ok("work".to_string()),
        "personal" => Ok("personal".to_string()),
        "combo" | "both" => Ok("combo".to_string()),
        other => Err(MempalaceError::InvalidArgument(format!(
            "Unsupported onboarding mode: {other}"
        ))),
    }
}

fn default_wings(mode: &str) -> Vec<String> {
    match mode {
        "work" => DEFAULT_WORK_WINGS,
        "personal" => DEFAULT_PERSONAL_WINGS,
        _ => DEFAULT_COMBO_WINGS,
    }
    .iter()
    .map(|value| (*value).to_string())
    .collect()
}

fn default_person_context(mode: &str) -> &'static str {
    match mode {
        "personal" => "personal",
        _ => "work",
    }
}

fn dedupe_list(values: &[String]) -> Vec<String> {
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

fn dedupe_people(values: &[SeedPerson], default_context: &str) -> Vec<SeedPerson> {
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

fn merge_detected_people(
    people: &mut Vec<SeedPerson>,
    detected_people: &[String],
    default_context: &str,
    auto_accept: bool,
    interactive: bool,
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
            ask_yes_no(&format!("Add detected person {name}?"), true)?
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

fn merge_detected_projects(
    projects: &mut Vec<String>,
    detected_projects: &[String],
    auto_accept: bool,
    interactive: bool,
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
            ask_yes_no(&format!("Add detected project {name}?"), true)?
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

fn prompt_for_request(
    project_dir: &Path,
    mut request: OnboardingRequest,
) -> Result<OnboardingRequest> {
    print_header("Welcome to MemPalace onboarding")?;
    println!("  This seeds your local registry and AAAK bootstrap docs before mining.");
    println!("  Project: {}", project_dir.display());
    println!();

    request.mode = Some(prompt_mode()?);
    let mode = request.mode.clone().unwrap_or_else(|| "work".to_string());
    let (people, aliases) = prompt_people(&mode)?;
    request.people = people;
    request.aliases = aliases;
    request.projects = prompt_projects(&mode)?;
    request.wings = prompt_wings(&mode)?;
    request.scan = Some(ask_yes_no("Scan local files for additional names?", true)?);
    Ok(request)
}

fn prompt_mode() -> Result<String> {
    loop {
        println!("  How are you using MemPalace?");
        println!("    [1] Work");
        println!("    [2] Personal");
        println!("    [3] Both");
        let choice = prompt("  Your choice [1/2/3]", None)?;
        match choice.trim() {
            "1" => return Ok("work".to_string()),
            "2" => return Ok("personal".to_string()),
            "3" => return Ok("combo".to_string()),
            _ => {
                println!("  Please enter 1, 2, or 3.");
            }
        }
    }
}

fn prompt_people(mode: &str) -> Result<(Vec<SeedPerson>, BTreeMap<String, String>)> {
    let mut people = Vec::new();
    let mut aliases = BTreeMap::new();

    if matches!(mode, "personal" | "combo") {
        print_rule()?;
        println!("  Personal people: name, relationship. Empty line to finish.");
        loop {
            let entry = prompt("  Person", None)?;
            if entry.trim().is_empty() {
                break;
            }
            let (name, relationship) = split_name_relationship(&entry);
            if name.is_empty() {
                continue;
            }
            let nickname = prompt(&format!("  Nickname for {name} (optional)"), None)?;
            if !nickname.trim().is_empty() {
                aliases.insert(nickname.trim().to_string(), name.to_string());
            }
            people.push(SeedPerson {
                name: name.to_string(),
                relationship: relationship.to_string(),
                context: "personal".to_string(),
            });
        }
    }

    if matches!(mode, "work" | "combo") {
        print_rule()?;
        println!("  Work people: name, role. Empty line to finish.");
        loop {
            let entry = prompt("  Person", None)?;
            if entry.trim().is_empty() {
                break;
            }
            let (name, relationship) = split_name_relationship(&entry);
            if name.is_empty() {
                continue;
            }
            people.push(SeedPerson {
                name: name.to_string(),
                relationship: relationship.to_string(),
                context: "work".to_string(),
            });
        }
    }

    Ok((people, aliases))
}

fn prompt_projects(mode: &str) -> Result<Vec<String>> {
    if mode == "personal" {
        return Ok(Vec::new());
    }
    print_rule()?;
    println!("  Projects: one per line. Empty line to finish.");
    let mut projects = Vec::new();
    loop {
        let entry = prompt("  Project", None)?;
        if entry.trim().is_empty() {
            break;
        }
        projects.push(entry.trim().to_string());
    }
    Ok(projects)
}

fn prompt_wings(mode: &str) -> Result<Vec<String>> {
    let defaults = default_wings(mode);
    print_rule()?;
    println!("  Suggested wings: {}", defaults.join(", "));
    let custom = prompt("  Wings (comma-separated, enter to keep defaults)", None)?;
    if custom.trim().is_empty() {
        return Ok(defaults);
    }
    Ok(custom
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect())
}

fn split_name_relationship(input: &str) -> (&str, &str) {
    let mut parts = input.splitn(2, ',').map(str::trim);
    let name = parts.next().unwrap_or_default();
    let relationship = parts.next().unwrap_or_default();
    (name, relationship)
}

fn prompt(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut stdout = io::stdout();
    if let Some(default) = default {
        write!(stdout, "{prompt} [{default}]: ")?;
    } else {
        write!(stdout, "{prompt}: ")?;
    }
    stdout.flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default.unwrap_or_default().to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn ask_yes_no(prompt_text: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    let answer = prompt(&format!("{prompt_text} {suffix}"), None)?;
    if answer.trim().is_empty() {
        return Ok(default_yes);
    }
    Ok(answer.to_ascii_lowercase().starts_with('y'))
}

fn print_header(text: &str) -> Result<()> {
    let mut stdout = io::stdout();
    writeln!(
        stdout,
        "\n=========================================================="
    )?;
    writeln!(stdout, "  {text}")?;
    writeln!(
        stdout,
        "=========================================================="
    )?;
    Ok(())
}

fn print_rule() -> Result<()> {
    let mut stdout = io::stdout();
    writeln!(
        stdout,
        "\n----------------------------------------------------------"
    )?;
    Ok(())
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
