use a4n_db::project::MangaCharacterPortraitRefSummary;

pub(in crate::manga) fn manga_reference_assets_json(
    refs: &[MangaCharacterPortraitRefSummary],
) -> serde_json::Value {
    a4n_manga_core::manga_reference_assets_json(refs)
}
