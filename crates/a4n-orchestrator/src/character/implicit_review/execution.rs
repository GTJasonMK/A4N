use a4n_core::error::AppError;
use a4n_domain::character::ImplicitUpdateDecision;
use a4n_llm::{provider::is_truncated_finish_reason, trace::context_aware_chat_request_trace_json};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    character::{
        AppliedCharacterImplicitDecisionRecord, CharacterAttributeLifecycleError,
        CharacterImplicitReviewDecisionRecordInput, category_attributes,
        implicit_review_rules::{
            AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT, AUTO_IMPLICIT_REVIEW_THRESHOLD,
            AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS, validate_implicit_review_params,
        },
        implicit_review_workflow_rules::implicit_update_review_workflow_run_input,
    },
    workflow::{self, CompletedLlmCallCheckpointInput, workflow_checkpoint_contract_app_error},
};

use super::{
    IMPLICIT_UPDATE_PROMPT_NAME,
    contracts::{
        ApplyImplicitUpdateDecisionInput, BehaviorForReview, CharacterSyncImplicitReviewInput,
        CheckImplicitUpdateInput, CheckImplicitUpdateRequestContext, GetImplicitReviewInput,
        ImplicitAttributeReviewSummary, ImplicitReviewSummary, ImplicitUpdateDecisionSummary,
        ImplicitUpdateLlmRequestInput, ImplicitUpdateReviewRequestPreview,
    },
    output::{
        implicit_update_decision_output_json, implicit_update_review_checkpoint_response,
        parse_implicit_update_decision,
    },
    request::build_implicit_update_request,
    stats::{
        behavior_for_review_from_sync, build_implicit_attribute_review,
        build_implicit_review_summary, implicit_deletion_candidate_keys,
    },
};

impl OrchestratorContext {
    pub async fn implicit_review_summary_validated(
        &self,
        user_id: i64,
        input: GetImplicitReviewInput,
    ) -> Result<ImplicitReviewSummary, AppError> {
        let (threshold, window_chapters, evidence_limit) = validate_implicit_review_params(
            input.threshold,
            input.window_chapters,
            input.evidence_limit,
        )
        .map_err(AppError::InvalidArgument)?;
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        let (profile, records) = self.implicit_review_context(input.profile_id).await?;
        build_implicit_review_summary(
            &profile.current_state,
            &records,
            threshold,
            window_chapters,
            evidence_limit,
        )
        .map_err(AppError::InvalidArgument)
    }

    pub async fn prepare_implicit_update_review_request_preview_validated(
        &self,
        user_id: i64,
        input: CheckImplicitUpdateInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ImplicitUpdateReviewRequestPreview, AppError> {
        let prepared = self
            .prepare_implicit_update_review(user_id, input, prompt_renderer)
            .await?;
        Ok(ImplicitUpdateReviewRequestPreview {
            request: prepared.request,
        })
    }

    pub async fn check_implicit_update_validated(
        &self,
        user_id: i64,
        input: CheckImplicitUpdateInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ImplicitUpdateDecisionSummary, AppError> {
        let threshold = input.threshold;
        let window_chapters = input.window_chapters;
        let evidence_limit = input.evidence_limit;
        let prepared = self
            .prepare_implicit_update_review(user_id, input, prompt_renderer)
            .await?;
        let attribute_key = prepared.stats.attribute_key.clone();
        let call_key = format!(
            "implicit_update_review:{}:{attribute_key}",
            prepared.profile_id
        );
        let request_json = context_aware_chat_request_trace_json(&prepared.request)?;
        let input_fingerprint_json = json!({
            "prompt_names": [IMPLICIT_UPDATE_PROMPT_NAME],
            "request": request_json,
            "input_variables": {
                "profile_id": prepared.profile_id,
                "attribute_key": attribute_key,
                "threshold": threshold,
                "window_chapters": window_chapters,
                "evidence_limit": evidence_limit,
            },
        });
        let workflow_input = implicit_update_review_workflow_run_input(
            prepared.project_id,
            prepared.profile_id,
            &prepared.stats.attribute_key,
            &call_key,
            input_fingerprint_json.clone(),
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow_run = self.begin_workflow_run_validated(workflow_input).await?;
        let result = self
            .run_implicit_review_call(
                prepared.request,
                prepared.current_value,
                prepared.stats,
                RunImplicitReviewCallInput {
                    workflow_run_id: workflow_run.id,
                    call_key,
                    call_order: 1,
                    input_fingerprint_json,
                    truncated_message: "隐性属性复审输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次复审上下文后重试",
                },
            )
            .await;
        self.finalize_workflow_result(workflow_run.id, result, "隐性属性复审")
            .await
    }

    pub(crate) async fn decide_implicit_reviews_for_sync_validated(
        &self,
        input: CharacterSyncImplicitReviewInput<'_>,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ImplicitUpdateDecisionSummary>, AppError> {
        let (threshold, window_chapters, evidence_limit) = validate_implicit_review_params(
            AUTO_IMPLICIT_REVIEW_THRESHOLD,
            AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS,
            AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT,
        )
        .map_err(AppError::InvalidArgument)?;
        let mut records = input
            .behaviors
            .iter()
            .enumerate()
            .map(|(index, behavior)| behavior_for_review_from_sync(index, behavior))
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::InvalidArgument)?;
        records.sort_by(|left, right| {
            right
                .chapter_number
                .cmp(&left.chapter_number)
                .then_with(|| left.behavior_id.cmp(&right.behavior_id))
        });
        let existing = self
            .behavior_records_before_chapter(input.profile.id, input.start_chapter)
            .await?;
        records.extend(existing.iter().map(BehaviorForReview::from));
        if records.is_empty() {
            return Ok(Vec::new());
        }

        let excluded_keys = implicit_deletion_candidate_keys(input.deletion_candidates)
            .map_err(AppError::InvalidArgument)?;
        let review = build_implicit_review_summary(
            input.final_attributes,
            &records,
            threshold,
            window_chapters,
            evidence_limit,
        )
        .map_err(AppError::InvalidArgument)?;
        let mut decisions = Vec::new();
        for stats in review.attributes.into_iter().filter(|stats| {
            stats.threshold_reached && !excluded_keys.contains(&stats.attribute_key)
        }) {
            let call_index = decisions.len();
            let call_key = format!(
                "implicit_update_review:{}:{}:{}",
                input.profile.id, input.start_chapter, stats.attribute_key
            );
            let workflow_run_id =
                workflow::required_llm_call_workflow_run_id(input.workflow_run_id, &call_key)
                    .map_err(workflow_checkpoint_contract_app_error)?;
            let request = build_implicit_update_request(
                prompt_renderer,
                ImplicitUpdateLlmRequestInput {
                    project_id: input.profile.project_id,
                    profile_id: input.profile.id,
                    profile_name: &input.profile.name,
                    attribute_key: &stats.attribute_key,
                    current_value: &stats.current_value,
                    stats: &stats,
                    threshold,
                    window_chapters,
                    source_refs_json: json!({
                        "trigger": "character_sync_auto_review",
                        "profile_id": input.profile.id,
                        "start_chapter": input.start_chapter,
                        "attribute_key": stats.attribute_key,
                        "window_chapters": window_chapters,
                        "evidence_limit": evidence_limit,
                    }),
                    call_key: call_key.clone(),
                },
            )
            .await?;
            let request_json = context_aware_chat_request_trace_json(&request)?;
            let input_fingerprint_json = json!({
                "request": request_json,
                "profile_id": input.profile.id,
                "start_chapter": input.start_chapter,
                "attribute_key": stats.attribute_key,
                "stats": stats,
                "threshold": threshold,
                "window_chapters": window_chapters,
                "evidence_limit": evidence_limit,
            });
            let call_order = i32::try_from(call_index)
                .map_err(|_| {
                    AppError::InvalidArgument(format!(
                        "隐性属性更新决策数量超过 i32 范围：{call_index}"
                    ))
                })?
                .checked_add(2000)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "隐性属性更新决策 call_order 超过 i32 范围：{call_index}"
                    ))
                })?;
            let current_value = stats.current_value.clone();
            decisions.push(
                self.run_implicit_review_call(
                    request,
                    current_value,
                    stats,
                    RunImplicitReviewCallInput {
                        workflow_run_id,
                        call_key,
                        call_order,
                        input_fingerprint_json,
                        truncated_message: "隐性属性更新决策输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次决策上下文后重试",
                    },
                )
                .await?,
            );
        }
        Ok(decisions)
    }

    pub async fn apply_implicit_update_decision_validated(
        &self,
        user_id: i64,
        input: ApplyImplicitUpdateDecisionInput,
    ) -> Result<AppliedCharacterImplicitDecisionRecord, AppError> {
        let decision = input.decision;
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        self.apply_implicit_review_decision_record(CharacterImplicitReviewDecisionRecordInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            attribute_key: decision.attribute_key,
            current_value: decision.current_value,
            decision: decision.decision.as_str().to_string(),
            suggested_new_value: decision.suggested_new_value,
            reasoning: decision.reasoning,
            evidence_summary: decision.evidence_summary,
        })
        .await
        .map_err(character_attribute_lifecycle_app_error)
    }

    async fn prepare_implicit_update_review(
        &self,
        user_id: i64,
        input: CheckImplicitUpdateInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CheckImplicitUpdateRequestContext, AppError> {
        let attribute_key = input.attribute_key.trim();
        if attribute_key.is_empty() {
            return Err(AppError::InvalidArgument(
                "隐性属性 key 不能为空".to_string(),
            ));
        }
        let (threshold, window_chapters, evidence_limit) = validate_implicit_review_params(
            input.threshold,
            input.window_chapters,
            input.evidence_limit,
        )
        .map_err(AppError::InvalidArgument)?;
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        let (profile, records) = self.implicit_review_context(input.profile_id).await?;
        let current_value = implicit_attribute_value(&profile.current_state, attribute_key)?;
        let stats = build_implicit_attribute_review(
            attribute_key,
            current_value.clone(),
            &records,
            threshold,
            window_chapters,
            evidence_limit,
        )
        .map_err(AppError::InvalidArgument)?;
        let request = build_implicit_update_request(
            prompt_renderer,
            ImplicitUpdateLlmRequestInput {
                project_id: profile.project_id,
                profile_id: profile.id,
                profile_name: &profile.name,
                attribute_key,
                current_value: &current_value,
                stats: &stats,
                threshold,
                window_chapters,
                source_refs_json: json!({
                    "trigger": "manual_review",
                    "profile_id": profile.id,
                    "attribute_key": attribute_key,
                    "window_chapters": window_chapters,
                    "evidence_limit": evidence_limit,
                }),
                call_key: format!("implicit_update_review:{}:{attribute_key}", profile.id),
            },
        )
        .await?;
        Ok(CheckImplicitUpdateRequestContext {
            profile_id: profile.id,
            project_id: profile.project_id,
            current_value,
            stats,
            request,
        })
    }

    async fn implicit_review_context(
        &self,
        profile_id: i64,
    ) -> Result<
        (
            a4n_core::models::character::protagonist_profile::Model,
            Vec<BehaviorForReview>,
        ),
        AppError,
    > {
        let profile = self
            .profile_by_id(profile_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("角色档案不存在: {profile_id}")))?;
        let records = self.implicit_review_behavior_records(profile_id).await?;
        Ok((
            profile,
            records.iter().map(BehaviorForReview::from).collect(),
        ))
    }

    async fn run_implicit_review_call(
        &self,
        mut request: a4n_llm::provider::ChatRequest,
        current_value: Value,
        stats: ImplicitAttributeReviewSummary,
        input: RunImplicitReviewCallInput<'_>,
    ) -> Result<ImplicitUpdateDecisionSummary, AppError> {
        workflow::attach_llm_call_checkpoint_trace(
            &mut request,
            input.workflow_run_id,
            &input.call_key,
            &input.input_fingerprint_json,
        )?;
        self.mark_workflow_current_step(input.workflow_run_id, &input.call_key)
            .await?;
        let response_text = if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(
                input.workflow_run_id,
                &input.call_key,
                &input.input_fingerprint_json,
            )
            .await?
        {
            let checkpoint_json =
                workflow::required_llm_call_checkpoint_json(checkpoint, &input.call_key)
                    .map_err(workflow_checkpoint_contract_app_error)?;
            let (response_text, _, _) = implicit_update_review_checkpoint_response(
                &checkpoint_json,
                &input.call_key,
                &stats,
            )
            .map_err(AppError::InvalidArgument)?;
            response_text
        } else {
            let response = self.stream_complete_llm(request).await?;
            if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                return Err(AppError::InvalidArgument(
                    input.truncated_message.to_string(),
                ));
            }
            let response_text = response.message.content;
            let parsed = parse_implicit_update_decision(&response_text, &stats)
                .map_err(AppError::InvalidArgument)?;
            let parsed_output = implicit_update_decision_output_json(&parsed);
            let usage = response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?;
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id: input.workflow_run_id,
                call_key: input.call_key.clone(),
                call_order: input.call_order,
                input_fingerprint_json: input.input_fingerprint_json,
                checkpoint_json: json!({
                    "output": parsed_output.clone(),
                    "parsed_output": parsed_output,
                    "response_content": response_text.clone(),
                    "usage": usage,
                    "finish_reason": response.finish_reason,
                }),
                artifacts_json: None,
            })
            .await?;
            response_text
        };
        let parsed = parse_implicit_update_decision(&response_text, &stats)
            .map_err(AppError::InvalidArgument)?;
        Ok(ImplicitUpdateDecisionSummary {
            attribute_key: stats.attribute_key.clone(),
            current_value,
            decision: ImplicitUpdateDecision::parse(&parsed.decision).ok_or_else(|| {
                AppError::InvalidArgument(format!("未知隐性属性复审决策: {}", parsed.decision))
            })?,
            reasoning: parsed.reasoning,
            suggested_new_value: parsed.suggested_new_value,
            evidence_summary: parsed.evidence_summary,
        })
    }
}

struct RunImplicitReviewCallInput<'a> {
    workflow_run_id: i64,
    call_key: String,
    call_order: i32,
    input_fingerprint_json: Value,
    truncated_message: &'a str,
}

fn character_attribute_lifecycle_app_error(error: CharacterAttributeLifecycleError) -> AppError {
    match error {
        CharacterAttributeLifecycleError::Persistence(error) => error,
        CharacterAttributeLifecycleError::Rag(message) => {
            AppError::Internal(anyhow::anyhow!(message))
        }
    }
}

fn implicit_attribute_value(current_state: &Value, attribute_key: &str) -> Result<Value, AppError> {
    category_attributes(current_state, "implicit")
        .map_err(AppError::InvalidArgument)?
        .get(attribute_key)
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument(format!("隐性属性不存在: {attribute_key}")))
}
