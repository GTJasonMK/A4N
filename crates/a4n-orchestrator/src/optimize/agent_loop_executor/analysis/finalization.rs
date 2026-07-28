use super::super::{
    AppError, OptimizeAnalysisTurnExecution, OrchestratorContext, Value, final_issues,
    final_response_from_issues, materialize_optimization_issue_evidence, optimization_session,
    parse_content_optimization_issues, parse_optimization_issues,
};

impl OrchestratorContext {
    pub(super) async fn finish_explicit_analysis_agent_loop(
        &self,
        session: optimization_session::Model,
        turn_id: i64,
        final_arguments: Value,
        analysis_context: &Value,
        has_read_current_chapter: bool,
    ) -> Result<OptimizeAnalysisTurnExecution, AppError> {
        if !has_read_current_chapter {
            let err = AppError::InvalidArgument(
                "Agent Loop 在读取 current_chapter 前不能返回 final".to_string(),
            );
            self.save_agent_event(session.id, &format!("Agent Loop final 前置条件失败：{err}"))
                .await?;
            return Err(err);
        }

        let issues = match final_issues(&final_arguments) {
            Ok(issues) => issues,
            Err(err) => {
                self.save_agent_event(session.id, &format!("Agent Loop final 参数无效：{err}"))
                    .await?;
                return Err(err);
            }
        };
        let assistant_response = final_response_from_issues(issues)?;
        let issues = match parse_content_optimization_issues(&assistant_response) {
            Ok(issues) => issues,
            Err(err) => {
                self.save_agent_event(session.id, &format!("分析结果解析失败：{err}"))
                    .await?;
                return Err(err);
            }
        };
        let parsed_issues = match parse_optimization_issues(&issues) {
            Ok(issues) => issues,
            Err(err) => {
                self.save_agent_event(session.id, &format!("分析问题契约解析失败：{err}"))
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
                    return Err(err);
                }
            };
        let canonical_issues = serde_json::to_value(parsed_issues)?;
        self.save_agent_event(
            session.id,
            "Agent 已返回并校验 final，等待 generation terminal 事务保存分析结果。",
        )
        .await?;
        Ok(OptimizeAnalysisTurnExecution {
            turn_id,
            assistant_response,
            canonical_issues,
        })
    }
}
