//! 视频提示词下游依赖与可变性规则（从 a4n-services 收敛，供 orchestrator facade 使用）。

use a4n_db_models::models::chapter::{chapter, chapter_video_prompt};
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, QuerySelect};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

use crate::video::VideoDbError;

async fn lock_chapter_video_prompt_scope<C>(db: &C, chapter_id: i64) -> Result<String, VideoDbError>
where
    C: ConnectionTrait,
{
    let query = chapter::Entity::find_by_id(chapter_id)
        .select_only()
        .column(chapter::Column::Content);
    let query = if db.get_database_backend() == DbBackend::Sqlite {
        query
    } else {
        query.lock_exclusive()
    };
    query
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("章节不存在: {chapter_id}")))
}

type PromptExecutionIdentity = (i64, String, i32, String, String);
type PromptIdentity = (
    String,
    String,
    Option<i32>,
    Option<String>,
    Option<PromptExecutionIdentity>,
);
type PromptScope = (
    String,
    String,
    Option<String>,
    Option<PromptExecutionIdentity>,
);

fn prompt_execution_identity(metadata: Option<&Value>) -> Option<PromptExecutionIdentity> {
    let identity = metadata?.get("video_execution_identity")?;
    let segment_plan_prompt_id = identity
        .get("segment_plan_prompt_id")?
        .as_i64()
        .filter(|value| *value > 0)?;
    let source_shot_id = identity.get("source_shot_id")?.as_str()?.trim();
    if source_shot_id.is_empty() {
        return None;
    }
    let segment_index_in_shot = identity
        .get("segment_index_in_shot")?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)?;
    let segment_sha256 = identity.get("segment_sha256")?.as_str()?.trim();
    if segment_sha256.is_empty() {
        return None;
    }
    let recipe_fingerprint = identity.get("segment_recipe_fingerprint")?.as_str()?.trim();
    if recipe_fingerprint.is_empty() {
        return None;
    }
    Some((
        segment_plan_prompt_id,
        source_shot_id.to_string(),
        segment_index_in_shot,
        segment_sha256.to_string(),
        recipe_fingerprint.to_string(),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PromptStage {
    SegmentPlan,
    FramePrompt,
    Video,
}

mod dependencies;
pub(crate) use dependencies::generated_frame_images_for_prompts;
pub use dependencies::{
    delete_chapter_video_prompts_with_policy, effective_frame_prompt_image_reference_ids,
    ensure_chapter_video_prompt_stage_mutable, ensure_chapter_video_prompt_stage_mutable_ignoring,
    ensure_chapter_video_prompt_stages_mutable,
    ensure_chapter_video_prompt_stages_mutable_ignoring,
    ensure_chapter_video_prompt_stages_mutable_skipping_generated_images,
    ensure_frame_prompt_image_generation_allowed, prompt_downstream_dependencies,
};

mod sync;
pub use sync::sync_chapter_video_prompts_with_generation_runs;

mod update;
pub use update::{
    sync_chapter_video_prompt_metadata_after_update,
    update_chapter_video_prompt_with_generation_run_checkpoint,
    update_chapter_video_prompt_with_policy, update_chapter_video_segment_plan_metadata,
    update_chapter_video_segment_recipe,
};

#[cfg(test)]
mod tests;
