use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;

const MIN_LENGTH: usize = 4;

static HAS_DIGIT: OnceLock<Regex> = OnceLock::new();
static IS_CAMEL: OnceLock<Regex> = OnceLock::new();
static IS_ALLCAPS: OnceLock<Regex> = OnceLock::new();
static IS_TECHNICAL: OnceLock<Regex> = OnceLock::new();
static IS_URL: OnceLock<Regex> = OnceLock::new();
static IS_CODE_OR_EMOJI: OnceLock<Regex> = OnceLock::new();
static TOKEN_RE: OnceLock<Regex> = OnceLock::new();

pub(crate) fn should_skip(token: &str, known_names: &HashSet<String>) -> bool {
    if token.len() < MIN_LENGTH {
        return true;
    }
    if has_digit().is_match(token)
        || is_camel().is_match(token)
        || is_allcaps().is_match(token)
        || is_technical().is_match(token)
        || is_url().is_match(token)
        || is_code_or_emoji().is_match(token)
        || known_names.contains(&token.to_ascii_lowercase())
    {
        return true;
    }
    token.chars().next().is_some_and(|ch| ch.is_uppercase())
}

fn has_digit() -> &'static Regex {
    HAS_DIGIT.get_or_init(|| Regex::new(r"\d").expect("digit regex"))
}

fn is_camel() -> &'static Regex {
    IS_CAMEL.get_or_init(|| Regex::new(r"[A-Z][a-z]+[A-Z]").expect("camel regex"))
}

fn is_allcaps() -> &'static Regex {
    IS_ALLCAPS
        .get_or_init(|| Regex::new(r"^[A-Z_@#$%^&*()+=\[\]{}|<>?.:/\\]+$").expect("allcaps regex"))
}

fn is_technical() -> &'static Regex {
    IS_TECHNICAL.get_or_init(|| Regex::new(r"[-_]").expect("technical regex"))
}

fn is_url() -> &'static Regex {
    IS_URL
        .get_or_init(|| Regex::new(r"https?://|www\.|/Users/|~/|\.[a-z]{2,4}$").expect("url regex"))
}

fn is_code_or_emoji() -> &'static Regex {
    IS_CODE_OR_EMOJI.get_or_init(|| Regex::new(r"[`*_#{}\[\]\\]").expect("code regex"))
}

pub(crate) fn token_re() -> &'static Regex {
    TOKEN_RE.get_or_init(|| Regex::new(r"(\S+)").expect("token regex"))
}
