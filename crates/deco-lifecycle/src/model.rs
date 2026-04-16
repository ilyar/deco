use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleStage {
    Initialize,
    OnCreate,
    UpdateContent,
    PostCreate,
    PostStart,
    PostAttach,
}

impl LifecycleStage {
    pub const ORDERED: [Self; 6] = [
        Self::Initialize,
        Self::OnCreate,
        Self::UpdateContent,
        Self::PostCreate,
        Self::PostStart,
        Self::PostAttach,
    ];

    pub fn ordered() -> &'static [Self] {
        &Self::ORDERED
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleCommand {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environment: BTreeMap<String, String>,
}

impl LifecycleCommand {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            working_directory: None,
            user: None,
            environment: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LifecycleHooks {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initialize: Vec<LifecycleCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_create: Vec<LifecycleCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub update_content: Vec<LifecycleCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_create: Vec<LifecycleCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_start: Vec<LifecycleCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_attach: Vec<LifecycleCommand>,
}

impl LifecycleHooks {
    pub fn is_empty(&self) -> bool {
        self.initialize.is_empty()
            && self.on_create.is_empty()
            && self.update_content.is_empty()
            && self.post_create.is_empty()
            && self.post_start.is_empty()
            && self.post_attach.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleStep {
    pub ordinal: usize,
    pub stage: LifecycleStage,
    pub command: LifecycleCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LifecyclePlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<LifecycleStep>,
}

impl LifecyclePlan {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}
