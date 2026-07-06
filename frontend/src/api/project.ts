import { apiInvoke, type PaginatedResponse } from "./index";
import { readCurrentUserId } from "./authStorage";
import { idNum, type ApiId as Id } from "./shared";

// ============================================================================
// 类型定义（对齐后端 commands/project.rs::ProjectSummary）
// ============================================================================

export interface NovelProject {
  id: number;
  title: string;
  description: string;
  project_type: string;
  project_kind: ProjectKind;
  source_mode: ProjectSourceMode;
  root_project_id: number | null;
  parent_project_id: number | null;
  source_snapshot_id: number | null;
  source_sync_status: SourceSyncStatus;
  derived_project_status: string | null;
  root_project_title: string | null;
  source_snapshot_created_at: string | null;
  derived_chapter_scope: unknown | null;
  status: string;
  current_chapter: number;
  target_chapters: number;
  total_chapters: number | null;
  chapters_per_part: number | null;
  owner_user_id: number;
  owner_username: string | null;
  my_role: ProjectAccessRole | null;
  collaboration_root_project_id: number;
  member_count: number;
  pending_invite_count: number | null;
  progress: ProjectProgressSummary;
  created_at: string | null;
  updated_at: string | null;
}

export interface SourceSnapshotChapter {
  id: number;
  snapshot_id: number;
  root_project_id: number;
  source_chapter_id: number;
  chapter_number: number;
  title: string | null;
  summary: string | null;
  content: string;
  analysis_data_json: unknown | null;
  content_hash: string;
  analysis_hash: string | null;
}

export type ProjectKind = "text" | "comic" | "video" | "galgame";
export type ProjectSourceMode = "self" | "derived" | "freeform";

type SourceSyncStatus = "current" | "root_updated" | "conflict" | "detached";

export type ProjectAccessRole = "owner" | "manager" | "editor" | "viewer";
export type ProjectMemberRole = "manager" | "editor" | "viewer";
export type ProjectAccessScope = "all" | "owned" | "collaborating";
export type ProjectProgressStatus = "idle" | "running" | "blocked" | "failed" | "completed";

export interface ProjectProgressSummary {
  kind: ProjectKind | string;
  stage: string;
  label: string;
  current: number | null;
  total: number | null;
  status: ProjectProgressStatus;
  detail: string | null;
}

export interface ProjectMember {
  project_id: number;
  user_id: number;
  username: string;
  email: string;
  role: ProjectAccessRole;
  is_owner: boolean;
  invited_by_user_id: number | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface ProjectInvite {
  id: number;
  project_id: number;
  project_title: string | null;
  inviter_user_id: number;
  inviter_username: string | null;
  invitee_user_id: number;
  invitee_username: string | null;
  invitee_email: string | null;
  role: ProjectMemberRole;
  status: "pending" | "accepted" | "declined" | "revoked" | "expired";
  expires_at: string;
  accepted_at: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface ProjectStatusHistory {
  id: number;
  project_id: number;
  from_status: string;
  to_status: string;
  reason: string | null;
  created_at: string | null;
}

export interface RollbackImpact {
  kind: string;
  title: string;
  tables: string[];
  count: number | null;
  chapter_start: number | null;
  chapter_end: number | null;
  part_start: number | null;
  part_end: number | null;
  note: string | null;
}

export interface RollbackPreview {
  from_status: string;
  target_status: string;
  path: string[];
  affected_chapters: number;
  affected_outlines: number;
  can_rollback: boolean;
  warning_message: string | null;
  summary: string;
  impacts: RollbackImpact[];
}

export interface AssetGraphNode {
  id: string;
  kind: "text" | "image" | "video" | "profile" | "prompt" | "task" | "media" | string;
  asset_type: string;
  label: string;
  count: number;
  status: string | null;
  layer: number;
  chapter_id: number | null;
  profile_id: number | null;
  record_id: number | null;
  editable: boolean;
  mutable: boolean;
  direct_downstream_count: number;
  downstream_count: number;
  mutation_policy:
    | "readonly_aggregate"
    | "leaf_editable"
    | "requires_conflict_resolution";
  needs_visualization: boolean;
  visualized: boolean;
  summary: string | null;
  details: Record<string, unknown> | null;
}

export interface AssetGraphEdge {
  from: string;
  to: string;
  relation: string;
  count: number;
}

export interface AssetGraphLayer {
  index: number;
  label: string;
  node_ids: string[];
}

export interface AssetDependencyGraph {
  project_id: number;
  generated_at: string;
  nodes: AssetGraphNode[];
  edges: AssetGraphEdge[];
  layers: AssetGraphLayer[];
  warnings: string[];
}

export interface AssetGraphNodeDetail {
  node_id: string;
  title: string;
  kind: string;
  asset_type: string;
  summary: string | null;
  records: AssetLineageRecord[];
  warnings: string[];
}

type AssetLineageOrigin =
  | "derived_text"
  | "llm_generated"
  | "image_generated"
  | "video_generated"
  | "uploaded"
  | "imported"
  | "manual"
  | "extracted_frame"
  | "unknown"
  | string;

export interface AssetLineageRecord {
  id: string;
  label: string;
  asset_kind: string;
  asset_id: number | null;
  status: string | null;
  origin: AssetLineageOrigin;
  source_refs: AssetLineageRef[];
  prompt_sources: PromptSource[];
  media_sources: AssetLineageRef[];
  request_snapshot: unknown | null;
  input_status: "complete" | "missing" | string;
  generation_runs: GenerationRunSnapshot[];
  generation_inputs: GenerationInputSource[];
  result: unknown;
  warnings: string[];
}

export interface AssetLineageRef {
  id: string;
  label: string;
  kind: string;
  asset_type: string;
  record_id: number | null;
}

export interface PromptSource {
  id: string;
  kind: string;
  text: string;
  source_record_id: number | null;
  edited_by_user: boolean | null;
}

export interface GenerationRunSnapshot {
  id: number | null;
  workflow: string;
  step_key: string;
  title: string;
  model_kind: string;
  provider: string | null;
  model: string | null;
  status: string;
  created_at: string | null;
  completed_at: string | null;
}

export interface GenerationInputSource {
  id: string;
  run_id: number | null;
  group: "prompt" | "variables" | "source_text" | "media" | "request" | "model" | "response" | string;
  kind: string;
  label: string;
  content_type: "text" | "json" | "media_ref" | string;
  text: string | null;
  value: unknown | null;
  source_record_id: number | null;
}

interface ProjectGeneratedDataResetSummary {
  project: NovelProject;
  deleted_galgame_scripts: number;
  deleted_galgame_scenes: number;
  deleted_galgame_lines: number;
  deleted_galgame_choices: number;
  deleted_chapters: number;
  deleted_chapter_outlines: number;
  deleted_part_outlines: number;
  deleted_chapter_derived_rows: number;
  deleted_current_blueprints: number;
  deleted_blueprint_characters: number;
  deleted_blueprint_relationships: number;
  preserved_blueprint_versions: number;
  reset_character_profiles: number;
  deleted_character_profiles: number;
  deleted_character_snapshots: number;
  deleted_character_timeline_rows: number;
  deleted_character_portraits: number;
  deleted_object_profiles: number;
  deleted_foreshadowing: number;
  deleted_scene_profiles: number;
  deleted_scene_occurrences: number;
  deleted_scene_transitions: number;
  deleted_scene_entity_states: number;
  deleted_text_anchors: number;
  deleted_profile_evidence: number;
  deleted_import_reviews: number;
  deleted_conversations: number;
  deleted_conversation_messages: number;
  deleted_optimization_sessions: number;
  deleted_optimization_session_messages: number;
  deleted_optimization_turns: number;
  deleted_optimization_events: number;
  deleted_optimization_patch_sets: number;
  deleted_optimization_patches: number;
  deleted_generated_images: number;
  deleted_generated_videos: number;
  deleted_generation_specs: number;
  deleted_prompt_artifacts: number;
  deleted_media_assets: number;
  deleted_media_provenance: number;
  deleted_media_bindings: number;
  deleted_tasks: number;
  deleted_task_dependencies: number;
  deleted_media_files: number;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问项目");
  return id;
};

// ============================================================================
// API 函数（参数名与后端 CreateProjectInput / UpdateProjectInput 对齐）
// ============================================================================

export async function listProjects(
  page: number,
  pageSize: number,
  filters: {
    project_kind?: ProjectKind;
    root_project_id?: number;
    access_scope?: ProjectAccessScope;
  } = {},
): Promise<PaginatedResponse<NovelProject>> {
  return apiInvoke<PaginatedResponse<NovelProject>>("list_projects", {
    userId: currentUserId(),
    page,
    pageSize,
    projectKind: filters.project_kind,
    rootProjectId: filters.root_project_id,
    accessScope: filters.access_scope,
  });
}

export async function getProject(id: Id): Promise<NovelProject> {
  return apiInvoke<NovelProject>("get_project", {
    userId: currentUserId(),
    projectId: idNum(id),
  });
}

export async function getProjectAssetDependencyGraph(
  id: Id,
): Promise<AssetDependencyGraph> {
  return apiInvoke<AssetDependencyGraph>("get_project_asset_dependency_graph", {
    userId: currentUserId(),
    projectId: idNum(id),
  });
}

export async function getProjectAssetDependencyGraphNodeDetail(
  id: Id,
  nodeId: string,
): Promise<AssetGraphNodeDetail> {
  return apiInvoke<AssetGraphNodeDetail>("get_project_asset_dependency_graph_node_detail", {
    userId: currentUserId(),
    projectId: idNum(id),
    nodeId,
  });
}

/**
 * 创建项目（后端 CreateProjectInput {title, description, project_type}）
 */
export async function createProject(input: {
  title: string;
  description: string;
  project_type: string;
}): Promise<NovelProject> {
  return apiInvoke<NovelProject>("create_project", { userId: currentUserId(), input });
}

export async function createDerivedProject(input: {
  root_project_id: number;
  derived_kind: Exclude<ProjectKind, "text">;
  title: string;
  description?: string;
  start_chapter: number;
  end_chapter: number;
  creation_options: Record<string, unknown>;
}): Promise<NovelProject> {
  return apiInvoke<NovelProject>("create_derived_project", {
    userId: currentUserId(),
    input,
  });
}

export async function createFreeformVideoProject(input: {
  title: string;
  description?: string;
}): Promise<NovelProject> {
  return apiInvoke<NovelProject>("create_freeform_video_project", {
    userId: currentUserId(),
    input,
  });
}

export async function listSourceSnapshotChapters(projectId: Id): Promise<SourceSnapshotChapter[]> {
  return apiInvoke<SourceSnapshotChapter[]>("list_source_snapshot_chapters", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

/**
 * 更新项目（后端：update_project(project_id, input: UpdateProjectInput { title?, description? })）
 */
export async function updateProject(
  id: Id,
  input: { title?: string; description?: string },
): Promise<NovelProject> {
  return apiInvoke<NovelProject>("update_project", {
    userId: currentUserId(),
    projectId: idNum(id),
    input,
  });
}

export async function deleteProject(id: Id): Promise<void> {
  return apiInvoke<void>("delete_project", {
    userId: currentUserId(),
    projectId: idNum(id),
  });
}

export async function resetProjectGeneratedData(
  id: Id,
): Promise<ProjectGeneratedDataResetSummary> {
  return apiInvoke<ProjectGeneratedDataResetSummary>(
    "reset_project_generated_data",
    {
      userId: currentUserId(),
      projectId: idNum(id),
    },
  );
}

export async function listProjectMembers(projectId: Id): Promise<ProjectMember[]> {
  return apiInvoke<ProjectMember[]>("list_project_members", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function inviteProjectMember(
  projectId: Id,
  input: { username_or_email: string; role: ProjectMemberRole },
): Promise<ProjectInvite> {
  return apiInvoke<ProjectInvite>("invite_project_member", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input,
  });
}

export async function listProjectInvites(projectId: Id): Promise<ProjectInvite[]> {
  return apiInvoke<ProjectInvite[]>("list_project_invites", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function listMyProjectInvites(): Promise<ProjectInvite[]> {
  return apiInvoke<ProjectInvite[]>("list_my_project_invites", {
    userId: currentUserId(),
  });
}

export async function acceptProjectInvite(inviteId: Id): Promise<ProjectMember> {
  return apiInvoke<ProjectMember>("accept_project_invite", {
    userId: currentUserId(),
    inviteId: idNum(inviteId),
  });
}

export async function declineProjectInvite(inviteId: Id): Promise<ProjectInvite> {
  return apiInvoke<ProjectInvite>("decline_project_invite", {
    userId: currentUserId(),
    inviteId: idNum(inviteId),
  });
}

export async function revokeProjectInvite(projectId: Id, inviteId: Id): Promise<ProjectInvite> {
  return apiInvoke<ProjectInvite>("revoke_project_invite", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    inviteId: idNum(inviteId),
  });
}

export async function updateProjectMemberRole(
  projectId: Id,
  memberUserId: Id,
  role: ProjectMemberRole,
): Promise<ProjectMember> {
  return apiInvoke<ProjectMember>("update_project_member_role", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    memberUserId: idNum(memberUserId),
    input: { role },
  });
}

export async function removeProjectMember(projectId: Id, memberUserId: Id): Promise<void> {
  return apiInvoke<void>("remove_project_member", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    memberUserId: idNum(memberUserId),
  });
}

export async function leaveProject(projectId: Id): Promise<void> {
  return apiInvoke<void>("leave_project", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function getProjectStatusHistory(id: Id): Promise<ProjectStatusHistory[]> {
  return apiInvoke<ProjectStatusHistory[]>("get_project_status_history", {
    userId: currentUserId(),
    projectId: idNum(id),
  });
}

export async function previewRollback(id: Id, target_status: string): Promise<RollbackPreview> {
  return apiInvoke<RollbackPreview>("preview_rollback", {
    userId: currentUserId(),
    projectId: idNum(id),
    targetStatus: target_status,
  });
}

export async function rollbackProjectStatus(id: Id, target: string): Promise<NovelProject> {
  return apiInvoke<NovelProject>("rollback_project_status", {
    userId: currentUserId(),
    projectId: idNum(id),
    target,
  });
}

export async function forwardProjectStatus(id: Id, target: string): Promise<NovelProject> {
  return apiInvoke<NovelProject>("forward_project_status", {
    userId: currentUserId(),
    projectId: idNum(id),
    target,
  });
}

/** 后端 duplicate_project(input: { project_id, new_title }) */
export async function duplicateProject(id: Id, new_title: string): Promise<NovelProject> {
  return apiInvoke<NovelProject>("duplicate_project", {
    userId: currentUserId(),
    input: { project_id: idNum(id), new_title },
  });
}

/** 后端 branch_project(project_id, branch_title, at_status) */
export async function branchProject(
  id: Id,
  branch_title: string,
  at_status: string,
): Promise<NovelProject> {
  return apiInvoke<NovelProject>("branch_project", {
    userId: currentUserId(),
    projectId: idNum(id),
    branchTitle: branch_title,
    atStatus: at_status,
  });
}

export interface ExportProjectPackageData {
  file_name: string;
  data_base64: string;
}

export async function exportProjectFamilyData(input: {
  root_project_id: number;
  include_derived_projects: boolean;
}): Promise<ExportProjectPackageData> {
  return apiInvoke<ExportProjectPackageData>("export_project_family_data", {
    userId: currentUserId(),
    input,
  });
}

export async function exportDerivedProjectData(input: {
  derived_project_id: number;
}): Promise<ExportProjectPackageData> {
  return apiInvoke<ExportProjectPackageData>("export_derived_project_data", {
    userId: currentUserId(),
    input,
  });
}

export async function importProjectFamilyUpload(input: {
  file_name: string;
  data_base64: string;
}): Promise<NovelProject[]> {
  return apiInvoke<NovelProject[]>("import_project_family_upload", {
    userId: currentUserId(),
    input,
  });
}

export async function exportVideoStudioProjectData(input: {
  project_id: number;
}): Promise<ExportProjectPackageData> {
  return apiInvoke<ExportProjectPackageData>("export_video_studio_project_data", {
    userId: currentUserId(),
    input,
  });
}

export async function importVideoStudioProjectUpload(input: {
  file_name: string;
  data_base64: string;
}): Promise<NovelProject> {
  return apiInvoke<NovelProject>("import_video_studio_project_upload", {
    userId: currentUserId(),
    input,
  });
}

export async function importProjectFullUpload(input: {
  file_name: string;
  data_base64: string;
}): Promise<NovelProject> {
  return apiInvoke<NovelProject>("import_project_full_upload", {
    userId: currentUserId(),
    input,
  });
}
