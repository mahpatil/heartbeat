use anyhow::Result;
use std::path::PathBuf;

use super::ipc_client::send_ipc;

pub async fn run(name: &str, hb_dir: &PathBuf) -> Result<()> {
    let resp = send_ipc(hb_dir, "run", Some(name)).await?;

    if resp.ok {
        println!("Triggered: {}", name);
    } else {
        anyhow::bail!("{}", resp.error.unwrap_or_default());
    }

    Ok(())
}
