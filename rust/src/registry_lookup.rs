use regex::Regex;

use super::{COMMON_ENGLISH_WORDS, EntityRegistry, RegistryLookupResult, RegistryPerson};

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

impl EntityRegistry {
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

fn regex_matches(pattern: &str, word_lower: &str, ctx_lower: &str) -> bool {
    let pattern = pattern.replace("{name}", &regex::escape(word_lower));
    Regex::new(&pattern)
        .map(|regex| regex.is_match(ctx_lower))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::registry::SeedPerson;

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
}
