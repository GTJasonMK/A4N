use std::collections::{BTreeSet, HashMap, HashSet};

use a4n_domain::video_prompt::UpdateChapterVideoPromptRecordInput;
use a4n_domain::video_prompt::{ChapterVideoPromptRecord, chapter_video_prompt};
use a4n_error::AppError;
use serde_json::{Value, json};

use super::{OrchestratorContext, PromptWorkflowMode, VideoFramePrompt};

pub(super) async fn sync_reusable_boundary_frame_previous_image_refs(
    service: &OrchestratorContext,
    rows: Vec<ChapterVideoPromptRecord>,
    image_id_by_frame_number: &HashMap<i32, i64>,
) -> Result<Vec<ChapterVideoPromptRecord>, AppError> {
    let image_ids = rows
        .iter()
        .filter_map(|row| row.reference_image_ids_json.as_ref())
        .filter_map(Value::as_array)
        .flat_map(|items| items.iter().filter_map(Value::as_i64))
        .collect::<BTreeSet<_>>();
    let existing_image_ids = service
        .generated_image_ids_by_ids(&image_ids.iter().copied().collect::<Vec<_>>())
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut synced = Vec::with_capacity(rows.len());
    for row in rows {
        synced.push(
            sync_boundary_frame_previous_image_ref(
                service,
                row,
                image_id_by_frame_number,
                &existing_image_ids,
            )
            .await?,
        );
    }
    Ok(synced)
}

pub(super) async fn sync_boundary_frame_previous_image_ref(
    service: &OrchestratorContext,
    row: ChapterVideoPromptRecord,
    image_id_by_frame_number: &HashMap<i32, i64>,
    existing_image_ids: &BTreeSet<i64>,
) -> Result<ChapterVideoPromptRecord, AppError> {
    if row.shot_number <= 1 {
        return Ok(row);
    }
    let Some(latest_previous_image_id) = image_id_by_frame_number
        .get(&(row.shot_number - 1))
        .copied()
    else {
        return Ok(row);
    };
    let refs = prompt_reference_ids(row.reference_image_ids_json.as_ref())?;
    if refs.is_empty() || refs.contains(&latest_previous_image_id) {
        return Ok(row);
    }
    let Some(last_ref_id) = refs.last().copied() else {
        return Ok(row);
    };
    if existing_image_ids.contains(&last_ref_id) {
        return Ok(row);
    }
    let mut seen = HashSet::new();
    let mut updated_refs = Vec::with_capacity(refs.len());
    for id in refs {
        let id = if id == last_ref_id {
            latest_previous_image_id
        } else {
            id
        };
        if seen.insert(id) {
            updated_refs.push(id);
        }
    }
    let mut metadata_json = row.metadata_json.clone();
    if let Some(metadata) = metadata_json.as_mut()
        && let Some(boundary_frame) = metadata.get_mut("boundary_frame")
        && let Some(object) = boundary_frame.as_object_mut()
    {
        object.insert(
            "reference_image_ids".to_string(),
            json!(updated_refs.clone()),
        );
    }
    a4n_db::video::update_chapter_video_prompt_record(
        &service.database,
        row,
        UpdateChapterVideoPromptRecordInput {
            reference_image_ids_json: Some(Some(json!(updated_refs))),
            metadata_json,
            ..Default::default()
        },
    )
    .await
    .map_err(crate::video::video_db_error)
}

pub(super) fn prompt_reference_ids(value: Option<&Value>) -> Result<Vec<i64>, AppError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("reference_image_ids_json 必须是数组".to_string())
    })?;
    let mut ids = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let id = item.as_i64().ok_or_else(|| {
            AppError::InvalidArgument(format!("reference_image_ids_json[{index}] 必须是整数"))
        })?;
        if id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "reference_image_ids_json[{index}] 必须为正整数"
            )));
        }
        ids.push(id);
    }
    Ok(ids)
}

pub(in crate::video) fn reusable_frame_prefix_len_with_previous_images(
    frames: &[VideoFramePrompt],
    image_id_by_frame_number: &HashMap<i32, i64>,
) -> usize {
    for (index, frame) in frames.iter().enumerate() {
        if frame.frame_number <= 1 || image_id_by_frame_number.contains_key(&frame.frame_number) {
            continue;
        }
        let Some(previous_image_id) = image_id_by_frame_number.get(&(frame.frame_number - 1))
        else {
            continue;
        };
        if !frame.reference_image_ids.contains(previous_image_id) {
            return index;
        }
    }
    frames.len()
}

pub(super) async fn ensure_boundary_frame_checkpoint_can_sync(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    kept_prompt_ids: &BTreeSet<i64>,
) -> Result<(), AppError> {
    let rows = a4n_db::video::chapter_video_prompt_workflow_records(
        &service.database,
        project_id,
        chapter_id,
        prompt_type,
        workflow_mode.as_str(),
        false,
    )
    .await
    .map_err(AppError::database)?;
    let mut prompts_to_validate = Vec::new();
    for row in rows {
        if row.status == chapter_video_prompt::STATUS_RUNNING {
            service
                .ensure_prompt_stages_mutable_skipping_generated_images(
                    &prompts_to_validate,
                    "删除",
                )
                .await?;
            return Err(AppError::InvalidArgument(format!(
                "当前章节已有边界帧提示词 #{} 正在运行，不能继续生成边界帧提示词",
                row.shot_number
            )));
        }
        if !kept_prompt_ids.contains(&row.id) {
            prompts_to_validate.push(row);
        }
    }
    service
        .ensure_prompt_stages_mutable_skipping_generated_images(&prompts_to_validate, "删除")
        .await?;
    Ok(())
}
