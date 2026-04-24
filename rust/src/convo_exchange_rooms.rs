const TOPIC_BUCKETS: &[(&str, &[&str])] = &[
    (
        "technical",
        &[
            "code", "python", "function", "bug", "error", "api", "database", "server", "deploy",
            "git", "test", "debug", "refactor",
        ],
    ),
    (
        "architecture",
        &[
            "architecture",
            "design",
            "pattern",
            "structure",
            "schema",
            "interface",
            "module",
            "component",
            "service",
            "layer",
        ],
    ),
    (
        "planning",
        &[
            "plan",
            "roadmap",
            "milestone",
            "deadline",
            "priority",
            "sprint",
            "backlog",
            "scope",
            "requirement",
            "spec",
        ],
    ),
    (
        "decisions",
        &[
            "decided",
            "chose",
            "picked",
            "switched",
            "migrated",
            "replaced",
            "trade-off",
            "alternative",
            "option",
            "approach",
        ],
    ),
    (
        "problems",
        &[
            "problem",
            "issue",
            "broken",
            "failed",
            "crash",
            "stuck",
            "workaround",
            "fix",
            "solved",
            "resolved",
        ],
    ),
];

pub fn exchange_rooms() -> Vec<String> {
    TOPIC_BUCKETS
        .iter()
        .map(|(room, _)| (*room).to_string())
        .chain(std::iter::once("general".to_string()))
        .collect()
}

pub fn detect_convo_room(content: &str) -> String {
    let content_lower = content
        .chars()
        .take(3_000)
        .collect::<String>()
        .to_ascii_lowercase();
    let mut best_room = "general";
    let mut best_score = 0usize;
    for (room, keywords) in TOPIC_BUCKETS {
        let score = keywords
            .iter()
            .map(|keyword| content_lower.matches(keyword).count())
            .sum::<usize>();
        if score > best_score {
            best_score = score;
            best_room = room;
        }
    }
    best_room.to_string()
}
