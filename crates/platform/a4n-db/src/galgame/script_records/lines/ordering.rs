use crate::galgame::{GalgameLineRecordError, GalgameLineReorderRecordError};
use a4n_db_models::models::galgame::galgame_line;
use a4n_domain::galgame::GalgameLineRecord;
use chrono::Utc;
use sea_orm::ConnectionTrait;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

pub async fn reorder_galgame_line_records_transaction(
    db: &impl crate::DbExecutor,
    scene_id: i64,
    line_ids: Vec<i64>,
) -> Result<Vec<GalgameLineRecord>, GalgameLineReorderRecordError> {
    let tx = db.begin().await?;
    let records = reorder_galgame_line_records(&tx, scene_id, line_ids).await?;
    tx.commit().await?;
    Ok(records
        .into_iter()
        .map(super::super::mapping::line_record)
        .collect())
}

async fn reorder_galgame_line_records<C>(
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
    let mut line_index_case = CaseStatement::new();
    for (index, line_id) in line_ids.iter().enumerate() {
        let line_index = usize_to_i32(index + 1, "line_index")
            .map_err(|error| GalgameLineReorderRecordError::InvalidArgument(error.to_string()))?;
        line_index_case = line_index_case.case(
            galgame_line::Column::Id.eq(*line_id),
            Expr::value(line_index),
        );
    }
    let updated = galgame_line::Entity::update_many()
        .col_expr(
            galgame_line::Column::LineIndex,
            line_index_case
                .finally(Expr::col(galgame_line::Column::LineIndex))
                .into(),
        )
        .col_expr(galgame_line::Column::UpdatedAt, Expr::value(now))
        .filter(galgame_line::Column::SceneId.eq(scene_id))
        .filter(galgame_line::Column::Id.is_in(line_ids.clone()))
        .exec(db)
        .await?;
    if updated.rows_affected != line_ids.len() as u64 {
        return Err(GalgameLineReorderRecordError::InvalidArgument(format!(
            "Galgame 行重排批量更新影响 {} 行，预期 {} 行",
            updated.rows_affected,
            line_ids.len()
        )));
    }

    galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.eq(scene_id))
        .order_by_asc(galgame_line::Column::LineIndex)
        .all(db)
        .await
        .map_err(GalgameLineReorderRecordError::Database)
}

pub(super) async fn compact_galgame_line_indexes_record<C>(
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
    let mut line_index_case = CaseStatement::new();
    let mut changed_ids = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let next = usize_to_i32(index + 1, "line_index")
            .map_err(|error| GalgameLineRecordError::InvalidArgument(error.to_string()))?;
        if line.line_index == next {
            continue;
        }
        line_index_case =
            line_index_case.case(galgame_line::Column::Id.eq(line.id), Expr::value(next));
        changed_ids.push(line.id);
    }
    if !changed_ids.is_empty() {
        let updated = galgame_line::Entity::update_many()
            .col_expr(
                galgame_line::Column::LineIndex,
                line_index_case
                    .finally(Expr::col(galgame_line::Column::LineIndex))
                    .into(),
            )
            .col_expr(galgame_line::Column::UpdatedAt, Expr::value(now))
            .filter(galgame_line::Column::SceneId.eq(scene_id))
            .filter(galgame_line::Column::Id.is_in(changed_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != changed_ids.len() as u64 {
            return Err(GalgameLineRecordError::InvalidArgument(format!(
                "Galgame 行索引压缩批量更新影响 {} 行，预期 {} 行",
                updated.rows_affected,
                changed_ids.len()
            )));
        }
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
