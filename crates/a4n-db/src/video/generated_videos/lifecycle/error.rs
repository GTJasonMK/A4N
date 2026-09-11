use a4n_db_models::models::{
    ai::generated_video,
    chapter::{chapter, chapter_video_prompt},
    project::novel_project,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, QueryFilter, QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

use super::super::records::{
    validate_generated_video_prompt_link_fields, validate_prompt_project_chapter_identity_fields,
};
use super::context::{
    ensure_generated_video_status_mutable, generated_video_with_prompt_record,
    update_prompt_status_record,
};
use crate::video::VideoDbError;
use crate::video::VideoProjectIdentity;

#[derive(Debug, FromQueryResult)]
struct StaleGeneratedVideoRow {
    id: i64,
    provider_job_id: Option<String>,
    metadata_json: Option<Value>,
}

#[derive(Debug, FromQueryResult)]
struct GeneratedVideoErrorStatusRow {
    id: i64,
    status: String,
    project_id: i64,
    chapter_id: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
struct GeneratedVideoPromptScopeRow {
    id: i64,
    project_id: i64,
    chapter_id: i64,
    generated_video_id: Option<i64>,
}

async fn mark_generated_video_error_status_and_prompt_record_in<C>(
    db: &C,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
    prompt_status: &str,
    message: &str,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let (video, prompt) = generated_video_with_prompt_record(db, video_id, prompt_id).await?;
    if video.status == status {
        return Ok(video);
    }
    ensure_generated_video_status_mutable(&video.status, "终止视频生成")?;
    let mut active: generated_video::ActiveModel = video.into();
    active.status = Set(status.to_string());
    active.error_message = Set(Some(message.to_string()));
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await?;
    if let Some(prompt) = prompt {
        let prompt_status = if prompt.generated_video_id.is_some() {
            chapter_video_prompt::STATUS_COMPLETED
        } else {
            prompt_status
        };
        update_prompt_status_record(db, prompt, prompt_status).await?;
    }
    Ok(updated)
}

pub(super) fn normalize_generated_video_error_message(message: &str) -> Result<&str, VideoDbError> {
    let message = message.trim();
    if message.is_empty() {
        Err(VideoDbError::InvalidArgument(
            "视频失败原因不能为空".to_string(),
        ))
    } else {
        Ok(message)
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedVideoErrorStatusBatchInput {
    pub video_id: i64,
    pub prompt_id: Option<i64>,
    pub status: String,
}

/// 在一个事务内批量终止相互独立的视频记录。
///
/// 每个提示词仍复用原有项目/章节归属校验；只有读取和两类状态写入集合化。
/// 返回本次请求中数据库不存在的视频 ID，调用方决定如何记录该缺失。
pub async fn mark_generated_videos_error_status_and_prompt_records(
    db: &DatabaseConnection,
    inputs: &[GeneratedVideoErrorStatusBatchInput],
    message: &str,
) -> Result<Vec<i64>, VideoDbError> {
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    for input in inputs {
        if !matches!(
            input.status.as_str(),
            a4n_domain::video_workflow::VIDEO_STATUS_FAILED
                | a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED
                | a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED
        ) {
            return Err(VideoDbError::InvalidArgument(format!(
                "视频终止状态无效: {}",
                input.status
            )));
        }
    }
    let message = normalize_generated_video_error_message(message)?;
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let ids = inputs
        .iter()
        .map(|input| input.video_id)
        .collect::<Vec<_>>();
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::Id.is_in(ids))
        .select_only()
        .column(generated_video::Column::Id)
        .column(generated_video::Column::Status)
        .column(generated_video::Column::ProjectId)
        .column(generated_video::Column::ChapterId)
        .lock_exclusive()
        .into_model::<GeneratedVideoErrorStatusRow>()
        .all(&txn)
        .await
        .map_err(VideoDbError::Database)?;
    let videos_by_id = videos
        .into_iter()
        .map(|video| (video.id, video))
        .collect::<HashMap<_, _>>();

    let prompt_ids = inputs
        .iter()
        .filter_map(|input| input.prompt_id)
        .collect::<HashSet<_>>();
    let prompts = if prompt_ids.is_empty() {
        Vec::new()
    } else {
        chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.iter().copied()))
            .select_only()
            .column(chapter_video_prompt::Column::Id)
            .column(chapter_video_prompt::Column::ProjectId)
            .column(chapter_video_prompt::Column::ChapterId)
            .column(chapter_video_prompt::Column::GeneratedVideoId)
            .into_model::<GeneratedVideoPromptScopeRow>()
            .all(&txn)
            .await
            .map_err(VideoDbError::Database)?
    };
    let prompt_by_id = prompts
        .iter()
        .map(|prompt| (prompt.id, prompt))
        .collect::<HashMap<_, _>>();
    let project_ids = prompts
        .iter()
        .map(|prompt| prompt.project_id)
        .collect::<HashSet<_>>();
    let projects = if project_ids.is_empty() {
        Vec::new()
    } else {
        novel_project::Entity::find()
            .filter(novel_project::Column::Id.is_in(project_ids.iter().copied()))
            .select_only()
            .column(novel_project::Column::Id)
            .column(novel_project::Column::ProjectKind)
            .column(novel_project::Column::RootProjectId)
            .into_model::<VideoProjectIdentity>()
            .all(&txn)
            .await
            .map_err(VideoDbError::Database)?
    };
    let project_by_id = projects
        .into_iter()
        .map(|project| (project.id, project))
        .collect::<HashMap<_, _>>();
    let chapter_ids = prompts
        .iter()
        .map(|prompt| prompt.chapter_id)
        .collect::<HashSet<_>>();
    let chapters = if chapter_ids.is_empty() {
        Vec::new()
    } else {
        chapter::Entity::find()
            .filter(chapter::Column::Id.is_in(chapter_ids.iter().copied()))
            .select_only()
            .column(chapter::Column::Id)
            .column(chapter::Column::ProjectId)
            .into_tuple::<(i64, i64)>()
            .all(&txn)
            .await
            .map_err(VideoDbError::Database)?
    };
    let chapter_by_id = chapters.into_iter().collect::<HashMap<_, _>>();

    let mut seen_video_ids = HashSet::new();
    let mut missing_video_ids = Vec::new();
    let mut video_ids_by_status = BTreeMap::<String, Vec<i64>>::new();
    let mut prompt_status_by_id = HashMap::<i64, String>::new();
    for input in inputs {
        if !seen_video_ids.insert(input.video_id) {
            continue;
        }
        let Some(video) = videos_by_id.get(&input.video_id) else {
            missing_video_ids.push(input.video_id);
            continue;
        };
        let prompt = input
            .prompt_id
            .and_then(|prompt_id| prompt_by_id.get(&prompt_id));
        if input.prompt_id.is_some() && prompt.is_none() {
            return Err(VideoDbError::NotFound("视频提示词不存在".to_string()));
        }
        if let Some(prompt) = prompt {
            let project = project_by_id.get(&prompt.project_id).ok_or_else(|| {
                VideoDbError::NotFound(format!("项目不存在: {}", prompt.project_id))
            })?;
            let chapter = chapter_by_id.get(&prompt.chapter_id).ok_or_else(|| {
                VideoDbError::NotFound(format!("章节不存在: {}", prompt.chapter_id))
            })?;
            validate_prompt_project_chapter_identity_fields(
                prompt.chapter_id,
                project.id,
                &project.project_kind,
                project.root_project_id,
                *chapter,
            )?;
            validate_generated_video_prompt_link_fields(
                video.id,
                video.project_id,
                video.chapter_id,
                prompt.project_id,
                prompt.chapter_id,
            )?;
        }
        if video.status == input.status {
            continue;
        }
        ensure_generated_video_status_mutable(&video.status, "终止视频生成")?;
        video_ids_by_status
            .entry(input.status.clone())
            .or_default()
            .push(video.id);
        if let Some(prompt) = prompt {
            let prompt_status = if prompt.generated_video_id.is_some() {
                chapter_video_prompt::STATUS_COMPLETED
            } else {
                match input.status.as_str() {
                    a4n_domain::video_workflow::VIDEO_STATUS_FAILED => {
                        chapter_video_prompt::STATUS_FAILED
                    }
                    a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED => {
                        chapter_video_prompt::STATUS_INTERRUPTED
                    }
                    a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED => {
                        chapter_video_prompt::STATUS_CANCELLED
                    }
                    _ => unreachable!("终止状态已在入口校验"),
                }
            };
            prompt_status_by_id.insert(prompt.id, prompt_status.to_string());
        }
    }
    let mut prompt_ids_by_status = BTreeMap::<String, Vec<i64>>::new();
    for (prompt_id, status) in prompt_status_by_id {
        prompt_ids_by_status
            .entry(status)
            .or_default()
            .push(prompt_id);
    }

    let now = Utc::now().naive_utc();
    for (status, ids) in video_ids_by_status {
        let updated = generated_video::Entity::update_many()
            .col_expr(generated_video::Column::Status, Expr::value(status.clone()))
            .col_expr(
                generated_video::Column::ErrorMessage,
                Expr::value(Some(message.to_string())),
            )
            .col_expr(generated_video::Column::UpdatedAt, Expr::value(now))
            .filter(generated_video::Column::Id.is_in(ids.clone()))
            .exec(&txn)
            .await
            .map_err(VideoDbError::Database)?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(VideoDbError::InvalidState {
                current: format!(
                    "批量终止视频状态更新影响 {} 行，预期 {} 行",
                    updated.rows_affected,
                    ids.len()
                ),
                operation: "批量终止视频生成".to_string(),
            });
        }
    }
    for (status, mut ids) in prompt_ids_by_status {
        ids.sort_unstable();
        ids.dedup();
        let updated = chapter_video_prompt::Entity::update_many()
            .col_expr(chapter_video_prompt::Column::Status, Expr::value(status))
            .col_expr(chapter_video_prompt::Column::UpdatedAt, Expr::value(now))
            .filter(chapter_video_prompt::Column::Id.is_in(ids.clone()))
            .exec(&txn)
            .await
            .map_err(VideoDbError::Database)?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(VideoDbError::InvalidState {
                current: format!(
                    "批量终止视频提示词状态更新影响 {} 行，预期 {} 行",
                    updated.rows_affected,
                    ids.len()
                ),
                operation: "批量终止视频生成".to_string(),
            });
        }
    }
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(missing_video_ids)
}

pub async fn mark_generated_video_error_status_and_prompt_record<C>(
    db: &C,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
    message: &str,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let prompt_status = match status {
        a4n_domain::video_workflow::VIDEO_STATUS_FAILED => chapter_video_prompt::STATUS_FAILED,
        a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED => {
            chapter_video_prompt::STATUS_INTERRUPTED
        }
        a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED => {
            chapter_video_prompt::STATUS_CANCELLED
        }
        _ => {
            return Err(VideoDbError::InvalidArgument(format!(
                "视频终止状态无效: {status}"
            )));
        }
    };
    let message = normalize_generated_video_error_message(message)?;
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let updated = mark_generated_video_error_status_and_prompt_record_in(
        &txn,
        video_id,
        prompt_id,
        status,
        prompt_status,
        message,
    )
    .await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(updated)
}

pub async fn recover_stale_generated_videos_without_provider_job(
    db: &DatabaseConnection,
) -> Result<u64, VideoDbError> {
    let stale = generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::Id)
        .column(generated_video::Column::ProviderJobId)
        .column(generated_video::Column::MetadataJson)
        .filter(generated_video::Column::Status.is_in([
            a4n_domain::video_workflow::VIDEO_STATUS_PENDING,
            a4n_domain::video_workflow::VIDEO_STATUS_RUNNING,
        ]))
        .into_model::<StaleGeneratedVideoRow>()
        .all(db)
        .await?
        .into_iter()
        .filter(|video| {
            video
                .provider_job_id
                .as_deref()
                .map(str::trim)
                .is_none_or(str::is_empty)
        })
        .collect::<Vec<_>>();
    let recovered = u64::try_from(stale.len()).map_err(|_| {
        VideoDbError::InvalidArgument("待恢复视频记录数量超出 u64 范围".to_string())
    })?;
    if stale.is_empty() {
        return Ok(0);
    }

    let message = "应用重启时视频尚未提交 provider，已中断；请重新生成";
    let video_ids = stale.iter().map(|video| video.id).collect::<Vec<_>>();
    crate::generation::interrupt_runs_for_targets(
        db,
        "generated_video",
        &video_ids,
        message.to_string(),
    )
    .await?;

    let inputs = stale
        .into_iter()
        .map(|video| GeneratedVideoErrorStatusBatchInput {
            video_id: video.id,
            prompt_id: a4n_domain::video::generated_video_prompt_id(video.metadata_json.as_ref()),
            status: a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED.to_string(),
        })
        .collect::<Vec<_>>();
    let missing =
        mark_generated_videos_error_status_and_prompt_records(db, &inputs, message).await?;
    if !missing.is_empty() {
        return Err(VideoDbError::NotFound("视频不存在".to_string()));
    }
    Ok(recovered)
}
