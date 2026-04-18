use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

pub const COMMON_ENGLISH_WORDS: &[&str] = &[
    "ever",
    "grace",
    "will",
    "bill",
    "mark",
    "april",
    "may",
    "june",
    "joy",
    "hope",
    "faith",
    "chance",
    "chase",
    "hunter",
    "dash",
    "flash",
    "star",
    "sky",
    "river",
    "brook",
    "lane",
    "art",
    "clay",
    "gil",
    "nat",
    "max",
    "rex",
    "ray",
    "jay",
    "rose",
    "violet",
    "lily",
    "ivy",
    "ash",
    "reed",
    "sage",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "january",
    "february",
    "march",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

const PERSON_CONTEXT_PATTERNS: &[&str] = &[
    r"\b{name}\s+said\b",
    r"\b{name}\s+told\b",
    r"\b{name}\s+asked\b",
    r"\b{name}\s+laughed\b",
    r"\b{name}\s+smiled\b",
    r"\b{name}\s+was\b",
    r"\b{name}\s+is\b",
    r"\b{name}\s+called\b",
    r"\b{name}\s+texted\b",
    r"\bwith\s+{name}\b",
    r"\bsaw\s+{name}\b",
    r"\bcalled\s+{name}\b",
    r"\btook\s+{name}\b",
    r"\bpicked\s+up\s+{name}\b",
    r"\bdrop(?:ped)?\s+(?:off\s+)?{name}\b",
    r"\b{name}(?:'s|s')\b",
    r"\bhey\s+{name}\b",
    r"\bthanks?\s+{name}\b",
    r"^{name}[:\s]",
    r"\bmy\s+(?:son|daughter|kid|child|brother|sister|friend|partner|colleague|coworker)\s+{name}\b",
];

const CONCEPT_CONTEXT_PATTERNS: &[&str] = &[
    r"\bhave\s+you\s+{name}\b",
    r"\bif\s+you\s+{name}\b",
    r"\b{name}\s+since\b",
    r"\b{name}\s+again\b",
    r"\bnot\s+{name}\b",
    r"\b{name}\s+more\b",
    r"\bwould\s+{name}\b",
    r"\bcould\s+{name}\b",
    r"\bwill\s+{name}\b",
    r"(?:the\s+)?{name}\s+(?:of|in|at|for|to)\b",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryPerson {
    pub source: String,
    pub contexts: Vec<String>,
    pub aliases: Vec<String>,
    pub relationship: String,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryResearchEntry {
    pub word: String,
    pub inferred_type: String,
    pub confidence: f64,
    pub wiki_summary: Option<String>,
    pub wiki_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub confirmed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntityRegistry {
    pub version: u8,
    pub mode: String,
    pub people: BTreeMap<String, RegistryPerson>,
    pub projects: Vec<String>,
    pub ambiguous_flags: Vec<String>,
    pub wiki_cache: BTreeMap<String, RegistryResearchEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryLookupResult {
    pub word: String,
    pub r#type: String,
    pub confidence: f64,
    pub source: String,
    pub name: String,
    pub context: Vec<String>,
    pub needs_disambiguation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disambiguated_by: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistrySummary {
    pub kind: String,
    pub registry_path: String,
    pub mode: String,
    pub people_count: usize,
    pub project_count: usize,
    pub ambiguous_flags: Vec<String>,
    pub people: Vec<String>,
    pub projects: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryLearnSummary {
    pub kind: String,
    pub project_path: String,
    pub registry_path: String,
    pub added_people: Vec<String>,
    pub added_projects: Vec<String>,
    pub total_people: usize,
    pub total_projects: usize,
}

const NAME_INDICATOR_PHRASES: &[&str] = &[
    "given name",
    "personal name",
    "first name",
    "forename",
    "masculine name",
    "feminine name",
    "boy's name",
    "girl's name",
    "male name",
    "female name",
    "irish name",
    "welsh name",
    "scottish name",
    "gaelic name",
    "hebrew name",
    "arabic name",
    "norse name",
    "old english name",
    "is a name",
    "as a name",
    "name meaning",
    "name derived from",
    "legendary irish",
    "legendary welsh",
    "legendary scottish",
];

const PLACE_INDICATOR_PHRASES: &[&str] = &[
    "city in",
    "town in",
    "village in",
    "municipality",
    "capital of",
    "district of",
    "county",
    "province",
    "region of",
    "island of",
    "mountain in",
    "river in",
];

impl EntityRegistry {
    pub fn empty(mode: &str) -> Self {
        Self {
            version: 1,
            mode: mode.to_string(),
            people: BTreeMap::new(),
            projects: Vec::new(),
            ambiguous_flags: Vec::new(),
            wiki_cache: BTreeMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::empty("work"))
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn seed(
        &mut self,
        mode: &str,
        people: &[SeedPerson],
        projects: &[String],
        aliases: &BTreeMap<String, String>,
    ) {
        self.mode = mode.to_string();
        self.projects = projects.to_vec();

        let reverse_aliases = aliases
            .iter()
            .map(|(alias, canonical)| (canonical.to_string(), alias.to_string()))
            .collect::<BTreeMap<_, _>>();

        for person in people {
            self.people.insert(
                person.name.clone(),
                RegistryPerson {
                    source: "onboarding".to_string(),
                    contexts: vec![person.context.clone()],
                    aliases: reverse_aliases
                        .get(&person.name)
                        .map(|alias| vec![alias.clone()])
                        .unwrap_or_default(),
                    relationship: person.relationship.clone(),
                    confidence: 1.0,
                    canonical: None,
                },
            );

            if let Some(alias) = reverse_aliases.get(&person.name) {
                self.people.insert(
                    alias.clone(),
                    RegistryPerson {
                        source: "onboarding".to_string(),
                        contexts: vec![person.context.clone()],
                        aliases: vec![person.name.clone()],
                        relationship: person.relationship.clone(),
                        confidence: 1.0,
                        canonical: Some(person.name.clone()),
                    },
                );
            }
        }

        self.recompute_ambiguous_flags();
    }

    pub fn bootstrap(mode: &str, people: &[String], projects: &[String]) -> Self {
        let mut registry = Self::empty(mode);
        for person in people {
            registry.people.insert(
                person.clone(),
                RegistryPerson {
                    source: "bootstrap".to_string(),
                    contexts: vec!["work".to_string()],
                    aliases: Vec::new(),
                    relationship: String::new(),
                    confidence: 1.0,
                    canonical: None,
                },
            );
        }
        registry.projects = projects.to_vec();
        registry.recompute_ambiguous_flags();
        registry
    }

    pub fn learn(&mut self, people: &[String], projects: &[String]) -> RegistryLearnSummaryFields {
        let mut added_people = Vec::new();
        let mut added_projects = Vec::new();

        for person in people {
            if !self
                .people
                .keys()
                .any(|existing| existing.eq_ignore_ascii_case(person))
            {
                self.people.insert(
                    person.clone(),
                    RegistryPerson {
                        source: "learned".to_string(),
                        contexts: vec![self.mode_context()],
                        aliases: Vec::new(),
                        relationship: String::new(),
                        confidence: 0.8,
                        canonical: None,
                    },
                );
                added_people.push(person.clone());
            }
        }

        for project in projects {
            if !self
                .projects
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(project))
            {
                self.projects.push(project.clone());
                added_projects.push(project.clone());
            }
        }

        self.projects.sort();
        self.projects
            .dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        self.recompute_ambiguous_flags();

        RegistryLearnSummaryFields {
            added_people,
            added_projects,
            total_people: self.people.len(),
            total_projects: self.projects.len(),
        }
    }

    pub fn add_person(&mut self, name: &str, relationship: &str, context: &str) {
        self.people.insert(
            name.to_string(),
            RegistryPerson {
                source: "manual".to_string(),
                contexts: vec![context.to_string()],
                aliases: Vec::new(),
                relationship: relationship.to_string(),
                confidence: 1.0,
                canonical: None,
            },
        );
        self.recompute_ambiguous_flags();
    }

    pub fn add_project(&mut self, project: &str) {
        if !self
            .projects
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(project))
        {
            self.projects.push(project.to_string());
            self.projects.sort();
        }
    }

    pub fn add_alias(&mut self, canonical: &str, alias: &str) {
        let cloned = if let Some(person) = self.people.get_mut(canonical) {
            if !person
                .aliases
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(alias))
            {
                person.aliases.push(alias.to_string());
                person.aliases.sort();
            }
            Some((
                person.source.clone(),
                person.contexts.clone(),
                person.relationship.clone(),
                person.confidence,
            ))
        } else {
            None
        };
        if let Some((source, contexts, relationship, confidence)) = cloned {
            self.people.insert(
                alias.to_string(),
                RegistryPerson {
                    source,
                    contexts,
                    aliases: vec![canonical.to_string()],
                    relationship,
                    confidence,
                    canonical: Some(canonical.to_string()),
                },
            );
        }
        self.recompute_ambiguous_flags();
    }

    pub fn lookup(&self, word: &str, context: &str) -> RegistryLookupResult {
        for (canonical, info) in &self.people {
            let aliases = info.aliases.iter().map(|alias| alias.to_ascii_lowercase());
            if word.eq_ignore_ascii_case(canonical)
                || aliases
                    .into_iter()
                    .any(|alias| alias == word.to_ascii_lowercase())
            {
                if self
                    .ambiguous_flags
                    .iter()
                    .any(|flag| flag.eq_ignore_ascii_case(word))
                    && !context.trim().is_empty()
                    && let Some(result) = self.disambiguate(word, context, canonical, info)
                {
                    return result;
                }

                return RegistryLookupResult {
                    word: word.to_string(),
                    r#type: "person".to_string(),
                    confidence: info.confidence,
                    source: info.source.clone(),
                    name: canonical.clone(),
                    context: info.contexts.clone(),
                    needs_disambiguation: false,
                    disambiguated_by: None,
                };
            }
        }

        for project in &self.projects {
            if word.eq_ignore_ascii_case(project) {
                return RegistryLookupResult {
                    word: word.to_string(),
                    r#type: "project".to_string(),
                    confidence: 1.0,
                    source: "onboarding".to_string(),
                    name: project.clone(),
                    context: vec!["work".to_string()],
                    needs_disambiguation: false,
                    disambiguated_by: None,
                };
            }
        }

        RegistryLookupResult {
            word: word.to_string(),
            r#type: "unknown".to_string(),
            confidence: 0.0,
            source: "none".to_string(),
            name: word.to_string(),
            context: Vec::new(),
            needs_disambiguation: false,
            disambiguated_by: None,
        }
    }

    pub fn summary(&self, registry_path: &Path) -> RegistrySummary {
        let mut people = self.people.keys().cloned().collect::<Vec<_>>();
        people.sort();
        RegistrySummary {
            kind: "registry_summary".to_string(),
            registry_path: registry_path.display().to_string(),
            mode: self.mode.clone(),
            people_count: self.people.len(),
            project_count: self.projects.len(),
            ambiguous_flags: self.ambiguous_flags.clone(),
            people,
            projects: self.projects.clone(),
        }
    }

    pub fn extract_people_from_query(&self, query: &str) -> Vec<String> {
        let mut found = Vec::new();
        for (canonical, info) in &self.people {
            let canonical_name = info
                .canonical
                .as_ref()
                .cloned()
                .unwrap_or_else(|| canonical.clone());
            let names = std::iter::once(canonical.as_str())
                .chain(info.aliases.iter().map(String::as_str))
                .collect::<Vec<_>>();
            for name in names {
                let pattern = format!(r"\b{}\b", regex::escape(name));
                let matches = Regex::new(&pattern)
                    .map(|regex| regex.is_match(query))
                    .unwrap_or(false);
                if !matches {
                    continue;
                }

                if self
                    .ambiguous_flags
                    .iter()
                    .any(|flag| flag.eq_ignore_ascii_case(name))
                {
                    let resolved = self.lookup(name, query);
                    if resolved.r#type == "person" && !found.contains(&canonical_name) {
                        found.push(canonical_name.clone());
                    }
                } else if !found.contains(&canonical_name) {
                    found.push(canonical_name.clone());
                }
            }
        }
        found
    }

    pub fn extract_unknown_candidates(&self, query: &str) -> Vec<String> {
        let regex = Regex::new(r"\b[A-Z][a-z]{2,15}\b").expect("capitalized word regex");
        let mut unknown = regex
            .captures_iter(query)
            .filter_map(|capture| capture.get(0).map(|value| value.as_str().to_string()))
            .filter(|word| {
                !COMMON_ENGLISH_WORDS
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(word))
                    && self.lookup(word, "").r#type == "unknown"
            })
            .collect::<Vec<_>>();
        unknown.sort();
        unknown.dedup();
        unknown
    }

    pub fn research(&mut self, word: &str, auto_confirm: bool) -> Result<RegistryResearchEntry> {
        if let Some(cached) = self.wiki_cache.get(word) {
            return Ok(cached.clone());
        }

        let mut result = wikipedia_lookup(word)?;
        result.confirmed = auto_confirm;
        self.wiki_cache.insert(word.to_string(), result.clone());
        Ok(result)
    }

    pub fn confirm_research(
        &mut self,
        word: &str,
        entity_type: &str,
        relationship: &str,
        context: &str,
    ) {
        if let Some(cached) = self.wiki_cache.get_mut(word) {
            cached.confirmed = true;
            cached.confirmed_type = Some(entity_type.to_string());
        }

        if entity_type == "person" {
            self.people.insert(
                word.to_string(),
                RegistryPerson {
                    source: "wiki".to_string(),
                    contexts: vec![context.to_string()],
                    aliases: Vec::new(),
                    relationship: relationship.to_string(),
                    confidence: 0.9,
                    canonical: None,
                },
            );
            if COMMON_ENGLISH_WORDS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(word))
                && !self
                    .ambiguous_flags
                    .iter()
                    .any(|flag| flag.eq_ignore_ascii_case(word))
            {
                self.ambiguous_flags.push(word.to_ascii_lowercase());
                self.ambiguous_flags.sort();
                self.ambiguous_flags.dedup();
            }
        }

        self.recompute_ambiguous_flags();
    }

    fn recompute_ambiguous_flags(&mut self) {
        let mut flags = self
            .people
            .keys()
            .filter(|person| {
                COMMON_ENGLISH_WORDS
                    .iter()
                    .any(|word| word.eq_ignore_ascii_case(person))
            })
            .map(|person| person.to_ascii_lowercase())
            .collect::<Vec<_>>();
        flags.sort();
        flags.dedup();
        self.ambiguous_flags = flags;
    }

    fn mode_context(&self) -> String {
        if self.mode == "combo" {
            "personal".to_string()
        } else {
            self.mode.clone()
        }
    }

    fn disambiguate(
        &self,
        word: &str,
        context: &str,
        canonical: &str,
        info: &RegistryPerson,
    ) -> Option<RegistryLookupResult> {
        let ctx_lower = context.to_ascii_lowercase();
        let word_lower = word.to_ascii_lowercase();

        let person_score = PERSON_CONTEXT_PATTERNS
            .iter()
            .filter(|pattern| regex_matches(pattern, &word_lower, &ctx_lower))
            .count();
        let concept_score = CONCEPT_CONTEXT_PATTERNS
            .iter()
            .filter(|pattern| regex_matches(pattern, &word_lower, &ctx_lower))
            .count();

        if person_score > concept_score {
            return Some(RegistryLookupResult {
                word: word.to_string(),
                r#type: "person".to_string(),
                confidence: (0.7 + person_score as f64 * 0.1).min(0.95),
                source: info.source.clone(),
                name: canonical.to_string(),
                context: info.contexts.clone(),
                needs_disambiguation: false,
                disambiguated_by: Some("context_patterns".to_string()),
            });
        }

        if concept_score > person_score {
            return Some(RegistryLookupResult {
                word: word.to_string(),
                r#type: "concept".to_string(),
                confidence: (0.7 + concept_score as f64 * 0.1).min(0.9),
                source: "context_disambiguated".to_string(),
                name: word.to_string(),
                context: Vec::new(),
                needs_disambiguation: false,
                disambiguated_by: Some("context_patterns".to_string()),
            });
        }

        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeedPerson {
    pub name: String,
    pub relationship: String,
    pub context: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegistryLearnSummaryFields {
    pub added_people: Vec<String>,
    pub added_projects: Vec<String>,
    pub total_people: usize,
    pub total_projects: usize,
}

fn regex_matches(pattern: &str, word_lower: &str, ctx_lower: &str) -> bool {
    let pattern = pattern.replace("{name}", &regex::escape(word_lower));
    Regex::new(&pattern)
        .map(|regex| regex.is_match(ctx_lower))
        .unwrap_or(false)
}

fn wikipedia_lookup(word: &str) -> Result<RegistryResearchEntry> {
    let url = format!(
        "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
        urlencoding(word)
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();

    match agent.get(&url).set("User-Agent", "MemPalace-RS/1.0").call() {
        Ok(response) => {
            let body: Value = response.into_json()?;
            Ok(classify_wikipedia_summary(word, &body))
        }
        Err(ureq::Error::Status(404, _)) => Ok(RegistryResearchEntry {
            word: word.to_string(),
            inferred_type: "person".to_string(),
            confidence: 0.7,
            wiki_summary: None,
            wiki_title: None,
            note: Some("not found in Wikipedia - likely a proper noun or unusual name".to_string()),
            confirmed: false,
            confirmed_type: None,
        }),
        Err(err) => Err(anyhow::anyhow!("Wikipedia lookup failed for {word}: {err}").into()),
    }
}

fn classify_wikipedia_summary(word: &str, body: &Value) -> RegistryResearchEntry {
    let page_type = body
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let extract = body
        .get("extract")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    let description = body
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();

    if page_type == "disambiguation" {
        if description.contains("name") || description.contains("given name") {
            return RegistryResearchEntry {
                word: word.to_string(),
                inferred_type: "person".to_string(),
                confidence: 0.65,
                wiki_summary: truncated_summary(&extract),
                wiki_title: title,
                note: Some("disambiguation page with name entries".to_string()),
                confirmed: false,
                confirmed_type: None,
            };
        }
        return RegistryResearchEntry {
            word: word.to_string(),
            inferred_type: "ambiguous".to_string(),
            confidence: 0.4,
            wiki_summary: truncated_summary(&extract),
            wiki_title: title,
            note: None,
            confirmed: false,
            confirmed_type: None,
        };
    }

    if NAME_INDICATOR_PHRASES
        .iter()
        .any(|phrase| extract.contains(phrase))
    {
        let exact_name_signal = extract.contains(&format!("{} is a", word.to_ascii_lowercase()))
            || extract.contains(&format!("{} (name", word.to_ascii_lowercase()));
        return RegistryResearchEntry {
            word: word.to_string(),
            inferred_type: "person".to_string(),
            confidence: if exact_name_signal { 0.9 } else { 0.8 },
            wiki_summary: truncated_summary(&extract),
            wiki_title: title,
            note: None,
            confirmed: false,
            confirmed_type: None,
        };
    }

    if PLACE_INDICATOR_PHRASES
        .iter()
        .any(|phrase| extract.contains(phrase))
    {
        return RegistryResearchEntry {
            word: word.to_string(),
            inferred_type: "place".to_string(),
            confidence: 0.8,
            wiki_summary: truncated_summary(&extract),
            wiki_title: title,
            note: None,
            confirmed: false,
            confirmed_type: None,
        };
    }

    RegistryResearchEntry {
        word: word.to_string(),
        inferred_type: "concept".to_string(),
        confidence: if extract.is_empty() { 0.0 } else { 0.6 },
        wiki_summary: truncated_summary(&extract),
        wiki_title: title,
        note: None,
        confirmed: false,
        confirmed_type: None,
    }
}

fn truncated_summary(summary: &str) -> Option<String> {
    if summary.is_empty() {
        return None;
    }
    Some(summary.chars().take(200).collect())
}

fn urlencoding(word: &str) -> String {
    word.chars()
        .flat_map(|ch| match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => vec![ch.to_string()],
            _ => ch
                .to_string()
                .as_bytes()
                .iter()
                .map(|byte| format!("%{byte:02X}"))
                .collect::<Vec<_>>(),
        })
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn lookup_disambiguates_ambiguous_names_with_context() {
        let mut registry = EntityRegistry::empty("personal");
        registry.seed(
            "personal",
            &[SeedPerson {
                name: "Ever".to_string(),
                relationship: "friend".to_string(),
                context: "personal".to_string(),
            }],
            &[],
            &BTreeMap::new(),
        );

        let as_person = registry.lookup("Ever", "Ever said the project was ready.");
        assert_eq!(as_person.r#type, "person");

        let as_concept = registry.lookup("Ever", "Have you ever tried this before?");
        assert_eq!(as_concept.r#type, "concept");
    }

    #[test]
    fn registry_load_save_round_trip() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("entity_registry.json");
        let mut registry = EntityRegistry::empty("work");
        registry.seed(
            "work",
            &[SeedPerson {
                name: "Jordan".to_string(),
                relationship: "coworker".to_string(),
                context: "work".to_string(),
            }],
            &["Atlas".to_string()],
            &BTreeMap::new(),
        );
        registry.save(&path).unwrap();

        let loaded = EntityRegistry::load(&path).unwrap();
        assert!(loaded.people.contains_key("Jordan"));
        assert_eq!(loaded.projects, vec!["Atlas".to_string()]);
    }

    #[test]
    fn registry_extracts_people_and_unknown_candidates_from_query() {
        let mut registry = EntityRegistry::empty("work");
        registry.seed(
            "work",
            &[SeedPerson {
                name: "Jordan".to_string(),
                relationship: "coworker".to_string(),
                context: "work".to_string(),
            }],
            &["Atlas".to_string()],
            &BTreeMap::new(),
        );
        registry.add_alias("Jordan", "Jordy");

        let people = registry.extract_people_from_query("Jordy said Atlas should ship with Riley.");
        assert_eq!(people, vec!["Jordan".to_string()]);

        let unknown =
            registry.extract_unknown_candidates("Jordy said Atlas should ship with Riley.");
        assert_eq!(unknown, vec!["Riley".to_string()]);
    }

    #[test]
    fn wikipedia_classifier_detects_names_and_places() {
        let person = classify_wikipedia_summary(
            "Riley",
            &serde_json::json!({
                "type": "standard",
                "title": "Riley",
                "extract": "Riley is a given name used in English."
            }),
        );
        assert_eq!(person.inferred_type, "person");
        assert!(person.confidence >= 0.8);

        let place = classify_wikipedia_summary(
            "Lanark",
            &serde_json::json!({
                "type": "standard",
                "title": "Lanark",
                "extract": "Lanark is a town in Scotland and a historic county seat."
            }),
        );
        assert_eq!(place.inferred_type, "place");
        assert!(place.confidence >= 0.8);
    }

    #[test]
    fn confirm_research_promotes_person_into_registry() {
        let mut registry = EntityRegistry::empty("personal");
        registry.wiki_cache.insert(
            "Riley".to_string(),
            RegistryResearchEntry {
                word: "Riley".to_string(),
                inferred_type: "person".to_string(),
                confidence: 0.9,
                wiki_summary: Some("riley is a given name".to_string()),
                wiki_title: Some("Riley".to_string()),
                note: None,
                confirmed: false,
                confirmed_type: None,
            },
        );

        registry.confirm_research("Riley", "person", "daughter", "personal");

        assert_eq!(registry.people["Riley"].source, "wiki");
        assert!(registry.wiki_cache["Riley"].confirmed);
        assert_eq!(
            registry.wiki_cache["Riley"].confirmed_type.as_deref(),
            Some("person")
        );
    }
}
