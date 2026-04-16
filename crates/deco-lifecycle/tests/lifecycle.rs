use deco_lifecycle::{
    LifecycleCommand, LifecycleExecutionStatus, LifecycleHooks, LifecyclePlanner, LifecycleStage,
    LifecycleStep, LifecycleStepError, LifecycleStepRunner, LifecycleStepStatus, execute_plan,
};

#[test]
fn plans_hooks_in_explicit_stage_order() {
    let hooks = LifecycleHooks {
        post_start: vec![LifecycleCommand::new("post-start")],
        initialize: vec![LifecycleCommand::new("init-1"), LifecycleCommand::new("init-2")],
        post_attach: vec![LifecycleCommand::new("attach")],
        on_create: vec![LifecycleCommand::new("create")],
        update_content: vec![LifecycleCommand::new("update")],
        post_create: vec![LifecycleCommand::new("post-create")],
    };

    let plan = LifecyclePlanner.plan(&hooks);

    let stages: Vec<_> = plan.steps.iter().map(|step| step.stage).collect();
    assert_eq!(
        stages,
        vec![
            LifecycleStage::Initialize,
            LifecycleStage::Initialize,
            LifecycleStage::OnCreate,
            LifecycleStage::UpdateContent,
            LifecycleStage::PostCreate,
            LifecycleStage::PostStart,
            LifecycleStage::PostAttach,
        ]
    );
    assert_eq!(
        plan.steps.iter().map(|step| step.ordinal).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn empty_hooks_produce_empty_plan() {
    let plan = LifecyclePlanner.plan(&LifecycleHooks::default());
    assert!(plan.is_empty());
    assert!(plan.steps.is_empty());
}

#[test]
fn execution_stops_on_first_failed_step() {
    let hooks = LifecycleHooks {
        initialize: vec![LifecycleCommand::new("init")],
        post_create: vec![LifecycleCommand::new("post-create")],
        ..Default::default()
    };
    let plan = LifecyclePlanner.plan(&hooks);
    let mut runner = FailingRunner { fail_on: 1, seen: Vec::new() };

    let report = execute_plan(&plan, &mut runner);

    assert_eq!(runner.seen.len(), 2);
    assert_eq!(runner.seen[0].command.command, "init");
    assert_eq!(runner.seen[1].command.command, "post-create");
    assert_eq!(
        report.status,
        LifecycleExecutionStatus::Failed { stage: LifecycleStage::PostCreate, ordinal: 1 }
    );
    assert_eq!(report.step_results.len(), 2);
    assert_eq!(report.step_results[0].status, LifecycleStepStatus::Succeeded);
    assert_eq!(report.step_results[1].status, LifecycleStepStatus::Failed);
    assert!(report.step_results[1].error.is_some());
}

struct FailingRunner {
    fail_on: usize,
    seen: Vec<LifecycleStep>,
}

impl LifecycleStepRunner for FailingRunner {
    fn run_step(&mut self, step: &LifecycleStep) -> Result<(), LifecycleStepError> {
        self.seen.push(step.clone());

        if self.seen.len() == self.fail_on + 1 {
            return Err(LifecycleStepError {
                message: format!("failed at {}", step.command.command),
                details: Some("simulated failure".to_string()),
            });
        }

        Ok(())
    }
}
