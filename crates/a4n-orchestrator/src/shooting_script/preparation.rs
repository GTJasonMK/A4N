use std::collections::HashMap;

use a4n_domain::source_spans::{source_span_prompt_values, source_spans_for_content};
use a4n_error::AppError;
use serde_json::json;

use crate::OrchestratorContext;

use super::ChapterShootingScriptSourceInput;
use super::contracts::{
    CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT, CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT,
    CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT, CHAPTER_SHOOTING_SCRIPT_WORKFLOW,
    PreparedChapterShootingScriptRequest, RegenerateChapterShootingScriptInput,
};

impl OrchestratorContext {
    pub(in crate::shooting_script) async fn prepare_chapter_shooting_script_request(
        &self,
        input: RegenerateChapterShootingScriptInput,
    ) -> Result<PreparedChapterShootingScriptRequest, AppError> {
        let source_input = ChapterShootingScriptSourceInput {
            project_id: input.project_id,
            source_chapter_id: input.source_chapter_id,
            visual_plan_id: Some(input.visual_plan_id),
        };
        let (source, existing) = self
            .load_chapter_shooting_script_source_record(&source_input)
            .await?;
        let visual_plan = self
            .confirmed_visual_plan_for_shooting_script(&source, Some(input.visual_plan_id))
            .await?;
        let source_spans = source_spans_for_content(
            source.source_chapter_id,
            source.chapter_number,
            &source.content,
        );
        if source_spans.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "源章节 {} 无法拆分出可引用 source span",
                source.source_chapter_id
            )));
        }
        let visual_plan_projection =
            crate::chapter_visual_plan::project_visual_plan_for_shooting_script(
                &visual_plan.plan,
                &source_spans,
            )?;
        let layout_unit_system_prompt = self.prompt_render_template(
            CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT,
            &HashMap::new(),
            Some(input.project_id),
        )?;
        let unit_adaptation_system_prompt = self.prompt_render_template(
            CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT,
            &HashMap::new(),
            Some(input.project_id),
        )?;
        let design_unit_system_prompt = self.prompt_render_template(
            CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT,
            &HashMap::new(),
            Some(input.project_id),
        )?;
        let user_payload = json!({
            "source": {
                "root_project_id": source.root_project_id,
                "source_chapter_id": source.source_chapter_id,
                "chapter_number": source.chapter_number,
                "title": source.title,
                "summary": source.summary,
                "analysis": source.analysis_data_json,
                "content": source.content,
                "content_sha256": source.content_sha256,
                "source_revision_key": source.source_revision_key,
            },
            "source_spans": source_span_prompt_values(&source_spans),
        });
        let input_fingerprint_json = json!({
            "workflow": CHAPTER_SHOOTING_SCRIPT_WORKFLOW,
            "stages": {
                "screenplay_layout_unit": {
                    "prompt_name": CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT,
                    "system_prompt": &layout_unit_system_prompt,
                },
                "screenplay_unit_adaptation": {
                    "prompt_name": CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT,
                    "system_prompt": &unit_adaptation_system_prompt,
                },
                "shooting_design_unit": {
                    "prompt_name": CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT,
                    "system_prompt": &design_unit_system_prompt,
                },
            },
            "user_payload": &user_payload,
            "visual_plan_identity": {
                "id": visual_plan.record.id,
                "version": visual_plan.record.plan_version,
                "sha256": visual_plan.record.plan_sha256,
            },
            "visual_plan": &visual_plan.plan,
            "temperature": 0.2,
        });
        let input_fingerprint_sha256 =
            a4n_db::generation::generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
        Ok(PreparedChapterShootingScriptRequest {
            source,
            source_spans,
            input_fingerprint_sha256,
            existing,
            visual_plan,
            visual_plan_projection,
            layout_unit_system_prompt,
            unit_adaptation_system_prompt,
            design_unit_system_prompt,
        })
    }
}
