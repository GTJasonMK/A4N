use crate::manga::{MangaPagePromptSyncDraft, MangaPipelineDbError};
use a4n_core::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use sea_orm::{ActiveModelTrait, Set};

pub(super) async fn persist_manga_prompt_draft<C>(
    existing: Option<chapter_manga_prompt::Model>,
    draft: MangaPagePromptSyncDraft,
    now: chrono::NaiveDateTime,
    db: &C,
) -> Result<chapter_manga_prompt::Model, MangaPipelineDbError>
where
    C: sea_orm::ConnectionTrait,
{
    match existing {
        Some(existing) => update_manga_prompt_from_draft(existing, draft, now, db).await,
        None => insert_manga_prompt_from_draft(draft, now, db).await,
    }
}

fn manga_prompt_content_matches(
    existing: &chapter_manga_prompt::Model,
    draft: &MangaPagePromptSyncDraft,
) -> bool {
    existing.chapter_id == draft.chapter_id
        && existing.project_id == draft.project_id
        && existing.page_number == draft.page_number
        && existing.image_prompt == draft.image_prompt
        && existing.dialogue_text == draft.dialogue_text
        && existing.characters_in_frame == draft.characters_in_frame
        && existing.layout == draft.layout
}

async fn update_manga_prompt_from_draft<C>(
    existing: chapter_manga_prompt::Model,
    draft: MangaPagePromptSyncDraft,
    now: chrono::NaiveDateTime,
    db: &C,
) -> Result<chapter_manga_prompt::Model, MangaPipelineDbError>
where
    C: sea_orm::ConnectionTrait,
{
    let content_matches = manga_prompt_content_matches(&existing, &draft);
    let keep_rendered_image = content_matches && existing.generated_image_id.is_some();
    if content_matches && (keep_rendered_image || existing.status == draft.status) {
        return Ok(existing);
    }
    let mut active: chapter_manga_prompt::ActiveModel = existing.into();
    active.chapter_id = Set(draft.chapter_id);
    active.project_id = Set(draft.project_id);
    active.page_number = Set(draft.page_number);
    active.image_prompt = Set(draft.image_prompt);
    active.dialogue_text = Set(draft.dialogue_text);
    active.characters_in_frame = Set(draft.characters_in_frame);
    active.layout = Set(draft.layout);
    if !keep_rendered_image {
        active.generated_image_id = Set(None);
        active.status = Set(draft.status);
    }
    active.updated_at = Set(now);
    Ok(active.update(db).await?)
}

async fn insert_manga_prompt_from_draft<C>(
    draft: MangaPagePromptSyncDraft,
    now: chrono::NaiveDateTime,
    db: &C,
) -> Result<chapter_manga_prompt::Model, MangaPipelineDbError>
where
    C: sea_orm::ConnectionTrait,
{
    Ok(chapter_manga_prompt::ActiveModel {
        chapter_id: Set(draft.chapter_id),
        project_id: Set(draft.project_id),
        page_number: Set(draft.page_number),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.to_string()),
        image_prompt: Set(draft.image_prompt),
        dialogue_text: Set(draft.dialogue_text),
        characters_in_frame: Set(draft.characters_in_frame),
        layout: Set(draft.layout),
        generated_image_id: Set(None),
        status: Set(draft.status),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?)
}
