use std::collections::HashMap;

use serde_json::{Value, json};

use super::super::super::{
    AppError, ChapterVideoPromptOptions, ChapterVideoSourceContext, OrchestratorContext,
    PreparedVideoPromptInputs, source_spans_for_video_prompt, validate_prompt_options,
    video_generation_asset_fingerprint, video_source_metadata,
};

impl OrchestratorContext {
    pub(in crate::video::prompt_generation::frame_prompts) async fn prepare_video_prompt_inputs(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: &ChapterVideoSourceContext<'_>,
        variables: &HashMap<String, Value>,
        options: ChapterVideoPromptOptions,
        include_project_context: bool,
    ) -> Result<PreparedVideoPromptInputs, AppError> {
        let options = validate_prompt_options(options)?;
        let source_metadata = video_source_metadata(source);
        self.ensure_prompt_generation_inputs(project_id, chapter_id, &options)
            .await?;
        let source_spans = source_spans_for_video_prompt(self, chapter_id, source).await?;
        let ChapterVideoPromptOptions {
            style,
            prompt_output_language,
            prompt_batch_size,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            video_library_image_ids,
            regenerate_existing,
            workflow_run_id,
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
        Ok(PreparedVideoPromptInputs {
            source_metadata,
            source_spans,
            regenerate_existing,
            prompt_batch_size: prompt_batch_size as usize,
            style,
            prompt_output_language,
            duration_secs,
            aspect_ratio,
            reference_image_ids,
            video_library_image_ids,
            reference_ids,
            generation_asset_fingerprint,
            project_context,
            available_assets,
            vars,
            workflow_run_id,
        })
    }
}
