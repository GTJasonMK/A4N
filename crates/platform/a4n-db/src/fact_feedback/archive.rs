use a4n_db_models::models::{
    character::{
        attribute_event, character_state_index, foreshadowing_index, protagonist_behavior_record,
        protagonist_deletion_mark, protagonist_profile, snapshot_reference,
    },
    project::{material_fact_change, project_fact_feedback},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    sea_query::Expr,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactFeedbackArchive {
    pub settings: Option<project_fact_feedback::Model>,
    pub changes: Vec<material_fact_change::Model>,
    pub foreshadowing: Vec<foreshadowing_index::Model>,
    pub attribute_events: Vec<attribute_event::Model>,
    pub snapshots: Vec<snapshot_reference::Model>,
    pub state_indexes: Vec<character_state_index::Model>,
    pub behaviors: Vec<protagonist_behavior_record::Model>,
    pub deletion_marks: Vec<protagonist_deletion_mark::Model>,
}

pub async fn load_feedback_archive<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<FactFeedbackArchive, DbErr> {
    let ids = protagonist_profile::Entity::find()
        .select_only()
        .column(protagonist_profile::Column::Id)
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let (
        settings,
        changes,
        foreshadowing,
        attribute_events,
        snapshots,
        state_indexes,
        behaviors,
        deletion_marks,
    ) = futures::try_join!(
        project_fact_feedback::Entity::find_by_id(project_id).one(db),
        material_fact_change::Entity::find()
            .filter(material_fact_change::Column::ProjectId.eq(project_id))
            .order_by_asc(material_fact_change::Column::Id)
            .all(db),
        foreshadowing_index::Entity::find()
            .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
            .order_by_asc(foreshadowing_index::Column::Id)
            .all(db),
        attribute_event::Entity::find()
            .filter(attribute_event::Column::ProfileId.is_in(ids.iter().copied()))
            .order_by_asc(attribute_event::Column::Id)
            .all(db),
        snapshot_reference::Entity::find()
            .filter(snapshot_reference::Column::ProfileId.is_in(ids.iter().copied()))
            .order_by_asc(snapshot_reference::Column::Id)
            .all(db),
        character_state_index::Entity::find()
            .filter(character_state_index::Column::ProfileId.is_in(ids.iter().copied()))
            .order_by_asc(character_state_index::Column::Id)
            .all(db),
        protagonist_behavior_record::Entity::find()
            .filter(protagonist_behavior_record::Column::ProfileId.is_in(ids.iter().copied()))
            .order_by_asc(protagonist_behavior_record::Column::Id)
            .all(db),
        protagonist_deletion_mark::Entity::find()
            .filter(protagonist_deletion_mark::Column::ProfileId.is_in(ids.iter().copied()))
            .order_by_asc(protagonist_deletion_mark::Column::Id)
            .all(db),
    )?;
    Ok(FactFeedbackArchive {
        settings,
        changes,
        foreshadowing,
        attribute_events,
        snapshots,
        state_indexes,
        behaviors,
        deletion_marks,
    })
}

pub async fn reset_feedback_coverage<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<(), DbErr> {
    project_fact_feedback::Entity::update_many()
        .filter(project_fact_feedback::Column::ProjectId.eq(project_id))
        .col_expr(
            project_fact_feedback::Column::ConsolidatedSourcesJson,
            Expr::value(json!([])),
        )
        .col_expr(
            project_fact_feedback::Column::LastConsolidatedAt,
            Expr::value(Option::<chrono::NaiveDateTime>::None),
        )
        .exec(db)
        .await?;
    material_fact_change::Entity::update_many()
        .filter(material_fact_change::Column::ProjectId.eq(project_id))
        .col_expr(
            material_fact_change::Column::Status,
            Expr::value("retracted"),
        )
        .col_expr(
            material_fact_change::Column::InvalidatedAt,
            Expr::value(chrono::Utc::now().naive_utc()),
        )
        .exec(db)
        .await?;
    Ok(())
}
