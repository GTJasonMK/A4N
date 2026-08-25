use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;
use a4n_db::workflow::{self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};
use chrono::Utc;
use serde_json::Value;

use crate::workflow::workflow_checkpoint_contract_app_error;

use super::super::super::chapter_contract::{
    parse_chapter_outline_batch, required_i32, sync_chapter_outlines_by_number_in,
};
use super::super::super::context::lock_outline_generation_sources;
use super::super::super::{
    BeatMappingApplyInput, OrchestratorContext, ensure_response_not_truncated, report_progress,
};
use super::checkpoint::restored_beat_mapping_checkpoint;
use super::preparation::prepare_beat_mapping;
use super::request::{beat_mapping_call_fingerprint_json, build_beat_mapping_request};

impl OrchestratorContext {
    /// 将节拍表映射为指定分部的章节大纲。
    pub(in crate::outline) async fn apply_beat_mapping(
        &self,
        input: BeatMappingApplyInput<'_, '_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let BeatMappingApplyInput {
            project_id,
            part_id,
            beat_sheet,
            prompt_renderer,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let prepared = prepare_beat_mapping(self, project_id, part_id, beat_sheet).await?;

        report_progress(
            progress,
            0,
            prepared.count,
            &format!(
                "正在为第 {} 部映射 {} 个章节大纲",
                prepared.part.part_number, prepared.count
            ),
        )?;
        let mut request =
            build_beat_mapping_request(project_id, &prepared, prompt_renderer, variables).await?;
        let call_key = format!("part_{}_beat_mapping", prepared.part.part_number);
        let workflow_run_id =
            workflow::required_llm_call_workflow_run_id(workflow_run_id, &call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let fingerprint_json = beat_mapping_call_fingerprint_json(&call_key, &prepared, &request)?;
        crate::workflow::attach_llm_call_checkpoint_trace(
            &mut request,
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )?;
        self.mark_workflow_current_step(workflow_run_id, &call_key)
            .await?;
        let chapters = if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &fingerprint_json)
            .await?
        {
            restored_beat_mapping_checkpoint(
                &checkpoint.parsed_output,
                &checkpoint.response_content,
                &prepared,
            )?
        } else {
            let response =
                crate::outline::execute_outline_llm_call(self, request.clone(), &call_key).await?;
            let content = response.message.content;
            ensure_response_not_truncated(
                response.finish_reason.as_deref(),
                &format!("第 {} 部节拍映射", prepared.part.part_number),
            )?;
            let chapters =
                parse_chapter_outline_batch(&content, prepared.part.start_chapter, prepared.count)?;
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: prepared.part.part_number,
                input_fingerprint_json: fingerprint_json,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: Value::Array(chapters.clone()),
                    response_content: content,
                    usage: response
                        .usage
                        .as_ref()
                        .map(serde_json::to_value)
                        .transpose()?,
                    finish_reason: response.finish_reason,
                    artifacts_json: None,
                },
            })
            .await?;
            chapters
        };
        let now = Utc::now().naive_utc();

        for (index, chapter_json) in chapters.iter().enumerate() {
            let current = i32::try_from(index)
                .map_err(|_| {
                    AppError::InvalidArgument(format!("节拍映射章节进度索引超过 i32 范围：{index}"))
                })?
                .checked_add(1)
                .ok_or_else(|| {
                    AppError::InvalidArgument("节拍映射章节进度超过 i32 范围".to_string())
                })?;
            let chapter_offset = i32::try_from(index).map_err(|_| {
                AppError::InvalidArgument(format!("节拍映射章节索引超过 i32 范围：{index}"))
            })?;
            let expected_chapter = prepared
                .part
                .start_chapter
                .checked_add(chapter_offset)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "节拍映射章节号超过 i32 范围：{} + {chapter_offset}",
                        prepared.part.start_chapter
                    ))
                })?;
            let chapter_number = required_i32(chapter_json, "chapter_number", expected_chapter)?;
            report_progress(
                progress,
                current - 1,
                prepared.count,
                &format!(
                    "正在保存第 {current}/{} 个章节大纲（第 {chapter_number} 章）",
                    prepared.count
                ),
            )?;
            report_progress(
                progress,
                current,
                prepared.count,
                &format!(
                    "已准备第 {current}/{} 个章节大纲（第 {chapter_number} 章）",
                    prepared.count
                ),
            )?;
        }
        let tx = lock_outline_generation_sources(
            self.database.connection(),
            project_id,
            &prepared.context.source_snapshot,
        )
        .await?;
        let outlines = sync_chapter_outlines_by_number_in(
            &tx,
            project_id,
            Some(prepared.part.id),
            chapters,
            now,
        )
        .await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(outlines)
    }
}
