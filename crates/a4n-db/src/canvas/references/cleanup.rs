mod freeform;
mod image;
mod scope;
mod video;

use a4n_core::models::canvas::canvas_node;
use sea_orm::sea_query::{Alias, CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde_json::Value;

pub(crate) use freeform::mark_freeform_canvas_generated_refs_missing;
pub(crate) use image::mark_canvas_generated_image_refs_missing;
pub(crate) use video::{
    mark_canvas_generated_video_refs_missing_for_projects,
    mark_canvas_generated_video_refs_missing_for_projects_and_videos,
};

pub(super) struct MissingCanvasNodeUpdate {
    pub(super) id: i64,
    pub(super) ref_type: Option<String>,
    pub(super) ref_id: Option<i64>,
    pub(super) config_json: Value,
}

pub(super) async fn batch_update_missing_canvas_nodes<C>(
    db: &C,
    updates: Vec<MissingCanvasNodeUpdate>,
    now: chrono::NaiveDateTime,
) -> Result<(), crate::canvas::CanvasDbError>
where
    C: ConnectionTrait,
{
    if updates.is_empty() {
        return Ok(());
    }
    let mut ref_type_case = CaseStatement::new();
    let mut ref_id_case = CaseStatement::new();
    let mut config_json_case = CaseStatement::new();
    let mut node_ids = Vec::with_capacity(updates.len());
    for update in updates {
        ref_type_case = ref_type_case.case(
            canvas_node::Column::Id.eq(update.id),
            Expr::value(update.ref_type),
        );
        ref_id_case = ref_id_case.case(
            canvas_node::Column::Id.eq(update.id),
            Expr::value(update.ref_id),
        );
        config_json_case = config_json_case.case(
            canvas_node::Column::Id.eq(update.id),
            Expr::value(update.config_json).cast_as(Alias::new("json")),
        );
        node_ids.push(update.id);
    }
    let updated = canvas_node::Entity::update_many()
        .col_expr(
            canvas_node::Column::RefType,
            ref_type_case
                .finally(Expr::col(canvas_node::Column::RefType))
                .into(),
        )
        .col_expr(
            canvas_node::Column::RefId,
            ref_id_case
                .finally(Expr::col(canvas_node::Column::RefId))
                .into(),
        )
        .col_expr(
            canvas_node::Column::ConfigJson,
            config_json_case
                .finally(Expr::col(canvas_node::Column::ConfigJson))
                .into(),
        )
        .col_expr(canvas_node::Column::State, Expr::value("missing"))
        .col_expr(canvas_node::Column::UpdatedAt, Expr::value(now))
        .filter(canvas_node::Column::Id.is_in(node_ids.clone()))
        .exec(db)
        .await?;
    if updated.rows_affected != node_ids.len() as u64 {
        return Err(crate::canvas::CanvasDbError::InvalidArgument(format!(
            "画布节点缺失状态批量更新数量不一致: updated={}, expected={}",
            updated.rows_affected,
            node_ids.len()
        )));
    }
    Ok(())
}
