mod assembly;
mod context;
mod contracts;
pub(crate) mod errors;
mod generation;
mod preparation;
mod progress;
mod stage;
mod workflow_rules;

pub use a4n_api_contract::shooting_script::{
    ChapterShootingScriptSourceInput, GenerateChapterShootingScriptInput,
};
#[cfg(test)]
pub(crate) use contracts::visual_plan_fixture;
pub use contracts::{
    CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL, ChapterShootingScriptUnitProgress,
    EditedChapterShootingScript, ResolvedChapterShootingScript, ShootingScriptProgressReporter,
    ShootingScriptStreamReporter,
};
pub(crate) use workflow_rules::chapter_shooting_script_workflow_run_input;
