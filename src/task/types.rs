use serde::{Deserialize, Serialize};

/// All task variants that a job can contain.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskDef {
    /// Run a shell command
    Run { command: String },
    /// HTTP reachability check
    Url {
        url: String,
        expected_status: Option<u16>,
    },
    /// Check that a file path exists
    FileExists { path: String },
    /// Invoke an AI agent via heartbeat-agent-runner.sh
    Agent {
        /// "claude" | "opencode" | "codex"
        kind: String,
        prompt: String,
        cwd: Option<String>,
    },
    /// Call Anthropic or OpenAI API directly (no shell runner)
    AgentApi {
        provider: String, // "anthropic" | "openai"
        model: String,
        prompt: String,
    },
}
