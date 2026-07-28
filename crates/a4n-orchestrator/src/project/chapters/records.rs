use a4n_core::models::chapter::chapter;

pub(crate) fn is_character_sync_available_chapter(chapter: &chapter::Model) -> bool {
    chapter.has_available_content()
}
