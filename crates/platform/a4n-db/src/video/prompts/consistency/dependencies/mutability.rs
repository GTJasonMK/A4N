use super::{
    frame_images::{generated_frame_images_for_prompt, generated_frame_images_for_prompts},
    rules::{
        PromptDependencyFields, prompt_same_generation_chain, prompt_stage, prompt_stage_label,
    },
};
use crate::video::{self, VideoDbError, prompts::consistency::PromptStage};
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::image::image_generation_prompt_id;
use a4n_domain::source_spans::metadata_source_content_sha256;
use sea_orm::ConnectionTrait;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PromptChainIdentityKey {
    style: Option<String>,
    aspect_ratio: Option<String>,
    source_content_sha256: Option<String>,
    contract_epoch: Option<Value>,
    identity_source_content_sha256: Option<Value>,
    style_sha256: Option<Value>,
    duration_secs: Option<Value>,
    identity_aspect_ratio: Option<Value>,
    source_shot_id: Value,
    segment_index_in_shot: Value,
    segment_sha256: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FrameVideoChainKey {
    workflow_mode: Option<String>,
    segment_plan_prompt_id: i64,
    identity: PromptChainIdentityKey,
}

#[derive(Debug, Default)]
pub(super) struct PromptDependencyIndex {
    downstream_by_plan: HashMap<i64, Vec<usize>>,
    video_by_frame_key: HashMap<FrameVideoChainKey, Vec<usize>>,
}

impl PromptDependencyIndex {
    pub(super) fn build(rows: &[crate::video::PromptDependencyRecord]) -> Self {
        let mut index = Self::default();
        for (row_index, row) in rows.iter().enumerate() {
            let Some((segment_plan_prompt_id, identity)) = prompt_chain_identity_key(row) else {
                continue;
            };
            if prompt_stage(row).is_some() {
                index
                    .downstream_by_plan
                    .entry(segment_plan_prompt_id)
                    .or_default()
                    .push(row_index);
            }
            if prompt_stage(row) == Some(PromptStage::Video) {
                index
                    .video_by_frame_key
                    .entry(FrameVideoChainKey {
                        workflow_mode: prompt_workflow_mode_key(row),
                        segment_plan_prompt_id,
                        identity,
                    })
                    .or_default()
                    .push(row_index);
            }
        }
        index
    }
}

fn prompt_chain_identity_key(
    row: &impl PromptDependencyFields,
) -> Option<(i64, PromptChainIdentityKey)> {
    let identity = row
        .metadata_json()?
        .get("video_execution_identity")?
        .as_object()?;
    let segment_plan_prompt_id = identity.get("segment_plan_prompt_id")?.as_i64()?;
    let identity = PromptChainIdentityKey {
        style: row.style().map(str::to_owned),
        aspect_ratio: row.aspect_ratio().map(str::to_owned),
        source_content_sha256: metadata_source_content_sha256(row.metadata_json())
            .map(str::to_owned),
        contract_epoch: identity.get("contract_epoch").cloned(),
        identity_source_content_sha256: identity.get("source_content_sha256").cloned(),
        style_sha256: identity.get("style_sha256").cloned(),
        duration_secs: identity.get("duration_secs").cloned(),
        identity_aspect_ratio: identity.get("aspect_ratio").cloned(),
        source_shot_id: identity.get("source_shot_id")?.clone(),
        segment_index_in_shot: identity.get("segment_index_in_shot")?.clone(),
        segment_sha256: identity.get("segment_sha256")?.clone(),
    };
    Some((segment_plan_prompt_id, identity))
}

fn prompt_workflow_mode_key(row: &impl PromptDependencyFields) -> Option<String> {
    match row.workflow_mode() {
        mode @ (chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES
        | chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN
        | chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO
        | chapter_video_prompt::WORKFLOW_MODE_STORYBOARD) => Some(mode.to_owned()),
        _ => None,
    }
}

fn frame_video_chain_key(row: &impl PromptDependencyFields) -> Option<FrameVideoChainKey> {
    let (segment_plan_prompt_id, identity) = prompt_chain_identity_key(row)?;
    Some(FrameVideoChainKey {
        workflow_mode: prompt_workflow_mode_key(row),
        segment_plan_prompt_id,
        identity,
    })
}

pub async fn ensure_chapter_video_prompt_stage_mutable<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
    operation: &str,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    ensure_chapter_video_prompt_stage_mutable_ignoring(db, prompt, operation, &BTreeSet::new())
        .await
}

pub async fn ensure_chapter_video_prompt_stage_mutable_ignoring<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
    operation: &str,
    ignored_prompt_ids: &BTreeSet<i64>,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let Some(stage) = validate_chapter_video_prompt_stage_mutability(db, prompt, operation).await?
    else {
        return Ok(());
    };

    let rows = video::chapter_video_prompt_sibling_dependency_records(
        db,
        prompt.project_id,
        prompt.chapter_id,
        prompt.id,
    )
    .await
    .map_err(VideoDbError::Database)?;
    ensure_no_later_prompt_dependency(prompt, operation, ignored_prompt_ids, stage, rows.iter())
}

pub async fn ensure_chapter_video_prompt_stages_mutable_ignoring<C>(
    db: &C,
    prompts: &[chapter_video_prompt::Model],
    operation: &str,
    ignored_prompt_ids: &BTreeSet<i64>,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    ensure_chapter_video_prompt_stages_mutable_with_options(
        db,
        prompts,
        operation,
        ignored_prompt_ids,
        false,
    )
    .await
}

pub async fn ensure_chapter_video_prompt_stages_mutable_skipping_generated_images<C>(
    db: &C,
    prompts: &[chapter_video_prompt::Model],
    operation: &str,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    ensure_chapter_video_prompt_stages_mutable_with_options(
        db,
        prompts,
        operation,
        &BTreeSet::new(),
        true,
    )
    .await
}

async fn ensure_chapter_video_prompt_stages_mutable_with_options<C>(
    db: &C,
    prompts: &[chapter_video_prompt::Model],
    operation: &str,
    ignored_prompt_ids: &BTreeSet<i64>,
    skip_generated_images: bool,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let Some(first) = prompts.first() else {
        return Ok(());
    };
    if prompts.iter().any(|prompt| {
        prompt.project_id != first.project_id || prompt.chapter_id != first.chapter_id
    }) {
        return Err(VideoDbError::InvalidArgument(
            "批量视频提示词可变性校验必须属于同一项目章节".to_string(),
        ));
    }

    let mut staged = Vec::with_capacity(prompts.len());
    for prompt in prompts {
        if let Some(stage) = prompt_stage(prompt) {
            staged.push((prompt, stage));
        }
    }
    if staged.is_empty() {
        return Ok(());
    }
    let frame_prompts = staged
        .iter()
        .filter_map(|(prompt, stage)| (*stage == PromptStage::FramePrompt).then_some(*prompt))
        .collect::<Vec<_>>();
    let generated_images = if frame_prompts.is_empty() {
        Vec::new()
    } else {
        generated_frame_images_for_prompts(db, first.project_id, first.chapter_id, &frame_prompts)
            .await?
    };
    let mut generated_image_by_prompt = HashMap::new();
    for image in generated_images {
        if let Some(prompt_id) = image_generation_prompt_id(image.metadata_json.as_ref()) {
            generated_image_by_prompt.entry(prompt_id).or_insert(image);
        }
    }
    let rows =
        video::chapter_video_prompt_dependency_records(db, first.project_id, first.chapter_id)
            .await
            .map_err(VideoDbError::Database)?;
    let dependency_index = PromptDependencyIndex::build(&rows);
    for (prompt, stage) in staged {
        ensure_prompt_stage_mutability_preconditions(prompt, operation)?;
        if stage == PromptStage::FramePrompt {
            if skip_generated_images && generated_image_by_prompt.contains_key(&prompt.id) {
                continue;
            }
            ensure_no_generated_frame_image(
                prompt,
                operation,
                generated_image_by_prompt.get(&prompt.id),
            )?;
        }
        ensure_no_later_prompt_dependency_indexed(
            prompt,
            operation,
            ignored_prompt_ids,
            stage,
            &dependency_index,
            &rows,
        )?;
    }
    Ok(())
}

pub async fn ensure_chapter_video_prompt_stages_mutable<C>(
    db: &C,
    prompts: &[chapter_video_prompt::Model],
    operation: &str,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    ensure_chapter_video_prompt_stages_mutable_ignoring(db, prompts, operation, &BTreeSet::new())
        .await
}

pub(super) async fn validate_chapter_video_prompt_stage_mutability<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
    operation: &str,
) -> Result<Option<PromptStage>, VideoDbError>
where
    C: ConnectionTrait,
{
    let Some(stage) = ensure_prompt_stage_mutability_preconditions(prompt, operation)? else {
        return Ok(None);
    };
    if stage == PromptStage::FramePrompt {
        ensure_frame_prompt_has_no_generated_image(db, prompt, operation).await?;
    }
    Ok(Some(stage))
}

pub(super) fn ensure_prompt_dependencies_mutable_with_index(
    prompt: &chapter_video_prompt::Model,
    operation: &str,
    ignored_prompt_ids: &BTreeSet<i64>,
    stage: PromptStage,
    sibling_rows: &[crate::video::PromptDependencyRecord],
    index: &PromptDependencyIndex,
) -> Result<(), VideoDbError> {
    ensure_no_later_prompt_dependency_indexed(
        prompt,
        operation,
        ignored_prompt_ids,
        stage,
        index,
        sibling_rows,
    )
}

pub(super) fn ensure_prompt_stage_mutability_preconditions(
    prompt: &chapter_video_prompt::Model,
    operation: &str,
) -> Result<Option<PromptStage>, VideoDbError> {
    let Some(stage) = prompt_stage(prompt) else {
        return Ok(None);
    };
    if stage == PromptStage::Video && prompt.generated_video_id.is_some() {
        return Err(VideoDbError::InvalidArgument(format!(
            "已存在由该视频提示词生成的视频，不能{operation}视频提示词；请先删除对应视频"
        )));
    }
    Ok(Some(stage))
}

fn ensure_no_later_prompt_dependency<'a>(
    prompt: &chapter_video_prompt::Model,
    operation: &str,
    ignored_prompt_ids: &BTreeSet<i64>,
    stage: PromptStage,
    rows: impl IntoIterator<Item = &'a crate::video::PromptDependencyRecord>,
) -> Result<(), VideoDbError> {
    if stage == PromptStage::Video {
        return Ok(());
    }
    if let Some(later) = rows
        .into_iter()
        .filter_map(|row| {
            if ignored_prompt_ids.contains(&row.id) || !prompt_same_generation_chain(prompt, row) {
                return None;
            }
            prompt_stage(row)
                .filter(|later_stage| *later_stage > stage)
                .map(|later_stage| (later_stage, row))
        })
        .min_by_key(|(later_stage, _)| *later_stage)
        .map(|(_, row)| row)
    {
        let current_label = prompt_stage_label(prompt);
        let later_label = prompt_stage_label(later);
        return Err(VideoDbError::InvalidArgument(format!(
            "已存在后续{later_label}，不能{operation}{current_label}；请先删除后续{later_label}"
        )));
    }
    Ok(())
}

fn ensure_no_later_prompt_dependency_indexed(
    prompt: &chapter_video_prompt::Model,
    operation: &str,
    ignored_prompt_ids: &BTreeSet<i64>,
    stage: PromptStage,
    index: &PromptDependencyIndex,
    rows: &[crate::video::PromptDependencyRecord],
) -> Result<(), VideoDbError> {
    match stage {
        PromptStage::SegmentPlan => {
            let Some(candidates) = index.downstream_by_plan.get(&prompt.id) else {
                return Ok(());
            };
            ensure_no_later_prompt_dependency(
                prompt,
                operation,
                ignored_prompt_ids,
                stage,
                candidates
                    .iter()
                    .filter_map(|row_index| rows.get(*row_index))
                    .filter(|row| row.id != prompt.id),
            )
        }
        PromptStage::FramePrompt => {
            let Some(key) = frame_video_chain_key(prompt) else {
                return Ok(());
            };
            let Some(candidates) = index.video_by_frame_key.get(&key) else {
                return Ok(());
            };
            ensure_no_later_prompt_dependency(
                prompt,
                operation,
                ignored_prompt_ids,
                stage,
                candidates
                    .iter()
                    .filter_map(|row_index| rows.get(*row_index))
                    .filter(|row| row.id != prompt.id),
            )
        }
        PromptStage::Video => Ok(()),
    }
}

pub(super) async fn ensure_frame_prompt_has_no_generated_image<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
    operation: &str,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    ensure_no_generated_frame_image(
        prompt,
        operation,
        generated_frame_images_for_prompt(db, prompt)
            .await?
            .into_iter()
            .next()
            .as_ref(),
    )
}

pub(super) fn ensure_no_generated_frame_image(
    prompt: &chapter_video_prompt::Model,
    operation: &str,
    image: Option<&a4n_db_models::models::ai::generated_image::Reference>,
) -> Result<(), VideoDbError> {
    let Some(image) = image else {
        return Ok(());
    };
    let current_label = prompt_stage_label(prompt);
    let image_label = match prompt.prompt_type.as_str() {
        chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME => "首帧图片",
        chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME => "Chat 首尾帧图片",
        chapter_video_prompt::PROMPT_TYPE_STORYBOARD => "分镜板图片",
        _ => "边界帧图片",
    };
    Err(VideoDbError::InvalidArgument(format!(
        "已存在由该{current_label}生成的{image_label} #{}，不能{operation}{current_label}；请先删除对应图片",
        image.id
    )))
}
