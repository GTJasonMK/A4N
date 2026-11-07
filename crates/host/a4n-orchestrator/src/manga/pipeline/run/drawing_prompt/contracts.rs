use std::collections::HashMap;

use a4n_domain::manga::{
    MangaPageAdaptationV1, MangaPageContentV1, MangaPageDrawingPromptV2, MangaPageReferenceImageV1,
    MangaPageReferenceNeedV1, MangaPageV4, MangaReferenceBindingSummary,
};
use a4n_domain::media::GeneratedImageRecord;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::super::super::prompt::MangaPromptGenerationTrace;
use crate::source_markers::SourceMarkerMap;

pub(in crate::manga::pipeline::run) struct SinglePageDrawingPromptInput<'a> {
    pub(in crate::manga::pipeline::run) chapter_id: i64,
    pub(in crate::manga::pipeline::run) project_id: i64,
    pub(in crate::manga::pipeline::run) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::manga::pipeline::run) vars: &'a HashMap<String, Value>,
    pub(in crate::manga::pipeline::run) workflow_run_id: i64,
    pub(in crate::manga::pipeline::run) reference_assets: &'a Value,
    pub(in crate::manga::pipeline::run) reference_bindings: &'a [MangaReferenceBindingSummary],
    pub(in crate::manga::pipeline::run) reference_requirements: &'a [MangaPageReferenceNeedV1],
    pub(in crate::manga::pipeline::run) system_reference_images: &'a [MangaPageReferenceImageV1],
    pub(in crate::manga::pipeline::run) available_reference_images: &'a [GeneratedImageRecord],
    pub(in crate::manga::pipeline::run) page_content: &'a MangaPageContentV1,
    pub(in crate::manga::pipeline::run) page_adaptation: &'a MangaPageAdaptationV1,
    pub(in crate::manga::pipeline::run) page_plan: &'a MangaPageV4,
    pub(in crate::manga::pipeline::run) page_index: usize,
    pub(in crate::manga::pipeline::run) extraction_summary: &'a Value,
    pub(in crate::manga::pipeline::run) source_markers: &'a SourceMarkerMap,
    pub(in crate::manga::pipeline::run) restore_from_checkpoint: bool,
    pub(in crate::manga::pipeline::run) cancel_token: Option<&'a CancellationToken>,
    pub(in crate::manga::pipeline::run) stream_reporter:
        Option<&'a (dyn Fn(&str, &str) + Send + Sync)>,
}

pub(in crate::manga::pipeline::run) struct SinglePageDrawingPromptOutput {
    pub(in crate::manga::pipeline::run) prompt: MangaPageDrawingPromptV2,
    pub(in crate::manga::pipeline::run) trace: MangaPromptGenerationTrace,
}
