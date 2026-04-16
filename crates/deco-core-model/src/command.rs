use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandKind {
    ReadConfiguration,
    Build,
    Up,
    Exec,
    RunUserCommands,
    SetUp,
    Features,
    Templates,
    Outdated,
    Upgrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
}
