use super::super::{
    AppError, OptimizationSessionUpdateInput, OptimizeProgress, OrchestratorContext, Value,
    final_issues, final_response_from_issues, materialize_optimization_issue_evidence,
    optimization_session, optimize_db_error, parse_content_optimization_issues,
    parse_optimization_issues, report_optimize_progress,
};

impl OrchestratorContext {
    pub(super) async fn finish_explicit_analysis_agent_loop(
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
            self.release_in_progress_session_after_failure(session.id, "内容优化 Agent Loop", &err)
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
                    &err,
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
                    &err,
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
                    &err,
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
                        &err,
                    )
                    .await?;
                    return Err(err);
                }
            };
        let parsed_issue_count = parsed_issues.len();
        let canonical_issues = serde_json::to_value(parsed_issues)?;

        let updated = a4n_db::optimize::update_optimization_session(
            self.db.connection(),
            OptimizationSessionUpdateInput {
                session,
                status: Some("waiting_user".to_string()),
                issues: Some(Some(canonical_issues)),
                agent_permissions: None,
                agent_instruction: None,
                fixes_applied: None,
                summary: None,
            },
        )
        .await
        .map_err(optimize_db_error)?;
        self.save_agent_event(
            updated.id,
            &format!("分析结果已解析并保存，共发现 {parsed_issue_count} 个问题。"),
        )
        .await?;
        report_optimize_progress(progress, 5, 5, "优化分析结果已保存")?;
        Ok(updated)
    }
}
