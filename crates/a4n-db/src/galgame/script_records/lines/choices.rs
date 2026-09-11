use crate::galgame::ReplaceGalgameChoiceRecordInput;
use a4n_db_models::models::galgame::galgame_choice;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};

pub async fn replace_galgame_choice_records_transaction(
    db: &DatabaseConnection,
    line_id: i64,
    choices: Vec<ReplaceGalgameChoiceRecordInput>,
) -> Result<Vec<galgame_choice::Model>, DbErr> {
    let tx = db.begin().await?;
    let records = replace_galgame_choice_records(&tx, line_id, choices).await?;
    tx.commit().await?;
    Ok(records)
}

async fn replace_galgame_choice_records<C>(
    db: &C,
    line_id: i64,
    choices: Vec<ReplaceGalgameChoiceRecordInput>,
) -> Result<Vec<galgame_choice::Model>, DbErr>
where
    C: ConnectionTrait,
{
    galgame_choice::Entity::delete_many()
        .filter(galgame_choice::Column::LineId.eq(line_id))
        .exec(db)
        .await?;

    let now = Utc::now().naive_utc();
    let rows = choices
        .into_iter()
        .map(|choice| galgame_choice::ActiveModel {
            line_id: Set(line_id),
            option_index: Set(choice.option_index),
            option_text: Set(choice.option_text),
            target_scene_id: Set(Some(choice.target_scene_id)),
            created_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    if !rows.is_empty() {
        galgame_choice::Entity::insert_many(rows)
            .exec_without_returning(db)
            .await?;
    }

    galgame_choice::Entity::find()
        .filter(galgame_choice::Column::LineId.eq(line_id))
        .order_by_asc(galgame_choice::Column::OptionIndex)
        .all(db)
        .await
}
