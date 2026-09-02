use super::mutability::ensure_frame_prompt_has_no_generated_image;
use super::rules::{
    ensure_frame_prompt_generation_source_matches, ensure_frame_prompt_workflow_matches_type,
    frame_image_generation_source_for_prompt, prompt_same_frame_reference_chain,
    prompt_stage_label,
};
use crate::video::{self, VideoDbError};
use a4n_core::models::{ai::generated_image, chapter::chapter_video_prompt};
use a4n_core::source_spans::metadata_source_content_sha256;
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
    IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE, image_generation_prompt_id,
    image_generation_shot_number, image_generation_source, image_generation_source_hash,
};
use sea_orm::ConnectionTrait;
use std::collections::{HashMap, HashSet};

pub async fn ensure_frame_prompt_image_generation_allowed<C>(
    db: &C,
    prompt_id: i64,
    generation_source: &str,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt = video::frame_chapter_video_prompt_record(db, prompt_id).await?;
    ensure_frame_prompt_generation_source_matches(&prompt, generation_source)?;
    ensure_frame_prompt_workflow_matches_type(&prompt)?;
    if prompt.status == chapter_video_prompt::STATUS_RUNNING {
        return Err(VideoDbError::InvalidState {
            current: prompt.status.clone(),
            operation: format!("生成{}", prompt_stage_label(&prompt)),
        });
    }
    let (project_result, generated_image_result) = futures::join!(
        video::ensure_video_chapter_belongs_to_project_record(
            db,
            prompt.project_id,
            prompt.chapter_id,
        ),
        ensure_frame_prompt_has_no_generated_image(db, &prompt, "成图"),
    );
    project_result?;
    generated_image_result
}

pub async fn effective_frame_prompt_image_reference_ids<C>(
    db: &C,
    prompt_id: i64,
    generation_source: &str,
    requested_reference_image_ids: &[i64],
) -> Result<Vec<i64>, VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt = video::frame_chapter_video_prompt_record(db, prompt_id).await?;
    let expected_source =
        ensure_frame_prompt_generation_source_matches(&prompt, generation_source)?;
    ensure_frame_prompt_workflow_matches_type(&prompt)?;
    if expected_source != IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT
        || prompt.shot_number <= 1
    {
        return Ok(requested_reference_image_ids.to_vec());
    }

    let previous_image = previous_boundary_frame_image_for_prompt(db, &prompt)
        .await?
        .ok_or_else(|| {
            VideoDbError::InvalidArgument(format!(
                "边界帧提示词 #{} 是第 {} 张边界帧，生成前必须先生成第 {} 张边界帧图片，上一帧图片会作为参考图 1 输入",
                prompt.id,
                prompt.shot_number,
                prompt.shot_number - 1
            ))
        })?;
    effective_boundary_frame_reference_image_ids(requested_reference_image_ids, previous_image.id)
}

pub(super) async fn generated_frame_images_for_prompt<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
) -> Result<Vec<generated_image::Reference>, VideoDbError>
where
    C: ConnectionTrait,
{
    generated_frame_images_for_prompts(db, prompt.project_id, prompt.chapter_id, &[prompt]).await
}

pub(crate) async fn generated_frame_images_for_prompts<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompts: &[&chapter_video_prompt::Model],
) -> Result<Vec<generated_image::Reference>, VideoDbError>
where
    C: ConnectionTrait,
{
    let expected_sources = prompts
        .iter()
        .filter_map(|prompt| {
            frame_image_generation_source_for_prompt(prompt).map(|source| (prompt.id, source))
        })
        .collect::<HashMap<_, _>>();
    if expected_sources.is_empty() {
        return Ok(Vec::new());
    }

    let images = video::generated_images_for_chapter_by_id_desc(db, project_id, chapter_id)
        .await
        .map_err(VideoDbError::Database)?;
    Ok(images
        .into_iter()
        .filter(|image| {
            let metadata = image.metadata_json.as_ref();
            let Some(prompt_id) = image_generation_prompt_id(metadata) else {
                return false;
            };
            expected_sources
                .get(&prompt_id)
                .is_some_and(|source| image_generation_source(metadata) == Some(*source))
        })
        .collect())
}

async fn previous_boundary_frame_image_for_prompt<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
) -> Result<Option<generated_image::Reference>, VideoDbError>
where
    C: ConnectionTrait,
{
    let Some(previous_frame_number) = prompt.shot_number.checked_sub(1) else {
        return Ok(None);
    };
    let previous_prompts = video::chapter_video_prompt_previous_frame_candidates(
        db,
        prompt.project_id,
        prompt.chapter_id,
        &prompt.workflow_mode,
        &prompt.prompt_type,
        previous_frame_number,
    )
    .await
    .map_err(VideoDbError::Database)?;
    let Some(previous_prompt) = previous_prompts
        .into_iter()
        .find(|row| prompt_same_frame_reference_chain(prompt, row))
    else {
        return Ok(None);
    };
    let previous_source_hash =
        metadata_source_content_sha256(previous_prompt.metadata_json.as_ref());
    let allowed_project_ids = video::video_asset_project_ids_record(db, prompt.project_id).await?;
    let images = video::generated_images_for_projects_chapter_by_id_desc(
        db,
        &allowed_project_ids,
        prompt.chapter_id,
    )
    .await
    .map_err(VideoDbError::Database)?;
    for image in images {
        let metadata = image.metadata_json.as_ref();
        match image_generation_source(metadata) {
            Some(IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT)
                if image_generation_prompt_id(metadata) == Some(previous_prompt.id) =>
            {
                return Ok(Some(image));
            }
            Some(IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE)
                if image_generation_shot_number(metadata)
                    .and_then(|shot_number| shot_number.checked_add(1))
                    == Some(previous_frame_number)
                    && previous_source_hash.is_some()
                    && image_generation_source_hash(metadata) == previous_source_hash =>
            {
                return Ok(Some(image));
            }
            _ => {}
        }
    }
    Ok(None)
}

fn effective_boundary_frame_reference_image_ids(
    requested_reference_image_ids: &[i64],
    previous_frame_image_id: i64,
) -> Result<Vec<i64>, VideoDbError> {
    if let Some(index) = requested_reference_image_ids
        .iter()
        .position(|id| *id == previous_frame_image_id)
    {
        if index > 0 {
            return Err(VideoDbError::InvalidArgument(format!(
                "上一帧边界帧图片 #{} 必须是 reference_image_ids 第 1 项，才能保证提示词中的“参考图 1”语义正确",
                previous_frame_image_id
            )));
        }
        return Ok(requested_reference_image_ids.to_vec());
    }

    let mut ids = Vec::with_capacity(requested_reference_image_ids.len() + 1);
    let mut seen = HashSet::with_capacity(requested_reference_image_ids.len() + 1);
    ids.push(previous_frame_image_id);
    seen.insert(previous_frame_image_id);
    for id in requested_reference_image_ids {
        if seen.insert(*id) {
            ids.push(*id);
        }
    }
    Ok(ids)
}
