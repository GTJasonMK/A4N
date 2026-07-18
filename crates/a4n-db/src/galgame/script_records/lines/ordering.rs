use crate::galgame::{GalgameLineRecordError, GalgameLineReorderRecordError};
use a4n_core::models::galgame::galgame_line;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};

pub async fn reorder_galgame_line_records_transaction(
    db: &DatabaseConnection,
    scene_id: i64,
    line_ids: Vec<i64>,
) -> Result<Vec<galgame_line::Model>, GalgameLineReorderRecordError> {
    let tx = db.begin().await?;
    let records = reorder_galgame_line_records(&tx, scene_id, line_ids).await?;
    tx.commit().await?;
    Ok(records)
}

pub async fn reorder_galgame_line_records<C>(
    db: &C,
    scene_id: i64,
    line_ids: Vec<i64>,
) -> Result<Vec<galgame_line::Model>, GalgameLineReorderRecordError>
where
    C: ConnectionTrait,
{
    if line_ids.is_empty() {
        return Err(GalgameLineReorderRecordError::InvalidArgument(
            "line_ids 不能为空".to_string(),
        ));
    }

    let existing = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.eq(scene_id))
        .order_by_asc(galgame_line::Column::LineIndex)
        .all(db)
        .await?;
    if existing.len() != line_ids.len() {
        return Err(GalgameLineReorderRecordError::InvalidArgument(
            "line_ids 必须包含该场全部行".to_string(),
        ));
    }

    let mut actual = line_ids.clone();
    actual.sort_unstable();
    let mut expected = existing.iter().map(|line| line.id).collect::<Vec<_>>();
    expected.sort_unstable();
    if actual != expected {
        return Err(GalgameLineReorderRecordError::InvalidArgument(
            "line_ids 与该场行集合不匹配".to_string(),
        ));
    }

    let now = Utc::now().naive_utc();
    for (index, line_id) in line_ids.iter().enumerate() {
        let line_index = usize_to_i32(index + 1, "line_index")
            .map_err(|error| GalgameLineReorderRecordError::InvalidArgument(error.to_string()))?;
        galgame_line::Entity::update_many()
            .col_expr(galgame_line::Column::LineIndex, Expr::value(line_index))
            .col_expr(galgame_line::Column::UpdatedAt, Expr::value(now))
            .filter(galgame_line::Column::Id.eq(*line_id))
            .exec(db)
            .await?;
    }

    galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.eq(scene_id))
        .order_by_asc(galgame_line::Column::LineIndex)
        .all(db)
        .await
        .map_err(GalgameLineReorderRecordError::Database)
}

pub async fn compact_galgame_line_indexes_record<C>(
    db: &C,
    scene_id: i64,
) -> Result<(), GalgameLineRecordError>
where
    C: ConnectionTrait,
{
    let lines = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.eq(scene_id))
        .order_by_asc(galgame_line::Column::LineIndex)
        .order_by_asc(galgame_line::Column::Id)
        .all(db)
        .await?;
    let now = Utc::now().naive_utc();
    for (index, line) in lines.iter().enumerate() {
        let next = usize_to_i32(index + 1, "line_index")
            .map_err(|error| GalgameLineRecordError::InvalidArgument(error.to_string()))?;
        if line.line_index == next {
            continue;
        }
        galgame_line::Entity::update_many()
            .col_expr(galgame_line::Column::LineIndex, Expr::value(next))
            .col_expr(galgame_line::Column::UpdatedAt, Expr::value(now))
            .filter(galgame_line::Column::Id.eq(line.id))
            .exec(db)
            .await?;
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("{label} 超出 i32 可表示范围")]
struct GalgameIntegerConversionError {
    label: String,
}

fn usize_to_i32(value: usize, label: &str) -> Result<i32, GalgameIntegerConversionError> {
    i32::try_from(value).map_err(|_| GalgameIntegerConversionError {
        label: label.to_string(),
    })
}
