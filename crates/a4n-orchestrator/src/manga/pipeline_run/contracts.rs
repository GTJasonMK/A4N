use std::collections::HashMap;

use serde_json::{Map, Value};

use a4n_core::source_spans::SourceSpan;
use a4n_db::shooting_script::ParsedChapterShootingScript;
use a4n_prompt::rendering::PromptRenderer;

use super::super::{MangaPipelineStartStage, OrchestratorContext};

pub(super) struct MangaRunningPipelineCheckpointInput<'a> {
    pub(super) chapter_id: i64,
    pub(super) project_id: i64,
    pub(super) stage: &'a str,
    pub(super) current: i32,
    pub(super) message: &'a str,
    pub(super) task_id: Option<&'a str>,
    pub(super) source_metadata: &'a Value,
    pub(super) reference_assets_metadata: &'a Value,
    pub(super) prompt_fingerprint: &'a Value,
    pub(super) checkpoint: &'a Map<String, Value>,
}

pub(super) struct MangaRunContext<'a> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) chapter_id: i64,
    pub(super) project_id: i64,
    pub(super) shooting_script: ParsedChapterShootingScript,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) task_id: Option<&'a str>,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) resume_from_checkpoint: bool,
    pub(super) start_from_stage: Option<MangaPipelineStartStage>,
    pub(super) chapter_content: String,
    pub(super) source_spans: Vec<SourceSpan>,
    pub(super) variables: HashMap<String, Value>,
    pub(super) reference_assets: Value,
    pub(super) source_metadata: Value,
    pub(super) reference_assets_metadata: Value,
    pub(super) prompt_fingerprint: Value,
}

impl MangaRunContext<'_> {
    pub(super) fn should_restore_stage(&self, stage_order: i32) -> bool {
        super::super::pipeline_state::should_restore_manga_stage_checkpoint(
            self.resume_from_checkpoint,
            self.start_from_stage,
            stage_order,
        )
    }
}

pub(super) struct MangaEarlyRun {
    pub(super) events_json: Value,
    pub(super) characters_json: Value,
    pub(super) speech_out: String,
    pub(super) speech_json: Value,
}

pub(super) struct MangaExtractionRun {
    pub(super) extraction_summary: Value,
}

pub(super) struct MangaPlanningRun {
    pub(super) extraction: MangaExtractionRun,
    pub(super) plan_json: Value,
}
