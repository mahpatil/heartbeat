//! Derive a job-name slug from a natural-language description.

const STOP_WORDS: &[&str] = &[
    "every", "each", "the", "a", "an", "to", "for", "in", "at", "on",
    "with", "using", "ask", "run", "check", "use", "my", "and", "or",
    "then", "please", "should", "can", "will", "also", "morning",
    "evening", "night", "daily", "hourly", "weekly", "claude", "opencode",
    "codex", "it", "is", "are", "of", "from", "that", "this", "be",
    "minutes", "minute", "hours", "hour", "days", "day", "weeks", "week",
];

pub fn suggest(description: &str) -> String {
    let slug: String = description
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| {
            let lower = w.to_lowercase();
            !lower.is_empty()
                && lower.len() > 2
                && !STOP_WORDS.contains(&lower.as_str())
                && !lower.chars().all(|c| c.is_ascii_digit())
        })
        .take(4)
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() {
        "my-job".to_string()
    } else {
        slug.chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_stop_words() {
        let s = suggest("Every morning ask Claude to summarize my GitHub PRs");
        assert!(!s.contains("every"));
        assert!(!s.contains("ask"));
        assert!(!s.contains("claude"));
        assert!(!s.is_empty());
    }

    #[test]
    fn empty_falls_back() {
        assert_eq!(suggest(""), "my-job");
    }
}
