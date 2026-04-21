const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "that", "this", "there", "their", "they", "then", "have",
    "has", "had", "when", "where", "which", "your", "about", "into", "could", "should", "would",
    "return", "import", "class", "usage", "step", "check", "build", "deploy", "project", "system",
    "service", "feature", "issue", "design", "notes", "graph", "search",
];

const PERSON_VERBS: &[&str] = &[
    "said", "asked", "told", "replied", "laughed", "smiled", "cried", "felt", "thinks", "think",
    "wants", "want", "loves", "love", "hates", "hate", "knows", "know", "decided", "pushed",
    "wrote",
];

const PRONOUNS: &[&str] = &[
    "she", "her", "hers", "he", "him", "his", "they", "them", "their",
];

const PROJECT_HINTS: &[&str] = &[
    "repo",
    "system",
    "service",
    "app",
    "architecture",
    "pipeline",
    "deploy",
    "deployment",
    "launch",
    "shipping",
    "shipped",
    "building",
    "built",
];

pub fn is_stopword(name: &str) -> bool {
    STOPWORDS.iter().any(|word| word.eq_ignore_ascii_case(name))
}

pub fn score_person(name: &str, text: &str, lines: &[String]) -> usize {
    let mut score = 0usize;
    let lower = name.to_ascii_lowercase();
    for verb in PERSON_VERBS {
        if text
            .to_ascii_lowercase()
            .contains(&format!("{lower} {verb}"))
        {
            score += 2;
        }
    }
    for line in lines {
        let line_lower = line.to_ascii_lowercase();
        if line_lower.starts_with(&format!("{lower}:"))
            || line_lower.starts_with(&format!("> {lower}:"))
            || line_lower.contains(&format!("hey {lower}"))
            || line_lower.contains(&format!("thanks {lower}"))
            || line_lower.contains(&format!("hi {lower}"))
        {
            score += 1;
        }
    }
    score += pronoun_proximity_hits(&lower, lines) * 2;
    score
}

pub fn person_signal_category_count(name: &str, text: &str, lines: &[String]) -> usize {
    let mut categories = Vec::new();
    let lower = name.to_ascii_lowercase();
    let lowered_text = text.to_ascii_lowercase();
    if PERSON_VERBS
        .iter()
        .any(|verb| lowered_text.contains(&format!("{lower} {verb}")))
    {
        categories.push("action");
    }
    if lines.iter().any(|line| {
        let line_lower = line.to_ascii_lowercase();
        line_lower.starts_with(&format!("{lower}:"))
            || line_lower.starts_with(&format!("> {lower}:"))
    }) {
        categories.push("dialogue");
    }
    if lines.iter().any(|line| {
        let line_lower = line.to_ascii_lowercase();
        line_lower.contains(&format!("hey {lower}"))
            || line_lower.contains(&format!("thanks {lower}"))
            || line_lower.contains(&format!("hi {lower}"))
    }) {
        categories.push("addressed");
    }
    if pronoun_proximity_hits(&lower, lines) > 0 {
        categories.push("pronoun");
    }
    categories.sort_unstable();
    categories.dedup();
    categories.len()
}

pub fn score_project(name: &str, text: &str, lines: &[String]) -> usize {
    let mut score = 0usize;
    let lower = name.to_ascii_lowercase();
    let lowered_text = text.to_ascii_lowercase();
    for hint in PROJECT_HINTS {
        if lowered_text.contains(&format!("{lower} {hint}"))
            || lowered_text.contains(&format!("{hint} {lower}"))
        {
            score += 2;
        }
    }
    for line in lines {
        let line_lower = line.to_ascii_lowercase();
        if line_lower.contains(&format!("building {lower}"))
            || line_lower.contains(&format!("built {lower}"))
            || line_lower.contains(&format!("deploy {lower}"))
            || line_lower.contains(&format!("launch {lower}"))
            || line_lower.contains(&format!("{lower}.py"))
            || line_lower.contains(&format!("{lower}-core"))
        {
            score += 1;
        }
    }
    score
}

fn pronoun_proximity_hits(name_lower: &str, lines: &[String]) -> usize {
    let mut hits = 0usize;
    for (index, line) in lines.iter().enumerate() {
        if !line.to_ascii_lowercase().contains(name_lower) {
            continue;
        }
        let start = index.saturating_sub(2);
        let end = (index + 3).min(lines.len());
        let window = lines[start..end].join(" ").to_ascii_lowercase();
        if contains_pronoun(&window) {
            hits += 1;
        }
    }
    hits
}

fn contains_pronoun(text: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_alphabetic())
        .any(|word| PRONOUNS.contains(&word))
}
