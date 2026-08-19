use crate::project_dependency_graph::AssetLineageRecord;
use crate::project_dependency_graph::details::records::{db_record_result, lineage_ref};
use a4n_core::{error::AppError, models::chapter::chapter};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(in crate::project_dependency_graph) async fn chapter_analysis_detail_records<C>(
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
        .filter(|chapter| {
            chapter
                .analysis_data
                .as_ref()
                .is_some_and(|value| !value.is_null())
        })
        .map(|chapter| AssetLineageRecord {
            id: format!("chapter_analysis:{}", chapter.id),
            label: format!("第 {} 章分析", chapter.chapter_number),
            asset_kind: "chapter_analysis".to_string(),
            asset_id: Some(chapter.id),
            status: None,
            origin: "llm_generated".to_string(),
            source_refs: vec![lineage_ref(
                &format!("chapter:{}", chapter.id),
                "章节正文",
                "text",
                "chapter_content",
                Some(chapter.id),
            )],
            prompt_sources: Vec::new(),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_inputs: Vec::new(),
            result: db_record_result("chapter", &chapter),
            warnings: Vec::new(),
        })
        .collect())
}
