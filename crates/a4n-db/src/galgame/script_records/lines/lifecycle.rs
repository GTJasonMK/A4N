use super::ordering::compact_galgame_line_indexes_record;
use crate::galgame::{
    GALGAME_LINE_TYPE_CHOICE, GalgameLineRecordError, UpdateGalgameLineRecordInput,
    delete_galgame_line_generation_media_graph_record,
    delete_galgame_line_visual_generation_media_graph_record,
};
use a4n_db_models::models::galgame::{galgame_choice, galgame_line};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait,
};

pub async fn delete_galgame_line_with_generation_media_graph_record(
    db: &DatabaseConnection,
    project_id: i64,
    line_id: i64,
) -> Result<(), GalgameLineRecordError> {
    let tx = db.begin().await?;
    delete_galgame_line_generation_media_graph_record(&tx, project_id, line_id).await?;
    delete_galgame_line_record(&tx, line_id).await?;
    tx.commit().await.map_err(GalgameLineRecordError::Database)
}

async fn delete_galgame_line_record<C>(db: &C, line_id: i64) -> Result<(), GalgameLineRecordError>
where
    C: ConnectionTrait,
{
    let line = galgame_line::Entity::find_by_id(line_id)
        .one(db)
        .await?
        .ok_or_else(|| GalgameLineRecordError::NotFound(format!("Galgame 行不存在: {line_id}")))?;
    galgame_choice::Entity::delete_many()
        .filter(galgame_choice::Column::LineId.eq(line.id))
        .exec(db)
        .await?;
    galgame_line::Entity::delete_by_id(line.id).exec(db).await?;
    compact_galgame_line_indexes_record(db, line.scene_id).await
}

pub async fn update_galgame_line_with_generation_cleanup_record(
    db: &DatabaseConnection,
    project_id: i64,
    input: UpdateGalgameLineRecordInput,
    cleanup_visual_generation: bool,
) -> Result<galgame_line::Model, GalgameLineRecordError> {
    let line_id = input.line_id;
    let tx = db.begin().await?;
    if cleanup_visual_generation {
        delete_galgame_line_visual_generation_media_graph_record(&tx, project_id, line_id).await?;
    }
    let updated = update_galgame_line_record(&tx, input).await?;
    tx.commit().await?;
    Ok(updated)
}

async fn update_galgame_line_record<C>(
    db: &C,
    input: UpdateGalgameLineRecordInput,
) -> Result<galgame_line::Model, GalgameLineRecordError>
where
    C: ConnectionTrait,
{
    let line = galgame_line::Entity::find_by_id(input.line_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            GalgameLineRecordError::NotFound(format!("Galgame 行不存在: {}", input.line_id))
        })?;
    let old_line_type = line.line_type.clone();
    let mut active: galgame_line::ActiveModel = line.into();
    if let Some(line_type) = input.line_type {
        active.line_type = Set(line_type);
    }
    if let Some(speaker_profile_id) = input.speaker_profile_id {
        active.speaker_profile_id = Set(speaker_profile_id);
    }
    if let Some(text) = input.text {
        active.text = Set(text);
    }
    if let Some(expression) = input.expression {
        active.expression = Set(expression);
    }
    if let Some(pose) = input.pose {
        active.pose = Set(pose);
    }
    if let Some(position) = input.position {
        active.position = Set(position);
    }
    if let Some(presentation_json) = input.presentation_json {
        active.presentation_json = Set(presentation_json);
    }
    if let Some(text_anchor_id) = input.text_anchor_id {
        active.text_anchor_id = Set(text_anchor_id);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active
        .update(db)
        .await
        .map_err(GalgameLineRecordError::Database)?;
    if old_line_type == GALGAME_LINE_TYPE_CHOICE && updated.line_type != GALGAME_LINE_TYPE_CHOICE {
        galgame_choice::Entity::delete_many()
            .filter(galgame_choice::Column::LineId.eq(updated.id))
            .exec(db)
            .await?;
    }
    Ok(updated)
}
