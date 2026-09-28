mod branches;
mod contracts;
mod plan;
mod rules;
mod sources;
mod validation;
mod visual;

#[cfg(test)]
pub use contracts::GalgameAdaptationSceneRef;
pub use contracts::{
    GalgamePersistChoice, GalgamePersistLine, GalgamePersistScene, GalgamePipelineSourceSpan,
    GalgameScriptPersistPlan,
};
pub use plan::build_galgame_script_persist_plan;
pub(crate) use validation::validate_galgame_branch_design;
pub(in crate::galgame) use validation::{
    validate_galgame_script_adaptation, validate_galgame_staging,
};

#[cfg(test)]
mod tests;
