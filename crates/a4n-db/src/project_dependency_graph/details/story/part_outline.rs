use crate::project_dependency_graph::{
    AssetLineageRecord, db_record_result, lineage_ref, optional_prompt_sources, prompt_source,
};
use a4n_core::{error::AppError, models::chapter::part_outline};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(in crate::project_dependency_graph) async fn part_outline_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(parts
        .into_iter()
        .map(|part| AssetLineageRecord {
            id: format!("part_outline:{}", part.id),
            label: format!("第 {} 部：{}", part.part_number, part.title),
            asset_kind: "part_outline".to_string(),
            asset_id: Some(part.id),
            status: None,
            origin: "llm_generated".to_string(),
            source_refs: vec![lineage_ref(
                "blueprint",
                "故事蓝图",
                "text",
                "novel_blueprint",
                None,
            )],
            prompt_sources: optional_prompt_sources(vec![prompt_source(
                &format!("part_outline_summary:{}", part.id),
                "part_outline_summary",
                part.summary.as_deref(),
                Some(part.id),
                None,
            )]),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_inputs: Vec::new(),
            result: db_record_result("part_outline", &part),
            warnings: Vec::new(),
        })
        .collect())
}
