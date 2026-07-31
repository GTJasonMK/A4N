use std::collections::{HashMap, HashSet};

use serde_json::Value;

use a4n_core::error::AppError;

use crate::video::library::available_video_image_ids;
pub(super) use crate::video::prompt_generation::PreparedVideoPromptInputs as PreparedVideoPromptPreviewInputs;
use crate::video::{ChapterVideoPromptOptions, ChapterVideoSourceContext};
use a4n_prompt::rendering::PromptRenderer;

pub(super) struct LocalPromptPreviewInput<'a, 'source> {
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) source: ChapterVideoSourceContext<'source>,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) variables: &'a HashMap<String, Value>,
    pub(super) options: ChapterVideoPromptOptions,
}

pub(super) fn allowed_reference_ids_for_preview(
    available_assets: &Value,
    reference_image_ids: &[i64],
) -> Result<HashSet<i64>, AppError> {
    available_video_image_ids(available_assets, reference_image_ids)
}
