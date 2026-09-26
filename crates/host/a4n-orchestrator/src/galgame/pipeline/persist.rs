mod branches;
mod contracts;
mod plan;
mod rules;
mod sources;
mod validation;
mod visual;

pub use contracts::{
    GalgamePersistChoice, GalgamePersistLine, GalgamePersistScene, GalgamePipelineSourceSpan,
    GalgameScriptPersistPlan, GalgameShootingSceneRef,
};
pub use plan::build_galgame_script_persist_plan;
pub(in crate::galgame) use validation::{
    validate_galgame_branch_design, validate_galgame_script_adaptation, validate_galgame_staging,
};

#[cfg(test)]
mod tests;
