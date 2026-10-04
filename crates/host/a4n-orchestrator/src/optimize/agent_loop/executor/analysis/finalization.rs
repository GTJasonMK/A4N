use super::super::{
    AppError, OptimizationSessionRecord, OptimizeAnalysisTurnExecution, OrchestratorContext, Value,
    final_issues, materialize_optimization_issue_evidence, parse_optimization_issues,
};

impl OrchestratorContext {
    pub(super) async fn finish_explicit_analysis_agent_loop(
        &self,
        session: OptimizationSessionRecord,
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
    observations: &[Value],
) -> Result<(String, Value), AppError> {
    let observed_context =
        crate::optimize::runtime_tools::observed_evidence_context(analysis_context, observations)?;
    let issues = parse_optimization_issues(&final_issues(arguments)?)?;
    let issues =
        materialize_optimization_issue_evidence(issues, analysis_context, &observed_context)?;
    let assistant_response = serde_json::to_string_pretty(arguments)?;
    Ok((assistant_response, serde_json::to_value(issues)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn final_call_validates_and_materializes_evidence_before_checkpoint() {
        let context = json!({"current_chapter": {
            "chapter_number": 1,
            "content": "走入旧城。",
            "source_spans": [{"id": "ch1:s1:f1", "text": "走入旧城", "paragraph_index": 1}]
        }});
        let markers = crate::optimize::runtime_tools::current_chapter_source_markers(
            &context["current_chapter"],
        )
        .unwrap();
        let marker = markers.marker_for("ch1:s1:f1").unwrap();
        let observations = [json!({"success": true, "result": {
            "source": "current_chapter",
            "mode": "read",
            "resource": {"hash": crate::optimize::runtime_tools::source_hash(&context["current_chapter"]).unwrap()},
            "content": {"source_span_ids": [marker]}
        }})];
        let mut arguments = json!({"issues": [{"id": "issue-1", "tool": "edit_article", "arguments": {
            "evidence_span_id": marker, "goal": "补足动作"}, "description": "动作不清楚", "severity": "minor"}]});
        let (_, issues) = parse_final_analysis_output(&arguments, &context, &observations).unwrap();
        assert_eq!(issues[0]["arguments"]["evidence"], "走入旧城");
        arguments["issues"][0]["arguments"]["evidence_span_id"] = json!("missing-span");
        assert!(
            parse_final_analysis_output(&arguments, &context, &observations)
                .unwrap_err()
                .to_string()
                .contains("missing-span")
        );
        for arguments in [
            json!({}),
            json!({"issues": {}}),
            json!({"issues": [], "extra": true}),
        ] {
            assert!(parse_final_analysis_output(&arguments, &context, &observations).is_err());
        }
    }
}
