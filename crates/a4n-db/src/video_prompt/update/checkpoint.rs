use a4n_db_models::models::chapter::chapter_video_prompt;
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::{Value, json};

use crate::video::{self, VideoDbError};

use super::{
    super::ChapterVideoPromptPatch, metadata::sync_chapter_video_prompt_metadata_after_update,
    policy::update_chapter_video_prompt_with_policy_in,
};

pub async fn update_chapter_video_prompt_with_generation_run_checkpoint<C>(
    db: &C,
    prompt_id: i64,
    input: ChapterVideoPromptPatch,
    metadata_json: Option<Value>,
    allow_downstream: bool,
    mut generation_run: crate::generation::RecordCompletedGenerationRunWithCheckpointInput,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if !generation_run.run.targets.is_empty() {
        return Err(VideoDbError::InvalidArgument(
            "视频提示词生成运行记录目标由视频服务绑定，调用方不能预填 targets".to_string(),
        ));
    }

    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let mut row =
        update_chapter_video_prompt_with_policy_in(&txn, prompt_id, input, allow_downstream)
            .await?;
    if let Some(metadata_json) = metadata_json {
        let metadata_json = sync_chapter_video_prompt_metadata_after_update(
            &row.prompt_type,
            Some(metadata_json),
            &row.visual_prompt,
            row.negative_prompt.as_deref(),
            row.duration_secs,
            row.reference_image_ids_json.as_ref(),
        )?
        .ok_or_else(|| {
            VideoDbError::InvalidArgument(format!(
                "视频提示词 metadata 同步返回空值，prompt_type={}",
                row.prompt_type
            ))
        })?;
        row = video::update_chapter_video_prompt_metadata_record(&txn, row, metadata_json).await?;
    }
    if generation_run.run.project_id != row.project_id {
        return Err(VideoDbError::InvalidArgument(
            "视频提示词生成运行记录 project_id 与提示词资产不一致".to_string(),
        ));
    }

    let target = crate::generation::GenerationRunTraceTarget {
        target_kind: "chapter_video_prompt".to_string(),
        target_table: "chapter_video_prompt".to_string(),
        target_id: row.id,
    };
    generation_run.run.targets = vec![target.clone()];
    generation_run.checkpoint.parsed_output_json =
        chapter_video_prompt_parsed_output(generation_run.checkpoint.parsed_output_json, &row)?;
    generation_run.checkpoint.artifact_refs_json = Some(
        crate::generation::append_generation_artifact_target(
            generation_run.checkpoint.artifact_refs_json,
            target,
        )
        .map_err(VideoDbError::from)?,
    );
    crate::generation::record_completed_generation_run_with_checkpoint_in(
        &txn,
        generation_run.run,
        generation_run.checkpoint,
    )
    .await
    .map_err(VideoDbError::from)?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(row)
}

fn chapter_video_prompt_parsed_output(
    parsed_output: Option<Value>,
    row: &chapter_video_prompt::Model,
) -> Result<Option<Value>, VideoDbError> {
    let prompt = json!({
        "chapter_video_prompt_id": row.id,
        "project_id": row.project_id,
        "chapter_id": row.chapter_id,
        "workflow_mode": &row.workflow_mode,
        "prompt_type": &row.prompt_type,
        "shot_number": row.shot_number,
        "title": &row.title,
        "scene_summary": &row.scene_summary,
        "visual_prompt": &row.visual_prompt,
        "motion_prompt": &row.motion_prompt,
        "negative_prompt": &row.negative_prompt,
        "duration_secs": row.duration_secs,
        "aspect_ratio": &row.aspect_ratio,
        "reference_image_ids": &row.reference_image_ids_json,
        "metadata": &row.metadata_json,
    });
    match parsed_output {
        Some(Value::Object(mut map)) => {
            map.insert("chapter_video_prompt".to_string(), prompt);
            Ok(Some(Value::Object(map)))
        }
        Some(_) => Err(VideoDbError::InvalidArgument(
            "视频提示词生成运行记录 parsed_output_json 必须是对象".to_string(),
        )),
        None => Ok(Some(prompt)),
    }
}
