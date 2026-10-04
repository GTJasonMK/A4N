use crate::galgame::{GalgameLineRecordError, InsertGalgameLineRecordInput};
use a4n_db_models::models::galgame::galgame_line;
use a4n_domain::galgame::GalgameLineRecord;
use chrono::Utc;
use sea_orm::ConnectionTrait;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

pub async fn insert_galgame_line_record_transaction(
    db: &impl crate::DbExecutor,
    input: InsertGalgameLineRecordInput,
) -> Result<GalgameLineRecord, GalgameLineRecordError> {
    let tx = db.begin().await?;
    let record = insert_galgame_line_record(&tx, input).await?;
    tx.commit().await?;
    Ok(super::super::mapping::line_record(record))
}

async fn insert_galgame_line_record<C>(
    db: &C,
    input: InsertGalgameLineRecordInput,
) -> Result<galgame_line::Model, GalgameLineRecordError>
where
    C: ConnectionTrait,
{
    let insert_index = if let Some(after_line_id) = input.after_line_id {
        let after = galgame_line::Entity::find_by_id(after_line_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                GalgameLineRecordError::NotFound(format!("Galgame 行不存在: {after_line_id}"))
            })?;
        if after.scene_id != input.scene_id {
            return Err(GalgameLineRecordError::InvalidArgument(
                "after_line_id 不属于目标场".to_string(),
            ));
        }
        after.line_index + 1
    } else {
        1
    };

    galgame_line::Entity::update_many()
        .col_expr(
            galgame_line::Column::LineIndex,
            Expr::col(galgame_line::Column::LineIndex).add(1),
        )
        .filter(galgame_line::Column::SceneId.eq(input.scene_id))
        .filter(galgame_line::Column::LineIndex.gte(insert_index))
        .exec(db)
        .await?;

    let now = Utc::now().naive_utc();
    galgame_line::ActiveModel {
        scene_id: Set(input.scene_id),
        line_index: Set(insert_index),
        line_type: Set(input.line_type),
        speaker_profile_id: Set(input.speaker_profile_id),
        text: Set(input.text),
        expression: Set(input.expression),
        pose: Set(input.pose),
        position: Set(input.position),
        presentation_json: Set(input.presentation_json),
        text_anchor_id: Set(input.text_anchor_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(GalgameLineRecordError::Database)
}
