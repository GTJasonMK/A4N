mod contracts;
mod errors;
mod rules;

pub use contracts::*;
pub use errors::{MangaPipelineDbError, MangaPromptRecordError};
pub use rules::{MANGA_PAGE_PROMPT_TYPE, MANGA_PANEL_PLAN_TYPE, PIPELINE_STATUS_PROMPT_TYPE};

pub(in crate::manga) use errors::manga_record_database;
pub(in crate::manga) use rules::{
    require_manga_text, require_positive_manga_id, validate_manga_page_position,
    validate_manga_prompt_identity,
};
