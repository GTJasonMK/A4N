use std::collections::BTreeSet;

use a4n_core::{error::AppError, structured_progress::StructuredJsonProgressSpec};
use a4n_llm::{
    context::chat_request_with_workflow_run_id, structured_stream::StructuredLlmProgress,
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    structured_llm::ChatJsonStreamInput,
    workflow::{self, CompletedLlmCallCheckpointInput, workflow_checkpoint_contract_app_error},
};

use super::{
    contracts::{
        ChapterAnchorMode, OBJECT_DISCOVERY_FIELDS, OBJECT_DISCOVERY_LLM_SPAN,
        OBJECT_DISCOVERY_PROGRESS_TOTAL, ObjectDiscoveryCallCheckpoint, ObjectDiscoveryCallResult,
        ObjectDiscoveryInput, ObjectDiscoveryProgress, ObjectDiscoveryRequestContext,
        ObjectDiscoveryRequestPreview, ObjectDiscoveryResult, PrepareObjectDiscoveryRequestInput,
        RequiredObjectDiscoveryInput,
    },
    output::parse_and_validate_object_discovery_output,
    request::build_object_discovery_request,
    rules::{
        OBJECT_DISCOVERY_CALL_KEY, object_discovery_call_fingerprint_json,
        object_discovery_workflow_run_input, select_object_discovery_chapters,
    },
};

impl OrchestratorContext {
    pub async fn prepare_object_discovery_request_preview_validated(
        &self,
        input: ObjectDiscoveryInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ObjectDiscoveryRequestPreview, AppError> {
        let prepared = self
            .prepare_object_discovery_request(PrepareObjectDiscoveryRequestInput {
                input,
                anchor_mode: ChapterAnchorMode::RequireExistingCurrent,
                prompt_renderer,
            })
            .await?;
        Ok(ObjectDiscoveryRequestPreview {
            analyzed_chapter_count: prepared.chapters.len(),
            request: prepared.request,
        })
    }

    pub async fn discover_object_profiles_validated(
        &self,
        input: ObjectDiscoveryInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<ObjectDiscoveryProgress<'_>>,
    ) -> Result<Value, AppError> {
        report_object_discovery_progress(progress, 0, "正在读取可分析章节和已有物体档案")?;
        let required = required_object_discovery_input(&input)?;
        let request_context = self
            .prepare_object_discovery_request(PrepareObjectDiscoveryRequestInput {
                input,
                anchor_mode: ChapterAnchorMode::EnsureCurrent,
                prompt_renderer,
            })
            .await?;
        let workflow_input = object_discovery_workflow_run_input(
            required.project_id,
            required.chapter_number,
            &request_context.chapters,
            &request_context.text_anchors,
            &request_context.request,
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow_run = self.begin_workflow_run_validated(workflow_input).await?;
        let result = async {
            let call_result = self
                .run_object_discovery_call(
                    workflow_run.id,
                    required.project_id,
                    required.chapter_number,
                    &request_context.request,
                    &request_context.chapters,
                    &request_context.text_anchors,
                    progress,
                )
                .await?;
            let result = self
                .persist_discovered_object_profiles(
                    required.project_id,
                    required.chapter_number,
                    request_context,
                    call_result,
                    progress,
                )
                .await?;
            report_object_discovery_progress(
                progress,
                OBJECT_DISCOVERY_PROGRESS_TOTAL,
                "物体档案发现已完成",
            )?;
            Ok(result)
        }
        .await;
        self.finalize_object_discovery(workflow_run.id, result)
            .await
    }

    async fn prepare_object_discovery_request(
        &self,
        input: PrepareObjectDiscoveryRequestInput<'_>,
    ) -> Result<ObjectDiscoveryRequestContext, AppError> {
        let required = required_object_discovery_input(&input.input)?;
        let available_chapters = self.character_sync_chapters(required.project_id).await?;
        let chapters = select_object_discovery_chapters(
            available_chapters,
            required.project_id,
            required.chapter_number,
        )
        .map_err(AppError::InvalidArgument)?;
        let existing_profiles = self.object_profiles_by_project(required.project_id).await?;
        let mut text_anchors = Vec::new();
        for chapter in &chapters {
            let anchors = match input.anchor_mode {
                ChapterAnchorMode::EnsureCurrent => {
                    self.ensure_chapter_content_anchor_records(required.project_id, chapter.id)
                        .await?
                }
                ChapterAnchorMode::RequireExistingCurrent => {
                    self.read_current_object_discovery_anchors(required.project_id, chapter.id)
                        .await?
                }
            };
            text_anchors.extend(anchors);
        }
        let request = build_object_discovery_request(
            input.prompt_renderer,
            required.project_id,
            required.chapter_number,
            &chapters,
            &text_anchors,
            &existing_profiles,
        )
        .await?;
        Ok(ObjectDiscoveryRequestContext {
            chapters,
            text_anchors,
            request,
        })
    }

    async fn read_current_object_discovery_anchors(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<a4n_core::models::project::text_anchor::Model>, AppError> {
        let rows = self
            .read_current_chapter_content_anchor_records(project_id, chapter_id)
            .await?;
        if rows.anchors.is_empty() {
            return Err(AppError::InvalidState {
                current: format!("第 {} 章缺少当前正文文本锚点", rows.chapter.chapter_number),
                operation: "预览模型请求".to_string(),
            });
        }
        if rows
            .anchors
            .iter()
            .any(|anchor| anchor.source_version_hash != rows.source_version_hash)
        {
            return Err(AppError::InvalidState {
                current: format!("第 {} 章正文文本锚点已过期", rows.chapter.chapter_number),
                operation: "预览模型请求".to_string(),
            });
        }
        Ok(rows.anchors)
    }

    async fn run_object_discovery_call(
        &self,
        workflow_run_id: i64,
        project_id: i64,
        chapter_number: Option<i32>,
        request: &a4n_llm::provider::ChatRequest,
        chapters: &[a4n_core::models::chapter::chapter::Model],
        text_anchors: &[a4n_core::models::project::text_anchor::Model],
        progress: Option<ObjectDiscoveryProgress<'_>>,
    ) -> Result<ObjectDiscoveryCallResult, AppError> {
        report_object_discovery_progress(
            progress,
            OBJECT_DISCOVERY_LLM_SPAN.start(),
            "正在调用模型识别关键物体",
        )?;
        let mut llm_request = chat_request_with_workflow_run_id(request.clone(), workflow_run_id);
        let analyzed_chapters = chapters
            .iter()
            .map(|chapter| chapter.chapter_number)
            .collect::<BTreeSet<_>>();
        let call_fingerprint_json =
            object_discovery_call_fingerprint_json(project_id, chapter_number, &llm_request)
                .map_err(AppError::InvalidArgument)?;
        let call_fingerprint_sha256 =
            workflow::llm_call_checkpoint_fingerprint_sha256(&call_fingerprint_json)
                .map_err(workflow_checkpoint_contract_app_error)?;
        if let Some(trace_context) = llm_request.trace_context.as_mut() {
            trace_context.call_key = Some(OBJECT_DISCOVERY_CALL_KEY.to_string());
            trace_context.input_fingerprint_sha256 = Some(call_fingerprint_sha256);
        }

        let (parsed_drafts, response_content) = if let Some(checkpoint) = self
            .load_object_discovery_checkpoint(workflow_run_id, &call_fingerprint_json)
            .await?
        {
            report_object_discovery_progress(
                progress,
                80,
                "物体档案发现已从断点恢复，正在写入物体档案",
            )?;
            let (parsed_drafts, parsed_output) = parse_and_validate_object_discovery_output(
                &checkpoint.response_content,
                &analyzed_chapters,
                chapters,
                text_anchors,
            )
            .map_err(AppError::InvalidArgument)?;
            if parsed_output != checkpoint.parsed_output {
                return Err(AppError::InvalidArgument(format!(
                    "{OBJECT_DISCOVERY_CALL_KEY} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
                )));
            }
            (parsed_drafts, checkpoint.response_content)
        } else {
            let response_content = self
                .chat_json_stream_with_progress(
                    ChatJsonStreamInput {
                        request: llm_request.clone(),
                        prompt_name: "object_discovery",
                        truncated_message: "物体档案发现输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“角色/档案发现”输出档位，或减少本次章节上下文后重试",
                        structured_progress: Some(StructuredLlmProgress {
                            spec: StructuredJsonProgressSpec {
                                root_array_key: "objects",
                                item_label: "物体",
                                expected_items: None,
                                milestones: OBJECT_DISCOVERY_FIELDS,
                            },
                            span: OBJECT_DISCOVERY_LLM_SPAN,
                        }),
                        cancel_token: progress.and_then(|progress| progress.cancel_token),
                        cancelled_message: "物体档案发现已停止",
                    },
                    |current, message| {
                        report_object_discovery_progress(progress, current, message)
                            .map_err(|error| error.to_string())
                    },
                )
                .await
                .map_err(AppError::LlmError)?;
            report_object_discovery_progress(progress, 80, "模型已返回，正在校验物体发现结果")?;
            let (parsed_drafts, parsed_output) = parse_and_validate_object_discovery_output(
                &response_content,
                &analyzed_chapters,
                chapters,
                text_anchors,
            )
            .map_err(AppError::InvalidArgument)?;
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: OBJECT_DISCOVERY_CALL_KEY.to_string(),
                call_order: 1,
                input_fingerprint_json: call_fingerprint_json,
                checkpoint_json: json!({
                    "output": parsed_output.clone(),
                    "parsed_output": parsed_output,
                    "response_content": response_content.clone(),
                    "usage": null,
                    "finish_reason": null,
                }),
                artifacts_json: None,
            })
            .await?;
            (parsed_drafts, response_content)
        };

        Ok(ObjectDiscoveryCallResult {
            parsed_drafts,
            response_content,
            llm_request,
            analyzed_chapters,
        })
    }

    async fn load_object_discovery_checkpoint(
        &self,
        workflow_run_id: i64,
        input_fingerprint_json: &Value,
    ) -> Result<Option<ObjectDiscoveryCallCheckpoint>, AppError> {
        let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(
                workflow_run_id,
                OBJECT_DISCOVERY_CALL_KEY,
                input_fingerprint_json,
            )
            .await?
        else {
            return Ok(None);
        };
        let checkpoint_json =
            workflow::required_llm_call_checkpoint_json(checkpoint, OBJECT_DISCOVERY_CALL_KEY)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let response_content = checkpoint_json
            .get("response_content")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "{OBJECT_DISCOVERY_CALL_KEY}.response_content 必须是字符串"
                ))
            })?;
        let parsed_output = checkpoint_json
            .get("parsed_output")
            .cloned()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("{OBJECT_DISCOVERY_CALL_KEY}.parsed_output 缺失"))
            })?;
        Ok(Some(ObjectDiscoveryCallCheckpoint {
            response_content,
            parsed_output,
        }))
    }

    async fn finalize_object_discovery(
        &self,
        workflow_run_id: i64,
        result: ObjectDiscoveryResult,
    ) -> ObjectDiscoveryResult {
        let (success, error_message) = match &result {
            Ok(_) => (true, None),
            Err(error) => (false, Some(error.to_string())),
        };
        let status_result = self
            .finalize_workflow_run_validated(workflow_run_id, success, error_message.as_deref())
            .await;
        match (result, status_result) {
            (result, Ok(_)) => result,
            (Ok(_), Err(status_error)) => Err(AppError::Internal(anyhow::anyhow!(
                "物体档案发现完成，但工作流状态写入失败: {status_error}"
            ))),
            (Err(primary), Err(status_error)) => Err(AppError::Internal(anyhow::anyhow!(
                "{primary}；工作流失败状态写入失败: {status_error}"
            ))),
        }
    }
}

fn required_object_discovery_input(
    input: &ObjectDiscoveryInput,
) -> Result<RequiredObjectDiscoveryInput, AppError> {
    if input.project_id <= 0 {
        return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
    }
    if input
        .chapter_number
        .is_some_and(|chapter_number| chapter_number <= 0)
    {
        return Err(AppError::InvalidArgument(
            "物体档案发现章节号必须大于 0".to_string(),
        ));
    }
    Ok(RequiredObjectDiscoveryInput {
        project_id: input.project_id,
        chapter_number: input.chapter_number,
    })
}

pub(super) fn report_object_discovery_progress(
    progress: Option<ObjectDiscoveryProgress<'_>>,
    current: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress {
        progress.report(current, message)?;
    }
    Ok(())
}
