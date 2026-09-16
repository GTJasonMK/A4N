use super::*;
use a4n_agent_runtime_contract::AgentExecutionSnapshot;
use a4n_api_contract::agent::*;
use a4n_api_contract::chapter_visual_plan::{
    EditChapterVisualPlanDraftInput, SaveChapterVisualPlanDraftInput,
};

pub(super) fn is_editor_tool(name: &str) -> bool {
    matches!(
        name,
        "inspect_editor_target" | "read_editor_draft" | "save_editor_draft"
    )
}

pub(super) fn needs_editor(name: &str) -> bool {
    is_editor_tool(name)
        || matches!(
            name,
            "edit_chapter_visual_plan_draft" | "save_chapter_visual_plan_draft"
        )
}

pub(super) fn request_for_call(
    context: &AgentRunContext,
    execution: &AgentExecutionSnapshot,
    call: &ToolCall,
) -> Result<AgentEditorRequest, AppError> {
    ensure_agent_scope_allows_tool(&context.scope, &call.name)?;
    ensure_argument_scope(
        &context.scope,
        &tool_descriptor(&call.name)?,
        &call.arguments,
    )?;
    let source = context
        .editor_source
        .clone()
        .ok_or_else(|| AppError::InvalidArgument("Turn 缺少编辑器来源".into()))?;
    let operation = match call.name.as_str() {
        "inspect_editor_target" => {
            if call.arguments != json!({}) {
                return Err(AppError::InvalidArgument(
                    "inspect_editor_target 不接受参数".into(),
                ));
            }
            AgentEditorOperation::Inspect
        }
        "read_editor_draft" => AgentEditorOperation::Read {
            input: parse_arguments(call.arguments.clone(), &call.name)?,
        },
        "save_editor_draft" => AgentEditorOperation::Save {
            input: parse_arguments(call.arguments.clone(), &call.name)?,
        },
        "edit_chapter_visual_plan_draft" => {
            let input: HostInput<EditChapterVisualPlanDraftInput> =
                parse_arguments(call.arguments.clone(), &call.name)?;
            AgentEditorOperation::PrepareWrite {
                expected_editor_revision: input.input.editor_revision.ok_or_else(|| {
                    AppError::InvalidArgument(
                        "编辑器内修改必须提供 inspect_editor_target 返回的 editor_revision".into(),
                    )
                })?,
                record_id: Some(input.input.record_id),
                record_revision: Some(input.input.expected_revision),
            }
        }
        "save_chapter_visual_plan_draft" => {
            let input: HostInput<SaveChapterVisualPlanDraftInput> =
                parse_arguments(call.arguments.clone(), &call.name)?;
            AgentEditorOperation::PrepareWrite {
                expected_editor_revision: input.input.editor_revision.ok_or_else(|| {
                    AppError::InvalidArgument(
                        "编辑器内保存必须提供 inspect_editor_target 返回的 editor_revision".into(),
                    )
                })?,
                record_id: input.input.record_id,
                record_revision: input.input.expected_revision,
            }
        }
        _ => return Err(AppError::InvalidArgument("不是编辑器工具".into())),
    };
    Ok(AgentEditorRequest {
        execution_id: execution.id.to_string(),
        source,
        operation,
    })
}
