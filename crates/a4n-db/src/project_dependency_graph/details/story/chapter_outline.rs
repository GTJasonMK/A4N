use crate::project_dependency_graph::{
    AssetLineageRecord, db_record_result, lineage_ref, optional_prompt_sources, prompt_source,
};
use a4n_core::{error::AppError, models::chapter::chapter_outline};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(in crate::project_dependency_graph) async fn chapter_outline_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(outlines
        .into_iter()
        .map(|outline| {
            let source_refs = if let Some(part_id) = outline.part_id {
                vec![lineage_ref(
                    &format!("part_outline:{part_id}"),
                    "分部大纲",
                    "text",
                    "part_outline",
                    Some(part_id),
                )]
            } else {
                vec![lineage_ref(
                    "blueprint",
                    "故事蓝图",
                    "text",
                    "novel_blueprint",
                    None,
                )]
            };
            AssetLineageRecord {
                id: format!("chapter_outline:{}", outline.id),
                label: outline
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("第 {} 章大纲", outline.chapter_number)),
                asset_kind: "chapter_outline".to_string(),
                asset_id: Some(outline.id),
                status: Some(outline.status.clone()),
                origin: "llm_generated".to_string(),
                source_refs,
                prompt_sources: optional_prompt_sources(vec![prompt_source(
                    &format!("chapter_outline_summary:{}", outline.id),
                    "chapter_outline_summary",
                    outline.summary.as_deref(),
                    Some(outline.id),
                    None,
                )]),
                media_sources: Vec::new(),
                request_snapshot: None,
                input_status: if outline
                    .summary
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    "complete"
                } else {
                    "missing"
                }
                .to_string(),
                generation_inputs: Vec::new(),
                result: db_record_result("chapter_outline", &outline),
                warnings: Vec::new(),
            }
        })
        .collect())
}
