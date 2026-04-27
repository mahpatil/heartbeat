use anyhow::{bail, Result};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::ipc::{new_id, IpcRequest, IpcResponse};

const IPC_TIMEOUT: Duration = Duration::from_secs(5);

pub fn sock_path(hb_dir: &PathBuf) -> PathBuf {
    hb_dir.join("heartbeat.sock")
}

/// Send a single IPC request to the daemon and return the response.
/// Fails with a clear error if the daemon does not respond within 5 seconds.
pub async fn send_ipc(hb_dir: &PathBuf, cmd: &str, name: Option<&str>) -> Result<IpcResponse> {
    let sock = sock_path(hb_dir);
    if !sock.exists() {
        bail!(
            "Daemon is not running (no socket at {}). Start it with: heartbeat daemon",
            sock.display()
        );
    }

    match tokio::time::timeout(IPC_TIMEOUT, do_send(&sock, cmd, name)).await {
        Ok(result) => result,
        Err(_) => bail!(
            "Daemon not responding (timed out after {}s). \
             It may be hung — restart with: heartbeat daemon",
            IPC_TIMEOUT.as_secs()
        ),
    }
}

async fn do_send(sock: &PathBuf, cmd: &str, name: Option<&str>) -> Result<IpcResponse> {
    let stream = UnixStream::connect(sock).await?;
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::net::UnixListener;

    /// A server that accepts connections but never writes a response.
    async fn start_silent_server(sock: PathBuf) {
        let listener = UnixListener::bind(&sock).unwrap();
        tokio::spawn(async move {
            loop {
                if let Ok((stream, _)) = listener.accept().await {
                    // Hold the stream open but write nothing.
                    tokio::spawn(async move {
                        let _stream = stream; // keep alive for 60 s
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    });
                }
            }
        });
        // Give the listener a moment to bind.
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    #[tokio::test]
    async fn send_ipc_times_out_when_daemon_silent() {
        let dir = TempDir::new().unwrap();
        let sock = dir.path().join("heartbeat.sock");
        start_silent_server(sock).await;

        let start = std::time::Instant::now();
        let result = send_ipc(&dir.path().to_path_buf(), "list", None).await;
        let elapsed = start.elapsed();

        assert!(result.is_err(), "expected timeout error");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("timed out"), "error should mention timeout: {}", msg);
        assert!(msg.contains("heartbeat daemon"), "error should suggest restart: {}", msg);
        // Should fire close to the 5 s deadline, not hang.
        assert!(elapsed < Duration::from_secs(7), "elapsed too long: {:?}", elapsed);
    }

    #[tokio::test]
    async fn send_ipc_no_socket_returns_clear_error() {
        let dir = TempDir::new().unwrap();
        let result = send_ipc(&dir.path().to_path_buf(), "list", None).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not running"), "got: {}", msg);
    }
}
