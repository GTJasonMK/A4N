use crate::file_utils::remove_data_file_in_data_dir;
use crate::{
    AppState,
    actions::access::{
        verify_project_access_string, verify_project_editor_string, verify_project_owner_string,
    },
};
use a4n_core::{
    models::project::novel_project,
    state::{
        rollback::{CleanupAction, rollback_actions},
        status::ProjectStatus,
        validator::StateValidator,
    },
};
use a4n_services::project::PROJECT_KIND_TEXT;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusHistorySummary {
    pub id: i64,
    pub project_id: i64,
    pub from_status: String,
    pub to_status: String,
    pub reason: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPreviewSummary {
    pub from_status: String,
    pub target_status: String,
    pub path: Vec<String>,
    pub affected_chapters: i32,
    pub affected_outlines: i32,
    pub can_rollback: bool,
    pub warning_message: Option<String>,
    pub summary: String,
    pub impacts: Vec<RollbackImpactSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackImpactSummary {
    pub kind: String,
    pub title: String,
    pub tables: Vec<String>,
    pub count: Option<i32>,
    pub chapter_start: Option<i32>,
    pub chapter_end: Option<i32>,
    pub part_start: Option<i32>,
    pub part_end: Option<i32>,
    pub note: Option<String>,
}

pub async fn forward_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: &str,
) -> Result<novel_project::Model, String> {
    verify_project_editor_string(state, user_id, project_id).await?;
    state
        .project
        .forward_status(project_id, target)
        .await
        .map_err(|e| e.to_string())
}

pub async fn rollback_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: &str,
) -> Result<novel_project::Model, String> {
    verify_project_owner_string(state, user_id, project_id).await?;
    let current_project = state
        .project
        .get(project_id)
        .await
        .map_err(|e| e.to_string())?;
    ensure_text_project_status_command(&current_project)?;
    let from = StateValidator::parse_status(&current_project.status).map_err(|e| e.to_string())?;
    let to = StateValidator::parse_status(target).map_err(|e| e.to_string())?;
    StateValidator::validate_rollback(&current_project.status, target)
        .map_err(|e| e.to_string())?;
    let cleanup_actions = rollback_actions(from, to);
    let media_paths = collect_rollback_media_paths(state, project_id, &cleanup_actions).await?;
    let needs_rag_clear = cleanup_actions
        .iter()
        .any(|action| matches!(action, CleanupAction::DeleteChapters));
    let project = state
        .project
        .rollback_status(project_id, target)
        .await
        .map_err(|e| e.to_string())?;
    if needs_rag_clear {
        state
            .rag
            .clear_project(project_id)
            .await
            .map_err(|e| e.to_string())?;
    }
    for path in media_paths {
        remove_data_file(state, &path)?;
    }

    Ok(project)
}

pub async fn get_project_status_history(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<StatusHistorySummary>, String> {
    use a4n_core::models::project::project_status_history;

    verify_project_access_string(state, user_id, project_id).await?;
    let history = project_status_history::Entity::find()
        .filter(project_status_history::Column::ProjectId.eq(project_id))
        .order_by_desc(project_status_history::Column::ChangedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(history
        .iter()
        .map(|item| StatusHistorySummary {
            id: item.id,
            project_id: item.project_id,
            from_status: item.from_status.clone(),
            to_status: item.to_status.clone(),
            reason: item.reason.clone(),
            created_at: Some(item.changed_at.to_string()),
        })
        .collect())
}

pub async fn preview_rollback(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target_status: &str,
) -> Result<RollbackPreviewSummary, String> {
    verify_project_access_string(state, user_id, project_id).await?;
    let project = state
        .project
        .get(project_id)
        .await
        .map_err(|e| e.to_string())?;
    ensure_text_project_status_command(&project)?;
    let from = StateValidator::parse_status(&project.status).map_err(|e| e.to_string())?;
    let to = StateValidator::parse_status(target_status).map_err(|e| e.to_string())?;
    StateValidator::validate_rollback(&project.status, target_status).map_err(|e| e.to_string())?;

    let mut path = vec![project.status.clone()];
    path.extend(rollback_path(from, to));
    let impacts = build_rollback_impacts(state, project_id, from, to).await?;
    let affected_chapters = rollback_impact_count(&impacts, "delete_chapters")?;
    let affected_outlines = rollback_impact_count(&impacts, "delete_chapter_outlines")?;
    let summary = build_rollback_summary(&path, &impacts);
    let warning_message = if impacts.is_empty() {
        None
    } else {
        Some("回退会删除依赖数据，此操作不可恢复。".to_string())
    };

    Ok(RollbackPreviewSummary {
        from_status: project.status,
        target_status: target_status.to_string(),
        path,
        affected_chapters,
        affected_outlines,
        can_rollback: true,
        warning_message,
        summary,
        impacts,
    })
}

fn ensure_text_project_status_command(project: &novel_project::Model) -> Result<(), String> {
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err("非文本项目不能使用文本写作状态机".to_string());
    }
    Ok(())
}

fn rollback_impact_count(impacts: &[RollbackImpactSummary], kind: &str) -> Result<i32, String> {
    let Some(impact) = impacts.iter().find(|impact| impact.kind == kind) else {
        return Ok(0);
    };

    impact
        .count
        .ok_or_else(|| format!("回退影响 {kind} 缺少数量"))
}

fn rollback_path(from: ProjectStatus, to: ProjectStatus) -> Vec<String> {
    ProjectStatus::all()
        .iter()
        .rev()
        .filter(|status| status.ordinal() < from.ordinal() && status.ordinal() >= to.ordinal())
        .map(|status| status.as_str().to_string())
        .collect()
}

async fn build_rollback_impacts(
    state: &AppState,
    project_id: i64,
    from: ProjectStatus,
    to: ProjectStatus,
) -> Result<Vec<RollbackImpactSummary>, String> {
    use a4n_core::models::ai::{generated_image, generated_video};
    use a4n_core::models::asset::object_profile;
    use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};
    use a4n_core::models::project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
    };
    use a4n_core::models::scene::{
        scene_entity_state, scene_occurrence, scene_profile, scene_transition,
    };
    use a4n_core::models::task::task_queue;

    let mut impacts = Vec::new();
    let actions = rollback_actions(from, to);

    let chapter_rows = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let chapter_ids: Vec<i64> = chapter_rows.iter().map(|row| row.id).collect();
    let chapter_numbers: Vec<i32> = chapter_rows.iter().map(|row| row.chapter_number).collect();

    let outline_rows = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let outline_numbers: Vec<i32> = outline_rows.iter().map(|row| row.chapter_number).collect();

    let part_rows = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let part_numbers: Vec<i32> = part_rows.iter().map(|row| row.part_number).collect();

    let media_image_count = if chapter_ids.is_empty() {
        0
    } else {
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::ChapterId.is_in(chapter_ids.clone()))
            .count(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())? as i32
    };
    let media_video_count = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())? as i32;
    let task_count = task_queue::Entity::find()
        .filter(task_queue::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())? as i32;
    let object_profile_count = object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())? as i32;
    let scene_profile_count = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())? as i32;
    let scene_occurrence_count = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())? as i32;
    let scene_transition_count = scene_transition::Entity::find()
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())? as i32;
    let scene_entity_state_count = scene_entity_state::Entity::find()
        .filter(scene_entity_state::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())? as i32;

    for action in actions {
        match action {
            CleanupAction::DeleteChapters => {
                impacts.push(RollbackImpactSummary {
                    kind: "delete_chapters".to_string(),
                    title: "删除章节正文与派生数据".to_string(),
                    tables: vec![
                        "chapters".to_string(),
                        "chapter_manga_prompts".to_string(),
                        "chapter_video_prompts".to_string(),
                        "chapter_scenes".to_string(),
                        "object_profile".to_string(),
                        "scene_profile".to_string(),
                        "scene_occurrence".to_string(),
                        "scene_transition".to_string(),
                        "scene_entity_state".to_string(),
                        "generated_images".to_string(),
                        "generated_videos".to_string(),
                        "task_queue".to_string(),
                        "task_dependency".to_string(),
                        "vector_store".to_string(),
                        "tantivy_index".to_string(),
                    ],
                    count: Some(chapter_rows.len() as i32),
                    chapter_start: chapter_numbers.iter().min().copied(),
                    chapter_end: chapter_numbers.iter().max().copied(),
                    part_start: None,
                    part_end: None,
                    note: Some(format!(
                        "将同步清理 {} 张章节图片、{} 条视频记录对应的本地文件、{} 个物体档案、{} 个场景档案、{} 条场景出场、{} 条场景位移、{} 条场景实体状态、项目 RAG 索引，并删除 {} 个项目任务。",
                        media_image_count,
                        media_video_count,
                        object_profile_count,
                        scene_profile_count,
                        scene_occurrence_count,
                        scene_transition_count,
                        scene_entity_state_count,
                        task_count
                    )),
                });
            }
            CleanupAction::DeleteChapterOutlines => {
                impacts.push(RollbackImpactSummary {
                    kind: "delete_chapter_outlines".to_string(),
                    title: "删除章节大纲".to_string(),
                    tables: vec!["chapter_outlines".to_string()],
                    count: Some(outline_rows.len() as i32),
                    chapter_start: outline_numbers.iter().min().copied(),
                    chapter_end: outline_numbers.iter().max().copied(),
                    part_start: None,
                    part_end: None,
                    note: None,
                });
            }
            CleanupAction::DeletePartOutlines => {
                impacts.push(RollbackImpactSummary {
                    kind: "delete_part_outlines".to_string(),
                    title: "删除分部大纲".to_string(),
                    tables: vec!["part_outlines".to_string()],
                    count: Some(part_rows.len() as i32),
                    chapter_start: None,
                    chapter_end: None,
                    part_start: part_numbers.iter().min().copied(),
                    part_end: part_numbers.iter().max().copied(),
                    note: None,
                });
            }
            CleanupAction::DeleteBlueprintVersions => {
                let character_count = blueprint_character::Entity::find()
                    .filter(blueprint_character::Column::ProjectId.eq(project_id))
                    .count(state.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())? as i32;
                let relation_count = blueprint_relationship::Entity::find()
                    .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                    .count(state.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())? as i32;
                let version_count = blueprint_version::Entity::find()
                    .filter(blueprint_version::Column::ProjectId.eq(project_id))
                    .count(state.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())? as i32;
                let blueprint_count = novel_blueprint::Entity::find()
                    .filter(novel_blueprint::Column::ProjectId.eq(project_id))
                    .count(state.db.as_ref())
                    .await
                    .map_err(|e| e.to_string())? as i32;

                impacts.push(RollbackImpactSummary {
                    kind: "delete_blueprint_versions".to_string(),
                    title: "删除蓝图与版本".to_string(),
                    tables: vec![
                        "novel_blueprint".to_string(),
                        "blueprint_version".to_string(),
                        "blueprint_character".to_string(),
                        "blueprint_relationship".to_string(),
                    ],
                    count: Some(blueprint_count + version_count + character_count + relation_count),
                    chapter_start: None,
                    chapter_end: None,
                    part_start: None,
                    part_end: None,
                    note: Some("会重置蓝图、角色和关系版本数据。".to_string()),
                });
            }
        }
    }

    Ok(impacts)
}

fn build_rollback_summary(path: &[String], impacts: &[RollbackImpactSummary]) -> String {
    if path.is_empty() {
        return "无需回退。".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!("将执行回退路径：{}", path.join(" -> ")));
    for impact in impacts {
        let mut line = format!("- {}", impact.title);
        if let Some(count) = impact.count {
            line.push_str(&format!("：{}", count));
        }
        if let (Some(start), Some(end)) = (impact.chapter_start, impact.chapter_end) {
            line.push_str(&format!("（第 {}-{} 章）", start, end));
        }
        if let (Some(start), Some(end)) = (impact.part_start, impact.part_end) {
            line.push_str(&format!("（第 {}-{} 部分）", start, end));
        }
        lines.push(line);
        if let Some(note) = &impact.note {
            lines.push(format!("  {}", note));
        }
    }
    lines.push("注意：回退会删除依赖数据，此操作不可恢复。".to_string());
    lines.join("\n")
}

async fn collect_rollback_media_paths(
    state: &AppState,
    project_id: i64,
    actions: &[CleanupAction],
) -> Result<Vec<String>, String> {
    let needs_chapter_cleanup = actions
        .iter()
        .any(|action| matches!(action, CleanupAction::DeleteChapters));
    if !needs_chapter_cleanup {
        return Ok(Vec::new());
    }

    use a4n_core::models::ai::{generated_image, generated_video};
    use a4n_core::models::chapter::chapter;

    let chapter_ids: Vec<i64> = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Id)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let images = if chapter_ids.is_empty() {
        Vec::new()
    } else {
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::ChapterId.is_in(chapter_ids.clone()))
            .all(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
    };
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut paths = Vec::new();
    for image in images {
        paths.push(image.file_path);
    }
    for video in videos {
        if let Some(path) = video.file_path {
            paths.push(path);
        }
        if let Some(path) = video.thumbnail_path {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn remove_data_file(state: &AppState, path: &str) -> Result<(), String> {
    remove_data_file_in_data_dir(&state.config.data_dir, path, "文件")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rollback_test_impact(kind: &str, count: Option<i32>) -> RollbackImpactSummary {
        RollbackImpactSummary {
            kind: kind.to_string(),
            title: kind.to_string(),
            tables: Vec::new(),
            count,
            chapter_start: None,
            chapter_end: None,
            part_start: None,
            part_end: None,
            note: None,
        }
    }

    #[test]
    fn rollback_impact_count_returns_zero_only_when_impact_is_absent() {
        let impacts = vec![rollback_test_impact("delete_chapters", None)];

        assert_eq!(
            rollback_impact_count(&impacts, "delete_chapter_outlines"),
            Ok(0)
        );

        let err = rollback_impact_count(&impacts, "delete_chapters")
            .expect_err("present impact count must be explicit");
        assert!(err.contains("回退影响 delete_chapters 缺少数量"));
    }

    #[test]
    fn rollback_impact_count_accepts_explicit_count() {
        let impacts = vec![rollback_test_impact("delete_chapters", Some(3))];

        assert_eq!(rollback_impact_count(&impacts, "delete_chapters"), Ok(3));
    }
}
