use a4n_api_contract::{
    admin::{AdminProjectSummary, InviteCodeSummary, TaskQueueSummary, UsageLogSummary},
    auth::UserSummary,
};
use a4n_core::models::{
    project::novel_project,
    task::task_queue,
    user::{invite_code, llm_usage_log, user},
};

use crate::actions::auth_records::to_user_summary;

pub(super) fn to_admin_user_summary(user: &user::Model) -> UserSummary {
    to_user_summary(user).with_display_name()
}

pub(super) fn to_invite_code_summary(code: &invite_code::Model) -> InviteCodeSummary {
    InviteCodeSummary {
        id: code.id,
        code: code.code.clone(),
        created_by_user_id: code.created_by_user_id,
        used_by_user_id: code.used_by_user_id,
        created_at: Some(code.created_at.to_string()),
        used_at: code.used_at.map(|value| value.to_string()),
    }
}

pub(super) fn to_usage_log_summary(log: &llm_usage_log::Model) -> UsageLogSummary {
    UsageLogSummary {
        id: log.id,
        user_id: log.user_id,
        project_id: log.project_id,
        model: log.model.clone(),
        operation: log.operation.clone(),
        prompt_tokens: log.prompt_tokens,
        completion_tokens: log.completion_tokens,
        total_tokens: log.total_tokens,
        latency_ms: log.latency_ms,
        is_success: log.is_success,
        error_message: log.error_message.clone(),
        created_at: Some(log.created_at.to_string()),
    }
}

pub(super) fn to_task_queue_summary(task: &task_queue::Model) -> TaskQueueSummary {
    TaskQueueSummary {
        id: task.id.clone(),
        task_type: task.task_type.clone(),
        priority: task.priority.clone(),
        status: task.status.clone(),
        project_id: task.project_id,
        root_project_id: task.root_project_id,
        source_snapshot_id: task.source_snapshot_id,
        task_scope: task.task_scope.clone(),
        payload: task.payload.clone(),
        progress: task.progress.clone(),
        error_message: task.error_message.clone(),
        dag_id: task.dag_id.clone(),
        created_at: Some(task.created_at.to_string()),
        started_at: task.started_at.map(|value| value.to_string()),
        completed_at: task.completed_at.map(|value| value.to_string()),
    }
}

pub(super) fn to_admin_project_summary(
    project: &novel_project::Model,
) -> Result<AdminProjectSummary, String> {
    let status = a4n_domain::project::ProjectStatus::parse(&project.status)
        .ok_or_else(|| format!("未知项目状态：{}", project.status))?;
    Ok(AdminProjectSummary {
        id: project.id,
        user_id: project.user_id,
        title: project.title.clone(),
        description: project.description.clone(),
        project_type: project.project_type.clone(),
        status,
        target_chapters: project.target_chapters,
        current_chapter: project.current_chapter,
        total_chapters: project.total_chapters,
        chapters_per_part: project.chapters_per_part,
        avatar_animal: project.avatar_animal.clone(),
        created_at: Some(project.created_at.to_string()),
        updated_at: Some(project.updated_at.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::*;

    #[test]
    fn admin_record_projections_preserve_host_fields() {
        let created_at = NaiveDate::from_ymd_opt(2026, 1, 2)
            .unwrap()
            .and_hms_opt(3, 4, 5)
            .unwrap();
        let updated_at = NaiveDate::from_ymd_opt(2026, 1, 3)
            .unwrap()
            .and_hms_opt(6, 7, 8)
            .unwrap();
        let user = user::Model {
            id: 7,
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password_hash: "hash".to_string(),
            role: "admin".to_string(),
            oauth_provider: Some("github".to_string()),
            oauth_subject: Some("42".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            is_active: true,
            lifecycle_gate: "active".to_string(),
            created_at,
            updated_at: created_at,
            last_login_at: Some(created_at),
        };
        let code = invite_code::Model {
            id: 11,
            code: "INVITE".to_string(),
            created_by_user_id: Some(1),
            used_by_user_id: Some(2),
            created_at,
            used_at: Some(updated_at),
        };
        let usage = llm_usage_log::Model {
            id: 13,
            user_id: 7,
            project_id: Some(17),
            model: "gpt-test".to_string(),
            operation: "chat".to_string(),
            prompt_tokens: 100,
            completion_tokens: 40,
            total_tokens: 140,
            latency_ms: 900,
            is_success: false,
            error_message: Some("timeout".to_string()),
            created_at,
        };
        let task = task_queue::Model {
            id: "task-1".to_string(),
            task_type: "chapter_generation".to_string(),
            priority: "high".to_string(),
            status: "pending".to_string(),
            project_id: Some(17),
            root_project_id: Some(17),
            source_snapshot_id: Some(19),
            task_scope: Some("chapter:1".to_string()),
            workflow_run_id: Some(23),
            payload: Some(json!({"chapter_id": 29})),
            progress: Some(json!({"percent": 25})),
            error_message: None,
            dag_id: Some("dag-1".to_string()),
            created_at,
            started_at: Some(updated_at),
            completed_at: None,
        };
        let project = novel_project::Model {
            id: 17,
            user_id: 7,
            title: "项目".to_string(),
            description: "描述".to_string(),
            project_type: "novel_long".to_string(),
            project_kind: "text".to_string(),
            source_mode: "self".to_string(),
            root_project_id: None,
            parent_project_id: None,
            source_snapshot_id: None,
            source_sync_status: "current".to_string(),
            derived_project_status: None,
            status: "writing".to_string(),
            lifecycle_gate: "active".to_string(),
            target_chapters: 80,
            current_chapter: 12,
            total_chapters: Some(72),
            chapters_per_part: Some(12),
            avatar_svg: None,
            avatar_animal: Some("fox".to_string()),
            raw_blueprint: None,
            import_analysis_progress: None,
            export_config: None,
            created_at,
            updated_at,
        };

        assert_eq!(
            serde_json::to_value(to_admin_user_summary(&user)).unwrap(),
            json!({
                "id": 7,
                "username": "alice",
                "email": "alice@example.com",
                "role": "admin",
                "oauth_provider": "github",
                "oauth_subject": "42",
                "avatar_url": "https://example.com/avatar.png",
                "is_active": true,
                "display_name": "alice",
                "created_at": "2026-01-02 03:04:05",
                "updated_at": "2026-01-02 03:04:05",
                "last_login_at": "2026-01-02 03:04:05"
            })
        );
        assert_eq!(
            serde_json::to_value(to_invite_code_summary(&code)).unwrap(),
            json!({
                "id": 11,
                "code": "INVITE",
                "created_by_user_id": 1,
                "used_by_user_id": 2,
                "created_at": "2026-01-02 03:04:05",
                "used_at": "2026-01-03 06:07:08"
            })
        );
        assert_eq!(
            serde_json::to_value(to_usage_log_summary(&usage)).unwrap(),
            json!({
                "id": 13,
                "user_id": 7,
                "project_id": 17,
                "model": "gpt-test",
                "operation": "chat",
                "prompt_tokens": 100,
                "completion_tokens": 40,
                "total_tokens": 140,
                "latency_ms": 900,
                "is_success": false,
                "error_message": "timeout",
                "created_at": "2026-01-02 03:04:05"
            })
        );
        assert_eq!(
            serde_json::to_value(to_task_queue_summary(&task)).unwrap(),
            json!({
                "id": "task-1",
                "task_type": "chapter_generation",
                "priority": "high",
                "status": "pending",
                "project_id": 17,
                "root_project_id": 17,
                "source_snapshot_id": 19,
                "task_scope": "chapter:1",
                "payload": {"chapter_id": 29},
                "progress": {"percent": 25},
                "error_message": null,
                "dag_id": "dag-1",
                "created_at": "2026-01-02 03:04:05",
                "started_at": "2026-01-03 06:07:08",
                "completed_at": null
            })
        );
        assert_eq!(
            serde_json::to_value(to_admin_project_summary(&project)).unwrap(),
            json!({
                "id": 17,
                "user_id": 7,
                "title": "项目",
                "description": "描述",
                "project_type": "novel_long",
                "status": "writing",
                "target_chapters": 80,
                "current_chapter": 12,
                "total_chapters": 72,
                "chapters_per_part": 12,
                "avatar_animal": "fox",
                "created_at": "2026-01-02 03:04:05",
                "updated_at": "2026-01-03 06:07:08"
            })
        );
    }
}
