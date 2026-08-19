mod contracts;
mod errors;
mod rules;

pub use contracts::{MangaPagePromptSyncDraft, MangaPromptUpdatedAt};
pub use errors::{MangaPipelineDbError, MangaPromptRecordError};

pub(in crate::manga) use errors::manga_record_database;
pub(in crate::manga) use rules::{
    require_manga_text, require_positive_manga_id, validate_manga_page_number,
    validate_manga_page_prompt_layout,
};
