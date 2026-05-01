//! Natural-language → [`ParsedIntent`] extraction for `heartbeat build`.

use regex::Regex;
use std::sync::OnceLock;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ParsedIntent {
    pub schedule: Option<String>,
    pub workspace: Option<String>,
    pub steps: Vec<ParsedStep>,
}

#[derive(Debug, Clone)]
pub enum ParsedStep {
    Agent { agent: String, prompt: String },
    Shell { command: String },
    UrlCheck { url: String },
    FileCheck { path: String },
}

impl ParsedStep {
    pub fn summary(&self) -> String {
        match self {
            Self::Agent { agent, prompt } => {
                format!("Ask {} to: {}", agent, truncate(prompt, 65))
            }
            Self::Shell { command } => format!("Run: {}", truncate(command, 65)),
            Self::UrlCheck { url } => format!("Check URL: {}", url),
            Self::FileCheck { path } => format!("Check file exists: {}", path),
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn parse(description: &str) -> ParsedIntent {
    let lower = description.to_lowercase();
    let agent = extract_agent(&lower).unwrap_or_else(|| "claude".to_string());

    let cleaned = clean_for_steps(description);
    ParsedIntent {
        schedule: extract_schedule(&lower),
        workspace: extract_workspace(description),
        steps: extract_steps(&cleaned, &agent),
    }
}

// ── Schedule extraction ───────────────────────────────────────────────────────

fn extract_schedule(lower: &str) -> Option<String> {
    static RE_N_UNIT: OnceLock<Regex> = OnceLock::new();
    static RE_TIME_COLON: OnceLock<Regex> = OnceLock::new();
    static RE_TIME_AMPM: OnceLock<Regex> = OnceLock::new();
    static RE_CRON: OnceLock<Regex> = OnceLock::new();

    let re_n = RE_N_UNIT.get_or_init(|| {
        Regex::new(r"every\s+(\d+)\s*(minutes?|mins?|hours?|hrs?|days?|weeks?)").unwrap()
    });
    if let Some(caps) = re_n.captures(lower) {
        let n: u32 = caps[1].parse().unwrap_or(1);
        return Some(match caps[2].chars().next().unwrap_or('m') {
            'm' => format!("every {}m", n),
            'h' => format!("every {}h", n),
            'd' => format!("every {}d", n),
            'w' => format!("every {}d", n * 7),
            _ => format!("every {}m", n),
        });
    }

    // Named frequencies
    if lower.contains("hourly") {
        return Some("every 1h".to_string());
    }

    // Time-of-day phrases
    for (phrase, sched) in &[
        ("every morning", "daily at 09:00"),
        ("each morning", "daily at 09:00"),
        ("in the morning", "daily at 09:00"),
        ("every evening", "daily at 18:00"),
        ("each evening", "daily at 18:00"),
        ("in the evening", "daily at 18:00"),
        ("every night", "daily at 22:00"),
        ("each night", "daily at 22:00"),
        ("nightly", "daily at 22:00"),
        ("at night", "daily at 22:00"),
        ("at midnight", "daily at 00:00"),
        ("every midnight", "daily at 00:00"),
        ("every noon", "daily at 12:00"),
    ] {
        if lower.contains(phrase) {
            return Some(sched.to_string());
        }
    }

    // "at HH:MM [am|pm]"
    let re_colon = RE_TIME_COLON.get_or_init(|| {
        Regex::new(r"at\s+(\d{1,2}):(\d{2})\s*(am|pm)?").unwrap()
    });
    if let Some(caps) = re_colon.captures(lower) {
        let mut h: u32 = caps[1].parse().unwrap_or(9);
        let m: u32 = caps[2].parse().unwrap_or(0);
        if caps.get(3).map(|x| x.as_str()) == Some("pm") && h < 12 {
            h += 12;
        }
        if caps.get(3).map(|x| x.as_str()) == Some("am") && h == 12 {
            h = 0;
        }
        return Some(format!("daily at {:02}:{:02}", h, m));
    }

    // "at 9am" / "at 3pm"
    let re_ampm =
        RE_TIME_AMPM.get_or_init(|| Regex::new(r"at\s+(\d{1,2})\s*(am|pm)").unwrap());
    if let Some(caps) = re_ampm.captures(lower) {
        let mut h: u32 = caps[1].parse().unwrap_or(9);
        if &caps[2] == "pm" && h < 12 {
            h += 12;
        }
        if &caps[2] == "am" && h == 12 {
            h = 0;
        }
        return Some(format!("daily at {:02}:00", h));
    }

    // "every day" / "daily" (no time specified — default 09:00)
    if lower.contains("every day") || lower.contains("daily") {
        return Some("daily at 09:00".to_string());
    }

    // cron expression: 5 space-separated fields with digits/wildcards
    let re_cron = RE_CRON.get_or_init(|| {
        Regex::new(r"[\(\s]?([\d*/,-]+\s+[\d*/,-]+\s+[\d*/,-]+\s+[\d*/,-]+\s+[\d*/,-]+)[\)\s]?").unwrap()
    });
    if let Some(caps) = re_cron.captures(lower) {
        return Some(caps[1].trim().to_string());
    }

    // every week / weekly
    if lower.contains("every week") || lower.contains("weekly") {
        return Some("every 7d".to_string());
    }

    None
}

// ── Workspace extraction ──────────────────────────────────────────────────────

fn extract_workspace(description: &str) -> Option<String> {
    static RE_PATH: OnceLock<Regex> = OnceLock::new();
    static RE_PROJ: OnceLock<Regex> = OnceLock::new();

    let re_path =
        RE_PATH.get_or_init(|| Regex::new(r"(?i)\bin\s+(~/[\S]+|/[\S]+)").unwrap());
    if let Some(caps) = re_path.captures(description) {
        return Some(caps[1].trim_end_matches(',').to_string());
    }

    let re_proj = RE_PROJ.get_or_init(|| {
        Regex::new(r"(?i)(?:in|for)\s+(?:my\s+)?(\w[\w-]*)\s+(?:project|repo|repository)")
            .unwrap()
    });
    if let Some(caps) = re_proj.captures(description) {
        return Some(format!("~/projects/{}", caps[1].to_lowercase()));
    }

    None
}

// ── Agent extraction ──────────────────────────────────────────────────────────

fn extract_agent(lower: &str) -> Option<String> {
    if lower.contains("opencode") {
        return Some("opencode".to_string());
    }
    if lower.contains("codex") {
        return Some("codex".to_string());
    }
    if lower.contains("claude") {
        return Some("claude".to_string());
    }
    None
}

// ── Step extraction ───────────────────────────────────────────────────────────

fn clean_for_steps(description: &str) -> String {
    static RE_SCHEDULE: OnceLock<Regex> = OnceLock::new();
    static RE_AGENT_PHRASE: OnceLock<Regex> = OnceLock::new();
    static RE_WORKSPACE: OnceLock<Regex> = OnceLock::new();

    let re_sched = RE_SCHEDULE.get_or_init(|| {
        Regex::new(
            r"(?i)(every\s+\d+\s*(?:minutes?|mins?|hours?|hrs?|days?|weeks?)|every\s+(?:morning|evening|night|day|week|midnight|noon)|(?:each\s+)?(?:morning|evening|night)|(?:daily|weekly|hourly|nightly)|at\s+\d{1,2}(?::\d{2})?\s*(?:am|pm)?|at\s+midnight)"
        ).unwrap()
    });
    let re_agent = RE_AGENT_PHRASE.get_or_init(|| {
        Regex::new(r"(?i)(?:using|with|via)\s+(?:claude|opencode|codex)").unwrap()
    });
    let re_ws =
        RE_WORKSPACE.get_or_init(|| Regex::new(r"(?i)\bin\s+(?:~/\S+|/\S+)").unwrap());

    let s = re_sched.replace_all(description, "").to_string();
    let s = re_agent.replace_all(&s, "").to_string();
    let s = re_ws.replace_all(&s, "").to_string();
    s.split_whitespace().collect::<Vec<_>>().join(" ").trim_matches([',', ';', ' ']).to_string()
}

fn extract_steps(cleaned: &str, default_agent: &str) -> Vec<ParsedStep> {
    static RE_SPLIT: OnceLock<Regex> = OnceLock::new();
    static RE_URL: OnceLock<Regex> = OnceLock::new();
    static RE_FILE: OnceLock<Regex> = OnceLock::new();
    static RE_SHELL: OnceLock<Regex> = OnceLock::new();
    static RE_AGENT_PREFIX: OnceLock<Regex> = OnceLock::new();

    let re_split = RE_SPLIT.get_or_init(|| {
        Regex::new(r"(?i),\s*(?:and\s+)?then\s+|;\s*(?:and\s+)?then\s+|,\s*and\s+also\s+|\band\s+also\b|\bthen\s+also\b").unwrap()
    });
    let re_url =
        RE_URL.get_or_init(|| Regex::new(r"https?://[^\s,;]+").unwrap());
    let re_file = RE_FILE.get_or_init(|| {
        Regex::new(r"(?i)check\s+(?:if\s+)?(?:file|path)\s+(\S+)").unwrap()
    });
    let re_shell =
        RE_SHELL.get_or_init(|| Regex::new(r"(?i)^(?:run|execute|shell)\s+(.+)$").unwrap());
    let re_agent_prefix = RE_AGENT_PREFIX.get_or_init(|| {
        Regex::new(r"(?i)^(?:ask\s+)?(?:claude|opencode|codex)\s+(?:to\s+)?").unwrap()
    });

    let parts: Vec<&str> = re_split
        .split(cleaned)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let mut steps = Vec::new();
    for part in &parts {
        if let Some(m) = re_url.find(part) {
            steps.push(ParsedStep::UrlCheck {
                url: m.as_str().trim_end_matches([',', ';', ')']).to_string(),
            });
        } else if let Some(caps) = re_file.captures(part) {
            steps.push(ParsedStep::FileCheck {
                path: caps[1].to_string(),
            });
        } else if let Some(caps) = re_shell.captures(part) {
            steps.push(ParsedStep::Shell {
                command: caps[1].trim().to_string(),
            });
        } else {
            let prompt = re_agent_prefix.replace(part, "").trim().to_string();
            if !prompt.is_empty() {
                steps.push(ParsedStep::Agent {
                    agent: default_agent.to_string(),
                    prompt,
                });
            }
        }
    }

    // Fallback: treat whole cleaned description as a single agent prompt.
    if steps.is_empty() && !cleaned.is_empty() {
        let prompt = re_agent_prefix.replace(cleaned, "").trim().to_string();
        steps.push(ParsedStep::Agent {
            agent: default_agent.to_string(),
            prompt: if prompt.is_empty() { cleaned.to_string() } else { prompt },
        });
    }

    steps
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_n_minutes() {
        let i = parse("Every 5 minutes ask Claude to check my logs");
        assert_eq!(i.schedule.as_deref(), Some("every 5m"));
    }

    #[test]
    fn parses_morning_schedule() {
        let i = parse("Every morning summarise my GitHub notifications");
        assert_eq!(i.schedule.as_deref(), Some("daily at 09:00"));
    }

    #[test]
    fn parses_agent_opencode() {
        let i = parse("Daily, ask opencode to refactor my code");
        assert!(matches!(&i.steps[0], ParsedStep::Agent { agent, .. } if agent == "opencode"));
    }

    #[test]
    fn parses_workspace_path() {
        let i = parse("Every hour in ~/projects/foo run the tests");
        assert_eq!(i.workspace.as_deref(), Some("~/projects/foo"));
    }

    #[test]
    fn parses_url_step() {
        let i = parse("Every 10 minutes check https://example.com");
        assert!(matches!(&i.steps[0], ParsedStep::UrlCheck { url } if url.contains("example.com")));
    }

    #[test]
    fn parses_multi_step() {
        let i = parse("Every hour check https://api.example.com, then ask Claude to summarize the results");
        assert_eq!(i.steps.len(), 2);
    }

    #[test]
    fn fallback_to_agent_step() {
        let i = parse("summarise my daily standup notes");
        assert_eq!(i.steps.len(), 1);
        assert!(matches!(&i.steps[0], ParsedStep::Agent { .. }));
    }
}
