use crate::AppState;
use crate::actions::access;
use crate::actions::galgame_records;
use crate::file_utils::{
    deletable_data_dir_path, deletable_data_file_path, remove_data_dir_in_data_dir,
    remove_data_file_in_data_dir,
};
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::canvas::canvas_graph;
use a4n_core::models::chapter::{chapter_manga_prompt, chapter_video_prompt};
use a4n_core::models::galgame::{galgame_line, galgame_scene, galgame_script};
use a4n_core::models::generation::{
    generation_run, generation_run_target, generation_spec, generation_spec_entity_ref,
    generation_spec_media_ref, generation_spec_text_anchor, prompt_artifact, workflow_checkpoint,
    workflow_progress_event, workflow_run,
};
use a4n_core::models::media::{media_asset, media_binding, media_provenance};
use a4n_core::models::project::{
    novel_project, project_derivation, project_source_snapshot, source_snapshot_chapter,
};
use a4n_core::models::task::{task_dependency, task_queue};
use a4n_services::collaboration::{AccessibleProjectListFilter, ProjectAccessScope};
use a4n_services::manga::{MANGA_PAGE_PROMPT_TYPE, PIPELINE_STATUS_PROMPT_TYPE};
use a4n_services::project::{
    AssetDependencyGraph, AssetGraphNodeDetail, CreateDerivedProjectParams,
    CreateFreeformVideoProjectParams, DerivedChapterScope, ProjectGeneratedDataResetResult,
    SOURCE_MODE_DERIVED, SOURCE_MODE_FREEFORM,
};
use a4n_services::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

mod freeform_reset;
#[cfg(test)]
mod tests;

use self::freeform_reset::{
    FreeformGeneratedResetTargets, collect_freeform_generated_reset_targets,
    mark_freeform_canvas_generated_refs_missing,
};
#[cfg(test)]
use self::freeform_reset::{
    MissingCanvasRef, clear_reset_generated_ref_payload, is_freeform_generated_media_asset,
    is_freeform_imported_canvas_image, is_freeform_imported_canvas_video,
    reset_missing_canvas_ref_for_node,
};

const PROJECT_KIND_TEXT: &str = "text";
const PROJECT_KIND_COMIC: &str = "comic";
const PROJECT_KIND_VIDEO: &str = "video";
const PROJECT_KIND_GALGAME: &str = "galgame";
const VIDEO_PROMPT_TYPE: &str = "video_prompt";
const VIDEO_FRAME_PROMPT_TYPE: &str = "frame_prompt";
const VIDEO_SEGMENT_PLAN_PROMPT_TYPE: &str = "frame_segment_plan";
const CHAPTER_VIDEO_TYPE: &str = "chapter_video";
const GALGAME_PIPELINE_WORKFLOW_KIND: &str = "galgame_pipeline";
const FREEFORM_VIDEO_PROGRESS_TYPES: [&str; 4] = [
    "canvas_video",
    "canvas_reference_media",
    "canvas_trim",
    "canvas_assembly",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub project_kind: String,
    pub source_mode: String,
    pub root_project_id: Option<i64>,
    pub parent_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_sync_status: String,
    pub derived_project_status: Option<String>,
    pub root_project_title: Option<String>,
    pub source_snapshot_created_at: Option<String>,
    pub derived_chapter_scope: Option<serde_json::Value>,
    pub status: String,
    pub current_chapter: i32,
    pub target_chapters: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub owner_user_id: i64,
    pub owner_username: Option<String>,
    pub my_role: Option<String>,
    pub collaboration_root_project_id: i64,
    pub member_count: u64,
    pub pending_invite_count: Option<u64>,
    pub progress: ProjectProgressSummary,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectProgressStatus {
    Idle,
    Running,
    Blocked,
    Failed,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectProgressSummary {
    pub kind: String,
    pub stage: String,
    pub label: String,
    pub current: Option<i64>,
    pub total: Option<i64>,
    pub status: ProjectProgressStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSnapshotChapterSummary {
    pub id: i64,
    pub snapshot_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: String,
    pub analysis_data_json: Option<serde_json::Value>,
    pub content_hash: String,
    pub analysis_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedProjects {
    pub items: Vec<ProjectSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProjectInput {
    pub title: String,
    pub description: Option<String>,
    pub project_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateFreeformVideoProjectInput {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateDerivedProjectInput {
    pub root_project_id: i64,
    pub derived_kind: String,
    pub title: String,
    pub description: Option<String>,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub creation_options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProjectInput {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGeneratedDataResetSummary {
    pub project: ProjectSummary,
    pub deleted_galgame_scripts: u64,
    pub deleted_galgame_scenes: u64,
    pub deleted_galgame_lines: u64,
    pub deleted_galgame_choices: u64,
    pub deleted_chapters: u64,
    pub deleted_chapter_outlines: u64,
    pub deleted_part_outlines: u64,
    pub deleted_chapter_derived_rows: u64,
    pub deleted_current_blueprints: u64,
    pub deleted_blueprint_characters: u64,
    pub deleted_blueprint_relationships: u64,
    pub preserved_blueprint_versions: u64,
    pub reset_character_profiles: u64,
    pub deleted_character_profiles: u64,
    pub deleted_character_snapshots: u64,
    pub deleted_character_timeline_rows: u64,
    pub deleted_character_portraits: u64,
    pub deleted_object_profiles: u64,
    pub deleted_foreshadowing: u64,
    pub deleted_scene_profiles: u64,
    pub deleted_scene_occurrences: u64,
    pub deleted_scene_transitions: u64,
    pub deleted_scene_entity_states: u64,
    pub deleted_text_anchors: u64,
    pub deleted_profile_evidence: u64,
    pub deleted_import_reviews: u64,
    pub deleted_conversations: u64,
    pub deleted_conversation_messages: u64,
    pub deleted_optimization_sessions: u64,
    pub deleted_optimization_session_messages: u64,
    pub deleted_optimization_turns: u64,
    pub deleted_optimization_events: u64,
    pub deleted_optimization_patch_sets: u64,
    pub deleted_optimization_patches: u64,
    pub deleted_generated_images: u64,
    pub deleted_generated_videos: u64,
    pub deleted_generation_specs: u64,
    pub deleted_prompt_artifacts: u64,
    pub deleted_media_assets: u64,
    pub deleted_media_provenance: u64,
    pub deleted_media_bindings: u64,
    pub deleted_tasks: u64,
    pub deleted_task_dependencies: u64,
    pub deleted_media_files: usize,
}

pub async fn get_project_summary(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectSummary, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let project = state
        .project
        .get(project_id)
        .await
        .map_err(|e| e.to_string())?;
    project_summary_with_family_for_user(state, user_id, &project).await
}

pub async fn get_project_asset_dependency_graph(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<AssetDependencyGraph, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    state
        .project
        .asset_dependency_graph(project_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_project_asset_dependency_graph_node_detail(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    node_id: String,
) -> Result<AssetGraphNodeDetail, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    state
        .project
        .asset_dependency_graph_node_detail(project_id, &node_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_projects(
    state: &AppState,
    user_id: i64,
    page: u64,
    page_size: u64,
    project_kind: Option<&str>,
    root_project_id: Option<i64>,
    access_scope: Option<&str>,
) -> Result<PaginatedProjects, String> {
    let offset = project_list_offset(page, page_size)?;
    let (projects, total) = state
        .collaboration
        .list_accessible_projects(
            user_id,
            offset,
            page_size,
            AccessibleProjectListFilter {
                project_kind,
                root_project_id,
                access_scope: ProjectAccessScope::from_optional(access_scope)
                    .map_err(|e| e.to_string())?,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    let items = project_summaries_with_family_for_user(state, user_id, &projects).await?;
    Ok(PaginatedProjects {
        items,
        total,
        page,
        page_size,
    })
}

pub async fn create_project(
    state: &AppState,
    user_id: i64,
    input: CreateProjectInput,
) -> Result<ProjectSummary, String> {
    let title = required_text(&input.title, "项目标题")?;
    let description = optional_text(input.description.as_deref());
    let project_type = required_optional_text(input.project_type.as_deref(), "项目类型")?;
    let project = state
        .project
        .create(user_id, title, &description, project_type)
        .await
        .map_err(|e| e.to_string())?;

    project_summary_with_family_for_user(state, user_id, &project).await
}

pub async fn create_derived_project(
    state: &AppState,
    user_id: i64,
    input: CreateDerivedProjectInput,
) -> Result<ProjectSummary, String> {
    let title = required_text(&input.title, "项目标题")?;
    let description = required_optional_text(input.description.as_deref(), "项目简介")?;
    let root = state
        .collaboration
        .resolve_root_project(input.root_project_id)
        .await
        .map_err(|e| e.to_string())?;
    state
        .collaboration
        .verify_permission(
            input.root_project_id,
            user_id,
            a4n_services::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(|e| e.to_string())?;
    let project = state
        .project
        .create_derived_project(
            root.user_id,
            CreateDerivedProjectParams {
                root_project_id: input.root_project_id,
                derived_kind: input.derived_kind,
                title: title.to_string(),
                description: description.to_string(),
                chapter_scope: DerivedChapterScope {
                    start_chapter: input.start_chapter,
                    end_chapter: input.end_chapter,
                },
                creation_options_json: input.creation_options,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    project_summary_with_family_for_user(state, user_id, &project).await
}

pub async fn create_freeform_video_project(
    state: &AppState,
    user_id: i64,
    input: CreateFreeformVideoProjectInput,
) -> Result<ProjectSummary, String> {
    let title = required_text(&input.title, "项目标题")?;
    let description = optional_text(input.description.as_deref());
    let project = state
        .project
        .create_freeform_video_project(
            user_id,
            CreateFreeformVideoProjectParams {
                title: title.to_string(),
                description,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    project_summary_with_family_for_user(state, user_id, &project).await
}

pub async fn list_source_snapshot_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<SourceSnapshotChapterSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let project = state
        .project
        .get(project_id)
        .await
        .map_err(|e| e.to_string())?;
    if project.project_kind == "text" {
        return Err("文本项目没有源文本快照章节".to_string());
    }
    if project.source_mode == SOURCE_MODE_FREEFORM {
        return Err("自由视频项目没有源文本快照章节".to_string());
    }
    let snapshot_id = project
        .source_snapshot_id
        .ok_or_else(|| format!("派生项目缺少源文本快照: {project_id}"))?;
    let chapters = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(chapters
        .iter()
        .map(to_source_snapshot_chapter_summary)
        .collect())
}

pub async fn update_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateProjectInput,
) -> Result<ProjectSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let project = state
        .project
        .update(
            project_id,
            input.title.as_deref(),
            input.description.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    project_summary_with_family_for_user(state, user_id, &project).await
}

pub async fn delete_project(state: &AppState, user_id: i64, project_id: i64) -> Result<(), String> {
    access::verify_project_owner_string(state, user_id, project_id).await?;
    let media_paths = collect_project_deletion_media_paths(state, project_id).await?;
    let project_paths = ProjectPaths::new(&state.config.data_dir, project_id);
    validate_data_file_delete_paths(state, &media_paths)?;
    deletable_data_dir_path(&state.config.data_dir, project_paths.root(), "项目数据目录")?;
    state
        .project
        .delete(project_id)
        .await
        .map_err(|e| e.to_string())?;
    state
        .rag
        .clear_project(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for path in &media_paths {
        remove_data_file(state, path)?;
    }
    remove_data_dir_in_data_dir(&state.config.data_dir, project_paths.root(), "项目数据目录")?;
    Ok(())
}

pub async fn reset_project_generated_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectGeneratedDataResetSummary, String> {
    access::verify_project_owner_string(state, user_id, project_id).await?;
    let project = state
        .project
        .get(project_id)
        .await
        .map_err(|e| e.to_string())?;
    if project.project_kind == PROJECT_KIND_VIDEO && project.source_mode == SOURCE_MODE_FREEFORM {
        let targets = collect_freeform_generated_reset_targets(state, project_id).await?;
        let media_paths = targets.media_paths.clone();
        let reset = reset_freeform_video_generated_data(state, user_id, &project, targets).await?;
        state
            .rag
            .clear_project(project_id)
            .await
            .map_err(|e| e.to_string())?;
        for path in &media_paths {
            remove_data_file(state, path)?;
        }
        return Ok(reset);
    }
    let media_paths = collect_project_generated_media_paths(state, project_id).await?;
    let reset = state
        .project
        .reset_generated_data(project_id)
        .await
        .map_err(|e| e.to_string())?;
    state
        .rag
        .clear_project(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for path in &media_paths {
        remove_data_file(state, path)?;
    }
    to_generated_data_reset_summary(state, user_id, &reset, media_paths.len()).await
}

async fn project_summary_with_family(
    state: &AppState,
    project: &novel_project::Model,
) -> Result<ProjectSummary, String> {
    let mut summary = to_summary(project);
    summary.progress = build_project_progress_summary(state, project).await?;
    if let Some(root_project_id) = project.root_project_id {
        let root = novel_project::Entity::find_by_id(root_project_id)
            .one(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("派生项目引用的根文本项目不存在: {root_project_id}"))?;
        summary.root_project_title = Some(root.title);
    }
    if let Some(source_snapshot_id) = project.source_snapshot_id {
        let snapshot = project_source_snapshot::Entity::find_by_id(source_snapshot_id)
            .one(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("派生项目引用的源文本快照不存在: {source_snapshot_id}"))?;
        summary.source_snapshot_created_at = Some(snapshot.created_at.to_string());
    }
    if project.source_mode == SOURCE_MODE_DERIVED {
        let derivation = project_derivation::Entity::find()
            .filter(project_derivation::Column::DerivedProjectId.eq(project.id))
            .one(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("派生项目缺少派生关系: {}", project.id))?;
        summary.derived_chapter_scope = Some(derivation.chapter_scope_json);
    }
    Ok(summary)
}

pub async fn project_summary_with_family_for_user(
    state: &AppState,
    user_id: i64,
    project: &novel_project::Model,
) -> Result<ProjectSummary, String> {
    let mut summary = project_summary_with_family(state, project).await?;
    let info = state
        .collaboration
        .access_info(project.id, user_id)
        .await
        .map_err(|e| e.to_string())?;
    summary.owner_user_id = info.owner_user_id;
    summary.owner_username = info.owner_username;
    summary.my_role = info.my_role.map(|role| role.as_str().to_string());
    summary.collaboration_root_project_id = info.collaboration_root_project_id;
    summary.member_count = info.member_count;
    summary.pending_invite_count = info.pending_invite_count;
    Ok(summary)
}

pub async fn project_summaries_with_family_for_user(
    state: &AppState,
    user_id: i64,
    projects: &[novel_project::Model],
) -> Result<Vec<ProjectSummary>, String> {
    let mut root_project_ids = Vec::new();
    let mut snapshot_ids = Vec::new();
    let mut derived_project_ids = Vec::new();
    for project in projects {
        if let Some(root_project_id) = project.root_project_id {
            root_project_ids.push(root_project_id);
        }
        if let Some(source_snapshot_id) = project.source_snapshot_id {
            snapshot_ids.push(source_snapshot_id);
        }
        if project.source_mode == SOURCE_MODE_DERIVED {
            derived_project_ids.push(project.id);
        }
    }
    root_project_ids.sort_unstable();
    root_project_ids.dedup();
    snapshot_ids.sort_unstable();
    snapshot_ids.dedup();
    derived_project_ids.sort_unstable();
    derived_project_ids.dedup();

    let roots_by_id = project_models_by_id(state.db.as_ref(), &root_project_ids).await?;
    let snapshots_by_id = source_snapshots_by_id(state.db.as_ref(), &snapshot_ids).await?;
    let derivations_by_project_id =
        derivations_by_project_id(state.db.as_ref(), &derived_project_ids).await?;
    let access_infos = state
        .collaboration
        .access_info_for_projects(projects, user_id)
        .await
        .map_err(|e| e.to_string())?;
    let progress_by_project_id = build_project_progress_summaries(state, projects).await?;

    let mut summaries = Vec::with_capacity(projects.len());
    for project in projects {
        let mut summary = to_summary(project);
        summary.progress = progress_by_project_id
            .get(&project.id)
            .cloned()
            .ok_or_else(|| format!("项目缺少进度摘要: {}", project.id))?;
        if let Some(root_project_id) = project.root_project_id {
            let root = roots_by_id
                .get(&root_project_id)
                .ok_or_else(|| format!("派生项目引用的根文本项目不存在: {root_project_id}"))?;
            summary.root_project_title = Some(root.title.clone());
        }
        if let Some(source_snapshot_id) = project.source_snapshot_id {
            let snapshot = snapshots_by_id
                .get(&source_snapshot_id)
                .ok_or_else(|| format!("派生项目引用的源文本快照不存在: {source_snapshot_id}"))?;
            summary.source_snapshot_created_at = Some(snapshot.created_at.to_string());
        }
        if project.source_mode == SOURCE_MODE_DERIVED {
            let derivation = derivations_by_project_id
                .get(&project.id)
                .ok_or_else(|| format!("派生项目缺少派生关系: {}", project.id))?;
            summary.derived_chapter_scope = Some(derivation.chapter_scope_json.clone());
        }
        let info = access_infos
            .get(&project.id)
            .ok_or_else(|| format!("项目缺少协作访问信息: {}", project.id))?;
        summary.owner_user_id = info.owner_user_id;
        summary.owner_username = info.owner_username.clone();
        summary.my_role = info.my_role.map(|role| role.as_str().to_string());
        summary.collaboration_root_project_id = info.collaboration_root_project_id;
        summary.member_count = info.member_count;
        summary.pending_invite_count = info.pending_invite_count;
        summaries.push(summary);
    }
    Ok(summaries)
}

async fn project_models_by_id<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<HashMap<i64, novel_project::Model>, String>
where
    C: sea_orm::ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = novel_project::Entity::find()
        .filter(novel_project::Column::Id.is_in(project_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

async fn source_snapshots_by_id<C>(
    db: &C,
    snapshot_ids: &[i64],
) -> Result<HashMap<i64, project_source_snapshot::Model>, String>
where
    C: sea_orm::ConnectionTrait,
{
    if snapshot_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = project_source_snapshot::Entity::find()
        .filter(project_source_snapshot::Column::Id.is_in(snapshot_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

async fn derivations_by_project_id<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<HashMap<i64, project_derivation::Model>, String>
where
    C: sea_orm::ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = project_derivation::Entity::find()
        .filter(project_derivation::Column::DerivedProjectId.is_in(project_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| (row.derived_project_id, row))
        .collect())
}

fn to_summary(project: &novel_project::Model) -> ProjectSummary {
    ProjectSummary {
        id: project.id,
        title: project.title.clone(),
        description: project.description.clone(),
        project_type: project.project_type.clone(),
        project_kind: project.project_kind.clone(),
        source_mode: project.source_mode.clone(),
        root_project_id: project.root_project_id,
        parent_project_id: project.parent_project_id,
        source_snapshot_id: project.source_snapshot_id,
        source_sync_status: project.source_sync_status.clone(),
        derived_project_status: project.derived_project_status.clone(),
        root_project_title: None,
        source_snapshot_created_at: None,
        derived_chapter_scope: None,
        status: project.status.clone(),
        current_chapter: project.current_chapter,
        target_chapters: project.target_chapters,
        total_chapters: project.total_chapters,
        chapters_per_part: project.chapters_per_part,
        owner_user_id: project.user_id,
        owner_username: None,
        my_role: None,
        collaboration_root_project_id: project.root_project_id.unwrap_or(project.id),
        member_count: 0,
        pending_invite_count: None,
        progress: text_project_progress(project),
        created_at: Some(project.created_at.to_string()),
        updated_at: Some(project.updated_at.to_string()),
    }
}

async fn build_project_progress_summary(
    state: &AppState,
    project: &novel_project::Model,
) -> Result<ProjectProgressSummary, String> {
    match project.project_kind.as_str() {
        PROJECT_KIND_TEXT => Ok(text_project_progress(project)),
        PROJECT_KIND_COMIC => comic_project_progress(state, project.id).await,
        PROJECT_KIND_VIDEO => video_project_progress(state, project).await,
        PROJECT_KIND_GALGAME => galgame_project_progress(state, project.id).await,
        _ => Ok(unknown_project_progress(&project.project_kind)),
    }
}

async fn build_project_progress_summaries(
    state: &AppState,
    projects: &[novel_project::Model],
) -> Result<HashMap<i64, ProjectProgressSummary>, String> {
    let mut progress_by_project_id = HashMap::with_capacity(projects.len());
    let mut comic_project_ids = Vec::new();
    let mut video_project_ids = Vec::new();
    let mut freeform_video_project_ids = Vec::new();
    let mut galgame_project_ids = Vec::new();

    for project in projects {
        match project.project_kind.as_str() {
            PROJECT_KIND_TEXT => {
                progress_by_project_id.insert(project.id, text_project_progress(project));
            }
            PROJECT_KIND_COMIC => comic_project_ids.push(project.id),
            PROJECT_KIND_VIDEO if project.source_mode == SOURCE_MODE_FREEFORM => {
                freeform_video_project_ids.push(project.id);
            }
            PROJECT_KIND_VIDEO => video_project_ids.push(project.id),
            PROJECT_KIND_GALGAME => galgame_project_ids.push(project.id),
            _ => {
                progress_by_project_id
                    .insert(project.id, unknown_project_progress(&project.project_kind));
            }
        }
    }

    progress_by_project_id
        .extend(comic_project_progress_by_project(state, &comic_project_ids).await?);
    progress_by_project_id
        .extend(video_project_progress_by_project(state, &video_project_ids).await?);
    progress_by_project_id.extend(
        freeform_video_project_progress_by_project(state, &freeform_video_project_ids).await?,
    );
    progress_by_project_id
        .extend(galgame_project_progress_by_project(state, &galgame_project_ids).await?);
    Ok(progress_by_project_id)
}

fn unknown_project_progress(project_kind: &str) -> ProjectProgressSummary {
    ProjectProgressSummary {
        kind: project_kind.to_string(),
        stage: "unknown".to_string(),
        label: "未知状态".to_string(),
        current: None,
        total: None,
        status: ProjectProgressStatus::Blocked,
        detail: Some(format!("未知项目类型：{project_kind}")),
    }
}

fn text_project_progress(project: &novel_project::Model) -> ProjectProgressSummary {
    let label = match project.status.as_str() {
        "draft" => "草稿",
        "blueprint_ready" => "蓝图就绪",
        "part_outlines_ready" => "分部大纲就绪",
        "chapter_outlines_ready" => "章节大纲就绪",
        "writing" => "写作中",
        "completed" => "已完成",
        _ => "未知状态",
    };
    let current = Some(i64::from(project.current_chapter.max(0)));
    let total = Some(i64::from(project.target_chapters.max(0)));
    ProjectProgressSummary {
        kind: PROJECT_KIND_TEXT.to_string(),
        stage: project.status.clone(),
        label: label.to_string(),
        current,
        total,
        status: if project.status == "completed" {
            ProjectProgressStatus::Completed
        } else if label == "未知状态" {
            ProjectProgressStatus::Blocked
        } else {
            ProjectProgressStatus::Idle
        },
        detail: progress_count_detail("当前章节", current, total),
    }
}

async fn comic_project_progress_by_project(
    state: &AppState,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, String> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(
            chapter_manga_prompt::Column::PromptType
                .is_in([MANGA_PAGE_PROMPT_TYPE, PIPELINE_STATUS_PROMPT_TYPE]),
        )
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let mut counts_by_project: HashMap<i64, ProgressCounts> = HashMap::new();
    let mut pipeline_by_project: HashMap<i64, PipelineProjectState> = HashMap::new();
    for row in rows {
        if row.prompt_type == MANGA_PAGE_PROMPT_TYPE {
            counts_by_project
                .entry(row.project_id)
                .or_default()
                .add(&row.status);
        } else if row.prompt_type == PIPELINE_STATUS_PROMPT_TYPE {
            pipeline_by_project
                .entry(row.project_id)
                .or_default()
                .add(row.layout.as_ref());
        }
    }
    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        progress.insert(
            *project_id,
            comic_project_progress_from_counts(
                counts_by_project.remove(project_id).unwrap_or_default(),
                pipeline_by_project.remove(project_id).unwrap_or_default(),
            ),
        );
    }
    Ok(progress)
}

async fn comic_project_progress(
    state: &AppState,
    project_id: i64,
) -> Result<ProjectProgressSummary, String> {
    let page_prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let status_rows = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(PIPELINE_STATUS_PROMPT_TYPE))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut counts = ProgressCounts::default();
    for prompt in &page_prompts {
        counts.add(&prompt.status);
    }
    let mut pipeline = PipelineProjectState::default();
    for row in &status_rows {
        pipeline.add(row.layout.as_ref());
    }

    Ok(comic_project_progress_from_counts(counts, pipeline))
}

fn comic_project_progress_from_counts(
    counts: ProgressCounts,
    pipeline: PipelineProjectState,
) -> ProjectProgressSummary {
    let total = non_zero(counts.total());
    let current = total.map(|_| counts.completed);
    if counts.failed > 0 || pipeline.failed {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_failed",
            "漫画生成失败",
            current,
            total,
            ProjectProgressStatus::Failed,
            Some("存在失败的漫画流水线或页面生成记录".to_string()),
        );
    }
    if counts.running > 0 || pipeline.running {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_running",
            "漫画生成中",
            current,
            total,
            ProjectProgressStatus::Running,
            pipeline
                .message
                .or_else(|| progress_count_detail("已完成页面", current, total)),
        );
    }
    if pipeline.cancelled {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_cancelled",
            "漫画生成已取消",
            current,
            total,
            ProjectProgressStatus::Blocked,
            pipeline.message,
        );
    }
    if total.is_some() && counts.completed >= counts.total() {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_ready",
            "漫画已生成",
            current,
            total,
            ProjectProgressStatus::Completed,
            progress_count_detail("已完成页面", current, total),
        );
    }
    if counts.completed > 0 {
        return project_progress(
            PROJECT_KIND_COMIC,
            "manga_partial",
            "漫画部分完成",
            current,
            total,
            ProjectProgressStatus::Idle,
            progress_count_detail("已完成页面", current, total),
        );
    }
    project_progress(
        PROJECT_KIND_COMIC,
        "manga_workspace",
        "漫画制作中",
        current,
        total,
        ProjectProgressStatus::Idle,
        progress_count_detail("页面", current, total),
    )
}

async fn video_project_progress_by_project(
    state: &AppState,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, String> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_video::Column::VideoType.eq(CHAPTER_VIDEO_TYPE))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut prompts_by_project: HashMap<i64, Vec<chapter_video_prompt::Model>> = HashMap::new();
    for prompt in prompts {
        prompts_by_project
            .entry(prompt.project_id)
            .or_default()
            .push(prompt);
    }
    let mut videos_by_project: HashMap<i64, Vec<generated_video::Model>> = HashMap::new();
    for video in videos {
        videos_by_project
            .entry(video.project_id)
            .or_default()
            .push(video);
    }

    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        let prompts = prompts_by_project.remove(project_id).unwrap_or_default();
        let videos = videos_by_project.remove(project_id).unwrap_or_default();
        progress.insert(
            *project_id,
            video_project_progress_from_counts(video_progress_facts_from_rows(&prompts, &videos)),
        );
    }
    Ok(progress)
}

async fn video_project_progress(
    state: &AppState,
    project: &novel_project::Model,
) -> Result<ProjectProgressSummary, String> {
    if project.source_mode == SOURCE_MODE_FREEFORM {
        return freeform_video_project_progress(state, project.id).await;
    }
    let project_id = project.id;
    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .filter(generated_video::Column::VideoType.eq(CHAPTER_VIDEO_TYPE))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(video_project_progress_from_counts(
        video_progress_facts_from_rows(&prompts, &videos),
    ))
}

async fn freeform_video_project_progress_by_project(
    state: &AppState,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, String> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let graph_counts = canvas_graph_counts_by_project(state, project_ids).await?;
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_video::Column::VideoType.is_in(FREEFORM_VIDEO_PROGRESS_TYPES))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let mut counts_by_project: HashMap<i64, ProgressCounts> = HashMap::new();
    for video in videos {
        counts_by_project
            .entry(video.project_id)
            .or_default()
            .add(&video.status);
    }
    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        progress.insert(
            *project_id,
            freeform_video_project_progress_from_counts(
                graph_counts.get(project_id).copied().unwrap_or(0),
                counts_by_project.remove(project_id).unwrap_or_default(),
            ),
        );
    }
    Ok(progress)
}

#[derive(Debug, FromQueryResult)]
struct ProjectCountRow {
    project_id: i64,
    count: i64,
}

async fn canvas_graph_counts_by_project(
    state: &AppState,
    project_ids: &[i64],
) -> Result<HashMap<i64, u64>, String> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = canvas_graph::Entity::find()
        .select_only()
        .column(canvas_graph::Column::ProjectId)
        .column_as(canvas_graph::Column::Id.count(), "count")
        .filter(canvas_graph::Column::ProjectId.is_in(project_ids.iter().copied()))
        .group_by(canvas_graph::Column::ProjectId)
        .into_model::<ProjectCountRow>()
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| (row.project_id, row.count.max(0) as u64))
        .collect())
}

async fn freeform_video_project_progress(
    state: &AppState,
    project_id: i64,
) -> Result<ProjectProgressSummary, String> {
    let graph_count = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(project_id))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .filter(generated_video::Column::VideoType.is_in(FREEFORM_VIDEO_PROGRESS_TYPES))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut counts = ProgressCounts::default();
    for video in &videos {
        counts.add(&video.status);
    }
    Ok(freeform_video_project_progress_from_counts(
        graph_count,
        counts,
    ))
}

fn freeform_video_project_progress_from_counts(
    graph_count: u64,
    counts: ProgressCounts,
) -> ProjectProgressSummary {
    let total = non_zero(counts.total());
    let current = total.map(|_| counts.completed);
    if counts.failed > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_canvas_failed",
            "画布视频生成失败",
            current,
            total,
            ProjectProgressStatus::Failed,
            Some("存在失败的画布视频产物".to_string()),
        );
    }
    if counts.running > 0 || counts.pending > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_canvas_running",
            "画布视频生成中",
            current,
            total,
            ProjectProgressStatus::Running,
            progress_count_detail("已完成视频", current, total),
        );
    }
    if total.is_some() && counts.completed >= counts.total() {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_canvas_ready",
            "画布视频已生成",
            current,
            total,
            ProjectProgressStatus::Completed,
            progress_count_detail("已完成视频", current, total),
        );
    }
    if counts.completed > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_canvas_partial",
            "画布视频部分完成",
            current,
            total,
            ProjectProgressStatus::Idle,
            progress_count_detail("已完成视频", current, total),
        );
    }
    project_progress(
        PROJECT_KIND_VIDEO,
        "video_canvas_workspace",
        "视频画布制作中",
        None,
        non_zero(i64::try_from(graph_count).unwrap_or(i64::MAX)),
        ProjectProgressStatus::Idle,
        None,
    )
}

fn video_progress_facts_from_rows(
    prompts: &[chapter_video_prompt::Model],
    videos: &[generated_video::Model],
) -> VideoProgressFacts {
    let current_prompts = current_video_prompt_rows(prompts);
    let mut prompt_counts = ProgressCounts::default();
    let mut video_prompt_total = 0;
    let mut frame_prompt_total = 0;
    let mut segment_plan_total = 0;
    let mut prompt_slot_statuses = BTreeMap::new();
    let mut prompt_id_statuses = BTreeMap::new();
    for prompt in current_prompts {
        prompt_counts.add(&prompt.status);
        match prompt.prompt_type.as_str() {
            VIDEO_PROMPT_TYPE => {
                video_prompt_total += 1;
                prompt_id_statuses.insert(prompt.id, prompt.status.clone());
                if let Some(slot) = video_slot_key_from_prompt(prompt) {
                    prompt_slot_statuses.insert(slot, prompt.status.clone());
                }
            }
            VIDEO_FRAME_PROMPT_TYPE => frame_prompt_total += 1,
            VIDEO_SEGMENT_PLAN_PROMPT_TYPE => segment_plan_total += 1,
            _ => {}
        }
    }

    let mut video_counts = ProgressCounts::default();
    for video in latest_video_rows(videos) {
        if should_count_video_for_progress(video, &prompt_slot_statuses, &prompt_id_statuses) {
            video_counts.add(&video.status);
        }
    }

    VideoProgressFacts {
        prompt_counts,
        video_counts,
        video_prompt_total,
        frame_prompt_total,
        segment_plan_total,
    }
}

fn video_project_progress_from_counts(facts: VideoProgressFacts) -> ProjectProgressSummary {
    let total = non_zero(facts.video_prompt_total.max(facts.video_counts.total()));
    let current = total.map(|_| facts.video_counts.completed);
    if facts.prompt_counts.failed > 0 || facts.video_counts.failed > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_failed",
            "视频生成失败",
            current,
            total,
            ProjectProgressStatus::Failed,
            Some("存在当前失败的视频提示词或视频生成记录".to_string()),
        );
    }
    if facts.prompt_counts.running > 0
        || facts.video_counts.running > 0
        || facts.video_counts.pending > 0
    {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_generating",
            "视频生成中",
            current,
            total,
            ProjectProgressStatus::Running,
            progress_count_detail("已完成视频片段", current, total),
        );
    }
    if let Some(total) = total
        && facts.video_counts.completed >= total
    {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_ready",
            "视频已生成",
            current,
            Some(total),
            ProjectProgressStatus::Completed,
            progress_count_detail("已完成视频片段", current, Some(total)),
        );
    }
    if facts.video_counts.completed > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_partial",
            "视频部分完成",
            current,
            total,
            ProjectProgressStatus::Idle,
            progress_count_detail("已完成视频片段", current, total),
        );
    }
    if facts.video_prompt_total > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_ready_to_generate",
            "视频待生成",
            current,
            total,
            ProjectProgressStatus::Idle,
            progress_count_detail("视频提示词", Some(0), total),
        );
    }
    if facts.frame_prompt_total > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_frame_prompting",
            "视频成图提示词制作中",
            None,
            Some(facts.frame_prompt_total),
            ProjectProgressStatus::Idle,
            Some(format!("已有 {} 条成图提示词", facts.frame_prompt_total)),
        );
    }
    if facts.segment_plan_total > 0 {
        return project_progress(
            PROJECT_KIND_VIDEO,
            "video_planning",
            "视频片段规划中",
            None,
            Some(facts.segment_plan_total),
            ProjectProgressStatus::Idle,
            Some(format!("已有 {} 条片段规划", facts.segment_plan_total)),
        );
    }
    project_progress(
        PROJECT_KIND_VIDEO,
        "video_workspace",
        "视频制作中",
        None,
        None,
        ProjectProgressStatus::Idle,
        None,
    )
}

async fn galgame_project_progress(
    state: &AppState,
    project_id: i64,
) -> Result<ProjectProgressSummary, String> {
    let (script, scene_count, line_count) =
        galgame_records::galgame_project_progress(state, project_id).await?;
    let pipeline = latest_galgame_pipeline_state(state, project_id).await?;
    Ok(galgame_project_progress_from_counts(
        script.is_some(),
        scene_count,
        line_count,
        pipeline,
    ))
}

fn galgame_project_progress_from_counts(
    script_exists: bool,
    scene_count: i64,
    line_count: i64,
    pipeline: PipelineProjectState,
) -> ProjectProgressSummary {
    let total = non_zero(line_count);
    let current = total.map(|_| line_count);
    if pipeline.failed {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_pipeline_failed",
            "Galgame 管线失败",
            current,
            total,
            ProjectProgressStatus::Failed,
            pipeline
                .message
                .or_else(|| Some("最近一次管线失败".to_string())),
        );
    }
    if pipeline.running {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_pipeline_running",
            "Galgame 生成中",
            current,
            total,
            ProjectProgressStatus::Running,
            pipeline
                .message
                .or_else(|| Some("Galgame 管线正在运行".to_string())),
        );
    }
    if pipeline.cancelled {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_pipeline_cancelled",
            "Galgame 管线已取消",
            current,
            total,
            ProjectProgressStatus::Blocked,
            pipeline.message,
        );
    }
    if !script_exists {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_empty",
            "Galgame 待生成",
            Some(0),
            None,
            ProjectProgressStatus::Idle,
            Some("尚未生成剧本".to_string()),
        );
    }
    if line_count <= 0 {
        return project_progress(
            PROJECT_KIND_GALGAME,
            "galgame_script_empty",
            "Galgame 剧本为空",
            Some(0),
            Some(scene_count.max(0)),
            ProjectProgressStatus::Blocked,
            Some("已创建剧本但没有行内容".to_string()),
        );
    }
    project_progress(
        PROJECT_KIND_GALGAME,
        "galgame_script_ready",
        "Galgame 剧本已生成",
        Some(line_count),
        Some(line_count),
        ProjectProgressStatus::Completed,
        Some(format!("{} 场，{} 行", scene_count, line_count)),
    )
}

async fn galgame_project_progress_by_project(
    state: &AppState,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, String> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let scripts = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .column(galgame_script::Column::ProjectId)
        .filter(galgame_script::Column::ProjectId.is_in(project_ids.iter().copied()))
        .into_model::<GalgameScriptProgressRow>()
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;

    let mut script_id_by_project_id = HashMap::with_capacity(scripts.len());
    let mut project_id_by_script_id = HashMap::with_capacity(scripts.len());
    let mut script_ids = Vec::with_capacity(scripts.len());
    for script in scripts {
        script_ids.push(script.id);
        script_id_by_project_id.insert(script.project_id, script.id);
        project_id_by_script_id.insert(script.id, script.project_id);
    }

    let scenes = if script_ids.is_empty() {
        Vec::new()
    } else {
        galgame_scene::Entity::find()
            .select_only()
            .column(galgame_scene::Column::Id)
            .column(galgame_scene::Column::ScriptId)
            .filter(galgame_scene::Column::ScriptId.is_in(script_ids.iter().copied()))
            .into_model::<GalgameSceneProgressRow>()
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
    };

    let mut scene_counts_by_project_id: HashMap<i64, i64> = HashMap::new();
    let mut project_id_by_scene_id = HashMap::with_capacity(scenes.len());
    let mut scene_ids = Vec::with_capacity(scenes.len());
    for scene in scenes {
        if let Some(project_id) = project_id_by_script_id.get(&scene.script_id).copied() {
            scene_ids.push(scene.id);
            project_id_by_scene_id.insert(scene.id, project_id);
            *scene_counts_by_project_id.entry(project_id).or_insert(0) += 1;
        }
    }

    let line_counts_by_scene_id = line_counts_by_scene(state, &scene_ids).await?;
    let mut line_counts_by_project_id: HashMap<i64, i64> = HashMap::new();
    for (scene_id, line_count) in line_counts_by_scene_id {
        if let Some(project_id) = project_id_by_scene_id.get(&scene_id).copied() {
            *line_counts_by_project_id.entry(project_id).or_insert(0) += line_count;
        }
    }

    let pipeline_by_project_id =
        latest_galgame_pipeline_states_by_project(state, project_ids).await?;
    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        progress.insert(
            *project_id,
            galgame_project_progress_from_counts(
                script_id_by_project_id.contains_key(project_id),
                scene_counts_by_project_id
                    .get(project_id)
                    .copied()
                    .unwrap_or(0),
                line_counts_by_project_id
                    .get(project_id)
                    .copied()
                    .unwrap_or(0),
                pipeline_by_project_id
                    .get(project_id)
                    .cloned()
                    .unwrap_or_default(),
            ),
        );
    }
    Ok(progress)
}

#[derive(Debug, FromQueryResult)]
struct GalgameScriptProgressRow {
    id: i64,
    project_id: i64,
}

#[derive(Debug, FromQueryResult)]
struct GalgameSceneProgressRow {
    id: i64,
    script_id: i64,
}

#[derive(Debug, FromQueryResult)]
struct GalgameLineCountRow {
    scene_id: i64,
    count: i64,
}

async fn line_counts_by_scene(
    state: &AppState,
    scene_ids: &[i64],
) -> Result<HashMap<i64, i64>, String> {
    if scene_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = galgame_line::Entity::find()
        .select_only()
        .column(galgame_line::Column::SceneId)
        .column_as(galgame_line::Column::Id.count(), "count")
        .filter(galgame_line::Column::SceneId.is_in(scene_ids.iter().copied()))
        .group_by(galgame_line::Column::SceneId)
        .into_model::<GalgameLineCountRow>()
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| (row.scene_id, row.count.max(0)))
        .collect())
}

async fn latest_galgame_pipeline_states_by_project(
    state: &AppState,
    project_ids: &[i64],
) -> Result<HashMap<i64, PipelineProjectState>, String> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let runs = workflow_run::Entity::find()
        .distinct_on([workflow_run::Column::ProjectId])
        .filter(workflow_run::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(workflow_run::Column::WorkflowKind.eq(GALGAME_PIPELINE_WORKFLOW_KIND))
        .order_by_asc(workflow_run::Column::ProjectId)
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(runs
        .iter()
        .filter_map(|run| {
            run.project_id
                .map(|project_id| (project_id, galgame_pipeline_state_from_workflow_run(run)))
        })
        .collect())
}

async fn latest_galgame_pipeline_state(
    state: &AppState,
    project_id: i64,
) -> Result<PipelineProjectState, String> {
    let run = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq(GALGAME_PIPELINE_WORKFLOW_KIND))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(run
        .as_ref()
        .map(galgame_pipeline_state_from_workflow_run)
        .unwrap_or_default())
}

fn galgame_pipeline_state_from_workflow_run(run: &workflow_run::Model) -> PipelineProjectState {
    let mut pipeline = PipelineProjectState::default();
    match run.status.trim() {
        WORKFLOW_STATUS_RUNNING => pipeline.running = true,
        WORKFLOW_STATUS_FAILED => pipeline.failed = true,
        WORKFLOW_STATUS_CANCELLED => pipeline.cancelled = true,
        _ => {}
    }
    pipeline.message = run
        .error_message
        .as_deref()
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_string)
        .or_else(|| {
            run.progress_json
                .as_ref()
                .and_then(|progress| progress.get("message"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .map(str::to_string)
        });
    pipeline
}

#[derive(Debug, Clone, Copy, Default)]
struct ProgressCounts {
    pending: i64,
    running: i64,
    completed: i64,
    failed: i64,
    cancelled: i64,
}

impl ProgressCounts {
    fn add(&mut self, status: &str) {
        match status.trim() {
            "pending" => self.pending += 1,
            "running" | "generating" => self.running += 1,
            "completed" => self.completed += 1,
            "failed" => self.failed += 1,
            "cancelled" => self.cancelled += 1,
            _ => {}
        }
    }

    fn total(self) -> i64 {
        self.pending + self.running + self.completed + self.failed + self.cancelled
    }
}

#[derive(Debug, Clone, Default)]
struct PipelineProjectState {
    running: bool,
    failed: bool,
    cancelled: bool,
    message: Option<String>,
}

impl PipelineProjectState {
    fn add(&mut self, layout: Option<&Value>) {
        let Some(layout) = layout else {
            return;
        };
        let status = layout
            .get("pipeline_status")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        match status {
            "running" => self.running = true,
            "failed" => self.failed = true,
            "cancelled" => self.cancelled = true,
            _ => {}
        }
        if self.message.is_none() {
            self.message = layout
                .get("message")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .map(str::to_string);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct VideoProgressFacts {
    prompt_counts: ProgressCounts,
    video_counts: ProgressCounts,
    video_prompt_total: i64,
    frame_prompt_total: i64,
    segment_plan_total: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VideoPromptProgressKey {
    chapter_id: i64,
    workflow_mode: String,
    prompt_type: String,
    shot_number: i32,
}

impl From<&chapter_video_prompt::Model> for VideoPromptProgressKey {
    fn from(prompt: &chapter_video_prompt::Model) -> Self {
        Self {
            chapter_id: prompt.chapter_id,
            workflow_mode: prompt.workflow_mode.clone(),
            prompt_type: prompt.prompt_type.clone(),
            shot_number: prompt.shot_number,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VideoSlotKey {
    chapter_id: i64,
    workflow_mode: String,
    shot_number: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum VideoProgressVideoKey {
    Slot(VideoSlotKey),
    Prompt(i64),
    Record(i64),
}

fn current_video_prompt_rows(
    prompts: &[chapter_video_prompt::Model],
) -> Vec<&chapter_video_prompt::Model> {
    let mut by_key = BTreeMap::<VideoPromptProgressKey, &chapter_video_prompt::Model>::new();
    for prompt in prompts {
        let key = VideoPromptProgressKey::from(prompt);
        match by_key.get(&key) {
            Some(existing) if !prompt_row_is_newer(prompt, existing) => {}
            _ => {
                by_key.insert(key, prompt);
            }
        }
    }
    by_key.into_values().collect()
}

fn latest_video_rows(videos: &[generated_video::Model]) -> Vec<&generated_video::Model> {
    let mut by_key = BTreeMap::<VideoProgressVideoKey, &generated_video::Model>::new();
    for video in videos {
        let key = video_progress_video_key(video);
        match by_key.get(&key) {
            Some(existing) if !video_row_is_newer(video, existing) => {}
            _ => {
                by_key.insert(key, video);
            }
        }
    }
    by_key.into_values().collect()
}

fn prompt_row_is_newer(
    candidate: &chapter_video_prompt::Model,
    existing: &chapter_video_prompt::Model,
) -> bool {
    candidate.updated_at > existing.updated_at
        || (candidate.updated_at == existing.updated_at && candidate.id > existing.id)
}

fn video_row_is_newer(
    candidate: &generated_video::Model,
    existing: &generated_video::Model,
) -> bool {
    candidate.updated_at > existing.updated_at
        || (candidate.updated_at == existing.updated_at && candidate.id > existing.id)
}

fn video_progress_video_key(video: &generated_video::Model) -> VideoProgressVideoKey {
    if let Some(slot) = video_slot_key_from_video(video) {
        return VideoProgressVideoKey::Slot(slot);
    }
    if let Some(prompt_id) = video_metadata_i64(video, "prompt_id") {
        return VideoProgressVideoKey::Prompt(prompt_id);
    }
    VideoProgressVideoKey::Record(video.id)
}

fn video_slot_key_from_prompt(prompt: &chapter_video_prompt::Model) -> Option<VideoSlotKey> {
    (prompt.shot_number > 0).then(|| VideoSlotKey {
        chapter_id: prompt.chapter_id,
        workflow_mode: prompt.workflow_mode.clone(),
        shot_number: prompt.shot_number,
    })
}

fn video_slot_key_from_video(video: &generated_video::Model) -> Option<VideoSlotKey> {
    Some(VideoSlotKey {
        chapter_id: video.chapter_id?,
        workflow_mode: video.workflow_mode.clone().unwrap_or_default(),
        shot_number: video_metadata_i64(video, "shot_number").and_then(|value| {
            i32::try_from(value)
                .ok()
                .and_then(|value| (value > 0).then_some(value))
        })?,
    })
}

fn video_metadata_i64(video: &generated_video::Model, key: &str) -> Option<i64> {
    video.metadata_json.as_ref()?.get(key)?.as_i64()
}

fn should_count_video_for_progress(
    video: &generated_video::Model,
    prompt_slot_statuses: &BTreeMap<VideoSlotKey, String>,
    prompt_id_statuses: &BTreeMap<i64, String>,
) -> bool {
    if video.status != "failed" {
        return true;
    }
    if let Some(slot) = video_slot_key_from_video(video)
        && let Some(status) = prompt_slot_statuses.get(&slot)
    {
        return status == "failed";
    }
    if let Some(prompt_id) = video_metadata_i64(video, "prompt_id")
        && let Some(status) = prompt_id_statuses.get(&prompt_id)
    {
        return status == "failed";
    }
    true
}

fn project_progress(
    kind: &str,
    stage: &str,
    label: &str,
    current: Option<i64>,
    total: Option<i64>,
    status: ProjectProgressStatus,
    detail: Option<String>,
) -> ProjectProgressSummary {
    ProjectProgressSummary {
        kind: kind.to_string(),
        stage: stage.to_string(),
        label: label.to_string(),
        current,
        total,
        status,
        detail,
    }
}

fn progress_count_detail(label: &str, current: Option<i64>, total: Option<i64>) -> Option<String> {
    match (current, total) {
        (Some(current), Some(total)) if total > 0 => Some(format!("{label} {current}/{total}")),
        _ => None,
    }
}

fn non_zero(value: i64) -> Option<i64> {
    (value > 0).then_some(value)
}

async fn reset_freeform_video_generated_data(
    state: &AppState,
    user_id: i64,
    project: &novel_project::Model,
    targets: FreeformGeneratedResetTargets,
) -> Result<ProjectGeneratedDataResetSummary, String> {
    let tx = state.db.begin().await.map_err(|e| e.to_string())?;
    let project_id = project.id;
    let deleted_media_files = targets.media_paths.len();

    let generation_run_ids = generation_run::Entity::find()
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .all(&tx)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    let workflow_run_ids = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .all(&tx)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    let task_ids = task_queue::Entity::find()
        .filter(task_queue::Column::ProjectId.eq(project_id))
        .all(&tx)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();

    if !generation_run_ids.is_empty() {
        generation_run_target::Entity::delete_many()
            .filter(generation_run_target::Column::GenerationRunId.is_in(generation_run_ids))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !targets.image_ids.is_empty() {
        generation_run_target::Entity::delete_many()
            .filter(generation_run_target::Column::TargetTable.eq("generated_image"))
            .filter(generation_run_target::Column::TargetId.is_in(targets.image_ids.clone()))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !targets.video_ids.is_empty() {
        generation_run_target::Entity::delete_many()
            .filter(generation_run_target::Column::TargetTable.eq("generated_video"))
            .filter(generation_run_target::Column::TargetId.is_in(targets.video_ids.clone()))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    generation_run::Entity::delete_many()
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .exec(&tx)
        .await
        .map_err(|e| e.to_string())?;

    let mut deleted_task_dependencies = 0;
    if !task_ids.is_empty() {
        deleted_task_dependencies += task_dependency::Entity::delete_many()
            .filter(task_dependency::Column::TaskId.is_in(task_ids.clone()))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected;
        deleted_task_dependencies += task_dependency::Entity::delete_many()
            .filter(task_dependency::Column::DependsOnTaskId.is_in(task_ids))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected;
    }
    let mut deleted_media_bindings = 0;
    let mut deleted_media_provenance = 0;
    if !targets.media_asset_ids.is_empty() {
        deleted_media_bindings = media_binding::Entity::delete_many()
            .filter(media_binding::Column::MediaAssetId.is_in(targets.media_asset_ids.clone()))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected;
        deleted_media_provenance = media_provenance::Entity::delete_many()
            .filter(media_provenance::Column::MediaAssetId.is_in(targets.media_asset_ids.clone()))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected;
    }
    let deleted_tasks = task_queue::Entity::delete_many()
        .filter(task_queue::Column::ProjectId.eq(project_id))
        .exec(&tx)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected;

    if !workflow_run_ids.is_empty() {
        workflow_checkpoint::Entity::delete_many()
            .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(workflow_run_ids.clone()))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?;
        workflow_progress_event::Entity::delete_many()
            .filter(workflow_progress_event::Column::WorkflowRunId.is_in(workflow_run_ids))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    workflow_run::Entity::delete_many()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .exec(&tx)
        .await
        .map_err(|e| e.to_string())?;

    let mut deleted_prompt_artifacts = 0;
    if !targets.generation_spec_ids.is_empty() {
        generation_spec_media_ref::Entity::delete_many()
            .filter(
                generation_spec_media_ref::Column::GenerationSpecId
                    .is_in(targets.generation_spec_ids.clone()),
            )
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?;
        if !targets.prompt_artifact_ids.is_empty() {
            deleted_prompt_artifacts = prompt_artifact::Entity::delete_many()
                .filter(prompt_artifact::Column::Id.is_in(targets.prompt_artifact_ids.clone()))
                .exec(&tx)
                .await
                .map_err(|e| e.to_string())?
                .rows_affected;
        }
        generation_spec_text_anchor::Entity::delete_many()
            .filter(
                generation_spec_text_anchor::Column::GenerationSpecId
                    .is_in(targets.generation_spec_ids.clone()),
            )
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?;
        generation_spec_entity_ref::Entity::delete_many()
            .filter(
                generation_spec_entity_ref::Column::GenerationSpecId
                    .is_in(targets.generation_spec_ids.clone()),
            )
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !targets.media_asset_ids.is_empty() {
        generation_spec_media_ref::Entity::delete_many()
            .filter(
                generation_spec_media_ref::Column::MediaAssetId
                    .is_in(targets.media_asset_ids.clone()),
            )
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    mark_freeform_canvas_generated_refs_missing(&tx, project_id, &targets).await?;
    let deleted_generation_specs = if targets.generation_spec_ids.is_empty() {
        0
    } else {
        generation_spec::Entity::delete_many()
            .filter(generation_spec::Column::Id.is_in(targets.generation_spec_ids.clone()))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected
    };
    let deleted_media_assets = if targets.media_asset_ids.is_empty() {
        0
    } else {
        media_asset::Entity::delete_many()
            .filter(media_asset::Column::Id.is_in(targets.media_asset_ids))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected
    };

    let deleted_generated_images = if targets.image_ids.is_empty() {
        0
    } else {
        generated_image::Entity::delete_many()
            .filter(generated_image::Column::Id.is_in(targets.image_ids))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected
    };
    let deleted_generated_videos = if targets.video_ids.is_empty() {
        0
    } else {
        generated_video::Entity::delete_many()
            .filter(generated_video::Column::Id.is_in(targets.video_ids))
            .exec(&tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected
    };

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(ProjectGeneratedDataResetSummary {
        project: project_summary_with_family_for_user(state, user_id, project).await?,
        deleted_galgame_scripts: 0,
        deleted_galgame_scenes: 0,
        deleted_galgame_lines: 0,
        deleted_galgame_choices: 0,
        deleted_chapters: 0,
        deleted_chapter_outlines: 0,
        deleted_part_outlines: 0,
        deleted_chapter_derived_rows: 0,
        deleted_current_blueprints: 0,
        deleted_blueprint_characters: 0,
        deleted_blueprint_relationships: 0,
        preserved_blueprint_versions: 0,
        reset_character_profiles: 0,
        deleted_character_profiles: 0,
        deleted_character_snapshots: 0,
        deleted_character_timeline_rows: 0,
        deleted_character_portraits: 0,
        deleted_object_profiles: 0,
        deleted_foreshadowing: 0,
        deleted_scene_profiles: 0,
        deleted_scene_occurrences: 0,
        deleted_scene_transitions: 0,
        deleted_scene_entity_states: 0,
        deleted_text_anchors: 0,
        deleted_profile_evidence: 0,
        deleted_import_reviews: 0,
        deleted_conversations: 0,
        deleted_conversation_messages: 0,
        deleted_optimization_sessions: 0,
        deleted_optimization_session_messages: 0,
        deleted_optimization_turns: 0,
        deleted_optimization_events: 0,
        deleted_optimization_patch_sets: 0,
        deleted_optimization_patches: 0,
        deleted_generated_images,
        deleted_generated_videos,
        deleted_generation_specs,
        deleted_prompt_artifacts,
        deleted_media_assets,
        deleted_media_provenance,
        deleted_media_bindings,
        deleted_tasks,
        deleted_task_dependencies,
        deleted_media_files,
    })
}

fn to_source_snapshot_chapter_summary(
    chapter: &source_snapshot_chapter::Model,
) -> SourceSnapshotChapterSummary {
    SourceSnapshotChapterSummary {
        id: chapter.id,
        snapshot_id: chapter.snapshot_id,
        root_project_id: chapter.root_project_id,
        source_chapter_id: chapter.source_chapter_id,
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        summary: chapter.summary.clone(),
        content: chapter.content.clone(),
        analysis_data_json: chapter.analysis_data_json.clone(),
        content_hash: chapter.content_hash.clone(),
        analysis_hash: chapter.analysis_hash.clone(),
    }
}

async fn to_generated_data_reset_summary(
    state: &AppState,
    user_id: i64,
    result: &ProjectGeneratedDataResetResult,
    deleted_media_files: usize,
) -> Result<ProjectGeneratedDataResetSummary, String> {
    Ok(ProjectGeneratedDataResetSummary {
        project: project_summary_with_family_for_user(state, user_id, &result.project).await?,
        deleted_galgame_scripts: result.deleted_galgame_scripts,
        deleted_galgame_scenes: result.deleted_galgame_scenes,
        deleted_galgame_lines: result.deleted_galgame_lines,
        deleted_galgame_choices: result.deleted_galgame_choices,
        deleted_chapters: result.deleted_chapters,
        deleted_chapter_outlines: result.deleted_chapter_outlines,
        deleted_part_outlines: result.deleted_part_outlines,
        deleted_chapter_derived_rows: result.deleted_chapter_derived_rows,
        deleted_current_blueprints: result.deleted_current_blueprints,
        deleted_blueprint_characters: result.deleted_blueprint_characters,
        deleted_blueprint_relationships: result.deleted_blueprint_relationships,
        preserved_blueprint_versions: result.preserved_blueprint_versions,
        reset_character_profiles: result.reset_character_profiles,
        deleted_character_profiles: result.deleted_character_profiles,
        deleted_character_snapshots: result.deleted_character_snapshots,
        deleted_character_timeline_rows: result.deleted_character_timeline_rows,
        deleted_character_portraits: result.deleted_character_portraits,
        deleted_object_profiles: result.deleted_object_profiles,
        deleted_foreshadowing: result.deleted_foreshadowing,
        deleted_scene_profiles: result.deleted_scene_profiles,
        deleted_scene_occurrences: result.deleted_scene_occurrences,
        deleted_scene_transitions: result.deleted_scene_transitions,
        deleted_scene_entity_states: result.deleted_scene_entity_states,
        deleted_text_anchors: result.deleted_text_anchors,
        deleted_profile_evidence: result.deleted_profile_evidence,
        deleted_import_reviews: result.deleted_import_reviews,
        deleted_conversations: result.deleted_conversations,
        deleted_conversation_messages: result.deleted_conversation_messages,
        deleted_optimization_sessions: result.deleted_optimization_sessions,
        deleted_optimization_session_messages: result.deleted_optimization_session_messages,
        deleted_optimization_turns: result.deleted_optimization_turns,
        deleted_optimization_events: result.deleted_optimization_events,
        deleted_optimization_patch_sets: result.deleted_optimization_patch_sets,
        deleted_optimization_patches: result.deleted_optimization_patches,
        deleted_generated_images: result.deleted_generated_images,
        deleted_generated_videos: result.deleted_generated_videos,
        deleted_generation_specs: result.deleted_generation_specs,
        deleted_prompt_artifacts: result.deleted_prompt_artifacts,
        deleted_media_assets: result.deleted_media_assets,
        deleted_media_provenance: result.deleted_media_provenance,
        deleted_media_bindings: result.deleted_media_bindings,
        deleted_tasks: result.deleted_tasks,
        deleted_task_dependencies: result.deleted_task_dependencies,
        deleted_media_files,
    })
}

async fn collect_project_generated_media_paths(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<String>, String> {
    use a4n_core::models::ai::{generated_image, generated_video};
    use a4n_core::models::character::character_portrait;

    let mut paths = BTreeSet::new();
    for image in generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
    {
        paths.insert(image.file_path);
    }
    for video in generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
    {
        if let Some(path) = video.file_path {
            paths.insert(path);
        }
        if let Some(path) = video.thumbnail_path {
            paths.insert(path);
        }
    }
    for portrait in character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
    {
        paths.insert(portrait.image_path);
    }
    for asset in media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
    {
        if let Some(path) = asset.file_path {
            paths.insert(path);
        }
        if let Some(path) = asset.thumbnail_path {
            paths.insert(path);
        }
    }
    Ok(paths.into_iter().collect())
}

async fn collect_project_deletion_media_paths(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<String>, String> {
    collect_project_generated_media_paths(state, project_id).await
}

fn validate_data_file_delete_paths<'a>(
    state: &AppState,
    paths: impl IntoIterator<Item = &'a String>,
) -> Result<(), String> {
    for path in paths {
        deletable_data_file_path(&state.config.data_dir, path, "项目媒体文件")?;
    }
    Ok(())
}

fn remove_data_file(state: &AppState, path: &str) -> Result<(), String> {
    remove_data_file_in_data_dir(&state.config.data_dir, path, "文件")
}

fn project_list_offset(page: u64, limit: u64) -> Result<u64, String> {
    page.checked_sub(1)
        .and_then(|page_index| page_index.checked_mul(limit))
        .ok_or_else(|| "项目分页参数过大".to_string())
}

fn required_text<'a>(value: &'a str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{field} 不能为空"))
    } else {
        Ok(value)
    }
}

fn required_optional_text<'a>(value: Option<&'a str>, field: &str) -> Result<&'a str, String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{field} 不能为空"))
}

fn optional_text(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}
