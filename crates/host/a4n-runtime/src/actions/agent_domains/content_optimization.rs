use crate::AppState;
use a4n_api_contract::{
    agent::AgentScope,
    generation_tasks::GenerationTaskSummary,
    optimize::{
        ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
        OptimizationPatchSetSummary, OptimizationSessionSummary, OptimizationTurnSummary,
        OptimizeSessionInput,
    },
};
use a4n_error::AppError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationTaskSummaryInput {
    pub task_id: String,
    pub attempt: u32,
}

pub(crate) async fn get_session(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: OptimizeSessionInput,
) -> Result<OptimizationSessionSummary, AppError> {
    scoped_session(state, user_id, scope, input.session_id).await
}

pub(crate) async fn list_turns(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: OptimizeSessionInput,
) -> Result<Vec<OptimizationTurnSummary>, AppError> {
    scoped_session(state, user_id, scope, input.session_id).await?;
    crate::actions::optimize::list_optimize_turns(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

pub(crate) async fn list_patch_sets(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: OptimizeSessionInput,
) -> Result<Vec<OptimizationPatchSetSummary>, AppError> {
    scoped_session(state, user_id, scope, input.session_id).await?;
    crate::actions::optimize::list_optimize_patch_sets(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

pub(crate) async fn apply_patches(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: ApplyOptimizationPatchesInput,
) -> Result<ApplyOptimizationPatchesResponse, AppError> {
    scoped_session(state, user_id, scope, input.session_id).await?;
    crate::actions::optimize::apply_optimization_patches(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

pub(crate) async fn generation_task_summary(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: GenerationTaskSummaryInput,
) -> Result<GenerationTaskSummary, AppError> {
    if input.task_id.trim().is_empty() || input.attempt == 0 {
        return Err(AppError::InvalidArgument(
            "taskId 不能为空且 attempt 必须大于 0".into(),
        ));
    }
    let summary = crate::actions::generation_tasks::generation_task_summary(
        state,
        user_id,
        input.task_id.trim(),
        input.attempt,
    )
    .await
    .map_err(|error| AppError::InvalidArgument(error.to_string()))?;

    // The generic generation-task authorization only checks the user's
    // project access. A bound Agent must also prove that a task belongs to its
    // exact chapter before observing it.
    if scope.project_id.is_none() && scope.chapter_id.is_none() && scope.resource_kind.is_none() {
        return Ok(summary);
    }
    let project_id = summary.started.project_id.ok_or_else(|| {
        AppError::Forbidden("Agent scope 受限时，generation task 缺少可验证的项目归属".into())
    })?;
    super::super::agent_runtime::ensure_project_scope(scope, project_id)?;

    if scope.chapter_id.is_none() {
        return Ok(summary);
    }
    let target = summary.started.target.as_ref().ok_or_else(|| {
        AppError::Forbidden("章节绑定的 Agent 只能读取带可验证章节归属的 generation task".into())
    })?;
    if matches!(
        target.kind.as_str(),
        "optimization_chat" | "optimization_analysis" | "optimization_patch"
    ) {
        let session_id = target
            .id
            .parse::<i64>()
            .ok()
            .filter(|id| *id > 0)
            .ok_or_else(|| {
                AppError::InvalidArgument("optimization generation task 的 sessionId 无效".into())
            })?;
        let session = crate::actions::optimize::get_optimize_session(
            state,
            user_id,
            OptimizeSessionInput { session_id },
        )
        .await
        .map_err(AppError::InvalidArgument)?;
        if session.project_id != project_id {
            return Err(AppError::InvalidState {
                current: format!(
                    "generation task {} 的项目归属与 optimization session 不一致",
                    summary.handle.task_id
                ),
                operation: "校验 Agent generation task scope".into(),
            });
        }
        super::super::agent_runtime::ensure_chapter_scope(scope, session.chapter_id)?;
        return Ok(summary);
    }

    let chapter_id = target_chapter_id(target).ok_or_else(|| {
        AppError::Forbidden("章节绑定的 Agent 只能读取带可验证章节归属的 generation task".into())
    })?;
    state
        .agent_resource_access_application()
        // This is the persisted task's source chapter, not a chapter mutation.
        .verify_chapters_in_project(user_id, project_id, &[], &[chapter_id])
        .await?;
    super::super::agent_runtime::ensure_chapter_scope(scope, chapter_id)?;
    Ok(summary)
}

/// Extract the chapter fence persisted in Director generation target IDs.
/// Older tasks intentionally return `None`: their target cannot prove chapter
/// ownership and must not bypass a bound Agent scope.
fn target_chapter_id(target: &a4n_service_protocol::GenerationTargetRef) -> Option<i64> {
    let mut parts = target.id.split(':');
    match target.kind.as_str() {
        "visual_plan_segment" => {
            (parts.next()? == "chapter").then_some(())?;
            parts.next()?.parse().ok()
        }
        "image_visual_plan_point" => {
            parts.next()?.parse::<i64>().ok()?;
            (parts.next()? == "visual_plan").then_some(())?;
            (parts.next()? == "chapter").then_some(())?;
            parts.next()?.parse().ok()
        }
        "image_object_profile" | "image_scene_profile" => {
            parts.next()?.parse::<i64>().ok()?;
            (parts.next()? == "chapter").then_some(())?;
            parts.next()?.parse().ok()
        }
        "image_scene" => {
            parts.next()?.parse::<i64>().ok()?;
            (parts.next()? == "scene").then_some(())?;
            (parts.next()? == "chapter").then_some(())?;
            parts.next()?.parse().ok()
        }
        _ => None,
    }
}

async fn scoped_session(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    session_id: i64,
) -> Result<OptimizationSessionSummary, AppError> {
    ensure_positive(session_id, "sessionId")?;
    let session = crate::actions::optimize::get_optimize_session(
        state,
        user_id,
        OptimizeSessionInput { session_id },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    super::super::agent_runtime::ensure_project_scope(scope, session.project_id)?;
    super::super::agent_runtime::ensure_chapter_scope(scope, session.chapter_id)?;
    Ok(session)
}

fn ensure_positive(value: i64, field: &str) -> Result<(), AppError> {
    if value > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("{field} 无效")))
    }
}

#[cfg(test)]
mod generation_task_scope_tests {
    use super::target_chapter_id;
    use a4n_service_protocol::GenerationTargetRef;

    fn target(kind: &str, id: &str) -> GenerationTargetRef {
        GenerationTargetRef {
            kind: kind.into(),
            id: id.into(),
            revision: None,
        }
    }

    #[test]
    fn director_targets_keep_the_chapter_fence() {
        assert_eq!(
            target_chapter_id(&target("visual_plan_segment", "chapter:11:42:shot-1")),
            Some(11)
        );
        assert_eq!(
            target_chapter_id(&target(
                "image_visual_plan_point",
                "7:visual_plan:chapter:11:42:point-1",
            )),
            Some(11)
        );
    }

    #[test]
    fn chapterless_or_legacy_targets_are_not_verifiable() {
        assert_eq!(
            target_chapter_id(&target("visual_plan_segment", "42:shot-1")),
            None
        );
        assert_eq!(target_chapter_id(&target("image_scene", "7")), None);
        assert_eq!(target_chapter_id(&target("optimization_chat", "9")), None);
    }
}
