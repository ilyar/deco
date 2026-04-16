use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParityManifest {
    pub fixtures: Vec<ParityFixture>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParityFixture {
    pub id: String,
    pub description: String,
    pub workspace: PathBuf,
    pub command: Vec<String>,
    pub expected: ParityExpectation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParityExpectation {
    pub exit_code: i32,
    pub stdout_contains: Vec<String>,
    pub stderr_contains: Vec<String>,
    #[serde(default)]
    pub allow_upstream_exit_code_difference: bool,
}

impl ParityManifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .map_err(|error| format!("failed to read manifest {}: {error}", path.display()))?;
        serde_json::from_str(&raw)
            .map_err(|error| format!("failed to parse manifest {}: {error}", path.display()))
    }
}
