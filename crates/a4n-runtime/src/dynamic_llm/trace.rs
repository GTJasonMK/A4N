mod preparation;
mod recording;

use a4n_core::error::AppError;
use a4n_llm::provider::LlmTraceContext;
use a4n_orchestrator::DynamicProviderFacade;

pub(in crate::dynamic_llm) use preparation::prepare_dynamic_llm_trace_context;
pub(in crate::dynamic_llm) use recording::record_dynamic_llm_trace;

pub(super) fn llm_error_kind(error: &AppError) -> &'static str {
    match error {
        AppError::Internal(_) => "internal",
        AppError::Database(_) => "database",
        AppError::NotFound(_) => "not_found",
        AppError::Forbidden(_) => "forbidden",
        AppError::Unauthorized(_) => "unauthorized",
        AppError::InvalidArgument(_) => "invalid_argument",
        AppError::InvalidState { .. } => "invalid_state",
        AppError::LlmError(_) => "llm_error",
        AppError::VectorStoreError(_) => "vector_store_error",
        AppError::EmbeddingError(_) => "embedding_error",
        AppError::Cancelled => "cancelled",
        AppError::Timeout(_) => "timeout",
        AppError::ConfigError(_) => "config_error",
        AppError::Io(_) => "io",
        AppError::Serialization(_) => "serialization",
    }
}

pub(super) fn llm_trace_status_for_error(error: &AppError) -> &'static str {
    match error {
        AppError::Cancelled => "cancelled",
        _ => "failed",
    }
}

pub(super) async fn complete_llm_trace_project_scope(
    facade: &DynamicProviderFacade,
    trace_context: &mut LlmTraceContext,
) -> Result<(), AppError> {
    let Some(project_id) = trace_context.project_id else {
        if trace_context.root_project_id.is_some() {
            return Err(AppError::InvalidArgument(format!(
                "LLM trace 缺少 project_id，不能单独设置 root_project_id：workflow={}, step={}",
                trace_context.workflow, trace_context.step_key
            )));
        }
        return Ok(());
    };
    if project_id <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "LLM trace.project_id 必须是正整数，实际为 {project_id}：workflow={}, step={}",
            trace_context.workflow, trace_context.step_key
        )));
    }
    let project = facade
        .project_by_id(project_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("LLM trace 引用的项目不存在: {project_id}")))?;
    trace_context.root_project_id = Some(validated_authoritative_trace_root_project_id(
        project_id,
        trace_context.root_project_id,
        project.root_project_id,
    )?);
    Ok(())
}

pub(super) fn validated_authoritative_trace_root_project_id(
    project_id: i64,
    declared_root_project_id: Option<i64>,
    stored_root_project_id: Option<i64>,
) -> Result<i64, AppError> {
    let authoritative_root_project_id = stored_root_project_id.unwrap_or(project_id);
    if authoritative_root_project_id <= 0 {
        return Err(AppError::InvalidState {
            current: format!(
                "project_id={project_id}, root_project_id={authoritative_root_project_id}"
            ),
            operation: "使用非正整数 root_project_id 构造 LLM trace".to_string(),
        });
    }
    if let Some(declared_root_project_id) = declared_root_project_id
        && declared_root_project_id != authoritative_root_project_id
    {
        return Err(AppError::InvalidArgument(format!(
            "LLM trace.root_project_id 与项目权威归属不一致: project_id={project_id}, declared_root_project_id={declared_root_project_id}, authoritative_root_project_id={authoritative_root_project_id}"
        )));
    }
    Ok(authoritative_root_project_id)
}
