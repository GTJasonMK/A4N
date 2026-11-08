use std::collections::{BTreeSet, HashMap};

use a4n_api_contract::character::{
    DiscoverCharacterProfilesInput, DiscoverCharacterProfilesResult,
};
use a4n_character_host_contract::validate_character_discovery_input;
use a4n_error::AppError;
use a4n_llm_context::chat_request_with_workflow_run_id;
use a4n_llm_structured_stream::StructuredLlmProgress;
use a4n_progress::StructuredJsonProgressSpec;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    character::discovery::{
        rules::{select_character_discovery_chapters, validate_discovered_character_chapter_scope},
        workflow::{
            character_discovery_call_fingerprint_json, character_discovery_workflow_run_input,
        },
    },
    structured_llm::{
        ChatJsonStreamInput, ensure_checkpoint_parsed_output_matches, stream_error_or_cancelled,
    },
    workflow::{
        self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput,
        workflow_checkpoint_contract_app_error,
    },
};

use super::{
    CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_PROGRESS_TOTAL,
    contracts::{
        CHARACTER_DISCOVERY_FIELDS, CHARACTER_DISCOVERY_LLM_SPAN, ChapterAnchorMode,
        CharacterDiscoveryCallResult, CharacterDiscoveryProgress, CharacterDiscoveryRequestContext,
        CharacterDiscoveryRequestPreview,
    },
    identity::{existing_character_keys, profile_character_keys},
    output::{character_discovery_parsed_output_json, parse_character_discovery_output},
    request::build_character_discovery_request,
};

impl OrchestratorContext {
    pub async fn prepare_character_discovery_request_preview_validated(
        &self,
        input: DiscoverCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterDiscoveryRequestPreview, AppError> {
        validate_character_discovery_input(&input)?;
        let (prepared, request) = self
            .prepare_character_discovery_request(
                input,
                ChapterAnchorMode::RequireExistingCurrent,
                prompt_renderer,
            )
            .await?;
        Ok(CharacterDiscoveryRequestPreview {
            analyzed_chapter_count: prepared.chapters.len(),
            request,
        })
    }

    pub async fn discover_character_profiles_validated(
        &self,
        input: DiscoverCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterDiscoveryProgress<'_>>,
    ) -> Result<DiscoverCharacterProfilesResult, AppError> {
        report_character_discovery_progress(progress, 0, "正在读取可分析章节和已有角色")?;
        validate_character_discovery_input(&input)?;
        let (request_context, request) = self
            .prepare_character_discovery_request(
                input,
                ChapterAnchorMode::EnsureCurrent,
                prompt_renderer,
            )
            .await?;
        let workflow_input = character_discovery_workflow_run_input(
            input.project_id,
            input.chapter_number,
            &request_context.chapters,
            &request_context.text_anchors,
            &request,
            progress.and_then(|progress| progress.active_task_id),
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow_run = self
            .begin_or_resume_workflow_run_validated(workflow_input)
            .await?;
        let result = async {
            let call_result = self
                .run_character_discovery_call(
                    workflow_run.id,
                    input.project_id,
                    input.chapter_number,
                    &request,
                    &request_context.chapters,
                    &request_context.text_anchors,
                    progress,
                )
                .await?;
            let result = self
                .persist_discovered_character_profiles(
                    input.project_id,
                    input.chapter_number,
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
        input: DiscoverCharacterProfilesInput,
        anchor_mode: ChapterAnchorMode,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<
        (
            CharacterDiscoveryRequestContext,
            a4n_llm_provider_contract::provider::ChatRequest,
        ),
        AppError,
    > {
        let (available_chapters, existing_profiles) = futures::join!(
            self.character_sync_chapter_records(input.project_id),
            self.profile_records_by_project(input.project_id),
        );
        let available_chapters = available_chapters?;
        let chapters = select_character_discovery_chapters(
            available_chapters,
            input.project_id,
            input.chapter_number,
        )
        .map_err(AppError::InvalidArgument)?;
        let existing_profiles = existing_profiles?;
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
        let chapter_ids = chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<Vec<_>>();
        let anchor_groups = match anchor_mode {
            ChapterAnchorMode::EnsureCurrent => {
                self.ensure_chapter_content_anchor_record_batch(input.project_id, &chapter_ids)
                    .await?
            }
            ChapterAnchorMode::RequireExistingCurrent => {
                self.require_current_chapter_content_anchor_records_for_preview_batch(
                    input.project_id,
                    &chapter_ids,
                )
                .await?
            }
        };
        for anchors in anchor_groups {
            text_anchors.extend(anchors);
        }
        let request = build_character_discovery_request(
            prompt_renderer,
            input.project_id,
            input.chapter_number,
            &chapters,
            &text_anchors,
            &existing_profiles,
        )
        .await?;
        Ok((
            CharacterDiscoveryRequestContext {
                chapters,
                text_anchors,
                existing_keys,
                existing_key_profiles,
            },
            request,
        ))
    }

    async fn run_character_discovery_call(
        &self,
        workflow_run_id: i64,
        project_id: i64,
        chapter_number: Option<i32>,
        request: &a4n_llm_provider_contract::provider::ChatRequest,
        chapters: &[a4n_domain::chapter::ChapterRecord],
        text_anchors: &[a4n_domain::text_anchor::TextAnchorRecord],
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
            .load_completed_llm_call_checkpoint(
                workflow_run_id,
                CHARACTER_DISCOVERY_CALL_KEY,
                &call_fingerprint_json,
            )
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
            ensure_checkpoint_parsed_output_matches(
                &checkpoint.parsed_output,
                &parsed_output,
                CHARACTER_DISCOVERY_CALL_KEY,
            )?;
            (discovered, checkpoint.response_content)
        } else {
            // 活体信号与字段级里程碑取自同一个 token 流，二者互补而非替代。
            let stream_reporter = progress.and_then(|progress| progress.stream_reporter);
            let mut streamed = String::new();
            let mut stream_sink = stream_reporter.map(|reporter| {
                move |delta: &str| {
                    streamed.push_str(delta);
                    reporter(&streamed);
                }
            });
            let response = self
                .chat_json_stream_result_with_sink(
                    ChatJsonStreamInput {
                        request: llm_request.clone(),
                        prompt_name: super::CHARACTER_DISCOVERY_KIND,
                        truncated_message: "角色发现输出达到当前 max_tokens 上限，返回 JSON 被截断；请在模型配置中提高 max_output_tokens，或减少本次章节上下文后重试",
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
                    stream_sink
                        .as_mut()
                        .map(|sink| sink as &mut (dyn FnMut(&str) + Send)),
                )
                .await
                .map_err(|error| {
                    stream_error_or_cancelled(
                        error,
                        progress.and_then(|progress| progress.cancel_token),
                    )
                })?;
            report_character_discovery_progress(progress, 80, "模型已返回，正在解析发现结果")?;
            let (response, discovered) = self
                .validate_llm_response(
                    &llm_request,
                    crate::structured_llm::structured_chat_response(response),
                    super::CHARACTER_DISCOVERY_KIND,
                    progress.and_then(|progress| progress.cancel_token),
                    |raw| {
                        parse_and_validate_character_discovery_content(
                            raw,
                            &analyzed_chapters,
                            chapters,
                            text_anchors,
                        )
                    },
                )
                .await?;
            let response_content = response.message.content;
            let parsed_output = character_discovery_parsed_output_json(&discovered);
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: CHARACTER_DISCOVERY_CALL_KEY.to_string(),
                call_order: 1,
                input_fingerprint_json: call_fingerprint_json,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output,
                    response_content: response_content.clone(),
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
            (discovered, response_content)
        };

        Ok(CharacterDiscoveryCallResult {
            discovered,
            response_content,
            llm_request,
            analyzed_chapters,
        })
    }
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
    chapters: &[a4n_domain::chapter::ChapterRecord],
    text_anchors: &[a4n_domain::text_anchor::TextAnchorRecord],
) -> Result<Vec<Value>, AppError> {
    let discovered = parse_character_discovery_output(content, chapters, text_anchors)
        .map_err(AppError::LlmError)?;
    validate_discovered_character_chapter_scope(&discovered, analyzed_chapters)
        .map_err(AppError::LlmError)?;
    Ok(discovered)
}
