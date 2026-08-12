use std::collections::HashMap;

use a4n_api_contract::chapter::{CHAPTER_SCENE_PROGRESS_TOTAL, GenerateChapterSceneInput};
use a4n_core::{error::AppError, models::chapter::chapter_scene};
use a4n_llm::provider::is_truncated_finish_reason;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    project::ProjectGeneratedChapterSceneSaveInput,
    workflow::{self, CompletedLlmCallCheckpointInput, workflow_checkpoint_contract_app_error},
};

use super::{
    context::build_scene_generation_context,
    contracts::{
        ChapterSceneGenerationInput, ChapterSceneGenerationResult, ChapterSceneProgressReporter,
        ChapterSceneRequestPreview, PreparedChapterSceneGeneration,
        RequiredChapterSceneGenerationInput,
    },
    output::{normalize_scene_generation_output, restored_chapter_scene_checkpoint},
    request::{
        BuildChapterSceneRequestInput, build_scene_generation_request, chapter_scene_call_key,
    },
    workflow::{chapter_scene_call_fingerprint_json, chapter_scene_workflow_run_input},
};

impl OrchestratorContext {
    async fn chapter_scene_generation_input_for_user(
        &self,
        user_id: i64,
        input: GenerateChapterSceneInput,
    ) -> Result<ChapterSceneGenerationInput, AppError> {
        let chapter = self
            .chapter_for_editor_validated(user_id, input.chapter_id)
            .await?;
        Ok(ChapterSceneGenerationInput {
            chapter,
            scene_number: input.scene_number,
            scene_prompt: input.scene_prompt,
        })
    }

    pub async fn chapter_scene_project_id_for_user_validated(
        &self,
        user_id: i64,
        input: &GenerateChapterSceneInput,
    ) -> Result<i64, AppError> {
        let input = self
            .chapter_scene_generation_input_for_user(user_id, input.clone())
            .await?;
        required_scene_generation_input(&input)?;
        Ok(input.chapter.project_id)
    }

    pub async fn prepare_chapter_scene_request_preview_for_user_validated(
        &self,
        user_id: i64,
        input: GenerateChapterSceneInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterSceneRequestPreview, AppError> {
        let input = self
            .chapter_scene_generation_input_for_user(user_id, input)
            .await?;
        self.prepare_chapter_scene_request_preview_validated(input, prompt_renderer)
            .await
    }

    pub async fn generate_chapter_scene_for_user_validated(
        &self,
        user_id: i64,
        input: GenerateChapterSceneInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: ChapterSceneProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<chapter_scene::Model, AppError> {
        let input = self
            .chapter_scene_generation_input_for_user(user_id, input)
            .await?;
        self.generate_chapter_scene_validated(input, prompt_renderer, progress, active_task_id)
            .await
    }

    async fn prepare_chapter_scene_request_preview_validated(
        &self,
        input: ChapterSceneGenerationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterSceneRequestPreview, AppError> {
        let prepared = self
            .prepare_chapter_scene_generation(input, prompt_renderer)
            .await?;
        Ok(ChapterSceneRequestPreview {
            chapter_number: prepared.chapter_number,
            request: prepared.request,
        })
    }

    async fn generate_chapter_scene_validated(
        &self,
        input: ChapterSceneGenerationInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: ChapterSceneProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<chapter_scene::Model, AppError> {
        progress(0, CHAPTER_SCENE_PROGRESS_TOTAL, "正在读取章节和场景参数")?;
        let prepared = self
            .prepare_chapter_scene_generation(input, prompt_renderer)
            .await?;
        progress(1, CHAPTER_SCENE_PROGRESS_TOTAL, "正在构造场景生成提示词")?;
        progress(2, CHAPTER_SCENE_PROGRESS_TOTAL, "正在调用模型生成场景正文")?;

        let (workflow_input, workflow_fingerprint) = chapter_scene_workflow_run_input(
            prepared.project_id,
            prepared.chapter_id,
            prepared.scene_number,
            &prepared.scene_prompt,
            &prepared.call_key,
            &prepared.request,
            active_task_id,
        )?;
        let workflow_run = self.begin_workflow_run_validated(workflow_input).await?;
        let result = self
            .execute_chapter_scene_generation(
                workflow_run.id,
                workflow_fingerprint,
                prepared,
                progress,
            )
            .await;
        self.finalize_workflow_result(workflow_run.id, result, None)
            .await
    }

    async fn prepare_chapter_scene_generation(
        &self,
        input: ChapterSceneGenerationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<PreparedChapterSceneGeneration, AppError> {
        let required = required_scene_generation_input(&input)?;
        let chapter = input.chapter;
        let outline = match chapter.outline_id {
            Some(outline_id) => self.chapter_outline_by_id(outline_id).await?,
            None => {
                self.chapter_outline_by_project_number(chapter.project_id, chapter.chapter_number)
                    .await?
            }
        };
        let existing_scenes = self.chapter_scenes_by_chapter(chapter.id).await?;
        let generation_context = build_scene_generation_context(
            &chapter,
            outline.as_ref(),
            &existing_scenes,
            input.scene_number,
        );
        let variables = HashMap::from([
            ("chapter_number".to_string(), json!(chapter.chapter_number)),
            ("chapter_title".to_string(), json!(required.title)),
            ("scene_number".to_string(), json!(input.scene_number)),
        ]);
        let system_prompt = prompt_renderer
            .render_prompt(
                "chapter_scene_generation",
                &variables,
                Some(chapter.project_id),
            )
            .await?;
        let call_key = chapter_scene_call_key(chapter.chapter_number, input.scene_number);
        let request = build_scene_generation_request(BuildChapterSceneRequestInput {
            system_prompt,
            project_id: chapter.project_id,
            chapter_number: chapter.chapter_number,
            title: &required.title,
            summary: &required.summary,
            generation_context: Some(&generation_context),
            scene_number: input.scene_number,
            scene_prompt: &required.scene_prompt,
        })?;
        Ok(PreparedChapterSceneGeneration {
            project_id: chapter.project_id,
            chapter_id: chapter.id,
            chapter_number: chapter.chapter_number,
            scene_number: input.scene_number,
            scene_prompt: required.scene_prompt,
            call_key,
            request,
        })
    }

    async fn execute_chapter_scene_generation(
        &self,
        workflow_run_id: i64,
        workflow_fingerprint: Value,
        prepared: PreparedChapterSceneGeneration,
        progress: ChapterSceneProgressReporter<'_>,
    ) -> ChapterSceneGenerationResult {
        let content = self
            .run_chapter_scene_llm_call(
                workflow_run_id,
                workflow_fingerprint,
                prepared.call_key,
                prepared.request,
                progress,
            )
            .await?;
        progress(3, CHAPTER_SCENE_PROGRESS_TOTAL, "模型已返回，正在保存场景")?;
        let scene = self
            .save_generated_chapter_scene_record(ProjectGeneratedChapterSceneSaveInput {
                chapter_id: prepared.chapter_id,
                scene_number: prepared.scene_number,
                content,
            })
            .await?;
        progress(
            CHAPTER_SCENE_PROGRESS_TOTAL,
            CHAPTER_SCENE_PROGRESS_TOTAL,
            "场景已保存",
        )?;
        Ok(scene)
    }

    async fn run_chapter_scene_llm_call(
        &self,
        workflow_run_id: i64,
        workflow_fingerprint: Value,
        call_key: String,
        mut request: a4n_llm::provider::ChatRequest,
        progress: ChapterSceneProgressReporter<'_>,
    ) -> Result<String, AppError> {
        let fingerprint_json =
            chapter_scene_call_fingerprint_json(&call_key, &workflow_fingerprint, &request)?;
        workflow::attach_llm_call_checkpoint_trace(
            &mut request,
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )?;
        self.mark_workflow_current_step(workflow_run_id, &call_key)
            .await?;
        if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &fingerprint_json)
            .await?
        {
            progress(
                3,
                CHAPTER_SCENE_PROGRESS_TOTAL,
                "场景正文已从断点恢复，正在保存场景",
            )?;
            let checkpoint_json =
                workflow::required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(workflow_checkpoint_contract_app_error)?;
            return restored_chapter_scene_checkpoint(checkpoint_json, &call_key);
        }

        let response = self.stream_complete_llm(request).await?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "场景正文生成输出达到当前 max_tokens 上限，正文可能被截断；请在设置页调大“标准”输出档位，或缩短本次场景目标后重试".to_string(),
            ));
        }
        let content = normalize_scene_generation_output(&response.message.content)?;
        let response_usage_json = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: 1,
            input_fingerprint_json: fingerprint_json,
            checkpoint_json: json!({
                "output": content.clone(),
                "parsed_output": content.clone(),
                "response_content": response.message.content,
                "finish_reason": response.finish_reason,
                "usage": response_usage_json,
            }),
            artifacts_json: None,
        })
        .await?;
        Ok(content)
    }
}

pub(super) fn required_scene_generation_input(
    input: &ChapterSceneGenerationInput,
) -> Result<RequiredChapterSceneGenerationInput, AppError> {
    if input.scene_number <= 0 {
        return Err(AppError::InvalidArgument("场景编号必须大于 0".to_string()));
    }
    let title = required_chapter_prompt_text(
        input.chapter.title.as_deref(),
        "标题",
        input.chapter.chapter_number,
    )?;
    let summary = required_chapter_prompt_text(
        input.chapter.summary.as_deref(),
        "摘要",
        input.chapter.chapter_number,
    )?;
    let scene_prompt = input.scene_prompt.trim();
    if scene_prompt.is_empty() {
        return Err(AppError::InvalidArgument(
            "生成章节场景缺少场景引导".to_string(),
        ));
    }
    Ok(RequiredChapterSceneGenerationInput {
        title: title.to_string(),
        summary: summary.to_string(),
        scene_prompt: scene_prompt.to_string(),
    })
}

fn required_chapter_prompt_text<'a>(
    value: Option<&'a str>,
    field: &str,
    chapter_number: i32,
) -> Result<&'a str, AppError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("生成章节场景需要第 {chapter_number} 章已有{field}"))
        })
}
