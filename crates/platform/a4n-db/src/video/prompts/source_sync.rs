use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_video_contract::VideoShootingScriptIdentityV1;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};

use crate::shooting_script::ShootingScriptDbError;

/// 状态只在读取时计算；历史规划和生成文件保留各自的真实来源。
pub async fn with_video_plan_source_sync<C: ConnectionTrait>(
    db: &C,
    mut row: chapter_video_prompt::Model,
) -> Result<chapter_video_prompt::Model, crate::video::VideoDbError> {
    if row.workflow_mode == chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
        && row.prompt_type == chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
        && row.status == chapter_video_prompt::STATUS_COMPLETED
    {
        let stale = video_plan_stale_shot_ids(db, &row)
            .await
            .map_err(|error| match error {
                ShootingScriptDbError::Database(error) => {
                    crate::video::VideoDbError::Database(error)
                }
                error => crate::video::VideoDbError::InvalidState {
                    current: error.to_string(),
                    operation: "读取视频规划来源".into(),
                },
            })?;
        let metadata = row
            .metadata_json
            .as_mut()
            .and_then(Value::as_object_mut)
            .ok_or_else(|| {
                crate::video::VideoDbError::InvalidArgument("视频规划缺少元数据".into())
            })?;
        metadata.insert("stale_source_shot_ids".into(), json!(stale));
    }
    Ok(row)
}

async fn video_plan_stale_shot_ids<C: ConnectionTrait>(
    db: &C,
    row: &chapter_video_prompt::Model,
) -> Result<Vec<String>, ShootingScriptDbError> {
    let metadata = row
        .metadata_json
        .as_ref()
        .ok_or_else(|| ShootingScriptDbError::InvalidState("视频规划缺少元数据".into()))?;
    let saved: VideoShootingScriptIdentityV1 = serde_json::from_value(
        metadata["video_execution_identity"]["shooting_script_identity"].clone(),
    )
    .map_err(|error| {
        ShootingScriptDbError::InvalidState(format!("视频规划的拍摄剧本来源无效: {error}"))
    })?;
    let segments = a4n_video_contract::video_segment_plan_from_metadata(metadata)
        .map_err(ShootingScriptDbError::InvalidState)?;
    a4n_video_contract::validate_video_segment_plan_identity(
        &metadata["video_execution_identity"],
        &segments,
    )
    .map_err(ShootingScriptDbError::InvalidState)?;
    let shot_ids = segments
        .iter()
        .map(|segment| segment.source_shot_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if saved.shot_fingerprints.is_empty() {
        return Ok(shot_ids.into_iter().collect());
    }
    let source = crate::project::resolve_chapter_source(db, row.project_id, row.chapter_id)
        .await
        .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))?;
    let script = if let Some(published) =
        crate::shooting_script::chapter_shooting_script_for_source(db, &source).await?
    {
        published.script
    } else if let Some(progress) =
        crate::shooting_script::chapter_shooting_script_progress_for_source(db, &source).await?
    {
        progress.script
    } else {
        return Ok(shot_ids.into_iter().collect());
    };
    let director = crate::shooting_script::shooting_script_director_plan(db, &script).await?;
    let plan_sync = script.plan_sync(&director);
    let fingerprints = script
        .shot_content_fingerprints(&director)
        .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))?;
    Ok(shot_ids
        .into_iter()
        .filter(|id| {
            plan_sync.stale_shot_ids.contains(id)
                || !saved.matches_shot(&source.content_sha256, id, fingerprints.get(id))
        })
        .collect())
}
