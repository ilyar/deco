use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::LifecycleStep;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{message}")]
pub struct LifecycleStepError {
    pub message: String,
    pub details: Option<String>,
}

pub trait LifecycleStepRunner {
    fn run_step(&mut self, step: &LifecycleStep) -> Result<(), LifecycleStepError>;
}
