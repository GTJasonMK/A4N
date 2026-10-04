#[cfg(test)]
use a4n_db_models::models::chapter::chapter;

#[cfg(test)]
pub(crate) fn is_character_sync_available_chapter(chapter: &chapter::Model) -> bool {
    chapter.has_available_content()
}
