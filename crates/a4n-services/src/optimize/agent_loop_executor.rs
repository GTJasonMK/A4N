use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use a4n_engine::llm::provider::{ChatMessage, ChatResponse, LlmProvider};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{Value, json};

use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use crate::workflow::WorkflowRunService;

use super::agent_loop::{
    AGENT_LOOP_RUNAWAY_GUARD_STEPS, AgentLoopAction, AgentLoopDecision, agent_loop_request_payload,
    build_agent_loop_chat_request, failed_tool_observation, final_issues,
    final_response_from_issues, parse_agent_loop_decision, run_tool_arguments,
    thinking_observation, tool_observation,
};
use super::messages::{
    materialize_optimization_issue_evidence, parse_content_optimization_issues,
    parse_optimization_issues,
};
use super::runtime_tools::{execute_optimization_runtime_tool, runtime_tool_reads_current_chapter};
use super::turns::{
    EVENT_KIND_ASSISTANT_UPDATE, EVENT_KIND_ERROR, EVENT_KIND_TOOL_CALL, EVENT_KIND_TOOL_RESULT,
    EVENT_STATUS_COMPLETED, EVENT_STATUS_FAILED, EVENT_STATUS_RUNNING,
};
use super::{
    OptimizeLlmStream, OptimizeProgress, OptimizeService, collect_streaming_chat_response,
    optimize_llm_call_input_fingerprint_json, report_optimize_progress,
    response_content_from_checkpoint,
};

pub(super) struct AgentLoopDecisionRequest<'a> {
    pub(super) project_id: i64,
    pub(super) workflow: &'a str,
    pub(super) phase: &'a str,
    pub(super) system_prompt: &'a str,
    pub(super) base_user_message: &'a str,
    pub(super) observations: &'a [Value],
    pub(super) step: usize,
    pub(super) build_error_label: &'a str,
    pub(super) request_event_label: &'a str,
    pub(super) llm_error_label: &'a str,
    pub(super) parse_error_label: &'a str,
    pub(super) progress_current: u32,
    pub(super) progress_total: u32,
    pub(super) progress_message: &'a str,
    pub(super) release_operation: Option<&'a str>,
    pub(super) cancel_session_on_progress_error: bool,
    pub(super) workflow_run_id: i64,
}

pub(super) struct AgentLoopThinkObservationInput<'a> {
    pub(super) turn_id: i64,
    pub(super) session_id: i64,
    pub(super) step: usize,
    pub(super) decision: &'a AgentLoopDecision,
    pub(super) think_event_prefix: &'a str,
    pub(super) error_context: &'a str,
    pub(super) release_operation: Option<&'a str>,
}

pub(super) struct FailedAgentLoopToolInput<'a> {
    pub(super) turn_id: i64,
    pub(super) session_id: i64,
    pub(super) step: usize,
    pub(super) tool_name: &'a str,
    pub(super) tool_arguments: Value,
    pub(super) err: &'a AppError,
    pub(super) tool_event_prefix: &'a str,
}

pub(super) struct ExplicitAnalysisAgentLoopInput<'a, 'progress, 'stream> {
    pub(super) turn_id: i64,
    pub(super) session: optimization_session::Model,
    pub(super) analysis_prompt: String,
    pub(super) base_user_message: String,
    pub(super) analysis_context: Value,
    pub(super) llm: &'a dyn LlmProvider,
    pub(super) workflow_run_id: i64,
    pub(super) progress: &'a mut Option<OptimizeProgress<'progress>>,
    pub(super) stream: &'a mut Option<OptimizeLlmStream<'stream>>,
}

fn agent_loop_decision_json(decision: &AgentLoopDecision) -> Value {
    json!({
        "thought": decision.thought,
        "public_update": decision.public_update,
        "action": decision.action.as_str(),
        "arguments": decision.arguments,
    })
}

fn restored_agent_loop_decision(
    checkpoint_json: Value,
    call_key: &str,
) -> Result<(ChatResponse, AgentLoopDecision), AppError> {
    let (response_content, usage, finish_reason) =
        response_content_from_checkpoint(&checkpoint_json, call_key)?;
    let decision = parse_agent_loop_decision(&response_content)?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_parsed_output".to_string(),
            operation: format!("读取内容优化 Agent Loop 调用断点 {call_key}"),
        })?;
    let reparsed_output = agent_loop_decision_json(&decision);
    if parsed_output != reparsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("恢复内容优化 Agent Loop 调用断点 {call_key}"),
        });
    }
    Ok((
        ChatResponse {
            message: ChatMessage::assistant(response_content),
            usage: usage
                .map(serde_json::from_value)
                .transpose()
                .map_err(|err| AppError::InvalidState {
                    current: "invalid_checkpoint_usage".to_string(),
                    operation: format!("恢复内容优化 Agent Loop 调用断点 {call_key}: {err}"),
                })?,
            finish_reason,
        },
        decision,
    ))
}

impl OptimizeService {
    pub(super) async fn request_agent_loop_decision(
        &self,
        turn_id: i64,
        session_id: i64,
        llm: &dyn LlmProvider,
        progress: &mut Option<OptimizeProgress<'_>>,
        stream: &mut Option<OptimizeLlmStream<'_>>,
        request: AgentLoopDecisionRequest<'_>,
    ) -> Result<AgentLoopDecision, AppError> {
        let mut chat_request = match build_agent_loop_chat_request(
            request.project_id,
            Some(session_id),
            request.workflow,
            request.system_prompt.to_string(),
            request.base_user_message,
            request.observations,
            request.step,
        ) {
            Ok(request) => request,
            Err(err) => {
                self.save_agent_event(
                    session_id,
                    &format!(
                        "构建 {} 第 {} 步输入失败：{err}",
                        request.build_error_label, request.step
                    ),
                )
                .await?;
                if let Some(operation) = request.release_operation {
                    self.release_in_progress_session_after_failure(
                        session_id,
                        operation,
                        &err.to_string(),
                    )
                    .await?;
                }
                return Err(err);
            }
        };
        let call_key = format!(
            "optimization_session:{}:{}:step:{}",
            session_id, request.workflow, request.step
        );
        if let Some(trace_context) = chat_request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(request.workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = None;
        }
        let input_fingerprint_json = optimize_llm_call_input_fingerprint_json(&chat_request)?;
        let input_fingerprint_sha256 =
            llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
        if let Some(trace_context) = chat_request.trace_context.as_mut() {
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        self.save_agent_event(
            session_id,
            &format!(
                "{} 第 {} 步：请求下一步决策。",
                request.request_event_label, request.step
            ),
        )
        .await?;
        let llm_request_payload = agent_loop_request_payload(&chat_request)?;
        self.record_llm_request(session_id, request.phase, &llm_request_payload)
            .await?;
        WorkflowRunService::new(self.db.clone())
            .mark_current_step(request.workflow_run_id, &call_key)
            .await?;
        if let Err(err) = report_optimize_progress(
            progress,
            request.progress_current,
            request.progress_total,
            request.progress_message,
        ) {
            if request.cancel_session_on_progress_error {
                self.cancel_session(session_id).await?;
            }
            return Err(err);
        }

        let (response, decision, restored) = if let Some(checkpoint) =
            load_completed_llm_call_checkpoint(
                self.db.clone(),
                request.workflow_run_id,
                &call_key,
                &input_fingerprint_json,
            )
            .await?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
            let (response, decision) = restored_agent_loop_decision(checkpoint_json, &call_key)?;
            self.save_agent_event(
                session_id,
                &format!(
                    "已恢复 {} 第 {} 步 LLM 调用断点。",
                    request.phase, request.step
                ),
            )
            .await?;
            (response, decision, true)
        } else {
            let response = match collect_streaming_chat_response(llm, chat_request, stream).await {
                Ok(response) => response,
                Err(err) => {
                    self.append_turn_event(
                        turn_id,
                        EVENT_KIND_ERROR,
                        EVENT_STATUS_FAILED,
                        &format!("LLM 调用失败：{err}"),
                        Some(json!({
                            "step": request.step,
                            "phase": request.phase,
                        })),
                    )
                    .await?;
                    self.save_agent_event(
                        session_id,
                        &format!("{} 调用失败：{err}", request.llm_error_label),
                    )
                    .await?;
                    if let Some(operation) = request.release_operation {
                        self.release_in_progress_session_after_failure(
                            session_id,
                            operation,
                            &err.to_string(),
                        )
                        .await?;
                    }
                    return Err(err);
                }
            };
            let decision = match parse_agent_loop_decision(&response.message.content) {
                Ok(decision) => decision,
                Err(err) => {
                    self.append_turn_event(
                        turn_id,
                        EVENT_KIND_ERROR,
                        EVENT_STATUS_FAILED,
                        &format!("模型决策解析失败：{err}"),
                        Some(json!({
                            "step": request.step,
                            "phase": request.phase,
                        })),
                    )
                    .await?;
                    self.save_agent_event(
                        session_id,
                        &format!("{} 决策解析失败：{err}", request.parse_error_label),
                    )
                    .await?;
                    if let Some(operation) = request.release_operation {
                        self.release_in_progress_session_after_failure(
                            session_id,
                            operation,
                            &err.to_string(),
                        )
                        .await?;
                    }
                    return Err(err);
                }
            };
            (response, decision, false)
        };
        self.record_llm_response(session_id, request.phase, &response.message.content)
            .await?;
        if !restored {
            let usage = response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?;
            save_completed_llm_call_checkpoint(
                self.db.clone(),
                CompletedLlmCallCheckpointInput {
                    workflow_run_id: request.workflow_run_id,
                    call_key,
                    call_order: request.step as i32,
                    input_fingerprint_json,
                    checkpoint_json: json!({
                        "output": agent_loop_decision_json(&decision),
                        "parsed_output": agent_loop_decision_json(&decision),
                        "response_content": response.message.content,
                        "usage": usage,
                        "finish_reason": response.finish_reason,
                    }),
                    artifacts_json: None,
                },
            )
            .await?;
        }
        Ok(decision)
    }

    pub(super) async fn record_agent_loop_think_observation(
        &self,
        input: AgentLoopThinkObservationInput<'_>,
    ) -> Result<Value, AppError> {
        let AgentLoopThinkObservationInput {
            turn_id,
            session_id,
            step,
            decision,
            think_event_prefix,
            error_context,
            release_operation,
        } = input;
        self.append_turn_event(
            turn_id,
            EVENT_KIND_ASSISTANT_UPDATE,
            EVENT_STATUS_COMPLETED,
            decision.public_update_for_non_final()?,
            Some(json!({
                "step": step,
                "action": decision.action.as_str(),
                "arguments": decision.arguments.clone(),
            })),
        )
        .await?;

        let observation = match thinking_observation(step, &decision.thought, &decision.arguments) {
            Ok(observation) => observation,
            Err(err) => {
                self.append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("思考参数无效：{err}"),
                    Some(json!({
                        "step": step,
                        "action": decision.action.as_str(),
                        "arguments": decision.arguments.clone(),
                    })),
                )
                .await?;
                self.save_agent_event(
                    session_id,
                    &format!("{error_context} think 参数无效：{err}"),
                )
                .await?;
                if let Some(operation) = release_operation {
                    self.release_in_progress_session_after_failure(
                        session_id,
                        operation,
                        &err.to_string(),
                    )
                    .await?;
                }
                return Err(err);
            }
        };

        self.save_agent_event(
            session_id,
            &format!(
                "{think_event_prefix}第 {step} 步：{}。",
                observation.event_summary
            ),
        )
        .await?;
        Ok(observation.value)
    }

    pub(super) async fn prepare_agent_loop_tool_call(
        &self,
        turn_id: i64,
        session_id: i64,
        step: usize,
        decision: &super::agent_loop::AgentLoopDecision,
        error_context: &str,
        release_operation: Option<&str>,
    ) -> Result<(String, Value), AppError> {
        let (tool_name, tool_arguments) = match run_tool_arguments(&decision.arguments) {
            Ok(value) => value,
            Err(err) => {
                self.append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("工具调用参数无效：{err}"),
                    Some(json!({
                        "step": step,
                        "action": decision.action.as_str(),
                        "arguments": decision.arguments.clone(),
                    })),
                )
                .await?;
                self.save_agent_event(
                    session_id,
                    &format!("{error_context} run_tool 参数无效：{err}"),
                )
                .await?;
                if let Some(operation) = release_operation {
                    self.release_in_progress_session_after_failure(
                        session_id,
                        operation,
                        &err.to_string(),
                    )
                    .await?;
                }
                return Err(err);
            }
        };
        let tool_name = tool_name.to_string();
        self.append_turn_event(
            turn_id,
            EVENT_KIND_ASSISTANT_UPDATE,
            EVENT_STATUS_COMPLETED,
            decision.public_update_for_non_final()?,
            Some(json!({
                "step": step,
                "action": decision.action.as_str(),
                "tool": tool_name,
                "arguments": tool_arguments.clone(),
            })),
        )
        .await?;
        self.append_turn_event(
            turn_id,
            EVENT_KIND_TOOL_CALL,
            EVENT_STATUS_RUNNING,
            &format!("调用工具：{tool_name}"),
            Some(json!({
                "step": step,
                "tool": tool_name,
                "arguments": tool_arguments.clone(),
            })),
        )
        .await?;
        Ok((tool_name, tool_arguments))
    }

    pub(super) async fn record_failed_agent_loop_tool(
        &self,
        input: FailedAgentLoopToolInput<'_>,
    ) -> Result<Value, AppError> {
        let FailedAgentLoopToolInput {
            turn_id,
            session_id,
            step,
            tool_name,
            tool_arguments,
            err,
            tool_event_prefix,
        } = input;
        let observation = failed_tool_observation(
            step,
            &AgentLoopAction::RunTool,
            tool_name,
            tool_arguments,
            err,
        );
        self.append_turn_event(
            turn_id,
            EVENT_KIND_TOOL_RESULT,
            EVENT_STATUS_FAILED,
            &format!("工具 {tool_name} 执行失败：{err}"),
            Some(observation.clone()),
        )
        .await?;
        self.save_message_with_tool_calls(
            session_id,
            "tool",
            &serde_json::to_string_pretty(&observation)?,
            Some(json!({
                "tool": observation["tool"].clone(),
                "step": step,
            })),
        )
        .await?;
        self.save_agent_event(
            session_id,
            &format!("{tool_event_prefix}工具 {tool_name} 执行失败，已交回 Agent 决策：{err}"),
        )
        .await?;
        Ok(observation)
    }

    pub(super) async fn record_successful_agent_loop_tool(
        &self,
        turn_id: i64,
        session_id: i64,
        step: usize,
        tool_name: &str,
        observation: &Value,
        tool_event_prefix: &str,
    ) -> Result<(), AppError> {
        self.append_turn_event(
            turn_id,
            EVENT_KIND_TOOL_RESULT,
            EVENT_STATUS_COMPLETED,
            &format!("工具 {tool_name} 已返回结果。"),
            Some(observation.clone()),
        )
        .await?;
        self.save_message_with_tool_calls(
            session_id,
            "tool",
            &serde_json::to_string_pretty(observation)?,
            Some(json!({
                "tool": tool_name,
                "step": step,
            })),
        )
        .await?;
        self.save_agent_event(
            session_id,
            &format!("{tool_event_prefix}工具 {tool_name} 已返回观察结果。"),
        )
        .await?;
        Ok(())
    }

    pub(super) async fn run_explicit_analysis_agent_loop(
        &self,
        input: ExplicitAnalysisAgentLoopInput<'_, '_, '_>,
    ) -> Result<optimization_session::Model, AppError> {
        let ExplicitAnalysisAgentLoopInput {
            turn_id,
            session,
            analysis_prompt,
            base_user_message,
            analysis_context,
            llm,
            workflow_run_id,
            progress,
            stream,
        } = input;
        self.save_agent_event(
            session.id,
            "启动显式 Agent Loop：由模型选择读取上下文、调用工具或输出 final。",
        )
        .await?;
        if let Err(err) = report_optimize_progress(progress, 1, 5, "正在准备 Agent Loop") {
            self.cancel_session(session.id).await?;
            return Err(err);
        }

        let mut observations = Vec::new();
        let mut has_read_current_chapter = false;
        for step in 1..=AGENT_LOOP_RUNAWAY_GUARD_STEPS {
            let phase = format!("内容优化 Agent Loop #{step}");
            let progress_message = format!("正在等待 Agent 第 {step} 步决策");
            let decision = self
                .request_agent_loop_decision(
                    turn_id,
                    session.id,
                    llm,
                    progress,
                    stream,
                    AgentLoopDecisionRequest {
                        project_id: session.project_id,
                        workflow: "content_optimization_agent_loop",
                        phase: &phase,
                        system_prompt: &analysis_prompt,
                        base_user_message: &base_user_message,
                        observations: &observations,
                        step,
                        build_error_label: "Agent Loop",
                        request_event_label: "Agent Loop",
                        llm_error_label: "LLM Agent Loop",
                        parse_error_label: "Agent Loop",
                        progress_current: 2,
                        progress_total: 5,
                        progress_message: &progress_message,
                        release_operation: Some("内容优化 Agent Loop"),
                        cancel_session_on_progress_error: true,
                        workflow_run_id,
                    },
                )
                .await?;
            let decision_event = match decision.action {
                AgentLoopAction::Think | AgentLoopAction::RunTool => format!(
                    "Agent 第 {step} 步：{}；动作：{}。",
                    decision.public_update_for_non_final()?,
                    decision.action.as_str()
                ),
                AgentLoopAction::Final => format!("Agent 第 {step} 步：final。"),
            };
            self.save_agent_event(session.id, &decision_event).await?;

            match decision.action {
                AgentLoopAction::Think => {
                    observations.push(
                        self.record_agent_loop_think_observation(AgentLoopThinkObservationInput {
                            turn_id,
                            session_id: session.id,
                            step,
                            decision: &decision,
                            think_event_prefix: "Agent 思考",
                            error_context: "Agent Loop",
                            release_operation: Some("内容优化 Agent Loop"),
                        })
                        .await?,
                    );
                }
                AgentLoopAction::RunTool => {
                    let (tool_name, tool_arguments) = self
                        .prepare_agent_loop_tool_call(
                            turn_id,
                            session.id,
                            step,
                            &decision,
                            "Agent Loop",
                            Some("内容优化 Agent Loop"),
                        )
                        .await?;
                    let result = match execute_optimization_runtime_tool(
                        &tool_name,
                        tool_arguments.clone(),
                        &analysis_context,
                        session.id,
                        session.project_id,
                        session.chapter_id,
                    ) {
                        Ok(result) => result,
                        Err(err) => {
                            observations.push(
                                self.record_failed_agent_loop_tool(FailedAgentLoopToolInput {
                                    turn_id,
                                    session_id: session.id,
                                    step,
                                    tool_name: &tool_name,
                                    tool_arguments,
                                    err: &err,
                                    tool_event_prefix: "",
                                })
                                .await?,
                            );
                            continue;
                        }
                    };
                    if runtime_tool_reads_current_chapter(&tool_name, &tool_arguments) {
                        has_read_current_chapter = true;
                    }
                    let observation = tool_observation(
                        step,
                        &AgentLoopAction::RunTool,
                        &tool_name,
                        tool_arguments,
                        result,
                    );
                    self.record_successful_agent_loop_tool(
                        turn_id,
                        session.id,
                        step,
                        &tool_name,
                        &observation,
                        "",
                    )
                    .await?;
                    observations.push(observation);
                }
                AgentLoopAction::Final => {
                    return self
                        .finish_explicit_analysis_agent_loop(
                            session,
                            decision.arguments,
                            &analysis_context,
                            has_read_current_chapter,
                            progress,
                        )
                        .await;
                }
            }
        }

        let err = AppError::InvalidArgument(
            "内容优化 Agent Loop 未收敛：模型持续请求新动作但没有返回 final".to_string(),
        );
        self.append_turn_event(
            turn_id,
            EVENT_KIND_ERROR,
            EVENT_STATUS_FAILED,
            &err.to_string(),
            None,
        )
        .await?;
        self.save_agent_event(session.id, &format!("Agent Loop 未完成：{err}"))
            .await?;
        self.release_in_progress_session_after_failure(
            session.id,
            "内容优化 Agent Loop",
            &err.to_string(),
        )
        .await?;
        Err(err)
    }

    async fn finish_explicit_analysis_agent_loop(
        &self,
        session: optimization_session::Model,
        final_arguments: Value,
        analysis_context: &Value,
        has_read_current_chapter: bool,
        progress: &mut Option<OptimizeProgress<'_>>,
    ) -> Result<optimization_session::Model, AppError> {
        if !has_read_current_chapter {
            let err = AppError::InvalidArgument(
                "Agent Loop 在读取 current_chapter 前不能返回 final".to_string(),
            );
            self.save_agent_event(session.id, &format!("Agent Loop final 前置条件失败：{err}"))
                .await?;
            self.release_in_progress_session_after_failure(
                session.id,
                "内容优化 Agent Loop",
                &err.to_string(),
            )
            .await?;
            return Err(err);
        }

        let issues = match final_issues(&final_arguments) {
            Ok(issues) => issues,
            Err(err) => {
                self.save_agent_event(session.id, &format!("Agent Loop final 参数无效：{err}"))
                    .await?;
                self.release_in_progress_session_after_failure(
                    session.id,
                    "内容优化 Agent Loop",
                    &err.to_string(),
                )
                .await?;
                return Err(err);
            }
        };
        let assistant_response = final_response_from_issues(issues)?;
        self.save_agent_event(session.id, "Agent 已返回 final，正在保存 assistant 回复。")
            .await?;
        self.save_message(session.id, "assistant", &assistant_response)
            .await?;
        if let Err(err) = report_optimize_progress(progress, 3, 5, "Agent 已返回 final") {
            self.cancel_session(session.id).await?;
            return Err(err);
        }

        report_optimize_progress(progress, 4, 5, "正在解析问题列表")?;
        let issues = match parse_content_optimization_issues(&assistant_response) {
            Ok(issues) => issues,
            Err(err) => {
                self.save_agent_event(session.id, &format!("分析结果解析失败：{err}"))
                    .await?;
                self.release_in_progress_session_after_failure(
                    session.id,
                    "内容优化 Agent Loop",
                    &err.to_string(),
                )
                .await?;
                return Err(err);
            }
        };
        let parsed_issues = match parse_optimization_issues(&issues) {
            Ok(issues) => issues,
            Err(err) => {
                self.save_agent_event(session.id, &format!("分析问题契约解析失败：{err}"))
                    .await?;
                self.release_in_progress_session_after_failure(
                    session.id,
                    "内容优化 Agent Loop",
                    &err.to_string(),
                )
                .await?;
                return Err(err);
            }
        };
        let parsed_issues =
            match materialize_optimization_issue_evidence(parsed_issues, analysis_context) {
                Ok(issues) => issues,
                Err(err) => {
                    self.save_agent_event(session.id, &format!("分析问题证据解析失败：{err}"))
                        .await?;
                    self.release_in_progress_session_after_failure(
                        session.id,
                        "内容优化 Agent Loop",
                        &err.to_string(),
                    )
                    .await?;
                    return Err(err);
                }
            };
        let parsed_issue_count = parsed_issues.len();
        let canonical_issues = serde_json::to_value(parsed_issues)?;

        let mut active: optimization_session::ActiveModel = session.into();
        active.issues = Set(Some(canonical_issues));
        active.status = Set("waiting_user".to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        self.save_agent_event(
            updated.id,
            &format!("分析结果已解析并保存，共发现 {parsed_issue_count} 个问题。"),
        )
        .await?;
        report_optimize_progress(progress, 5, 5, "优化分析结果已保存")?;
        Ok(updated)
    }
}
