use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

/// Install a macOS LaunchAgent plist so the daemon auto-starts on login.
pub async fn run(hb_dir: &PathBuf, autostart: bool) -> Result<()> {
    let bin = hb_dir.join("heartbeat");

    if !bin.exists() {
        anyhow::bail!(
            "Heartbeat binary not found at {}. Install it first.",
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

async fn install_launchagent(hb_dir: &PathBuf, bin: &PathBuf) -> Result<()> {
    let home = std::env::var("HOME")?;
    let agents_dir = PathBuf::from(&home).join("Library/LaunchAgents");
    tokio::fs::create_dir_all(&agents_dir).await?;

    let plist_path = agents_dir.join("com.heartbeat.plist");
    let log_out = hb_dir.join("daemon.log");
    let log_err = hb_dir.join("daemon.err");

    let plist = format!(
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
    <string>{log_out}</string>
    <key>StandardErrorPath</key>
    <string>{log_err}</string>
</dict>
</plist>
"#,
        bin = bin.display(),
        log_out = log_out.display(),
        log_err = log_err.display(),
    );

    tokio::fs::write(&plist_path, plist).await?;
    info!("Wrote LaunchAgent plist: {}", plist_path.display());

    // Load immediately
    let status = tokio::process::Command::new("launchctl")
        .args(["load", "-w", plist_path.to_str().unwrap()])
        .status()
        .await?;

    if status.success() {
        println!("Autostart enabled. Heartbeat will start on login.");
        println!("Plist: {}", plist_path.display());
    } else {
        anyhow::bail!("launchctl load failed with status {}", status);
    }

    Ok(())
}
