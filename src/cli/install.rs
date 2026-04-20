use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

// ── Public entry points ───────────────────────────────────────────────────────

/// `heartbeat install [--autostart]`
pub async fn run(hb_dir: &PathBuf, autostart: bool) -> Result<()> {
    let bin = hb_dir.join("heartbeat");
    if !bin.exists() {
        anyhow::bail!(
            "Heartbeat binary not found at {}. Run install.sh first.",
            bin.display()
        );
    }

    if autostart {
        install_launchagent(hb_dir, &bin).await?;
    } else {
        println!("Run `heartbeat install --autostart` to enable auto-start on login.");
    }

    Ok(())
}

/// `heartbeat uninstall --autostart`
pub async fn uninstall() -> Result<()> {
    let home = home_dir()?;
    let plist = plist_path(&home);

    if !plist.exists() {
        println!("LaunchAgent not installed — nothing to remove.");
        return Ok(());
    }

    // Unload (ignore errors — agent may already be stopped)
    tokio::process::Command::new("launchctl")
        .args(["unload", plist.to_str().unwrap()])
        .status()
        .await
        .ok();

    tokio::fs::remove_file(&plist).await?;
    println!("Removed LaunchAgent. heartbeat will no longer start at login.");
    Ok(())
}

// ── Core implementation ───────────────────────────────────────────────────────

async fn install_launchagent(hb_dir: &PathBuf, bin: &PathBuf) -> Result<()> {
    let home = home_dir()?;
    let path_env = std::env::var("PATH").unwrap_or_default();

    let agents_dir = PathBuf::from(&home).join("Library/LaunchAgents");
    tokio::fs::create_dir_all(&agents_dir).await?;

    let plist = plist_path(&home);
    let log_path = hb_dir.join("logs").join("daemon.log");

    // Idempotent: unload existing agent before overwriting
    if plist.exists() {
        info!("Unloading existing LaunchAgent before reinstall");
        tokio::process::Command::new("launchctl")
            .args(["unload", plist.to_str().unwrap()])
            .status()
            .await
            .ok();
    }

    let xml = plist_contents(
        bin.to_str().unwrap(),
        log_path.to_str().unwrap(),
        &home,
        &path_env,
    );

    tokio::fs::write(&plist, xml).await?;
    info!("Wrote LaunchAgent: {}", plist.display());

    let status = tokio::process::Command::new("launchctl")
        .args(["load", "-w", plist.to_str().unwrap()])
        .status()
        .await?;

    if status.success() {
        println!("Autostart enabled. Heartbeat will start on login.");
        println!("Plist: {}", plist.display());
        println!("To disable: heartbeat uninstall --autostart");
    } else {
        anyhow::bail!("launchctl load failed (exit {})", status);
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn plist_path(home: &str) -> PathBuf {
    PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join("com.heartbeat.plist")
}

fn home_dir() -> Result<String> {
    std::env::var("HOME").map_err(|_| anyhow::anyhow!("$HOME is not set"))
}

/// Build the plist XML. Pure function — easy to test.
pub fn plist_contents(bin: &str, log_path: &str, home: &str, path_env: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
    "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.heartbeat</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>HOME</key>
        <string>{home}</string>
        <key>PATH</key>
        <string>{path}</string>
    </dict>
</dict>
</plist>
"#,
        bin = bin,
        log = log_path,
        home = home,
        path = path_env,
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const BIN: &str = "/Users/mahesh/.heartbeat/heartbeat";
    const LOG: &str = "/Users/mahesh/.heartbeat/logs/daemon.log";
    const HOME: &str = "/Users/mahesh";
    const PATH_ENV: &str = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin";

    fn plist() -> String {
        plist_contents(BIN, LOG, HOME, PATH_ENV)
    }

    // ── plist structure ───────────────────────────────────────────────────────

    #[test]
    fn plist_has_correct_label() {
        assert!(plist().contains("<string>com.heartbeat</string>"));
    }

    #[test]
    fn plist_program_arguments_contains_bin() {
        let xml = plist();
        assert!(xml.contains(&format!("<string>{}</string>", BIN)));
        assert!(xml.contains("<string>daemon</string>"));
    }

    #[test]
    fn plist_run_at_load_true() {
        let xml = plist();
        let run_at = xml.find("<key>RunAtLoad</key>").unwrap();
        let after = &xml[run_at..];
        assert!(after.contains("<true/>"), "RunAtLoad should be true");
    }

    #[test]
    fn plist_keep_alive_true() {
        let xml = plist();
        let pos = xml.find("<key>KeepAlive</key>").unwrap();
        let after = &xml[pos..];
        assert!(after.contains("<true/>"), "KeepAlive should be true");
    }

    #[test]
    fn plist_stdout_and_stderr_same_log() {
        let xml = plist();
        assert!(
            xml.contains(&format!("<string>{}</string>", LOG)),
            "log path missing"
        );
        // Both StandardOutPath and StandardErrorPath should reference the log
        assert!(xml.contains("<key>StandardOutPath</key>"));
        assert!(xml.contains("<key>StandardErrorPath</key>"));
        // Both must point to the same file
        let count = xml.matches(LOG).count();
        assert_eq!(count, 2, "expected two references to log path, got {}", count);
    }

    #[test]
    fn plist_environment_variables_home() {
        let xml = plist();
        assert!(xml.contains("<key>EnvironmentVariables</key>"));
        assert!(xml.contains("<key>HOME</key>"));
        assert!(xml.contains(&format!("<string>{}</string>", HOME)));
    }

    #[test]
    fn plist_environment_variables_path() {
        let xml = plist();
        assert!(xml.contains("<key>PATH</key>"));
        assert!(xml.contains(&format!("<string>{}</string>", PATH_ENV)));
    }

    #[test]
    fn plist_is_valid_xml_root() {
        let xml = plist();
        assert!(xml.starts_with("<?xml version=\"1.0\""));
        assert!(xml.contains("<plist version=\"1.0\">"));
        assert!(xml.contains("</plist>"));
    }

    // ── Path helpers ──────────────────────────────────────────────────────────

    #[test]
    fn plist_path_is_launchagents() {
        let p = plist_path("/Users/mahesh");
        assert!(
            p.to_str().unwrap().contains("Library/LaunchAgents"),
            "must be in LaunchAgents, not LaunchDaemons"
        );
        assert!(p.to_str().unwrap().contains("com.heartbeat.plist"));
    }

    #[test]
    fn plist_path_not_in_system_launchdaemons() {
        let p = plist_path("/Users/mahesh");
        assert!(!p.starts_with("/Library/LaunchDaemons"));
    }

    // ── Different users ───────────────────────────────────────────────────────

    #[test]
    fn plist_contents_reflect_actual_user() {
        let xml = plist_contents(
            "/Users/alice/.heartbeat/heartbeat",
            "/Users/alice/.heartbeat/logs/daemon.log",
            "/Users/alice",
            "/usr/bin:/bin",
        );
        assert!(xml.contains("/Users/alice/.heartbeat/heartbeat"));
        assert!(xml.contains("<string>/Users/alice</string>"));
    }
}
