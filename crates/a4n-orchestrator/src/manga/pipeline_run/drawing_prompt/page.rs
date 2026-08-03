use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::super::super::pipeline_json::{
    page_drawing_prompt_from_response, page_plan_page_number,
};
use super::super::super::pipeline_prompt::{
    MangaContextManifestInput, MangaRenderTraceInput, build_manga_context_manifest,
    manga_page_drawing_prompt_user_message,
};
use super::super::super::pipeline_status::{
    MangaPipelineCheckpointInput, MangaPipelineStatusInput,
};
use super::super::super::{OrchestratorContext, PIPELINE_TOTAL_STEPS};
use super::contracts::{SinglePageDrawingPromptInput, SinglePageDrawingPromptOutput};

impl OrchestratorContext {
    pub(super) async fn run_single_page_drawing_prompt(
        &self,
        input: SinglePageDrawingPromptInput<'_>,
    ) -> Result<SinglePageDrawingPromptOutput, AppError> {
        let SinglePageDrawingPromptInput {
            chapter_id,
            project_id,
            prompt_renderer,
            vars,
            task_id,
            workflow_run_id,
            source_metadata,
            reference_assets_metadata,
            prompt_fingerprint,
            checkpoint,
            chapter_content,
            source_spans,
            reference_assets,
            page_plan,
            page_index,
            total_pages,
            extraction_summary,
            restore_from_checkpoint,
        } = input;
        let page_number = page_plan_page_number(
            page_plan,
            &format!("manga_page_planning.pages[{page_index}]"),
        )?;
        let page_out_key = format!("dp_page_{page_number}_out");
        let page_trace_key = format!("dp_page_{page_number}_trace");
        let page_json_key = format!("dp_page_{page_number}_json");
        let page_context = format!("manga_page_drawing_prompt.page_{page_number}");
        let dp_user =
            manga_page_drawing_prompt_user_message(page_number, page_plan, extraction_summary)?;
        let dp_manifest = build_manga_context_manifest(MangaContextManifestInput {
            prompt_renderer,
            project_id,
            variables: vars,
            chapter_content,
            source_spans,
            prompt_names: &["manga_page_drawing_prompt"],
            previous_outputs: &[],
            reference_assets: Some(reference_assets),
            dynamic_user_content: dp_user,
        })
        .await?;
        let call_index = i32::try_from(page_index).map_err(|_| {
            AppError::InvalidArgument(format!("漫画绘图提示词页索引超出 i32 范围：{page_index}"))
        })?;
        let call_order = 800_i32.checked_add(call_index).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画绘图提示词 call_order 超出 i32 范围：page_index={page_index}"
            ))
        })?;
        let trace = self
            .render_and_chat_trace(MangaRenderTraceInput {
                prompt_renderer,
                prompt_name: "manga_page_drawing_prompt",
                project_id,
                workflow_run_id,
                call_key: format!(
                    "manga_pipeline:{chapter_id}:manga_page_drawing_prompt:page:{page_number}"
                ),
                call_order,
                restore_from_checkpoint,
                variables: vars,
                manifest: dp_manifest,
                temperature: 0.5,
            })
            .await?;
        self.report_pipeline_llm_usage(
            chapter_id,
            project_id,
            "drawing_prompt",
            7,
            task_id,
            &trace,
        )
        .await?;
        let page_out = trace.response_text.clone();
        let page_json = self
            .repair_and_validate_manga_llm_output(&trace, |value| {
                page_drawing_prompt_from_response(value, page_number, &page_context)
            })
            .await?;
        let page_prompt = page_json.clone();
        self.save_or_validate_manga_llm_checkpoint(&trace, &page_json)
            .await?;
        checkpoint_insert(checkpoint, &page_out_key, json!(page_out));
        checkpoint_insert_trace(checkpoint, &page_trace_key, &trace)?;
        checkpoint_insert(checkpoint, &page_json_key, page_json);
        let message = format!(
            "已完成第 {page_number} 页整页提示词 ({}/{total_pages})",
            page_index + 1
        );
        let checkpoint_data = Value::Object(checkpoint.clone());
        self.save_pipeline_checkpoint(MangaPipelineCheckpointInput {
            status: MangaPipelineStatusInput {
                chapter_id,
                project_id,
                pipeline_status: "running",
                stage: "drawing_prompt",
                current: 7,
                total: PIPELINE_TOTAL_STEPS,
                message: &message,
                task_id,
                allow_cancel_override: false,
            },
            source_metadata,
            reference_assets_metadata,
            prompt_fingerprint,
            checkpoint_data: &checkpoint_data,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;

        Ok(SinglePageDrawingPromptOutput {
            page_number,
            prompt: page_prompt,
            trace,
        })
    }
}
