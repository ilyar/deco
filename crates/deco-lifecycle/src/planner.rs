use crate::model::{LifecycleHooks, LifecyclePlan, LifecycleStage, LifecycleStep};

#[derive(Debug, Default, Clone, Copy)]
pub struct LifecyclePlanner;

impl LifecyclePlanner {
    pub fn plan(&self, hooks: &LifecycleHooks) -> LifecyclePlan {
        let mut steps = Vec::new();
        let mut ordinal = 0usize;

        for stage in LifecycleStage::ordered() {
            let commands = match stage {
                LifecycleStage::Initialize => &hooks.initialize,
                LifecycleStage::OnCreate => &hooks.on_create,
                LifecycleStage::UpdateContent => &hooks.update_content,
                LifecycleStage::PostCreate => &hooks.post_create,
                LifecycleStage::PostStart => &hooks.post_start,
                LifecycleStage::PostAttach => &hooks.post_attach,
            };

            for command in commands.iter().cloned() {
                steps.push(LifecycleStep { ordinal, stage: *stage, command });
                ordinal += 1;
            }
        }

        LifecyclePlan { steps }
    }
}
