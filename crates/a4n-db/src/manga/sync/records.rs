use crate::manga::MangaPagePromptSyncDraft;
use a4n_core::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use sea_orm::Set;

pub(super) fn manga_prompt_active_model(
    draft: MangaPagePromptSyncDraft,
    now: chrono::NaiveDateTime,
) -> chapter_manga_prompt::ActiveModel {
    chapter_manga_prompt::ActiveModel {
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

pub(super) fn manga_prompt_updated_model(
    existing: &chapter_manga_prompt::Model,
    draft: MangaPagePromptSyncDraft,
    now: chrono::NaiveDateTime,
) -> Option<chapter_manga_prompt::Model> {
    let content_matches = manga_prompt_content_matches(existing, &draft);
    let keep_rendered_image = content_matches && existing.generated_image_id.is_some();
    if content_matches && (keep_rendered_image || existing.status == draft.status) {
        return None;
    }
    let mut updated = existing.clone();
    updated.chapter_id = draft.chapter_id;
    updated.project_id = draft.project_id;
    updated.page_number = draft.page_number;
    updated.image_prompt = draft.image_prompt;
    updated.dialogue_text = draft.dialogue_text;
    updated.characters_in_frame = draft.characters_in_frame;
    updated.layout = draft.layout;
    if !keep_rendered_image {
        updated.generated_image_id = None;
        updated.status = draft.status;
    }
    updated.updated_at =
        std::cmp::max(now, existing.updated_at + chrono::Duration::microseconds(1));
    Some(updated)
}
