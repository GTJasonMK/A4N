use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use sea_orm::{ColumnTrait, DatabaseConnection, QueryFilter, TransactionError, TransactionTrait};
use serde_json::{Map, Value};

use super::metadata::validate_video_metadata_update_preserves_system_fields;
use crate::video::{
    UpdateGeneratedVideoRecordInput, VideoDbError, normalize_optional_video_record_text,
    required_video_record_text,
};

pub async fn toggle_generated_video_favorite_record<C>(
    db: &C,
    video_id: i64,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let video = generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    let mut active: generated_video::ActiveModel = video.clone().into();
    active.is_favorite = Set(!video.is_favorite);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(VideoDbError::Database)
}

pub async fn update_generated_video_record<C>(
    db: &C,
    video_id: i64,
    input: UpdateGeneratedVideoRecordInput,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let video = generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    if video.status == "running" {
        return Err(VideoDbError::InvalidArgument(
            "视频生成中，不能编辑视频记录".to_string(),
        ));
    }
    validate_video_metadata_update_preserves_system_fields(
        video.metadata_json.as_ref(),
        input.metadata_json.as_ref(),
        input.clear_metadata_json,
    )?;
    let mut active: generated_video::ActiveModel = video.into();
    if let Some(prompt) = input.prompt {
        active.prompt = Set(required_video_record_text(&prompt, "prompt")?);
    }
    if input.clear_negative_prompt {
        active.negative_prompt = Set(None);
    } else if let Some(negative_prompt) = input.negative_prompt {
        active.negative_prompt = Set(normalize_optional_video_record_text(Some(
            negative_prompt.as_str(),
        )));
    }
    if let Some(is_favorite) = input.is_favorite {
        active.is_favorite = Set(is_favorite);
    }
    if input.clear_metadata_json {
        active.metadata_json = Set(None);
    } else if input.metadata_json.is_some() {
        active.metadata_json = Set(input.metadata_json);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(VideoDbError::Database)
}

pub async fn select_segment_video_record(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    shot_number: i32,
    video_id: i64,
) -> Result<generated_video::Model, VideoDbError> {
    if project_id <= 0 || chapter_id <= 0 || shot_number <= 0 || video_id <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "project_id、chapter_id、shot_number 和 video_id 必须为正数".to_string(),
        ));
    }
    db.transaction::<_, generated_video::Model, VideoDbError>(|txn| {
        Box::pin(async move {
            let rows = generated_video::Entity::find()
                .filter(generated_video::Column::ProjectId.eq(project_id))
                .filter(generated_video::Column::ChapterId.eq(chapter_id))
                .all(txn)
                .await?;
            let selected = rows
                .iter()
                .find(|row| row.id == video_id)
                .ok_or_else(|| VideoDbError::NotFound("视频不存在或不属于当前章节".to_string()))?;
            if selected.status != "completed"
                || selected
                    .file_path
                    .as_deref()
                    .map(str::trim)
                    .is_none_or(str::is_empty)
            {
                return Err(VideoDbError::InvalidArgument(
                    "只能采用已完成且文件存在的视频".to_string(),
                ));
            }
            if metadata_shot_number(selected.metadata_json.as_ref()) != Some(shot_number) {
                return Err(VideoDbError::InvalidArgument(format!(
                    "视频 #{} 不属于片段 {}",
                    video_id, shot_number
                )));
            }

            let now = Utc::now().naive_utc();
            let mut result = None;
            for row in rows
                .into_iter()
                .filter(|row| metadata_shot_number(row.metadata_json.as_ref()) == Some(shot_number))
            {
                let should_select = row.id == video_id;
                if metadata_selected_for_segment(row.metadata_json.as_ref()) == should_select {
                    if should_select {
                        result = Some(row);
                    }
                    continue;
                }
                let mut metadata = match row.metadata_json.clone() {
                    Some(Value::Object(value)) => value,
                    _ => Map::new(),
                };
                metadata.insert(
                    "selected_for_segment".to_string(),
                    Value::Bool(should_select),
                );
                if should_select {
                    metadata.insert(
                        "selected_for_segment_at".to_string(),
                        Value::String(Utc::now().to_rfc3339()),
                    );
                } else {
                    metadata.remove("selected_for_segment_at");
                }
                let mut active: generated_video::ActiveModel = row.into();
                active.metadata_json = Set(Some(Value::Object(metadata)));
                active.updated_at = Set(now);
                let updated = active.update(txn).await?;
                if should_select {
                    result = Some(updated);
                }
            }
            let prompts = chapter_video_prompt::Entity::find()
                .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
                .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
                .all(txn)
                .await?;
            for prompt in prompts {
                let source = prompt
                    .metadata_json
                    .as_ref()
                    .and_then(|metadata| metadata.get("continuity_source"));
                let source_video_id = source
                    .and_then(|source| source.get("video_id"))
                    .and_then(Value::as_i64);
                let source_shot_number = source
                    .and_then(|source| source.get("shot_number"))
                    .and_then(Value::as_i64);
                if source_shot_number != Some(i64::from(shot_number))
                    || source_video_id.is_none_or(|source_id| source_id == video_id)
                {
                    continue;
                }
                let mut metadata = match prompt.metadata_json.clone() {
                    Some(Value::Object(value)) => value,
                    _ => Map::new(),
                };
                metadata.insert("continuity_stale".to_string(), Value::Bool(true));
                metadata.insert(
                    "continuity_replacement_video_id".to_string(),
                    Value::from(video_id),
                );
                let mut active: chapter_video_prompt::ActiveModel = prompt.into();
                active.metadata_json = Set(Some(Value::Object(metadata)));
                active.updated_at = Set(now);
                active.update(txn).await?;
            }
            result.ok_or_else(|| VideoDbError::NotFound("采用的视频不存在".to_string()))
        })
    })
    .await
    .map_err(|error| match error {
        TransactionError::Connection(error) => VideoDbError::Database(error),
        TransactionError::Transaction(error) => error,
    })
}

fn metadata_shot_number(metadata: Option<&Value>) -> Option<i32> {
    metadata
        .and_then(|value| value.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
}

fn metadata_selected_for_segment(metadata: Option<&Value>) -> bool {
    metadata
        .and_then(|value| value.get("selected_for_segment"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}
