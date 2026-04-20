use anyhow::{bail, Result};
use chrono::{DateTime, Local, NaiveDateTime, NaiveTime, TimeZone, Utc};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Schedule {
    /// Repeat every fixed duration. First fire is immediate.
    Every(Duration),
    /// Fire once per day at a local-timezone HH:MM.
    DailyAt { hour: u32, minute: u32 },
    /// Fire once at a specific UTC instant.
    OnceAt(DateTime<Utc>),
}

impl Schedule {
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        if let Some(rest) = s.strip_prefix("every ") {
            return parse_interval(rest.trim());
        }
        if let Some(rest) = s.strip_prefix("daily at ") {
            let (h, m) = parse_hhmm(rest.trim())?;
            return Ok(Schedule::DailyAt { hour: h, minute: m });
        }
        if let Some(rest) = s.strip_prefix("once at ") {
            let rest = rest.trim();
            // Try RFC-3339 first
            if let Ok(dt) = DateTime::parse_from_rfc3339(rest) {
                return Ok(Schedule::OnceAt(dt.with_timezone(&Utc)));
            }
            // Try bare HH:MM (today or tomorrow)
            let (h, m) = parse_hhmm(rest)?;
            return Ok(Schedule::OnceAt(next_hhmm_utc(h, m)));
        }

        bail!("unrecognised schedule: {:?}", s)
    }

    /// Next scheduled fire time, if computable from the schedule alone.
    /// Returns `None` for `Every` intervals (requires job state to know next fire).
    pub fn next_run(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        match self {
            Schedule::Every(_) => None,
            Schedule::DailyAt { hour, minute } => Some(next_hhmm_utc(*hour, *minute)),
            Schedule::OnceAt(dt) => {
                if *dt > chrono::Utc::now() { Some(*dt) } else { None }
            }
        }
    }

    pub fn display(&self) -> String {
        match self {
            Schedule::Every(d) => {
                let s = d.as_secs();
                if s % 86_400 == 0 { format!("every {}d", s / 86_400) }
                else if s % 3600 == 0 { format!("every {}h", s / 3600) }
                else if s % 60 == 0 { format!("every {}m", s / 60) }
                else { format!("every {}s", s) }
            }
            Schedule::DailyAt { hour, minute } => {
                format!("daily at {:02}:{:02}", hour, minute)
            }
            Schedule::OnceAt(dt) => {
                format!("once at {}", dt.format("%Y-%m-%dT%H:%M:%SZ"))
            }
        }
    }
}

fn parse_interval(s: &str) -> Result<Schedule> {
    // Find where digits end
    let split = s
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| anyhow::anyhow!("missing time unit in {:?}", s))?;

    if split == 0 {
        bail!("missing number before time unit in {:?}", s);
    }

    let n: u64 = s[..split]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid number in {:?}", s))?;

    if n == 0 {
        bail!("interval must be greater than 0");
    }

    let unit = s[split..].trim();
    let secs = match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => n,
        "m" | "min" | "mins" | "minute" | "minutes" => n * 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => n * 3600,
        "d" | "day" | "days" => n * 86_400,
        other => bail!("unknown time unit {:?}", other),
    };

    Ok(Schedule::Every(Duration::from_secs(secs)))
}

fn parse_hhmm(s: &str) -> Result<(u32, u32)> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        bail!("invalid time format {:?}, expected HH:MM", s);
    }
    let h: u32 = parts[0]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid hours in {:?}", s))?;
    let m: u32 = parts[1]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid minutes in {:?}", s))?;
    if h > 23 || m > 59 {
        bail!("invalid time: {}", s);
    }
    Ok((h, m))
}

/// Returns the next UTC DateTime for local HH:MM — today if still in the
/// future, tomorrow otherwise.
pub fn next_hhmm_utc(hour: u32, minute: u32) -> DateTime<Utc> {
    let now = Local::now();
    let naive_today = now
        .date_naive()
        .and_time(NaiveTime::from_hms_opt(hour, minute, 0).unwrap());

    let target_local = local_from_naive(naive_today);
    if target_local > now {
        return target_local.with_timezone(&Utc);
    }

    // Tomorrow
    let naive_tomorrow = now
        .date_naive()
        .succ_opt()
        .unwrap_or(now.date_naive())
        .and_time(NaiveTime::from_hms_opt(hour, minute, 0).unwrap());

    local_from_naive(naive_tomorrow).with_timezone(&Utc)
}

/// Seconds until the next occurrence of local HH:MM.
pub fn secs_until_hhmm(hour: u32, minute: u32) -> u64 {
    let target = next_hhmm_utc(hour, minute);
    let now = Utc::now();
    if target <= now {
        0
    } else {
        (target - now).num_seconds().max(0) as u64
    }
}

fn local_from_naive(naive: NaiveDateTime) -> DateTime<Local> {
    Local
        .from_local_datetime(&naive)
        .single()
        .unwrap_or_else(|| {
            // DST ambiguity fallback: add 24h
            Local::now() + chrono::Duration::hours(24)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_every_5m() {
        let s = Schedule::parse("every 5m").unwrap();
        assert!(matches!(s, Schedule::Every(d) if d.as_secs() == 300));
    }

    #[test]
    fn parse_every_2h() {
        let s = Schedule::parse("every 2h").unwrap();
        assert!(matches!(s, Schedule::Every(d) if d.as_secs() == 7200));
    }

    #[test]
    fn parse_every_30s() {
        let s = Schedule::parse("every 30s").unwrap();
        assert!(matches!(s, Schedule::Every(d) if d.as_secs() == 30));
    }

    #[test]
    fn parse_daily_at() {
        let s = Schedule::parse("daily at 02:30").unwrap();
        assert!(matches!(s, Schedule::DailyAt { hour: 2, minute: 30 }));
    }

    #[test]
    fn parse_once_at_rfc3339() {
        let s = Schedule::parse("once at 2030-01-01T00:00:00Z").unwrap();
        assert!(matches!(s, Schedule::OnceAt(_)));
    }

    #[test]
    fn parse_unknown_unit() {
        assert!(Schedule::parse("every 5x").is_err());
    }

    #[test]
    fn parse_invalid_daily() {
        assert!(Schedule::parse("daily at 25:00").is_err());
    }

    #[test]
    fn parse_unrecognised() {
        let e = Schedule::parse("run it sometime").unwrap_err();
        assert!(e.to_string().contains("unrecognised schedule"));
    }

    // ── every Nd ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_every_1d() {
        let s = Schedule::parse("every 1d").unwrap();
        assert!(matches!(s, Schedule::Every(d) if d.as_secs() == 86_400));
    }

    #[test]
    fn parse_every_day_long_form() {
        let s = Schedule::parse("every 2 days").unwrap();
        assert!(matches!(s, Schedule::Every(d) if d.as_secs() == 172_800));
    }

    // ── once at HH:MM ────────────────────────────────────────────────────────

    #[test]
    fn parse_once_at_bare_hhmm() {
        // Should parse without error and return a future UTC instant.
        let s = Schedule::parse("once at 04:00").unwrap();
        match s {
            Schedule::OnceAt(dt) => {
                assert!(dt > Utc::now(), "once at 04:00 resolved to the past");
            }
            _ => panic!("expected OnceAt"),
        }
    }

    #[test]
    fn parse_once_at_bare_hhmm_midnight() {
        let s = Schedule::parse("once at 00:00").unwrap();
        assert!(matches!(s, Schedule::OnceAt(_)));
    }

    #[test]
    fn parse_once_at_invalid_hhmm() {
        // 25:00 is out of range
        assert!(Schedule::parse("once at 25:00").is_err());
    }

    // ── display() ────────────────────────────────────────────────────────────

    #[test]
    fn display_every_seconds() {
        let s = Schedule::Every(Duration::from_secs(45));
        assert_eq!(s.display(), "every 45s");
    }

    #[test]
    fn display_every_minutes() {
        let s = Schedule::Every(Duration::from_secs(300));
        assert_eq!(s.display(), "every 5m");
    }

    #[test]
    fn display_every_hours() {
        let s = Schedule::Every(Duration::from_secs(7200));
        assert_eq!(s.display(), "every 2h");
    }

    #[test]
    fn display_every_days() {
        let s = Schedule::Every(Duration::from_secs(86_400));
        assert_eq!(s.display(), "every 1d");
    }

    #[test]
    fn display_every_multi_day() {
        let s = Schedule::Every(Duration::from_secs(3 * 86_400));
        assert_eq!(s.display(), "every 3d");
    }

    #[test]
    fn display_daily_at() {
        let s = Schedule::DailyAt { hour: 2, minute: 30 };
        assert_eq!(s.display(), "daily at 02:30");
    }

    #[test]
    fn display_once_at_rfc3339() {
        let dt = DateTime::parse_from_rfc3339("2030-06-01T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let s = Schedule::OnceAt(dt);
        assert_eq!(s.display(), "once at 2030-06-01T09:00:00Z");
    }

    // ── zero / edge cases ─────────────────────────────────────────────────────

    #[test]
    fn parse_zero_interval_is_error() {
        assert!(Schedule::parse("every 0m").is_err());
    }

    #[test]
    fn parse_missing_number_is_error() {
        assert!(Schedule::parse("every m").is_err());
    }

    #[test]
    fn parse_missing_unit_is_error() {
        assert!(Schedule::parse("every 5").is_err());
    }
}
