use a4n_api_contract::manga::MangaPagePromptSourceSummary;
use serde_json::Value;

pub(in crate::manga) struct MangaPipelinePersistInput<'a> {
    pub(in crate::manga) chapter_id: i64,
    pub(in crate::manga) project_id: i64,
    pub(in crate::manga) source_metadata: &'a MangaPagePromptSourceSummary,
    pub(in crate::manga) reference_assets_metadata: &'a Value,
    pub(in crate::manga) prompt_fingerprint: &'a Value,
    pub(in crate::manga) page_plan_ref:
        Option<&'a a4n_domain::media_authoring::AuthoringDocumentRef>,
}
