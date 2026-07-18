mod branches;
mod contracts;
mod plan;
mod rules;
mod sources;
mod visual;

pub use contracts::{
    GalgamePersistChoice, GalgamePersistLine, GalgamePersistScene, GalgamePipelineSourceSpan,
    GalgameScriptPersistPlan,
};
pub use plan::build_galgame_script_persist_plan;

#[cfg(test)]
mod tests;
