use super::records::persist_manga_prompt_draft;
use super::selection::select_manga_prompt_rows;
use super::validation::validate_manga_sync_drafts;
use crate::generation::delete_generation_run_targets_for;
use crate::manga::{MangaPipelineDbError, MangaPromptSyncDraft, PIPELINE_STATUS_PROMPT_TYPE};
use a4n_core::models::chapter::chapter_manga_prompt;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};

pub async fn sync_manga_prompts_by_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    drafts: Vec<MangaPromptSyncDraft>,
    now: chrono::NaiveDateTime,
) -> Result<Vec<chapter_manga_prompt::Model>, MangaPipelineDbError> {
    validate_manga_sync_drafts(&drafts, project_id, chapter_id)?;

    let txn = db.begin().await?;
    let existing = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.ne(PIPELINE_STATUS_PROMPT_TYPE))
        .order_by_asc(chapter_manga_prompt::Column::PromptType)
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .all(&txn)
        .await?;
    let (mut existing_by_identity, duplicate_ids) = select_manga_prompt_rows(existing);
    delete_manga_prompt_rows(&txn, duplicate_ids).await?;

    let mut rows = Vec::with_capacity(drafts.len());
    for draft in drafts {
        let identity = (
            draft.prompt_type.clone(),
            draft.page_number,
            draft.panel_number,
        );
        let row =
            persist_manga_prompt_draft(existing_by_identity.remove(&identity), draft, now, &txn)
                .await?;
        rows.push(row);
    }

    let stale_ids = existing_by_identity
        .into_values()
        .map(|row| row.id)
        .collect();
    delete_manga_prompt_rows(&txn, stale_ids).await?;
    txn.commit().await?;
    Ok(rows)
}

async fn delete_manga_prompt_rows<C>(
    db: &C,
    prompt_ids: Vec<i64>,
) -> Result<(), MangaPipelineDbError>
where
    C: sea_orm::ConnectionTrait,
{
    if prompt_ids.is_empty() {
        return Ok(());
    }
    delete_generation_run_targets_for(db, "chapter_manga_prompt", &prompt_ids).await?;
    chapter_manga_prompt::Entity::delete_many()
        .filter(chapter_manga_prompt::Column::Id.is_in(prompt_ids))
        .exec(db)
        .await?;
    Ok(())
}
