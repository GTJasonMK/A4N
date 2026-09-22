use super::super::{
    AppError, OptimizeAnalysisTurnExecution, OrchestratorContext, Value, final_issues,
    materialize_optimization_issue_evidence, optimization_session, parse_optimization_issues,
};

impl OrchestratorContext {
    pub(super) async fn finish_explicit_analysis_agent_loop(
        &self,
        session: optimization_session::Model,
        turn_id: i64,
        output: (String, Value),
    ) -> Result<OptimizeAnalysisTurnExecution, AppError> {
        let (assistant_response, canonical_issues) = output;
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

pub(super) fn parse_final_analysis_output(
    arguments: &Value,
    analysis_context: &Value,
) -> Result<(String, Value), AppError> {
    let issues = parse_optimization_issues(&final_issues(arguments)?)?;
    let issues = materialize_optimization_issue_evidence(issues, analysis_context)?;
    let assistant_response = serde_json::to_string_pretty(arguments)?;
    Ok((assistant_response, serde_json::to_value(issues)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn final_call_validates_and_materializes_evidence_before_checkpoint() {
        let context =
            json!({"current_chapter": {"source_spans": [{"id": "span-1", "text": "走入旧城"}]}});
        let mut arguments = json!({"issues": [{"id": "issue-1", "tool": "edit_article", "arguments": {
            "evidence_span_id": "span-1", "goal": "补足动作"}, "description": "动作不清楚", "severity": "minor"}]});
        let (_, issues) = parse_final_analysis_output(&arguments, &context).unwrap();
        assert_eq!(issues[0]["arguments"]["evidence"], "走入旧城");
        arguments["issues"][0]["arguments"]["evidence_span_id"] = json!("missing-span");
        assert!(
            parse_final_analysis_output(&arguments, &context)
                .unwrap_err()
                .to_string()
                .contains("missing-span")
        );
        for arguments in [
            json!({}),
            json!({"issues": {}}),
            json!({"issues": [], "extra": true}),
        ] {
            assert!(parse_final_analysis_output(&arguments, &context).is_err());
        }
    }
}
