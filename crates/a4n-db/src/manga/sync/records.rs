use crate::manga::{MANGA_PAGE_PROMPT_TYPE, MangaPipelineDbError, MangaPromptSyncDraft};
use a4n_core::models::chapter::chapter_manga_prompt;
use sea_orm::{ActiveModelTrait, Set};

pub(super) async fn persist_manga_prompt_draft<C>(
    existing: Option<chapter_manga_prompt::Model>,
    draft: MangaPromptSyncDraft,
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
    draft: &MangaPromptSyncDraft,
) -> bool {
    existing.chapter_id == draft.chapter_id
        && existing.project_id == draft.project_id
        && existing.page_number == draft.page_number
        && existing.panel_number == draft.panel_number
        && existing.prompt_type == draft.prompt_type
        && existing.image_prompt == draft.image_prompt
        && existing.dialogue_text == draft.dialogue_text
        && existing.characters_in_frame == draft.characters_in_frame
        && existing.layout == draft.layout
        && existing.camera_angle == draft.camera_angle
        && existing.dialogue_position == draft.dialogue_position
}

async fn update_manga_prompt_from_draft<C>(
    existing: chapter_manga_prompt::Model,
    draft: MangaPromptSyncDraft,
    now: chrono::NaiveDateTime,
    db: &C,
) -> Result<chapter_manga_prompt::Model, MangaPipelineDbError>
where
    C: sea_orm::ConnectionTrait,
{
    let content_matches = manga_prompt_content_matches(&existing, &draft);
    let keep_rendered_image = content_matches
        && existing.prompt_type == MANGA_PAGE_PROMPT_TYPE
        && existing.generated_image_id.is_some();
    if content_matches && (keep_rendered_image || existing.status == draft.status) {
        return Ok(existing);
    }
    let mut active: chapter_manga_prompt::ActiveModel = existing.into();
    active.chapter_id = Set(draft.chapter_id);
    active.project_id = Set(draft.project_id);
    active.page_number = Set(draft.page_number);
    active.panel_number = Set(draft.panel_number);
    active.prompt_type = Set(draft.prompt_type);
    active.image_prompt = Set(draft.image_prompt);
    active.dialogue_text = Set(draft.dialogue_text);
    active.characters_in_frame = Set(draft.characters_in_frame);
    active.layout = Set(draft.layout);
    if !keep_rendered_image {
        active.generated_image_id = Set(None);
        active.status = Set(draft.status);
    }
    active.camera_angle = Set(draft.camera_angle);
    active.dialogue_position = Set(draft.dialogue_position);
    active.updated_at = Set(now);
    Ok(active.update(db).await?)
}

async fn insert_manga_prompt_from_draft<C>(
    draft: MangaPromptSyncDraft,
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
        panel_number: Set(draft.panel_number),
        prompt_type: Set(draft.prompt_type),
        image_prompt: Set(draft.image_prompt),
        dialogue_text: Set(draft.dialogue_text),
        characters_in_frame: Set(draft.characters_in_frame),
        layout: Set(draft.layout),
        generated_image_id: Set(None),
        camera_angle: Set(draft.camera_angle),
        dialogue_position: Set(draft.dialogue_position),
        status: Set(draft.status),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?)
}
