use crate::galgame::{GalgamePipelinePromptChapterRecord, GalgamePipelinePromptContextRecord};
use a4n_core::models::{
    character::protagonist_profile,
    project::{source_snapshot_chapter, text_anchor},
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};
use std::collections::BTreeMap;

pub async fn galgame_pipeline_prompt_context_record<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Result<GalgamePipelinePromptContextRecord, DbErr>
where
    C: ConnectionTrait,
{
    let chapters = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(source_snapshot_id))
        .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
        .all(db)
        .await?;
    if chapters.is_empty() {
        return Ok(GalgamePipelinePromptContextRecord::default());
    }

    let chapter_ids = chapters
        .iter()
        .map(|chapter| chapter.id)
        .collect::<Vec<_>>();
    let mut anchors_by_chapter = text_anchor::Entity::find()
        .filter(text_anchor::Column::ProjectId.eq(root_project_id))
        .filter(text_anchor::Column::SourceSnapshotId.eq(source_snapshot_id))
        .filter(text_anchor::Column::SourceSnapshotChapterId.is_in(chapter_ids))
        .order_by_asc(text_anchor::Column::SourceSnapshotChapterId)
        .order_by_asc(text_anchor::Column::StartOffset)
        .all(db)
        .await?
        .into_iter()
        .fold(
            BTreeMap::<i64, Vec<text_anchor::Model>>::new(),
            |mut acc, anchor| {
                if let Some(chapter_id) = anchor.source_snapshot_chapter_id {
                    acc.entry(chapter_id).or_default().push(anchor);
                }
                acc
            },
        );
    let speaker_profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await?;
    let chapters = chapters
        .into_iter()
        .map(|chapter| GalgamePipelinePromptChapterRecord {
            text_anchors: anchors_by_chapter.remove(&chapter.id).unwrap_or_default(),
            chapter,
        })
        .collect();
    Ok(GalgamePipelinePromptContextRecord {
        chapters,
        speaker_profiles,
    })
}
