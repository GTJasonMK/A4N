use super::contracts::ChatAgentLoopInput;
use crate::optimize::OrchestratorContext;
use crate::optimize::agent_loop::{
    AGENT_LOOP_RUNAWAY_GUARD_STEPS, AgentLoopAction, tool_observation,
};
use crate::optimize::agent_loop_executor::{
    AgentLoopDecisionRequest, AgentLoopExecutionError, AgentLoopThinkObservationInput,
    FailedAgentLoopToolInput,
};
use crate::optimize::runtime_tools::execute_optimization_runtime_tool;
use serde_json::json;

impl OrchestratorContext {
    pub(in crate::optimize) async fn run_chat_agent_loop(
        &self,
        input: ChatAgentLoopInput<'_>,
    ) -> Result<String, AgentLoopExecutionError> {
        let ChatAgentLoopInput {
            turn_id,
            session,
            system_prompt,
            base_user_message,
            chat_context,
            workflow_run_id,
            generation,
        } = input;
        self.save_agent_event(
            session.id,
            "启动普通对话 Agent Loop：由模型决定直接回复、读取上下文或调用工具。",
        )
        .await?;

        let mut observations = Vec::new();
        for step in 1..=AGENT_LOOP_RUNAWAY_GUARD_STEPS {
            let phase = format!("内容优化对话 Agent Loop #{step}");
            let decision = self
                .request_agent_loop_decision(
                    turn_id,
                    session.id,
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
                        parse_error_label: "普通对话 Loop",
                        workflow_run_id,
                        generation,
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
                    return self
                        .finish_chat_agent_loop(session, turn_id, step, &decision)
                        .await
                        .map_err(AgentLoopExecutionError::App);
                }
            }
        }

        self.fail_chat_agent_loop_runaway(session, turn_id)
            .await
            .map_err(AgentLoopExecutionError::App)
    }
}
