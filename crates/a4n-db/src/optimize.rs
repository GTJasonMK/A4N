use a4n_core::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_session_message,
    optimization_turn, optimization_turn_event,
};
use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};
use a4n_core::models::character::{foreshadowing_index, protagonist_profile};
use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, novel_blueprint, novel_project,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::Value;

mod context;
mod patches;
mod sessions;
mod turns;
mod types;

pub use context::{
    optimization_chapter_by_id, optimization_context_rows, write_optimization_chapter_patch_content,
};
pub use patches::{
    create_optimization_patch_set, matching_pending_optimization_patch_sets,
    optimization_patch_set_patches, optimization_patch_sets, pending_optimization_patch_sets,
    update_optimization_patch_set_status,
};
pub use sessions::{
    create_optimization_session, delete_optimization_session_with_messages,
    insert_optimization_session_message, optimization_session_by_id, optimization_session_messages,
    optimization_session_name_exists, optimization_sessions, update_optimization_session,
};
pub use turns::{
    append_optimization_turn_event, complete_optimization_analysis_turn,
    complete_optimization_chat_turn, complete_optimization_patch_turn, create_optimization_turn,
    optimization_turn_by_id, optimization_turn_events, optimization_turns,
    update_optimization_turn,
};
pub use types::{
    CompleteOptimizationPatchTurnInput, CompleteOptimizationPatchTurnResult,
    OptimizationBlueprintRows, OptimizationChapterPatchWriteInput, OptimizationContextReadInput,
    OptimizationContextRows, OptimizationPatchCreateInput, OptimizationPatchSetCreateInput,
    OptimizationSessionCreateInput, OptimizationSessionMessageInsertInput,
    OptimizationSessionUpdateInput, OptimizationTurnCreateInput, OptimizationTurnEventAppendInput,
    OptimizationTurnUpdateInput, OptimizeDbError,
};

#[cfg(test)]
mod tests;
