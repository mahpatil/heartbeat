use anyhow::{bail, Result};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::ipc::{new_id, IpcRequest, IpcResponse};

pub fn sock_path(hb_dir: &PathBuf) -> PathBuf {
    hb_dir.join("heartbeat.sock")
}

/// Send a single IPC request to the daemon and return the response.
pub async fn send_ipc(hb_dir: &PathBuf, cmd: &str, name: Option<&str>) -> Result<IpcResponse> {
    let sock = sock_path(hb_dir);
    if !sock.exists() {
        bail!("Daemon is not running (no socket at {}). Start it with: heartbeat daemon", sock.display());
    }

    let stream = UnixStream::connect(&sock).await?;
    let (reader, mut writer) = tokio::io::split(stream);

    let req = IpcRequest {
        id: new_id(),
        cmd: cmd.to_string(),
        name: name.map(|s| s.to_string()),
    };
    let mut line = serde_json::to_string(&req)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;

    let mut resp_line = String::new();
    BufReader::new(reader).read_line(&mut resp_line).await?;
    let resp: IpcResponse = serde_json::from_str(resp_line.trim())?;
    Ok(resp)
}
