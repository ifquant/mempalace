const STOPWORDS: &[&str] = &[
    "a",
    "about",
    "actor",
    "add",
    "agents",
    "all",
    "already",
    "also",
    "an",
    "and",
    "answer",
    "any",
    "anyone",
    "anything",
    "applications",
    "are",
    "args",
    "as",
    "at",
    "be",
    "been",
    "being",
    "bool",
    "but",
    "by",
    "can",
    "cancel",
    "cars",
    "case",
    "check",
    "choose",
    "class",
    "click",
    "close",
    "cls",
    "come",
    "confirm",
    "control",
    "copy",
    "could",
    "culture",
    "data",
    "day",
    "def",
    "delete",
    "desktop",
    "deus",
    "dict",
    "did",
    "do",
    "documents",
    "does",
    "download",
    "downloads",
    "drag",
    "drop",
    "duration",
    "each",
    "enter",
    "error",
    "ethics",
    "even",
    "every",
    "everyone",
    "everything",
    "ex",
    "example",
    "fact",
    "false",
    "fetch",
    "file",
    "find",
    "first",
    "for",
    "from",
    "future",
    "get",
    "go",
    "got",
    "guards",
    "had",
    "has",
    "have",
    "he",
    "healthcare",
    "hello",
    "her",
    "here",
    "hey",
    "hi",
    "hide",
    "his",
    "history",
    "hit",
    "home",
    "how",
    "human",
    "humans",
    "i",
    "idea",
    "if",
    "import",
    "in",
    "inference",
    "info",
    "input",
    "install",
    "int",
    "intelligence",
    "is",
    "it",
    "item",
    "its",
    "just",
    "key",
    "kind",
    "know",
    "kwargs",
    "language",
    "last",
    "launch",
    "layer",
    "learning",
    "less",
    "let",
    "library",
    "life",
    "like",
    "list",
    "load",
    "machina",
    "made",
    "make",
    "may",
    "me",
    "memory",
    "might",
    "mode",
    "model",
    "models",
    "more",
    "move",
    "must",
    "my",
    "name",
    "network",
    "networks",
    "new",
    "next",
    "no",
    "none",
    "not",
    "note",
    "nothing",
    "now",
    "null",
    "number",
    "of",
    "ok",
    "okay",
    "old",
    "on",
    "only",
    "open",
    "option",
    "or",
    "others",
    "our",
    "out",
    "output",
    "part",
    "paste",
    "path",
    "people",
    "phones",
    "place",
    "point",
    "preferences",
    "press",
    "print",
    "put",
    "question",
    "raises",
    "read",
    "really",
    "reason",
    "regulation",
    "remote",
    "result",
    "return",
    "returns",
    "right",
    "run",
    "save",
    "science",
    "scroll",
    "search",
    "second",
    "see",
    "select",
    "self",
    "sense",
    "set",
    "settings",
    "shall",
    "she",
    "should",
    "show",
    "so",
    "social",
    "society",
    "some",
    "someone",
    "something",
    "sort",
    "source",
    "stack",
    "start",
    "step",
    "still",
    "stop",
    "str",
    "submit",
    "system",
    "take",
    "tap",
    "target",
    "technology",
    "terminal",
    "test",
    "thank",
    "thanks",
    "that",
    "the",
    "their",
    "them",
    "then",
    "there",
    "these",
    "they",
    "thing",
    "things",
    "think",
    "thinking",
    "this",
    "those",
    "time",
    "to",
    "too",
    "tools",
    "topic",
    "training",
    "true",
    "type",
    "up",
    "upload",
    "usage",
    "use",
    "users",
    "value",
    "vector",
    "version",
    "very",
    "want",
    "warning",
    "was",
    "way",
    "we",
    "well",
    "were",
    "what",
    "when",
    "where",
    "which",
    "who",
    "why",
    "will",
    "with",
    "world",
    "would",
    "write",
    "yes",
    "yields",
    "you",
    "your",
];

const PERSON_VERBS: &[&str] = &[
    "said", "asked", "told", "replied", "laughed", "smiled", "cried", "felt", "thinks", "think",
    "wants", "want", "loves", "love", "hates", "hate", "knows", "know", "decided", "pushed",
    "wrote",
];

const PRONOUNS: &[&str] = &[
    "she", "her", "hers", "he", "him", "his", "they", "them", "their",
];

/// Returns whether a candidate token is too generic to treat as an entity.
pub fn is_stopword(name: &str) -> bool {
    STOPWORDS.iter().any(|word| word.eq_ignore_ascii_case(name))
}

/// Scores how strongly a candidate looks like a person mention.
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
    if text.to_ascii_lowercase().contains(&format!("dear {lower}")) {
        score += 2;
    }
    for line in lines {
        let line_lower = line.to_ascii_lowercase();
        if is_dialogue_marker(&line_lower, &lower) {
            score += 3;
        }
        if is_direct_address(&line_lower, &lower) {
            score += 4;
        }
    }
    score += pronoun_proximity_hits(&lower, lines) * 2;
    score
}

/// Counts distinct person-signal categories for a candidate.
pub fn person_signal_category_count(name: &str, text: &str, lines: &[String]) -> usize {
    let mut categories = Vec::new();
    let lower = name.to_ascii_lowercase();
    let lowered_text = text.to_ascii_lowercase();
    if PERSON_VERBS
        .iter()
        .any(|verb| lowered_text.contains(&format!("{lower} {verb}")))
        || lowered_text.contains(&format!("dear {lower}"))
    {
        categories.push("action");
    }
    if lines.iter().any(|line| {
        let line_lower = line.to_ascii_lowercase();
        is_dialogue_marker(&line_lower, &lower)
    }) {
        categories.push("dialogue");
    }
    if lines.iter().any(|line| {
        let line_lower = line.to_ascii_lowercase();
        is_direct_address(&line_lower, &lower)
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

/// Scores how strongly a candidate looks like a project name.
pub fn score_project(name: &str, _text: &str, lines: &[String]) -> usize {
    let mut score = 0usize;
    let lower = name.to_ascii_lowercase();
    for line in lines {
        let line_lower = line.to_ascii_lowercase();
        score += project_marker_score(&line_lower, &lower);
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

fn is_dialogue_marker(line_lower: &str, name_lower: &str) -> bool {
    line_lower.starts_with(&format!("{name_lower}:"))
        || line_lower.starts_with(&format!("> {name_lower}:"))
        || line_lower.starts_with(&format!("> {name_lower} "))
        || line_lower.starts_with(&format!("[{name_lower}]"))
        || line_lower.contains(&format!("\"{name_lower} said"))
}

fn is_direct_address(line_lower: &str, name_lower: &str) -> bool {
    line_lower.contains(&format!("hey {name_lower}"))
        || line_lower.contains(&format!("thanks {name_lower}"))
        || line_lower.contains(&format!("thank {name_lower}"))
        || line_lower.contains(&format!("hi {name_lower}"))
}

fn project_marker_score(line_lower: &str, name_lower: &str) -> usize {
    let mut score = 0usize;
    if line_lower.contains(&format!("building {name_lower}"))
        || line_lower.contains(&format!("built {name_lower}"))
        || line_lower.contains(&format!("shipping {name_lower}"))
        || line_lower.contains(&format!("shipped {name_lower}"))
        || line_lower.contains(&format!("ship {name_lower}"))
        || line_lower.contains(&format!("launching {name_lower}"))
        || line_lower.contains(&format!("launched {name_lower}"))
        || line_lower.contains(&format!("launch {name_lower}"))
        || line_lower.contains(&format!("deploying {name_lower}"))
        || line_lower.contains(&format!("deployed {name_lower}"))
        || line_lower.contains(&format!("deploy {name_lower}"))
        || line_lower.contains(&format!("installing {name_lower}"))
        || line_lower.contains(&format!("installed {name_lower}"))
        || line_lower.contains(&format!("install {name_lower}"))
        || line_lower.contains(&format!("the {name_lower} architecture"))
        || line_lower.contains(&format!("the {name_lower} pipeline"))
        || line_lower.contains(&format!("the {name_lower} system"))
        || line_lower.contains(&format!("the {name_lower} repo"))
        || line_lower.contains(&format!("import {name_lower}"))
        || line_lower.contains(&format!("pip install {name_lower}"))
        || line_lower.contains(&format!("{name_lower} v"))
        || line_lower.contains(&format!("{name_lower}-core"))
        || line_lower.contains(&format!("{name_lower}-local"))
    {
        score += 2;
    }
    if line_lower.contains(&format!("{name_lower}-")) {
        score += 3;
    }
    if has_code_reference(line_lower, name_lower) {
        score += 3;
    }
    score
}

fn has_code_reference(line_lower: &str, name_lower: &str) -> bool {
    [".py", ".js", ".ts", ".yaml", ".yml", ".json", ".sh"]
        .iter()
        .any(|suffix| line_lower.contains(&format!("{name_lower}{suffix}")))
}
