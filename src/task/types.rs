#[derive(Debug, Clone)]
pub enum StepDef {
    Agent {
        name: Option<String>,
        agent: String,
        prompt: String,
        flags: Vec<String>,
        /// Per-step workspace override (Milestone 3 chained-steps).
        workspace: Option<String>,
    },
    Shell {
        name: Option<String>,
        command: String,
        workspace: Option<String>,
    },
    UrlCheck {
        name: Option<String>,
        url: String,
        expected_status: Option<u16>,
    },
    FileCheck {
        name: Option<String>,
        path: String,
    },
}

impl StepDef {
    /// Human-readable label for log output.
    pub fn display_name(&self, idx: usize) -> String {
        let name = match self {
            StepDef::Agent { name, .. } => name,
            StepDef::Shell { name, .. } => name,
            StepDef::UrlCheck { name, .. } => name,
            StepDef::FileCheck { name, .. } => name,
        };
        name.clone()
            .unwrap_or_else(|| format!("step[{}]", idx))
    }
}
