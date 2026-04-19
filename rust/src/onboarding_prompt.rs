use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::Path;

use crate::error::Result;
use crate::registry::SeedPerson;

use crate::onboarding::OnboardingRequest;
use crate::onboarding_support::{default_wings, split_name_relationship};

pub fn prompt_for_request(
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

pub fn ask_yes_no(prompt_text: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    let answer = prompt(&format!("{prompt_text} {suffix}"), None)?;
    if answer.trim().is_empty() {
        return Ok(default_yes);
    }
    Ok(answer.to_ascii_lowercase().starts_with('y'))
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
