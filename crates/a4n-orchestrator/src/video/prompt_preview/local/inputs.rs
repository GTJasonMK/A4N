use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

use a4n_core::error::AppError;

use crate::video::library::available_video_image_ids;
use crate::video::prompt_context::{validate_prompt_options, video_source_metadata};
use crate::video::prompt_records::video_generation_asset_fingerprint;
use crate::video::{
    ChapterVideoPromptOptions, ChapterVideoSourceContext, OrchestratorContext, PromptOutputLanguage,
};
use a4n_core::source_spans::{SourceSpan, source_spans_for_content};
use a4n_prompt::rendering::PromptRenderer;

async fn source_spans_for_prompt_preview(
    service: &OrchestratorContext,
    chapter_id: i64,
    source: &ChapterVideoSourceContext<'_>,
) -> Result<Vec<SourceSpan>, AppError> {
    let chapter = service.get_chapter(chapter_id).await?;
    Ok(source_spans_for_content(
        chapter.id,
        chapter.chapter_number,
        source.content,
    ))
}

pub(super) struct PreparedVideoPromptPreviewInputs {
    pub(super) source_metadata: Value,
    pub(super) source_spans: Vec<SourceSpan>,
    pub(super) regenerate_existing: bool,
    pub(super) style: String,
    pub(super) prompt_output_language: PromptOutputLanguage,
    pub(super) duration_secs: i32,
    pub(super) aspect_ratio: String,
    pub(super) reference_image_ids: Vec<i64>,
    pub(super) reference_ids: Value,
    pub(super) generation_asset_fingerprint: Value,
    pub(super) project_context: Option<Value>,
    pub(super) available_assets: Value,
    pub(super) vars: HashMap<String, Value>,
}

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

impl OrchestratorContext {
    pub(super) async fn prepare_video_prompt_preview_inputs(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: &ChapterVideoSourceContext<'_>,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
        include_project_context: bool,
    ) -> Result<PreparedVideoPromptPreviewInputs, AppError> {
        let options = validate_prompt_options(options)?;
        let source_metadata = video_source_metadata(source);
        self.ensure_prompt_generation_inputs(project_id, chapter_id, &options)
            .await?;
        let source_spans = source_spans_for_prompt_preview(self, chapter_id, source).await?;
        let ChapterVideoPromptOptions {
            style,
            prompt_output_language,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            video_library_image_ids,
            regenerate_existing,
            ..
        } = options;
        let reference_ids = json!(reference_image_ids.clone());
        let project_context = if include_project_context {
            self.chapter_video_project_context(project_id, source.analysis)
                .await?
        } else {
            None
        };
        let available_assets = self
            .video_library_image_context(
                project_id,
                chapter_id,
                source,
                &reference_image_ids,
                &video_library_image_ids,
            )
            .await?;
        let generation_asset_fingerprint = video_generation_asset_fingerprint(
            &reference_image_ids,
            &video_library_image_ids,
            &available_assets,
        );
        let mut vars = variables.clone();
        vars.insert("style".into(), json!(style));
        vars.insert(
            "prompt_output_language".into(),
            json!(prompt_output_language.as_str()),
        );
        vars.insert(
            "prompt_output_language_instruction".into(),
            json!(prompt_output_language.instruction()),
        );
        vars.insert("duration_secs".into(), json!(duration_secs));
        vars.insert("aspect_ratio".into(), json!(aspect_ratio));
        vars.insert("reference_image_ids".into(), reference_ids.clone());
        vars.insert("available_video_assets".into(), available_assets.clone());
        Ok(PreparedVideoPromptPreviewInputs {
            source_metadata,
            source_spans,
            regenerate_existing,
            style,
            prompt_output_language,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            reference_ids,
            generation_asset_fingerprint,
            project_context,
            available_assets,
            vars,
        })
    }
}
