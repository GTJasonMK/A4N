use std::collections::{BTreeSet, HashMap};

use a4n_core::{error::AppError, structured_progress::StructuredJsonProgressSpec};
use a4n_llm::{
    context::chat_request_with_workflow_run_id, structured_stream::StructuredLlmProgress,
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    character::{
        discovery_rules::{
            select_character_discovery_chapters, validate_discovered_character_chapter_scope,
        },
        discovery_workflow_rules::{
            CHARACTER_DISCOVERY_CALL_KEY, character_discovery_call_fingerprint_json,
            character_discovery_workflow_run_input,
        },
    },
    structured_llm::{ChatJsonStreamInput, stream_error_or_cancelled},
    workflow::{self, CompletedLlmCallCheckpointInput, workflow_checkpoint_contract_app_error},
};

use super::{
    contracts::{
        CHARACTER_DISCOVERY_FIELDS, CHARACTER_DISCOVERY_LLM_SPAN,
        CHARACTER_DISCOVERY_PROGRESS_TOTAL, ChapterAnchorMode, CharacterDiscoveryCallCheckpoint,
        CharacterDiscoveryCallResult, CharacterDiscoveryInput, CharacterDiscoveryProgress,
        CharacterDiscoveryRequestContext, CharacterDiscoveryRequestPreview,
        CharacterDiscoveryTaskScope, RequiredCharacterDiscoveryInput,
    },
    identity::{existing_character_keys, profile_character_keys},
    output::{character_discovery_parsed_output_json, parse_character_discovery_output},
    request::build_character_discovery_request,
};

impl OrchestratorContext {
    pub async fn character_discovery_task_scope_validated(
        &self,
        input: &CharacterDiscoveryInput,
    ) -> Result<CharacterDiscoveryTaskScope, AppError> {
        let required = required_character_discovery_input(input)?;
        Ok(CharacterDiscoveryTaskScope {
            project_id: required.project_id,
            total: CHARACTER_DISCOVERY_PROGRESS_TOTAL,
        })
    }

    pub async fn prepare_character_discovery_request_preview_validated(
        &self,
        input: CharacterDiscoveryInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterDiscoveryRequestPreview, AppError> {
        let prepared = self
            .prepare_character_discovery_request(
                input,
                ChapterAnchorMode::RequireExistingCurrent,
                prompt_renderer,
            )
            .await?;
        Ok(CharacterDiscoveryRequestPreview {
            analyzed_chapter_count: prepared.chapters.len(),
            request: prepared.request,
        })
    }

    pub async fn discover_character_profiles_validated(
        &self,
        input: CharacterDiscoveryInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterDiscoveryProgress<'_>>,
    ) -> Result<Value, AppError> {
        report_character_discovery_progress(progress, 0, "正在读取可分析章节和已有角色")?;
        let required = required_character_discovery_input(&input)?;
        let request_context = self
            .prepare_character_discovery_request(
                input,
                ChapterAnchorMode::EnsureCurrent,
                prompt_renderer,
            )
            .await?;
        let workflow_input = character_discovery_workflow_run_input(
            required.project_id,
            required.chapter_number,
            &request_context.chapters,
            &request_context.text_anchors,
            &request_context.request,
            progress.and_then(|progress| progress.active_task_id),
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow_run = self.begin_workflow_run_validated(workflow_input).await?;
        let result = async {
            let call_result = self
                .run_character_discovery_call(
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
                .persist_discovered_character_profiles(
                    required.project_id,
                    required.chapter_number,
                    request_context,
                    call_result,
                    progress,
                )
                .await?;
            report_character_discovery_progress(
                progress,
                CHARACTER_DISCOVERY_PROGRESS_TOTAL,
                "角色发现已完成",
            )?;
            Ok(result)
        }
        .await;
        self.finalize_workflow_result(workflow_run.id, result, "角色发现")
            .await
    }

    async fn prepare_character_discovery_request(
        &self,
        input: CharacterDiscoveryInput,
        anchor_mode: ChapterAnchorMode,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterDiscoveryRequestContext, AppError> {
        let required = required_character_discovery_input(&input)?;
        let available_chapters = self.character_sync_chapters(required.project_id).await?;
        let chapters = select_character_discovery_chapters(
            available_chapters,
            required.project_id,
            required.chapter_number,
        )
        .map_err(AppError::InvalidArgument)?;
        let existing_profiles = self
            .protagonist_profiles_for_project(required.project_id)
            .await?;
        let existing_keys = existing_character_keys(&existing_profiles);
        let existing_key_profiles = existing_profiles
            .iter()
            .flat_map(|profile| {
                profile_character_keys(profile)
                    .into_iter()
                    .map(|key| (key, profile.id))
            })
            .collect::<HashMap<_, _>>();
        let mut text_anchors = Vec::new();
        for chapter in &chapters {
            let anchors = match anchor_mode {
                ChapterAnchorMode::EnsureCurrent => {
                    self.ensure_chapter_content_anchor_records(required.project_id, chapter.id)
                        .await?
                }
                ChapterAnchorMode::RequireExistingCurrent => {
                    self.require_current_chapter_content_anchors_for_preview(
                        required.project_id,
                        chapter.id,
                    )
                    .await?
                }
            };
            text_anchors.extend(anchors);
        }
        let request = build_character_discovery_request(
            prompt_renderer,
            required.project_id,
            required.chapter_number,
            &chapters,
            &text_anchors,
            &existing_profiles,
        )
        .await?;
        Ok(CharacterDiscoveryRequestContext {
            chapters,
            text_anchors,
            existing_keys,
            existing_key_profiles,
            request,
        })
    }

    async fn run_character_discovery_call(
        &self,
        workflow_run_id: i64,
        project_id: i64,
        chapter_number: Option<i32>,
        request: &a4n_llm::provider::ChatRequest,
        chapters: &[a4n_core::models::chapter::chapter::Model],
        text_anchors: &[a4n_core::models::project::text_anchor::Model],
        progress: Option<CharacterDiscoveryProgress<'_>>,
    ) -> Result<CharacterDiscoveryCallResult, AppError> {
        report_character_discovery_progress(progress, 10, "正在构造角色发现提示词")?;
        report_character_discovery_progress(
            progress,
            CHARACTER_DISCOVERY_LLM_SPAN.start(),
            "正在调用模型发现角色",
        )?;
        let mut llm_request = chat_request_with_workflow_run_id(request.clone(), workflow_run_id);
        let analyzed_chapters = chapters
            .iter()
            .map(|chapter| chapter.chapter_number)
            .collect::<BTreeSet<_>>();
        let call_fingerprint_json =
            character_discovery_call_fingerprint_json(project_id, chapter_number, &llm_request)
                .map_err(AppError::InvalidArgument)?;
        let call_fingerprint_sha256 =
            workflow::llm_call_checkpoint_fingerprint_sha256(&call_fingerprint_json)
                .map_err(workflow_checkpoint_contract_app_error)?;
        if let Some(trace_context) = llm_request.trace_context.as_mut() {
            trace_context.call_key = Some(CHARACTER_DISCOVERY_CALL_KEY.to_string());
            trace_context.input_fingerprint_sha256 = Some(call_fingerprint_sha256);
        }

        let (discovered, response_content) = if let Some(checkpoint) = self
            .load_character_discovery_checkpoint(workflow_run_id, &call_fingerprint_json)
            .await?
        {
            report_character_discovery_progress(
                progress,
                80,
                "角色发现已从断点恢复，正在写入发现结果",
            )?;
            let discovered = parse_and_validate_character_discovery_content(
                &checkpoint.response_content,
                &analyzed_chapters,
                chapters,
                text_anchors,
            )?;
            let parsed_output = character_discovery_parsed_output_json(&discovered);
            if parsed_output != checkpoint.parsed_output {
                return Err(AppError::InvalidArgument(format!(
                    "{CHARACTER_DISCOVERY_CALL_KEY} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
                )));
            }
            (discovered, checkpoint.response_content)
        } else {
            // 活体信号与字段级里程碑取自同一个 token 流，二者互补而非替代。
            let stream_reporter = progress.and_then(|progress| progress.stream_reporter);
            let mut streamed = String::new();
            let stream_sink = stream_reporter.map(|reporter| {
                move |delta: &str| {
                    streamed.push_str(delta);
                    reporter(&streamed);
                }
            });
            let response_content = self
                .chat_json_stream_result_with_sink(
                    ChatJsonStreamInput {
                        request: llm_request.clone(),
                        prompt_name: "character_discovery",
                        truncated_message: "角色发现输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“角色发现”输出档位，或减少本次章节上下文后重试",
                        structured_progress: Some(StructuredLlmProgress {
                            spec: StructuredJsonProgressSpec {
                                root_array_key: "characters",
                                item_label: "角色",
                                expected_items: None,
                                milestones: CHARACTER_DISCOVERY_FIELDS,
                            },
                            span: CHARACTER_DISCOVERY_LLM_SPAN,
                        }),
                        cancel_token: progress.and_then(|progress| progress.cancel_token),
                        cancelled_message: "角色发现已停止",
                    },
                    |current, message| {
                        report_character_discovery_progress(progress, current, message)
                            .map_err(|error| error.to_string())
                    },
                    stream_sink,
                )
                .await
                .map(|result| result.response_content)
                .map_err(|error| {
                    stream_error_or_cancelled(
                        error,
                        progress.and_then(|progress| progress.cancel_token),
                    )
                })?;
            report_character_discovery_progress(progress, 80, "模型已返回，正在解析发现结果")?;
            let discovered = parse_and_validate_character_discovery_content(
                &response_content,
                &analyzed_chapters,
                chapters,
                text_anchors,
            )?;
            let parsed_output = character_discovery_parsed_output_json(&discovered);
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: CHARACTER_DISCOVERY_CALL_KEY.to_string(),
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
            (discovered, response_content)
        };

        Ok(CharacterDiscoveryCallResult {
            discovered,
            response_content,
            llm_request,
            analyzed_chapters,
        })
    }

    async fn load_character_discovery_checkpoint(
        &self,
        workflow_run_id: i64,
        input_fingerprint_json: &Value,
    ) -> Result<Option<CharacterDiscoveryCallCheckpoint>, AppError> {
        let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(
                workflow_run_id,
                CHARACTER_DISCOVERY_CALL_KEY,
                input_fingerprint_json,
            )
            .await?
        else {
            return Ok(None);
        };
        let checkpoint_json =
            workflow::required_llm_call_checkpoint_json(checkpoint, CHARACTER_DISCOVERY_CALL_KEY)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let response_content = checkpoint_json
            .get("response_content")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "{CHARACTER_DISCOVERY_CALL_KEY}.response_content 必须是字符串"
                ))
            })?;
        let parsed_output = checkpoint_json
            .get("parsed_output")
            .cloned()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "{CHARACTER_DISCOVERY_CALL_KEY}.parsed_output 缺失"
                ))
            })?;
        Ok(Some(CharacterDiscoveryCallCheckpoint {
            response_content,
            parsed_output,
        }))
    }
}

pub(super) fn required_character_discovery_input(
    input: &CharacterDiscoveryInput,
) -> Result<RequiredCharacterDiscoveryInput, AppError> {
    if input.project_id <= 0 {
        return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
    }
    if input
        .chapter_number
        .is_some_and(|chapter_number| chapter_number <= 0)
    {
        return Err(AppError::InvalidArgument(
            "角色发现章节号必须大于 0".to_string(),
        ));
    }
    Ok(RequiredCharacterDiscoveryInput {
        project_id: input.project_id,
        chapter_number: input.chapter_number,
    })
}

pub(super) fn report_character_discovery_progress(
    progress: Option<CharacterDiscoveryProgress<'_>>,
    current: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress {
        progress.report(current, message)?;
    }
    Ok(())
}

fn parse_and_validate_character_discovery_content(
    content: &str,
    analyzed_chapters: &BTreeSet<i32>,
    chapters: &[a4n_core::models::chapter::chapter::Model],
    text_anchors: &[a4n_core::models::project::text_anchor::Model],
) -> Result<Vec<Value>, AppError> {
    let discovered = parse_character_discovery_output(content, chapters, text_anchors)
        .map_err(AppError::InvalidArgument)?;
    validate_discovered_character_chapter_scope(&discovered, analyzed_chapters)
        .map_err(AppError::InvalidArgument)?;
    Ok(discovered)
}
