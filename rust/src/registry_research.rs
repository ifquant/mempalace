use std::time::Duration;

use serde_json::Value;

use crate::error::Result;
use crate::registry_types::RegistryResearchEntry;

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

pub fn wikipedia_lookup(word: &str) -> Result<RegistryResearchEntry> {
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

pub fn classify_wikipedia_summary(word: &str, body: &Value) -> RegistryResearchEntry {
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
    use super::classify_wikipedia_summary;

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
}
