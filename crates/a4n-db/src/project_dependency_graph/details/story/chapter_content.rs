use crate::project_dependency_graph::AssetLineageRecord;
use crate::project_dependency_graph::details::records::{
    db_record_result, lineage_ref, optional_prompt_sources, prompt_source,
};
use a4n_db_models::models::chapter::chapter;
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(in crate::project_dependency_graph) async fn chapter_content_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(chapters
        .into_iter()
        .map(|chapter| AssetLineageRecord {
            id: format!("chapter:{}", chapter.id),
            label: chapter
                .title
                .clone()
                .unwrap_or_else(|| format!("第 {} 章正文", chapter.chapter_number)),
            asset_kind: "chapter_content".to_string(),
            asset_id: Some(chapter.id),
            status: Some(chapter.status.clone()),
            origin: if chapter.status == chapter::STATUS_IMPORTED {
                "imported"
            } else {
                "llm_generated"
            }
            .to_string(),
            source_refs: vec![lineage_ref(
                "chapter_outlines",
                "章节大纲",
                "text",
                "chapter_outline",
                chapter.outline_id,
            )],
            prompt_sources: optional_prompt_sources(vec![prompt_source(
                &format!("chapter_content:{}", chapter.id),
                "chapter_content",
                Some(&chapter.content),
                Some(chapter.id),
                None,
            )]),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_inputs: Vec::new(),
            result: db_record_result("chapter", &chapter),
            warnings: Vec::new(),
        })
        .collect())
}
