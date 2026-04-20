use anyhow::Result;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::ipc::{ControlCmd, IpcRequest, IpcResponse};

/// Start the Unix socket server. Accepts connections in a loop;
/// each connection is handled in its own spawned task.
pub async fn serve(
    sock_path: PathBuf,
    cmd_tx: mpsc::Sender<ControlCmd>,
) -> Result<()> {
    // Remove stale socket from a crashed daemon
    if sock_path.exists() {
        tokio::fs::remove_file(&sock_path).await?;
    }

    let listener = UnixListener::bind(&sock_path)?;
    info!("IPC socket: {}", sock_path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let tx = cmd_tx.clone();
                tokio::spawn(handle_connection(stream, tx));
            }
            Err(e) => {
                // Socket was removed (clean shutdown) — exit silently
                if sock_path.exists() {
                    error!("IPC accept error: {}", e);
                }
                break;
            }
        }
    }
    Ok(())
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    cmd_tx: mpsc::Sender<ControlCmd>,
) {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();

    let line = match lines.next_line().await {
        Ok(Some(l)) => l,
        _ => return,
    };

    let req: IpcRequest = match serde_json::from_str(&line) {
        Ok(r) => r,
        Err(e) => {
            let resp = IpcResponse::err("", format!("invalid request: {}", e));
            write_response(&mut writer, resp).await;
            return;
        }
    };

    let resp = dispatch(req, &cmd_tx).await;
    write_response(&mut writer, resp).await;
}

async fn dispatch(req: IpcRequest, cmd_tx: &mpsc::Sender<ControlCmd>) -> IpcResponse {
    let id = &req.id;
    match req.cmd.as_str() {
        "ping" => {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx.send(ControlCmd::Ping { reply: tx }).await;
            let _ = rx.await;
            IpcResponse::ok(id, serde_json::json!({"pong": true}))
        }

        "list" => {
            let (tx, rx) = tokio::sync::oneshot::channel();
            if cmd_tx.send(ControlCmd::List { reply: tx }).await.is_err() {
                return IpcResponse::err(id, "controller unavailable");
            }
            match rx.await {
                Ok(jobs) => IpcResponse::ok(id, serde_json::json!({"jobs": jobs})),
                Err(_) => IpcResponse::err(id, "controller unavailable"),
            }
        }

        "run" => {
            let name = match req.name {
                Some(n) => n,
                None => return IpcResponse::err(id, "missing 'name' field"),
            };
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx.send(ControlCmd::RunNow { name, reply: tx }).await;
            match rx.await {
                Ok(Ok(())) => IpcResponse::ok(id, serde_json::json!(null)),
                Ok(Err(e)) => IpcResponse::err(id, e.to_string()),
                Err(_) => IpcResponse::err(id, "controller unavailable"),
            }
        }

        "stop" => {
            let name = match req.name {
                Some(n) => n,
                None => return IpcResponse::err(id, "missing 'name' field"),
            };
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx.send(ControlCmd::Stop { name, reply: tx }).await;
            match rx.await {
                Ok(Ok(())) => IpcResponse::ok(id, serde_json::json!(null)),
                Ok(Err(e)) => IpcResponse::err(id, e.to_string()),
                Err(_) => IpcResponse::err(id, "controller unavailable"),
            }
        }

        "reload" => {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx.send(ControlCmd::Reload { reply: tx }).await;
            let _ = rx.await;
            IpcResponse::ok(id, serde_json::json!(null))
        }

        other => IpcResponse::err(id, format!("unknown command: {}", other)),
    }
}

async fn write_response<W>(writer: &mut W, resp: IpcResponse)
where
    W: AsyncWriteExt + Unpin,
{
    let mut out = serde_json::to_string(&resp).unwrap_or_default();
    out.push('\n');
    writer.write_all(out.as_bytes()).await.ok();
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::JobSummary;
    use tempfile::TempDir;
    use tokio::net::UnixStream;

    async fn start_test_server(dir: &TempDir) -> (PathBuf, mpsc::Receiver<ControlCmd>) {
        let sock = dir.path().join("test.sock");
        let (tx, rx) = mpsc::channel::<ControlCmd>(8);
        let sock_c = sock.clone();
        tokio::spawn(async move {
            serve(sock_c, tx).await.ok();
        });
        // Give server a moment to bind
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        (sock, rx)
    }

    async fn send_raw(sock: &PathBuf, msg: &str) -> String {
        let mut stream = UnixStream::connect(sock).await.unwrap();
        let mut line = msg.to_string();
        line.push('\n');
        stream.write_all(line.as_bytes()).await.unwrap();
        let mut out = String::new();
        use tokio::io::AsyncBufReadExt;
        BufReader::new(stream).read_line(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn test_ping() {
        let dir = TempDir::new().unwrap();
        let (sock, mut rx) = start_test_server(&dir).await;

        // Respond to ping in background
        tokio::spawn(async move {
            if let Some(ControlCmd::Ping { reply }) = rx.recv().await {
                reply.send(()).ok();
            }
        });

        let raw = send_raw(&sock, r#"{"id":"t1","cmd":"ping"}"#).await;
        let resp: IpcResponse = serde_json::from_str(&raw).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.id, "t1");
        assert_eq!(resp.data.unwrap()["pong"], true);
    }

    #[tokio::test]
    async fn test_list_empty() {
        let dir = TempDir::new().unwrap();
        let (sock, mut rx) = start_test_server(&dir).await;

        tokio::spawn(async move {
            if let Some(ControlCmd::List { reply }) = rx.recv().await {
                reply.send(vec![]).ok();
            }
        });

        let raw = send_raw(&sock, r#"{"id":"t2","cmd":"list"}"#).await;
        let resp: IpcResponse = serde_json::from_str(&raw).unwrap();
        assert!(resp.ok);
        let jobs = &resp.data.unwrap()["jobs"];
        assert_eq!(jobs.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_run_job_not_found() {
        let dir = TempDir::new().unwrap();
        let (sock, mut rx) = start_test_server(&dir).await;

        tokio::spawn(async move {
            if let Some(ControlCmd::RunNow { reply, .. }) = rx.recv().await {
                reply
                    .send(Err(anyhow::anyhow!("job not found: missing")))
                    .ok();
            }
        });

        let raw = send_raw(
            &sock,
            r#"{"id":"t3","cmd":"run","name":"missing"}"#,
        )
        .await;
        let resp: IpcResponse = serde_json::from_str(&raw).unwrap();
        assert!(!resp.ok);
        assert!(resp.error.unwrap().contains("job not found"));
    }

    #[tokio::test]
    async fn test_unknown_command() {
        let dir = TempDir::new().unwrap();
        let (sock, _rx) = start_test_server(&dir).await;

        let raw = send_raw(&sock, r#"{"id":"t4","cmd":"banana"}"#).await;
        let resp: IpcResponse = serde_json::from_str(&raw).unwrap();
        assert!(!resp.ok);
        assert!(resp.error.unwrap().contains("unknown command"));
    }

    #[tokio::test]
    async fn test_stale_socket_cleaned() {
        let dir = TempDir::new().unwrap();
        let sock = dir.path().join("stale.sock");
        // Create a fake stale socket file
        std::fs::write(&sock, b"stale").unwrap();
        let (tx, _rx) = mpsc::channel::<ControlCmd>(1);
        // Should not error — removes stale file and binds
        tokio::spawn(serve(sock.clone(), tx));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(sock.exists());
    }

    #[tokio::test]
    async fn test_list_with_jobs() {
        let dir = TempDir::new().unwrap();
        let (sock, mut rx) = start_test_server(&dir).await;

        tokio::spawn(async move {
            if let Some(ControlCmd::List { reply }) = rx.recv().await {
                reply
                    .send(vec![JobSummary {
                        name: "my-job".into(),
                        status: "idle".into(),
                        schedule: "every 5m".into(),
                        workspace: "~".into(),
                        next_run: None,
                    }])
                    .ok();
            }
        });

        let raw = send_raw(&sock, r#"{"id":"t5","cmd":"list"}"#).await;
        let resp: IpcResponse = serde_json::from_str(&raw).unwrap();
        assert!(resp.ok);
        let jobs = &resp.data.unwrap()["jobs"];
        assert_eq!(jobs[0]["name"], "my-job");
    }
}
