use crate::generation::delete_generation_run_targets_for;
use crate::manga::{
    MangaPipelineDbError, PIPELINE_STATUS_PROMPT_TYPE, SetMangaPipelineStatusInput,
};
use a4n_core::models::chapter::chapter_manga_prompt;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::{Value, json};

pub async fn manga_pipeline_status_row(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<Option<chapter_manga_prompt::Model>, MangaPipelineDbError> {
    Ok(chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(PIPELINE_STATUS_PROMPT_TYPE))
        .order_by_desc(chapter_manga_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_manga_prompt::Column::Id)
        .one(db)
        .await?)
}

/// 写入/更新漫画流水线状态记录；已取消且未允许覆盖时返回 Cancelled。
pub async fn set_manga_pipeline_status(
    db: &DatabaseConnection,
    input: SetMangaPipelineStatusInput,
) -> Result<(), MangaPipelineDbError> {
    let now = Utc::now().naive_utc();
    let existing_rows = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(input.project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(input.chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(PIPELINE_STATUS_PROMPT_TYPE))
        .order_by_desc(chapter_manga_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_manga_prompt::Column::Id)
        .all(db)
        .await?;

    if !input.allow_cancel_override
        && let Some(existing) = existing_rows.first()
    {
        let existing_status = layout_pipeline_status(existing.layout.as_ref())?;
        if existing_status == "cancelled" || existing.status == "cancelled" {
            return Err(MangaPipelineDbError::Cancelled);
        }
    }

    let can_cancel = input.pipeline_status == "running";
    let can_resume = matches!(input.pipeline_status.as_str(), "failed" | "cancelled");
    let prompt_status = match input.pipeline_status.as_str() {
        "running" => "generating".to_string(),
        other => other.to_string(),
    };
    let existing_layout = existing_rows.first().and_then(|row| row.layout.as_ref());
    let mut layout_map = serde_json::Map::new();
    layout_map.insert("pipeline_status".into(), json!(input.pipeline_status));
    layout_map.insert("stage".into(), json!(input.stage));
    layout_map.insert("current".into(), json!(input.current));
    layout_map.insert("total".into(), json!(input.total));
    layout_map.insert("message".into(), json!(input.message));
    layout_map.insert("task_id".into(), json!(input.task_id));
    layout_map.insert("can_cancel".into(), json!(can_cancel));
    layout_map.insert("can_resume".into(), json!(can_resume));
    layout_map.insert("updated_at".into(), json!(now.to_string()));
    if let Some(existing_layout) = existing_layout {
        for key in [
            "checkpoint_data",
            "source",
            "reference_assets",
            "prompt_fingerprint",
            "checkpoint_updated_at",
        ] {
            if let Some(value) = existing_layout.get(key) {
                layout_map.insert(key.into(), value.clone());
            }
        }
        if input.task_id.is_none()
            && let Some(value) = existing_layout.get("task_id")
        {
            layout_map.insert("task_id".into(), value.clone());
        }
    }
    let layout = Some(Value::Object(layout_map));

    if let Some(existing) = existing_rows.first() {
        let keep_id = existing.id;
        let mut active: chapter_manga_prompt::ActiveModel = existing.clone().into();
        active.project_id = Set(input.project_id);
        active.page_number = Set(0);
        active.panel_number = Set(0);
        active.image_prompt = Set(input.message.clone());
        active.dialogue_text = Set(None);
        active.characters_in_frame = Set(None);
        active.layout = Set(layout);
        active.generated_image_id = Set(None);
        active.camera_angle = Set(None);
        active.dialogue_position = Set(None);
        active.status = Set(prompt_status);
        active.updated_at = Set(now);
        active.update(db).await?;

        let duplicate_ids = existing_rows
            .iter()
            .skip(1)
            .map(|row| row.id)
            .collect::<Vec<_>>();
        if !duplicate_ids.is_empty() {
            delete_generation_run_targets_for(db, "chapter_manga_prompt", &duplicate_ids).await?;
            chapter_manga_prompt::Entity::delete_many()
                .filter(chapter_manga_prompt::Column::Id.is_in(duplicate_ids))
                .filter(chapter_manga_prompt::Column::Id.ne(keep_id))
                .exec(db)
                .await?;
        }
        return Ok(());
    }

    chapter_manga_prompt::ActiveModel {
        chapter_id: Set(input.chapter_id),
        project_id: Set(input.project_id),
        page_number: Set(0),
        panel_number: Set(0),
        prompt_type: Set(PIPELINE_STATUS_PROMPT_TYPE.to_string()),
        image_prompt: Set(input.message),
        dialogue_text: Set(None),
        characters_in_frame: Set(None),
        layout: Set(layout),
        generated_image_id: Set(None),
        camera_angle: Set(None),
        dialogue_position: Set(None),
        status: Set(prompt_status),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

fn layout_pipeline_status(layout: Option<&Value>) -> Result<&str, MangaPipelineDbError> {
    let layout = layout.and_then(Value::as_object).ok_or_else(|| {
        MangaPipelineDbError::InvalidArgument("漫画流水线状态.layout 必须是 JSON 对象".to_string())
    })?;
    layout
        .get("pipeline_status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|status| !status.is_empty())
        .ok_or_else(|| {
            MangaPipelineDbError::InvalidArgument(
                "漫画流水线状态.pipeline_status 必须是非空字符串".to_string(),
            )
        })
}
