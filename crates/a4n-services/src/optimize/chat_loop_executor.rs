use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use a4n_engine::llm::provider::LlmProvider;
use serde_json::{Value, json};

use super::agent_loop::{
    AGENT_LOOP_RUNAWAY_GUARD_STEPS, AgentLoopAction, final_reply, tool_observation,
};
use super::agent_loop_executor::{
    AgentLoopDecisionRequest, AgentLoopThinkObservationInput, FailedAgentLoopToolInput,
};
use super::runtime_tools::execute_optimization_runtime_tool;
use super::turns::{EVENT_KIND_ERROR, EVENT_STATUS_FAILED};
use super::{OptimizeLlmStream, OptimizeProgress, OptimizeService, report_optimize_progress};

pub(super) struct ChatAgentLoopInput<'a, 'progress, 'stream> {
    pub(super) turn_id: i64,
    pub(super) session: &'a optimization_session::Model,
    pub(super) system_prompt: String,
    pub(super) base_user_message: String,
    pub(super) chat_context: Value,
    pub(super) llm: &'a dyn LlmProvider,
    pub(super) workflow_run_id: i64,
    pub(super) progress: &'a mut Option<OptimizeProgress<'progress>>,
    pub(super) stream: &'a mut Option<OptimizeLlmStream<'stream>>,
}

impl OptimizeService {
    pub(super) async fn run_chat_agent_loop(
        &self,
        input: ChatAgentLoopInput<'_, '_, '_>,
    ) -> Result<String, AppError> {
        let ChatAgentLoopInput {
            turn_id,
            session,
            system_prompt,
            base_user_message,
            chat_context,
            llm,
            workflow_run_id,
            progress,
            stream,
        } = input;
        self.save_agent_event(
            session.id,
            "启动普通对话 Agent Loop：由模型决定直接回复、读取上下文或调用工具。",
        )
        .await?;

        let mut observations = Vec::new();
        for step in 1..=AGENT_LOOP_RUNAWAY_GUARD_STEPS {
            let phase = format!("内容优化对话 Agent Loop #{step}");
            let progress_message = format!("正在等待 Agent 对话第 {step} 步决策");
            let decision = self
                .request_agent_loop_decision(
                    turn_id,
                    session.id,
                    llm,
                    progress,
                    stream,
                    AgentLoopDecisionRequest {
                        project_id: session.project_id,
                        workflow: "content_optimization_chat_agent_loop",
                        phase: &phase,
                        system_prompt: &system_prompt,
                        base_user_message: &base_user_message,
                        observations: &observations,
                        step,
                        build_error_label: "普通对话 Agent Loop",
                        request_event_label: "普通对话 Agent Loop",
                        llm_error_label: "LLM 普通对话 Loop",
                        parse_error_label: "普通对话 Loop",
                        progress_current: 2,
                        progress_total: 4,
                        progress_message: &progress_message,
                        release_operation: None,
                        cancel_session_on_progress_error: false,
                        workflow_run_id,
                    },
                )
                .await?;
            let decision_event = match decision.action {
                AgentLoopAction::Think | AgentLoopAction::RunTool => format!(
                    "普通对话第 {step} 步：{}；动作：{}。",
                    decision.public_update_for_non_final()?,
                    decision.action.as_str()
                ),
                AgentLoopAction::Final => format!("普通对话第 {step} 步：final。"),
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
                            think_event_prefix: "普通对话思考",
                            error_context: "普通对话",
                            release_operation: None,
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
                            "普通对话",
                            None,
                        )
                        .await?;
                    let result = match execute_optimization_runtime_tool(
                        &tool_name,
                        tool_arguments.clone(),
                        &chat_context,
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
                                    tool_event_prefix: "普通对话",
                                })
                                .await?,
                            );
                            continue;
                        }
                    };
                    let patch_set = self
                        .persist_patch_tool_result(
                            turn_id,
                            session.id,
                            session.project_id,
                            session.chapter_id,
                            &result,
                        )
                        .await?;
                    let mut observation = tool_observation(
                        step,
                        &AgentLoopAction::RunTool,
                        &tool_name,
                        tool_arguments,
                        result,
                    );
                    if let Some(patch_set) = patch_set {
                        observation["patch_set_id"] = json!(patch_set.id);
                    }
                    self.record_successful_agent_loop_tool(
                        turn_id,
                        session.id,
                        step,
                        &tool_name,
                        &observation,
                        "普通对话",
                    )
                    .await?;
                    observations.push(observation);
                }
                AgentLoopAction::Final => {
                    let reply = match final_reply(&decision.arguments) {
                        Ok(reply) => reply,
                        Err(err) => {
                            self.append_turn_event(
                                turn_id,
                                EVENT_KIND_ERROR,
                                EVENT_STATUS_FAILED,
                                &format!("final 参数无效：{err}"),
                                Some(json!({
                                    "step": step,
                                    "action": decision.action.as_str(),
                                    "arguments": decision.arguments.clone(),
                                })),
                            )
                            .await?;
                            self.save_agent_event(
                                session.id,
                                &format!("普通对话 final 参数无效：{err}"),
                            )
                            .await?;
                            return Err(err);
                        }
                    };
                    self.save_agent_event(
                        session.id,
                        "普通对话 Agent 已返回 final，正在保存 assistant 消息。",
                    )
                    .await?;
                    self.save_message(session.id, "assistant", &reply).await?;
                    report_optimize_progress(progress, 3, 4, "Agent 已返回，已保存回复")?;
                    return Ok(reply);
                }
            }
        }

        let err = AppError::InvalidArgument(
            "普通对话 Agent Loop 未收敛：模型持续请求新动作但没有返回 final".to_string(),
        );
        self.append_turn_event(
            turn_id,
            EVENT_KIND_ERROR,
            EVENT_STATUS_FAILED,
            &err.to_string(),
            None,
        )
        .await?;
        self.save_agent_event(session.id, &format!("普通对话 Agent Loop 未完成：{err}"))
            .await?;
        Err(err)
    }
}
