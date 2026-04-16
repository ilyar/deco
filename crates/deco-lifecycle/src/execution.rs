use serde::{Deserialize, Serialize};

use crate::{
    model::{LifecyclePlan, LifecycleStage, LifecycleStep},
    runner::{LifecycleStepError, LifecycleStepRunner},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleStepStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleStepResult {
    pub step: LifecycleStep,
    pub status: LifecycleStepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LifecycleStepError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleExecutionStatus {
    Completed,
    Failed { stage: LifecycleStage, ordinal: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleExecutionReport {
    pub status: LifecycleExecutionStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub step_results: Vec<LifecycleStepResult>,
}

pub fn execute_plan<R: LifecycleStepRunner>(
    plan: &LifecyclePlan,
    runner: &mut R,
) -> LifecycleExecutionReport {
    let mut step_results = Vec::with_capacity(plan.steps.len());

    for step in &plan.steps {
        match runner.run_step(step) {
            Ok(()) => {
                step_results.push(LifecycleStepResult {
                    step: step.clone(),
                    status: LifecycleStepStatus::Succeeded,
                    error: None,
                });
            }
            Err(error) => {
                step_results.push(LifecycleStepResult {
                    step: step.clone(),
                    status: LifecycleStepStatus::Failed,
                    error: Some(error),
                });

                return LifecycleExecutionReport {
                    status: LifecycleExecutionStatus::Failed {
                        stage: step.stage,
                        ordinal: step.ordinal,
                    },
                    step_results,
                };
            }
        }
    }

    LifecycleExecutionReport { status: LifecycleExecutionStatus::Completed, step_results }
}
