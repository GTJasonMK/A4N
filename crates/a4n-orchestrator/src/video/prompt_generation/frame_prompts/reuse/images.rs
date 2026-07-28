use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::Value;

use a4n_core::source_spans::{SourceSpan, source_spans_for_content};

use crate::video::{ChapterVideoSourceContext, OrchestratorContext};

pub(in crate::video::prompt_generation::frame_prompts) fn image_generation_prompt_id(
    metadata: Option<&Value>,
) -> Option<i64> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("prompt_id"))
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
}

pub(in crate::video::prompt_generation::frame_prompts) fn image_generation_source(
    metadata: Option<&Value>,
) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::video::prompt_generation::frame_prompts) fn image_generation_shot_number(
    metadata: Option<&Value>,
) -> Option<i32> {
    let raw = metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("shot_number"))?;
    raw.as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
}

pub(in crate::video::prompt_generation::frame_prompts) fn image_generation_source_hash(
    metadata: Option<&Value>,
) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source_content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::video::prompt_generation::frame_prompts) fn prompt_source_hash(
    metadata: Option<&Value>,
) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::video::prompt_generation::frame_prompts) async fn source_spans_for_video_prompt(
    service: &OrchestratorContext,
    chapter_id: i64,
    source: &ChapterVideoSourceContext<'_>,
) -> Result<Vec<SourceSpan>, AppError> {
    let chapter = service.get_chapter(chapter_id).await?;
    Ok(source_spans_for_content(
        chapter.id,
        chapter.chapter_number,
        source.content,
    ))
}

pub(in crate::video) async fn boundary_frame_image_ids_by_number(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    frame_rows: &[chapter_video_prompt::Model],
) -> Result<HashMap<i32, i64>, AppError> {
    let prompt_id_by_frame_number = frame_rows
        .iter()
        .map(|row| (row.id, row.shot_number))
        .collect::<HashMap<_, _>>();
    let source_hash_by_frame_number = frame_rows
        .iter()
        .filter_map(|row| {
            prompt_source_hash(row.metadata_json.as_ref()).map(|hash| (row.shot_number, hash))
        })
        .collect::<HashMap<_, _>>();
    if prompt_id_by_frame_number.is_empty() {
        return Ok(HashMap::new());
    }

    let allowed_project_ids = service.video_asset_project_ids_app(project_id).await?;
    let images = a4n_db::video::generated_images_for_projects_chapter_by_id_desc(
        service.database.connection(),
        &allowed_project_ids,
        chapter_id,
    )
    .await
    .map_err(AppError::Database)?;

    let mut image_id_by_frame_number = HashMap::new();
    for image in images {
        let metadata = image.metadata_json.as_ref();
        match image_generation_source(metadata) {
            Some("video_boundary_frame_prompt") => {
                let Some(prompt_id) = image_generation_prompt_id(metadata) else {
                    continue;
                };
                let Some(frame_number) = prompt_id_by_frame_number.get(&prompt_id).copied() else {
                    continue;
                };
                image_id_by_frame_number
                    .entry(frame_number)
                    .or_insert(image.id);
            }
            Some("video_tail_frame_capture") => {
                let Some(frame_number) = image_generation_shot_number(metadata)
                    .and_then(|shot_number| shot_number.checked_add(1))
                else {
                    continue;
                };
                let Some(expected_hash) = source_hash_by_frame_number.get(&frame_number) else {
                    continue;
                };
                if image_generation_source_hash(metadata) != Some(*expected_hash) {
                    continue;
                }
                image_id_by_frame_number
                    .entry(frame_number)
                    .or_insert(image.id);
            }
            _ => {}
        }
    }
    Ok(image_id_by_frame_number)
}

pub(in crate::video::prompt_generation::frame_prompts) async fn ensure_no_orphan_boundary_frame_images(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    prompt_type: &str,
) -> Result<(), AppError> {
    let prompt_ids = a4n_db::video::chapter_video_prompt_workflow_records(
        service.database.connection(),
        project_id,
        chapter_id,
        prompt_type,
        workflow_mode,
        false,
    )
    .await
    .map_err(AppError::Database)?
    .into_iter()
    .map(|row| row.id)
    .collect::<HashSet<_>>();
    let images = a4n_db::video::generated_images_for_projects_chapter_by_id_desc(
        service.database.connection(),
        &[project_id],
        chapter_id,
    )
    .await
    .map_err(AppError::Database)?;
    if let Some((image_id, prompt_id)) = images.into_iter().find_map(|image| {
        let metadata = image.metadata_json.as_ref();
        (image_generation_source(metadata) == Some("video_boundary_frame_prompt"))
            .then(|| image_generation_prompt_id(metadata))
            .flatten()
            .filter(|prompt_id| !prompt_ids.contains(prompt_id))
            .map(|prompt_id| (image.id, prompt_id))
    }) {
        return Err(AppError::InvalidArgument(format!(
            "边界帧图片 {image_id} 仍引用已删除的提示词 {prompt_id}，不能重新生成边界帧提示词；请先删除该孤立边界帧图片"
        )));
    }
    Ok(())
}

pub(in crate::video) async fn storyboard_frame_image_ids_by_number(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    frame_rows: &[chapter_video_prompt::Model],
) -> Result<HashMap<i32, i64>, AppError> {
    let prompt_id_by_frame_number = frame_rows
        .iter()
        .map(|row| (row.id, row.shot_number))
        .collect::<HashMap<_, _>>();
    if prompt_id_by_frame_number.is_empty() {
        return Ok(HashMap::new());
    }

    let allowed_project_ids = service.video_asset_project_ids_app(project_id).await?;
    let images = a4n_db::video::generated_images_for_projects_chapter_by_id_desc(
        service.database.connection(),
        &allowed_project_ids,
        chapter_id,
    )
    .await
    .map_err(AppError::Database)?;

    let mut image_id_by_frame_number = HashMap::new();
    for image in images {
        let metadata = image.metadata_json.as_ref();
        if image_generation_source(metadata) != Some("video_storyboard_prompt") {
            continue;
        }
        let Some(prompt_id) = image_generation_prompt_id(metadata) else {
            continue;
        };
        let Some(frame_number) = prompt_id_by_frame_number.get(&prompt_id).copied() else {
            continue;
        };
        image_id_by_frame_number
            .entry(frame_number)
            .or_insert(image.id);
    }
    Ok(image_id_by_frame_number)
}
