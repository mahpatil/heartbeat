/// Shared IPC wire types used by both the daemon server and CLI client.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub id: String,
    pub cmd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobSummary {
    pub name: String,
    pub status: String,
    pub schedule: String,
    pub workspace: String,
    pub next_run: Option<String>,
}

// ── Control commands (daemon-side only) ───────────────────────────────────────

pub enum ControlCmd {
    Ping   { reply: oneshot::Sender<()> },
    List   { reply: oneshot::Sender<Vec<JobSummary>> },
    RunNow { name: String, reply: oneshot::Sender<Result<()>> },
    Stop   { name: String, reply: oneshot::Sender<Result<()>> },
    Reload { reply: oneshot::Sender<()> },
}

// ── Helpers ───────────────────────────────────────────────────────────────────

impl IpcResponse {
    pub fn ok(id: &str, data: serde_json::Value) -> Self {
        let data = if data.is_null() { None } else { Some(data) };
        Self { id: id.to_string(), ok: true, data, error: None }
    }

    pub fn err(id: &str, msg: impl Into<String>) -> Self {
        Self { id: id.to_string(), ok: false, data: None, error: Some(msg.into()) }
    }
}

/// Tiny pseudo-UUID from system time — avoids a uuid crate dependency.
pub fn new_id() -> String {
    let ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:032x}", ns)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = IpcRequest {
            id: "abc-123".into(),
            cmd: "ping".into(),
            name: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: IpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "abc-123");
        assert_eq!(back.cmd, "ping");
        assert!(back.name.is_none());
    }

    #[test]
    fn response_ok_roundtrip() {
        let resp = IpcResponse::ok("abc-123", serde_json::json!({"pong": true}));
        let json = serde_json::to_string(&resp).unwrap();
        let back: IpcResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "abc-123");
        assert!(back.ok);
        assert!(back.error.is_none());
    }

    #[test]
    fn response_err_roundtrip() {
        let resp = IpcResponse::err("abc-123", "job not found: foo");
        let json = serde_json::to_string(&resp).unwrap();
        let back: IpcResponse = serde_json::from_str(&json).unwrap();
        assert!(!back.ok);
        assert_eq!(back.error.as_deref(), Some("job not found: foo"));
    }

    #[test]
    fn new_id_is_nonempty() {
        assert!(!new_id().is_empty());
    }
}
