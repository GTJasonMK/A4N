use super::super::{
    AGENT_LOOP_RUNAWAY_GUARD_STEPS, AgentLoopAction, AgentLoopDecisionRequest,
    AgentLoopExecutionError, AgentLoopThinkObservationInput, ExplicitAnalysisAgentLoopInput,
    FailedAgentLoopToolInput, OptimizeAnalysisTurnExecution, OrchestratorContext, tool_observation,
};
use a4n_error::AppError;

impl OrchestratorContext {
    pub(in crate::optimize) async fn run_explicit_analysis_agent_loop(
        &self,
        input: ExplicitAnalysisAgentLoopInput<'_>,
    ) -> Result<OptimizeAnalysisTurnExecution, AgentLoopExecutionError> {
        let ExplicitAnalysisAgentLoopInput {
            turn_id,
            session,
            analysis_prompt,
            base_user_message,
            analysis_context,
            workflow_run_id,
            generation,
        } = input;
        self.save_agent_event(
            session.id,
            "启动显式 Agent Loop：由模型选择读取上下文、调用工具或输出 final。",
        )
        .await?;

        let mut observations = Vec::new();
        for step in 1..=AGENT_LOOP_RUNAWAY_GUARD_STEPS {
            let phase = format!("内容优化 Agent Loop #{step}");
            let decision = self
                .request_agent_loop_decision(
                    turn_id,
                    session.id,
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
                        parse_error_label: "Agent Loop",
                        workflow_run_id,
                        generation,
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
                        )
                        .await?;
                    let result = match self
                        .execute_accounted_optimization_tool(
                            generation,
                            &session,
                            turn_id,
                            step,
                            &tool_name,
                            tool_arguments.clone(),
                            &analysis_context,
                        )
                        .await
                    {
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
                    if crate::optimize::runtime_tools::accounting::has_new_user_input(
                        self, generation,
                    )
                    .await?
                    {
                        continue;
                    }
                    return self
                        .finish_explicit_analysis_agent_loop(
                            session,
                            turn_id,
                            decision.arguments,
                            &crate::optimize::runtime_tools::observed_evidence_context(
                                &analysis_context,
                                &observations,
                            )?,
                        )
                        .await
                        .map_err(AgentLoopExecutionError::App);
                }
            }
        }

        let err = AppError::InvalidArgument(
            "内容优化 Agent Loop 未收敛：模型持续请求新动作但没有返回 final".to_string(),
        );
        self.append_turn_event(
            turn_id,
            super::super::EVENT_KIND_ERROR,
            super::super::EVENT_STATUS_FAILED,
            &err.to_string(),
            None,
        )
        .await?;
        self.save_agent_event(session.id, &format!("Agent Loop 未完成：{err}"))
            .await?;
        Err(AgentLoopExecutionError::App(err))
    }
}
