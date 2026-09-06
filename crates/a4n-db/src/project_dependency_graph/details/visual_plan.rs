use a4n_core::{
    error::AppError,
    models::chapter::{chapter_shooting_script, chapter_visual_plan},
};
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde_json::json;

use crate::project_dependency_graph::{
    AssetLineageRecord,
    details::records::{db_record_result, lineage_ref},
};

#[derive(Debug, FromQueryResult)]
struct VisualPlanDetailRow {
    id: i64,
    root_project_id: i64,
    source_chapter_id: i64,
    source_snapshot_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
    source_content_sha256: String,
    contract_version: String,
    plan_version: i32,
    revision: i32,
    status: String,
    authoring_mode: String,
    plan_sha256: String,
    confirmed_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

pub(in crate::project_dependency_graph) async fn visual_plan_detail_records<C>(
    db: &C,
    root_project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let rows = chapter_visual_plan::Entity::find()
        .select_only()
        .column(chapter_visual_plan::Column::Id)
        .column(chapter_visual_plan::Column::RootProjectId)
        .column(chapter_visual_plan::Column::SourceChapterId)
        .column(chapter_visual_plan::Column::SourceSnapshotId)
        .column(chapter_visual_plan::Column::SourceSnapshotChapterId)
        .column(chapter_visual_plan::Column::SourceContentSha256)
        .column(chapter_visual_plan::Column::ContractVersion)
        .column(chapter_visual_plan::Column::PlanVersion)
        .column(chapter_visual_plan::Column::Revision)
        .column(chapter_visual_plan::Column::Status)
        .column(chapter_visual_plan::Column::AuthoringMode)
        .column(chapter_visual_plan::Column::PlanSha256)
        .column(chapter_visual_plan::Column::ConfirmedAt)
        .column(chapter_visual_plan::Column::CreatedAt)
        .column(chapter_visual_plan::Column::UpdatedAt)
        .filter(chapter_visual_plan::Column::RootProjectId.eq(root_project_id))
        .order_by_asc(chapter_visual_plan::Column::SourceChapterId)
        .order_by_asc(chapter_visual_plan::Column::PlanVersion)
        .into_model::<VisualPlanDetailRow>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    rows.into_iter()
        .map(|plan| {
            let origin = authoring_mode_origin(&plan.authoring_mode)?;
            Ok(AssetLineageRecord {
                id: format!("chapter_visual_plan:{}", plan.id),
                label: format!(
                    "章节 {} 导演编排 v{}",
                    plan.source_chapter_id, plan.plan_version
                ),
                asset_kind: "chapter_visual_plan".to_string(),
                asset_id: Some(plan.id),
                status: Some(plan.status.clone()),
                origin: origin.to_string(),
                source_refs: vec![lineage_ref(
                    &format!("chapter:{}", plan.source_chapter_id),
                    "章节正文",
                    "text",
                    "chapter_content",
                    Some(plan.source_chapter_id),
                )],
                prompt_sources: Vec::new(),
                media_sources: Vec::new(),
                request_snapshot: None,
                input_status: "complete".to_string(),
                generation_inputs: Vec::new(),
                result: json!({
                    "table": "chapter_visual_plan",
                    "record": {
                        "id": plan.id,
                        "root_project_id": plan.root_project_id,
                        "source_chapter_id": plan.source_chapter_id,
                        "source_snapshot_id": plan.source_snapshot_id,
                        "source_snapshot_chapter_id": plan.source_snapshot_chapter_id,
                        "source_content_sha256": plan.source_content_sha256,
                        "contract_version": plan.contract_version,
                        "plan_version": plan.plan_version,
                        "revision": plan.revision,
                        "status": plan.status,
                        "authoring_mode": plan.authoring_mode,
                        "plan_sha256": plan.plan_sha256,
                        "confirmed_at": plan.confirmed_at,
                        "created_at": plan.created_at,
                        "updated_at": plan.updated_at,
                    }
                }),
                warnings: Vec::new(),
            })
        })
        .collect()
}

fn authoring_mode_origin(mode: &str) -> Result<&'static str, AppError> {
    match mode {
        "manual" => Ok("user_authored"),
        "collaborative" => Ok("user_ai_collaborative"),
        _ => Err(AppError::InvalidState {
            current: mode.to_string(),
            operation: "构建章节视觉编排依赖详情".to_string(),
        }),
    }
}

pub(in crate::project_dependency_graph) async fn shooting_script_detail_records<C>(
    db: &C,
    root_project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    Ok(chapter_shooting_script::Entity::find()
        .filter(chapter_shooting_script::Column::RootProjectId.eq(root_project_id))
        .order_by_asc(chapter_shooting_script::Column::SourceChapterId)
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|script| {
            let source_refs = script
                .visual_plan_id
                .map(|plan_id| {
                    vec![lineage_ref(
                        &format!("chapter_visual_plan:{plan_id}"),
                        "导演编排",
                        "text",
                        "chapter_visual_plan",
                        Some(plan_id),
                    )]
                })
                .unwrap_or_else(|| {
                    vec![lineage_ref(
                        &format!("chapter:{}", script.source_chapter_id),
                        "章节正文",
                        "text",
                        "chapter_content",
                        Some(script.source_chapter_id),
                    )]
                });
            AssetLineageRecord {
                id: format!("chapter_shooting_script:{}", script.id),
                label: format!("章节 {} 拍摄剧本", script.source_chapter_id),
                asset_kind: "chapter_shooting_script".to_string(),
                asset_id: Some(script.id),
                status: None,
                origin: "llm_generated".to_string(),
                source_refs,
                prompt_sources: Vec::new(),
                media_sources: Vec::new(),
                request_snapshot: None,
                input_status: "complete".to_string(),
                generation_inputs: Vec::new(),
                result: db_record_result("chapter_shooting_script", &script),
                warnings: Vec::new(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::authoring_mode_origin;
    use a4n_core::error::AppError;

    #[test]
    fn authoring_modes_map_to_lineage_origins() {
        assert_eq!(
            authoring_mode_origin("collaborative").unwrap(),
            "user_ai_collaborative"
        );
        assert_eq!(authoring_mode_origin("manual").unwrap(), "user_authored");
        assert!(matches!(
            authoring_mode_origin("future_mode"),
            Err(AppError::InvalidState { .. })
        ));
    }
}
