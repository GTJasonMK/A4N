use a4n_core::models::chapter::chapter_manga_prompt;
use std::collections::HashMap;

type MangaPromptIdentity = (String, i32, i32);

pub(super) fn select_manga_prompt_rows(
    existing: Vec<chapter_manga_prompt::Model>,
) -> (
    HashMap<MangaPromptIdentity, chapter_manga_prompt::Model>,
    Vec<i64>,
) {
    let mut existing_by_identity = HashMap::new();
    let mut duplicate_ids = Vec::new();
    for row in existing {
        let identity = (row.prompt_type.clone(), row.page_number, row.panel_number);
        if let Some(current) = existing_by_identity.get(&identity) {
            if should_prefer_existing_manga_prompt(&row, current) {
                duplicate_ids.push(current.id);
                existing_by_identity.insert(identity, row);
            } else {
                duplicate_ids.push(row.id);
            }
            continue;
        }
        existing_by_identity.insert(identity, row);
    }
    (existing_by_identity, duplicate_ids)
}

fn should_prefer_existing_manga_prompt(
    candidate: &chapter_manga_prompt::Model,
    current: &chapter_manga_prompt::Model,
) -> bool {
    match (
        candidate.generated_image_id.is_some(),
        current.generated_image_id.is_some(),
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => candidate.updated_at > current.updated_at,
    }
}
