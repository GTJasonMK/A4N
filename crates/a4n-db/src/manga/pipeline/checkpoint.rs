use super::{manga_pipeline_status_row, set_manga_pipeline_status};
use crate::manga::{MangaPipelineDbError, SaveMangaPipelineCheckpointInput};
use a4n_core::models::chapter::chapter_manga_prompt;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde_json::{Value, json};

/// 清除流水线 checkpoint 相关 layout 字段。
pub async fn clear_manga_pipeline_checkpoint(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), MangaPipelineDbError> {
    let Some(status_row) = manga_pipeline_status_row(db, project_id, chapter_id).await? else {
        return Ok(());
    };
    let Some(Value::Object(mut layout)) = status_row.layout.clone() else {
        return Err(MangaPipelineDbError::InvalidArgument(
            "漫画流水线状态.layout 必须是 JSON 对象".to_string(),
        ));
    };
    let mut changed = false;
    for key in [
        "checkpoint_data",
        "source",
        "reference_assets",
        "prompt_fingerprint",
        "checkpoint_updated_at",
    ] {
        changed |= layout.remove(key).is_some();
    }
    if !changed {
        return Ok(());
    }
    let mut active: chapter_manga_prompt::ActiveModel = status_row.into();
    active.layout = Set(Some(Value::Object(layout)));
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await?;
    Ok(())
}

/// 先写入状态再写入 checkpoint 元数据。
pub async fn save_manga_pipeline_checkpoint(
    db: &DatabaseConnection,
    input: SaveMangaPipelineCheckpointInput,
) -> Result<(), MangaPipelineDbError> {
    set_manga_pipeline_status(db, input.status.clone()).await?;
    let status_row =
        manga_pipeline_status_row(db, input.status.project_id, input.status.chapter_id)
            .await?
            .ok_or_else(|| {
                MangaPipelineDbError::InvalidArgument(
                    "漫画流水线状态记录写入后不可读取".to_string(),
                )
            })?;
    let mut layout = match status_row.layout.clone() {
        Some(Value::Object(map)) => map,
        _ => {
            return Err(MangaPipelineDbError::InvalidArgument(
                "漫画流水线状态.layout 必须是 JSON 对象".to_string(),
            ));
        }
    };
    layout.insert("source".into(), input.source_metadata);
    layout.insert("reference_assets".into(), input.reference_assets_metadata);
    layout.insert("prompt_fingerprint".into(), input.prompt_fingerprint);
    layout.insert("checkpoint_data".into(), input.checkpoint_data);
    layout.insert(
        "checkpoint_updated_at".into(),
        json!(Utc::now().to_rfc3339()),
    );
    let mut active: chapter_manga_prompt::ActiveModel = status_row.into();
    active.layout = Set(Some(Value::Object(layout)));
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await?;
    Ok(())
}
