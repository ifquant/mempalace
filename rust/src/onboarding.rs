//! Interactive and non-interactive onboarding flow for project-local world setup.
//!
//! This module coordinates prompt collection, optional auto-detection, dedupe, and
//! final bootstrap file writes for users who want more control than `init`.

use std::collections::BTreeMap;
use std::io::{self, IsTerminal};
use std::path::Path;

use crate::VERSION;
use crate::bootstrap::{
    default_wing, write_aaak_entities, write_critical_facts, write_entities,
    write_project_config_from_names,
};
use crate::error::{MempalaceError, Result};
use crate::model::OnboardingSummary;
use crate::onboarding_prompt::{ask_yes_no, prompt_for_request};
use crate::onboarding_support::{
    auto_detect_entities, dedupe_aliases, dedupe_list, dedupe_people, default_person_context,
    default_wings, merge_detected_people, merge_detected_projects, normalize_mode, should_prompt,
};
use crate::registry::{EntityRegistry, SeedPerson};

pub use crate::onboarding_support::{ambiguous_names, parse_alias_arg, parse_person_arg};

#[derive(Clone, Debug, Default, PartialEq)]
/// User-supplied onboarding inputs before prompting/dedupe/default expansion.
pub struct OnboardingRequest {
    pub mode: Option<String>,
    pub people: Vec<SeedPerson>,
    pub projects: Vec<String>,
    pub aliases: BTreeMap<String, String>,
    pub wings: Vec<String>,
    pub scan: Option<bool>,
    pub auto_accept_detected: bool,
}

/// Runs onboarding for one project directory and writes the requested bootstrap artifacts.
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
    request.aliases = dedupe_aliases(&request.aliases);

    let should_scan = request.scan.unwrap_or(false);
    let mut auto_detected_people = Vec::new();
    let mut auto_detected_projects = Vec::new();

    if should_scan {
        let (detected_people, detected_projects) = auto_detect_entities(&project_dir)?;
        // Merge is case-insensitive and asks/auto-accepts only for names that are
        // not already present in the explicit onboarding request.
        auto_detected_people = merge_detected_people(
            &mut request.people,
            &detected_people,
            context_default,
            request.auto_accept_detected,
            interactive,
            |name| ask_yes_no(&format!("Add detected person {name}?"), true),
        )?;
        auto_detected_projects = merge_detected_projects(
            &mut request.projects,
            &detected_projects,
            request.auto_accept_detected,
            interactive,
            |name| ask_yes_no(&format!("Add detected project {name}?"), true),
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
