use a4n_llm::provider::LlmTraceContext;
use a4n_service_protocol::TraceContext;

use super::context::SidecarRequestContext;

pub(crate) fn sidecar_context_for_llm_trace(
    trace_context: &LlmTraceContext,
) -> Result<SidecarRequestContext, String> {
    trace_context.validate_business_trace()?;
    Ok(SidecarRequestContext::current_global()
        .with_project_scope(trace_context.project_id, trace_context.root_project_id)
        .with_trace_context(TraceContext {
            trace_id: llm_service_trace_id(trace_context)?,
            parent_span_id: Some(llm_service_parent_span_id(trace_context)?),
        }))
}

pub(in crate::sidecar_clients) fn sidecar_context_for_workflow_operation_with_root(
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    operation: &str,
) -> Result<SidecarRequestContext, String> {
    let context =
        SidecarRequestContext::current_global().with_project_scope(project_id, root_project_id);
    let Some(workflow_run_id) = workflow_run_id else {
        return Ok(context);
    };
    if workflow_run_id <= 0 {
        return Err(format!(
            "workflow_run_id 必须是正整数，实际为 {workflow_run_id}"
        ));
    }
    Ok(context.with_trace_context(TraceContext {
        trace_id: format!("workflow_run:{workflow_run_id}"),
        parent_span_id: Some(trace_id_component("operation", operation)?),
    }))
}

fn llm_service_trace_id(trace_context: &LlmTraceContext) -> Result<String, String> {
    if let Some(generation_run_id) = trace_context.generation_run_id {
        validate_positive_trace_run_id("generation_run_id", generation_run_id)?;
    }
    if let Some(workflow_run_id) = trace_context.workflow_run_id {
        validate_positive_trace_run_id("workflow_run_id", workflow_run_id)?;
    }

    if let Some(generation_run_id) = trace_context.generation_run_id {
        return Ok(format!("generation_run:{generation_run_id}"));
    }
    if let Some(workflow_run_id) = trace_context.workflow_run_id {
        return Ok(format!("workflow_run:{workflow_run_id}"));
    }
    let project = trace_context
        .project_id
        .map(|value| value.to_string())
        .unwrap_or_else(|| "global".to_string());
    let mut parts = vec![
        "llm".to_string(),
        "project".to_string(),
        project,
        trace_id_component("workflow", &trace_context.workflow)?,
        trace_id_component("step_key", &trace_context.step_key)?,
    ];
    if let Some(call_key) = trace_context.call_key.as_deref() {
        parts.push(trace_id_component("call_key", call_key)?);
    }
    Ok(parts.join(":"))
}

fn validate_positive_trace_run_id(field: &str, value: i64) -> Result<(), String> {
    if value <= 0 {
        return Err(format!("LLM trace {field} 必须是正整数，实际为 {value}"));
    }
    Ok(())
}

fn llm_service_parent_span_id(trace_context: &LlmTraceContext) -> Result<String, String> {
    let mut parts = vec![
        trace_id_component("workflow", &trace_context.workflow)?,
        trace_id_component("step_key", &trace_context.step_key)?,
    ];
    if let Some(call_key) = trace_context.call_key.as_deref() {
        parts.push(trace_id_component("call_key", call_key)?);
    }
    Ok(parts.join(":"))
}

fn trace_id_component(field: &str, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("LLM trace {field} 不能为空"));
    }
    let component = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    Ok(component)
}
