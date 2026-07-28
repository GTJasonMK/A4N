use std::collections::HashMap;

use a4n_core::{
    config::app_config::llm_token_profiles, error::AppError, source_spans::source_spans_for_content,
};
use serde_json::json;

use crate::OrchestratorContext;

use super::{
    contracts::{
        CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT, CHAPTER_SCREENPLAY_SKELETON_PROMPT,
        CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT, CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT,
        CHAPTER_SHOOTING_SCRIPT_WORKFLOW, ChapterShootingScriptSourceInput,
        PreparedChapterShootingScriptRequest, RegenerateChapterShootingScriptInput,
    },
    input::shooting_script_user_payload,
};

impl OrchestratorContext {
    pub(crate) async fn prepare_chapter_shooting_script_request(
        &self,
        input: RegenerateChapterShootingScriptInput,
    ) -> Result<PreparedChapterShootingScriptRequest, AppError> {
        let source_input = ChapterShootingScriptSourceInput {
            project_id: input.project_id,
            source_chapter_id: input.source_chapter_id,
        };
        let (source, existing) = self
            .load_chapter_shooting_script_source(&source_input)
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
        let skeleton_system_prompt = self.prompt_render_template(
            CHAPTER_SCREENPLAY_SKELETON_PROMPT,
            &HashMap::new(),
            Some(input.project_id),
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
        let user_payload = shooting_script_user_payload(&source, &source_spans);
        let input_fingerprint_json = json!({
            "workflow": CHAPTER_SHOOTING_SCRIPT_WORKFLOW,
            "stages": {
                "screenplay_skeleton": {
                    "prompt_name": CHAPTER_SCREENPLAY_SKELETON_PROMPT,
                    "system_prompt": &skeleton_system_prompt,
                },
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
            "temperature": 0.2,
            "max_tokens_profiles": {
                "skeleton": llm_token_profiles::LONG,
                "layout_unit": llm_token_profiles::LONG,
                "unit_adaptation": llm_token_profiles::LONG,
                "design_unit": llm_token_profiles::EXTRA_LONG,
            },
        });
        let input_fingerprint_sha256 =
            a4n_db::generation::generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
        Ok(PreparedChapterShootingScriptRequest {
            source,
            source_spans,
            input_fingerprint_sha256,
            existing,
            skeleton_system_prompt,
            layout_unit_system_prompt,
            unit_adaptation_system_prompt,
            design_unit_system_prompt,
        })
    }
}
