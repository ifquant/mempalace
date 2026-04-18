use std::collections::BTreeMap;
use std::path::Path;

pub const AAAK_SPEC: &str = "AAAK is a compressed memory dialect that MemPalace uses for efficient storage.\nIt is designed to be readable by both humans and LLMs without decoding.\n\nFORMAT:\n  ENTITIES: 3-letter uppercase codes. ALC=Alice, JOR=Jordan, RIL=Riley, MAX=Max, BEN=Ben.\n  EMOTIONS: *action markers* before/during text. *warm*=joy, *fierce*=determined, *raw*=vulnerable, *bloom*=tenderness.\n  STRUCTURE: Pipe-separated fields. FAM: family | PROJ: projects | ⚠: warnings/reminders.\n  DATES: ISO format (2026-03-31). COUNTS: Nx = N mentions (e.g., 570x).\n  IMPORTANCE: ★ to ★★★★★ (1-5 scale).\n  HALLS: hall_facts, hall_events, hall_discoveries, hall_preferences, hall_advice.\n  WINGS: wing_user, wing_agent, wing_team, wing_code, wing_myproject, wing_hardware, wing_ue5, wing_ai_research.\n  ROOMS: Hyphenated slugs representing named ideas (e.g., chromadb-setup, gpu-pricing).\n\nEXAMPLE:\n  FAM: ALC→♡JOR | 2D(kids): RIL(18,sports) MAX(11,chess+swimming) | BEN(contributor)\n\nRead AAAK naturally — expand codes mentally, treat *markers* as emotional context.\nWhen WRITING AAAK: use entity codes, mark emotions, keep structure tight.";

const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
    "do", "does", "did", "will", "would", "could", "should", "may", "might", "shall", "can", "to",
    "of", "in", "for", "on", "with", "at", "by", "from", "as", "into", "about", "between",
    "through", "during", "before", "after", "above", "below", "up", "down", "out", "off", "over",
    "under", "again", "further", "then", "once", "here", "there", "when", "where", "why", "how",
    "all", "each", "every", "both", "few", "more", "most", "other", "some", "such", "no", "nor",
    "not", "only", "own", "same", "so", "than", "too", "very", "just", "don", "now", "and", "but",
    "or", "if", "while", "that", "this", "these", "those", "it", "its", "i", "we", "you", "he",
    "she", "they", "me", "him", "her", "us", "them", "my", "your", "his", "our", "their", "what",
    "which", "who", "whom", "also", "much", "many", "like", "because", "since", "get", "got",
    "use", "used", "using", "make", "made", "thing", "things", "way", "well", "really", "want",
    "need",
];

const EMOTION_SIGNALS: &[(&str, &str)] = &[
    ("decided", "determ"),
    ("prefer", "convict"),
    ("worried", "anx"),
    ("excited", "excite"),
    ("frustrated", "frust"),
    ("confused", "confuse"),
    ("love", "love"),
    ("hate", "rage"),
    ("hope", "hope"),
    ("fear", "fear"),
    ("trust", "trust"),
    ("happy", "joy"),
    ("sad", "grief"),
    ("surprised", "surprise"),
    ("grateful", "grat"),
    ("curious", "curious"),
    ("wonder", "wonder"),
    ("anxious", "anx"),
    ("relieved", "relief"),
    ("satisf", "satis"),
    ("disappoint", "grief"),
    ("concern", "anx"),
];

const FLAG_SIGNALS: &[(&str, &str)] = &[
    ("decided", "DECISION"),
    ("chose", "DECISION"),
    ("switched", "DECISION"),
    ("migrated", "DECISION"),
    ("replaced", "DECISION"),
    ("instead of", "DECISION"),
    ("because", "DECISION"),
    ("founded", "ORIGIN"),
    ("created", "ORIGIN"),
    ("started", "ORIGIN"),
    ("born", "ORIGIN"),
    ("launched", "ORIGIN"),
    ("first time", "ORIGIN"),
    ("core", "CORE"),
    ("fundamental", "CORE"),
    ("essential", "CORE"),
    ("principle", "CORE"),
    ("belief", "CORE"),
    ("always", "CORE"),
    ("never forget", "CORE"),
    ("turning point", "PIVOT"),
    ("changed everything", "PIVOT"),
    ("realized", "PIVOT"),
    ("breakthrough", "PIVOT"),
    ("epiphany", "PIVOT"),
    ("api", "TECHNICAL"),
    ("database", "TECHNICAL"),
    ("architecture", "TECHNICAL"),
    ("deploy", "TECHNICAL"),
    ("infrastructure", "TECHNICAL"),
    ("algorithm", "TECHNICAL"),
    ("framework", "TECHNICAL"),
    ("server", "TECHNICAL"),
    ("config", "TECHNICAL"),
];

#[derive(Clone, Debug)]
pub struct CompressMetadata<'a> {
    pub wing: &'a str,
    pub room: &'a str,
    pub source_file: &'a str,
    pub filed_at: Option<&'a str>,
}

#[derive(Clone, Debug)]
pub struct CompressionStats {
    pub original_chars: usize,
    pub compressed_chars: usize,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub ratio: f64,
}

#[derive(Default)]
pub struct Dialect;

impl Dialect {
    pub fn compress(&self, text: &str, metadata: CompressMetadata<'_>) -> String {
        let entities = detect_entities(text);
        let topics = extract_topics(text);
        let quote = extract_key_sentence(text);
        let emotions = detect_codes(text, EMOTION_SIGNALS);
        let flags = detect_codes(text, FLAG_SIGNALS);

        let mut lines = Vec::new();
        let header = [
            metadata.wing,
            metadata.room,
            metadata.filed_at.unwrap_or("?"),
            Path::new(metadata.source_file)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("?"),
        ]
        .join("|");
        lines.push(header);

        let mut parts = vec![
            format!(
                "0:{}",
                if entities.is_empty() {
                    "???".to_string()
                } else {
                    entities.join("+")
                }
            ),
            if topics.is_empty() {
                "misc".to_string()
            } else {
                topics.join("_")
            },
        ];
        if !quote.is_empty() {
            parts.push(format!("\"{quote}\""));
        }
        if !emotions.is_empty() {
            parts.push(emotions.join("+"));
        }
        if !flags.is_empty() {
            parts.push(flags.join("+"));
        }
        lines.push(parts.join("|"));
        lines.join("\n")
    }

    pub fn compression_stats(&self, original: &str, compressed: &str) -> CompressionStats {
        let original_chars = original.chars().count();
        let compressed_chars = compressed.chars().count();
        let original_tokens = count_tokens(original);
        let compressed_tokens = count_tokens(compressed);
        let ratio = original_chars as f64 / compressed_chars.max(1) as f64;
        CompressionStats {
            original_chars,
            compressed_chars,
            original_tokens,
            compressed_tokens,
            ratio,
        }
    }
}

pub fn count_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

fn detect_codes(text: &str, mapping: &[(&str, &str)]) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let mut detected = Vec::new();
    for (keyword, code) in mapping {
        if text_lower.contains(keyword) && !detected.iter().any(|existing| existing == code) {
            detected.push((*code).to_string());
        }
    }
    detected.truncate(3);
    detected
}

fn extract_topics(text: &str) -> Vec<String> {
    let mut freq = BTreeMap::new();
    for raw in text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-') {
        if raw.len() < 3 {
            continue;
        }
        let lowered = raw.to_ascii_lowercase();
        if STOP_WORDS.contains(&lowered.as_str()) {
            continue;
        }
        *freq.entry(lowered).or_insert(0usize) += 1;
    }
    let mut ranked = freq.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
    ranked.into_iter().take(3).map(|(word, _)| word).collect()
}

fn extract_key_sentence(text: &str) -> String {
    let decision_words = [
        "decided",
        "because",
        "instead",
        "prefer",
        "switched",
        "chose",
        "realized",
        "important",
        "key",
        "critical",
        "discovered",
        "learned",
        "conclusion",
        "solution",
        "reason",
        "why",
        "breakthrough",
        "insight",
    ];
    let mut best = String::new();
    let mut best_score = i32::MIN;
    for sentence in text
        .split(['.', '!', '?', '\n'])
        .map(str::trim)
        .filter(|sentence| sentence.len() > 10)
    {
        let lowered = sentence.to_ascii_lowercase();
        let mut score = 0;
        for word in decision_words {
            if lowered.contains(word) {
                score += 2;
            }
        }
        if sentence.len() < 80 {
            score += 1;
        }
        if sentence.len() < 40 {
            score += 1;
        }
        if sentence.len() > 150 {
            score -= 2;
        }
        if score > best_score {
            best_score = score;
            best = sentence.to_string();
        }
    }
    if best.len() > 55 {
        format!("{}...", &best[..52])
    } else {
        best
    }
}

fn detect_entities(text: &str) -> Vec<String> {
    let mut entities = Vec::new();
    let words = text.split_whitespace().collect::<Vec<_>>();
    for (index, raw) in words.iter().enumerate() {
        let clean = raw
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .collect::<String>();
        if clean.len() < 2
            || index == 0
            || !clean.chars().next().is_some_and(char::is_uppercase)
            || !clean.chars().skip(1).all(char::is_lowercase)
        {
            continue;
        }
        let lowered = clean.to_ascii_lowercase();
        if STOP_WORDS.contains(&lowered.as_str()) {
            continue;
        }
        let code = clean
            .chars()
            .take(3)
            .collect::<String>()
            .to_ascii_uppercase();
        if !entities.iter().any(|existing| existing == &code) {
            entities.push(code);
        }
        if entities.len() >= 3 {
            break;
        }
    }
    entities
}
