import { apiInvoke } from "./index";
import { idNum, type ApiId as Id } from "./shared";
import type {
  AssetDependencyGraph,
  AssetGraphNodeDetail,
  CreateDerivedProjectInput,
  CreateFreeformVideoProjectInput,
  CreateProjectInput,
  ExportDerivedProjectDataInput,
  ExportProjectFamilyDataInput,
  ExportProjectPackageData,
  ExportVideoStudioProjectDataInput,
  ImportProjectPackageUploadInput,
  InviteProjectMemberInput,
  PaginatedProjects as ApiPaginatedProjects,
  ProjectAccessScope as ApiProjectAccessScope,
  ProjectGeneratedDataResetResponse as ApiProjectGeneratedDataResetResponse,
  ProjectGeneratedDataResetSummary as ApiProjectGeneratedDataResetSummary,
  ProjectInviteSummary,
  ProjectKind,
  ProjectMemberRole as ApiProjectMemberRole,
  ProjectMemberSummary,
  ProjectRollbackPreview,
  ProjectStatus,
  ProjectStatusMachineSummary,
  ProjectSummary,
  SourceSnapshotChapterSummary,
  UpdateProjectInput,
} from "./generated/host";

export {
  PROJECT_FAMILY_PACKAGE_SUFFIX,
  VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
} from "./generated/host";

export type {
  AssetDependencyGraph,
  AssetGraphEdge,
  AssetGraphLayer,
  AssetGraphNode,
  AssetGraphNodeDetail,
  AssetLineageRecord,
  AssetLineageRef,
  ExportDerivedProjectDataInput,
  ExportProjectFamilyDataInput,
  ExportProjectPackageData,
  ExportVideoStudioProjectDataInput,
  ImportProjectPackageUploadInput,
  GenerationInputSource,
  PromptSource,
  ProjectRollbackImpactSummary,
  ProjectRollbackPreview,
  ProjectStatus,
  ProjectStatusHistorySummary,
  ProjectStatusMachineSummary,
  ProjectKind,
  SourceSnapshotChapterSummary,
} from "./generated/host";

// ============================================================================
// 类型定义（对齐后端 commands/project.rs::ProjectSummary）
// ============================================================================

export type NovelProject = Omit<
  ProjectSummary,
  "lifecycle_gate" | "project_kind" | "source_mode" | "source_sync_status"
> & {
  lifecycle_gate: ProjectLifecycleGate;
  project_kind: ProjectKind;
  source_mode: ProjectSourceMode;
  source_sync_status: SourceSyncStatus;
};

export type ProjectLifecycleGate = "active" | "deleting" | "resetting";
type ProjectSourceMode = "self" | "derived" | "freeform";

type SourceSyncStatus = "current" | "root_updated" | "conflict" | "detached";

export type ProjectMemberRole = ApiProjectMemberRole;
export type ProjectAccessScope = ApiProjectAccessScope;

type PaginatedProjects = Omit<ApiPaginatedProjects, "items"> & {
  items: NovelProject[];
};

type ProjectListFilters = {
  project_kind?: ProjectKind;
  root_project_id?: number;
  access_scope: ProjectAccessScope;
};

export type ProjectGeneratedDataResetSummary = Omit<
  ApiProjectGeneratedDataResetSummary,
  "project"
> & {
  project: NovelProject;
};

export type ProjectGeneratedDataResetResponse = Omit<
  ApiProjectGeneratedDataResetResponse,
  "result"
> & {
  result: ProjectGeneratedDataResetSummary | null;
};

export const hasProjectEditorRole = (project: Pick<NovelProject, "my_role">): boolean =>
  project.my_role === "owner" ||
  project.my_role === "manager" ||
  project.my_role === "editor";

export const projectLifecycleBlockedReason = (
  project: Pick<NovelProject, "lifecycle_gate">,
): string | null => {
  if (project.lifecycle_gate === "active") return null;
  if (project.lifecycle_gate === "resetting") {
    return "项目正在重置并清理资料索引，完成前暂不可编辑";
  }
  return "项目正在删除，不能再修改";
};

export const canEditProjectMaterials = (
  project: Pick<NovelProject, "lifecycle_gate" | "my_role">,
): boolean => hasProjectEditorRole(project) && project.lifecycle_gate === "active";

export type ProjectMember = ProjectMemberSummary;
export type ProjectInvite = ProjectInviteSummary;

// ============================================================================
// API 函数（参数名与后端 CreateProjectInput / UpdateProjectInput 对齐）
// ============================================================================

export async function listProjects(
  page: number,
  pageSize: number,
  filters: ProjectListFilters,
): Promise<PaginatedProjects> {
  return apiInvoke<PaginatedProjects>("list_projects", {
    page,
    pageSize,
    projectKind: filters.project_kind,
    rootProjectId: filters.root_project_id,
    accessScope: filters.access_scope,
  });
}

export async function listAllProjects(
  filters: ProjectListFilters,
): Promise<NovelProject[]> {
  const projects: NovelProject[] = [];
  let page = 1;

  while (true) {
    const result = await listProjects(page, 100, filters);
    projects.push(...result.items);
    if (projects.length >= result.total || result.items.length === 0) return projects;
    page += 1;
  }
}

export async function getProject(id: Id): Promise<NovelProject> {
  return apiInvoke<NovelProject>("get_project", {
    projectId: idNum(id),
  });
}

export async function getProjectAssetDependencyGraph(
  id: Id,
): Promise<AssetDependencyGraph> {
  return apiInvoke<AssetDependencyGraph>("get_project_asset_dependency_graph", {
    projectId: idNum(id),
  });
}

export async function getProjectAssetDependencyGraphNodeDetail(
  id: Id,
  nodeId: string,
): Promise<AssetGraphNodeDetail> {
  return apiInvoke<AssetGraphNodeDetail>("get_project_asset_dependency_graph_node_detail", {
    projectId: idNum(id),
    nodeId,
  });
}

/**
 * 创建项目（后端 CreateProjectInput {title, description, project_type}）
 */
export async function createProject(input: CreateProjectInput): Promise<NovelProject> {
  return apiInvoke<NovelProject>("create_project", { input });
}

export async function createDerivedProject(
  input: CreateDerivedProjectInput,
): Promise<NovelProject> {
  return apiInvoke<NovelProject>("create_derived_project", {
    input,
  });
}

export async function createFreeformVideoProject(
  input: CreateFreeformVideoProjectInput,
): Promise<NovelProject> {
  return apiInvoke<NovelProject>("create_freeform_video_project", {
    input,
  });
}

export async function listSourceSnapshotChapters(projectId: Id): Promise<SourceSnapshotChapterSummary[]> {
  return apiInvoke<SourceSnapshotChapterSummary[]>("list_source_snapshot_chapters", {
    projectId: idNum(projectId),
  });
}

/**
 * 更新项目（后端：update_project(project_id, input: UpdateProjectInput { title?, description? })）
 */
export async function updateProject(
  id: Id,
  input: UpdateProjectInput,
): Promise<NovelProject> {
  return apiInvoke<NovelProject>("update_project", {
    projectId: idNum(id),
    input,
  });
}

export async function deleteProject(id: Id): Promise<void> {
  return apiInvoke<void>("delete_project", {
    projectId: idNum(id),
  });
}

export async function resetProjectGeneratedData(
  id: Id,
): Promise<ProjectGeneratedDataResetResponse> {
  return apiInvoke<ProjectGeneratedDataResetResponse>(
    "reset_project_generated_data",
    {
      projectId: idNum(id),
    },
  );
}

export async function listProjectMembers(projectId: Id): Promise<ProjectMember[]> {
  return apiInvoke<ProjectMember[]>("list_project_members", {
    projectId: idNum(projectId),
  });
}

export async function inviteProjectMember(
  projectId: Id,
  input: InviteProjectMemberInput,
): Promise<void> {
  return apiInvoke<void>("invite_project_member", {
    projectId: idNum(projectId),
    input,
  });
}

export async function listProjectInvites(projectId: Id): Promise<ProjectInvite[]> {
  return apiInvoke<ProjectInvite[]>("list_project_invites", {
    projectId: idNum(projectId),
  });
}

export async function listMyProjectInvites(): Promise<ProjectInvite[]> {
  return apiInvoke<ProjectInvite[]>("list_my_project_invites");
}

export async function acceptProjectInvite(inviteId: Id): Promise<void> {
  return apiInvoke<void>("accept_project_invite", {
    inviteId: idNum(inviteId),
  });
}

export async function declineProjectInvite(inviteId: Id): Promise<void> {
  return apiInvoke<void>("decline_project_invite", {
    inviteId: idNum(inviteId),
  });
}

export async function revokeProjectInvite(projectId: Id, inviteId: Id): Promise<void> {
  return apiInvoke<void>("revoke_project_invite", {
    projectId: idNum(projectId),
    inviteId: idNum(inviteId),
  });
}

export async function updateProjectMemberRole(
  projectId: Id,
  memberUserId: Id,
  role: ProjectMemberRole,
): Promise<void> {
  return apiInvoke<void>("update_project_member_role", {
    projectId: idNum(projectId),
    memberUserId: idNum(memberUserId),
    input: { role },
  });
}

export async function removeProjectMember(projectId: Id, memberUserId: Id): Promise<void> {
  return apiInvoke<void>("remove_project_member", {
    projectId: idNum(projectId),
    memberUserId: idNum(memberUserId),
  });
}

export async function leaveProject(projectId: Id): Promise<void> {
  return apiInvoke<void>("leave_project", {
    projectId: idNum(projectId),
  });
}

export async function getProjectStatusMachine(id: Id): Promise<ProjectStatusMachineSummary> {
  return apiInvoke<ProjectStatusMachineSummary>("get_project_status_machine", {
    projectId: idNum(id),
  });
}

export async function previewRollback(
  id: Id,
  target: ProjectStatus,
): Promise<ProjectRollbackPreview> {
  return apiInvoke<ProjectRollbackPreview>("preview_rollback", {
    projectId: idNum(id),
    target,
  });
}

export async function rollbackProjectStatus(
  id: Id,
  target: ProjectStatus,
): Promise<NovelProject> {
  return apiInvoke<NovelProject>("rollback_project_status", {
    projectId: idNum(id),
    target,
  });
}

export async function forwardProjectStatus(
  id: Id,
  target: ProjectStatus,
): Promise<NovelProject> {
  return apiInvoke<NovelProject>("forward_project_status", {
    projectId: idNum(id),
    target,
  });
}

/** 后端 duplicate_project(input: { project_id, new_title }) */
export async function duplicateProject(id: Id, new_title: string): Promise<NovelProject> {
  return apiInvoke<NovelProject>("duplicate_project", {
    input: { project_id: idNum(id), new_title },
  });
}

export async function exportProjectFamilyData(
  input: ExportProjectFamilyDataInput,
): Promise<ExportProjectPackageData> {
  return apiInvoke<ExportProjectPackageData>("export_project_family_data", {
    input,
  });
}

export async function exportDerivedProjectData(
  input: ExportDerivedProjectDataInput,
): Promise<ExportProjectPackageData> {
  return apiInvoke<ExportProjectPackageData>("export_derived_project_data", {
    input,
  });
}

export async function importProjectFamilyUpload(
  input: ImportProjectPackageUploadInput,
): Promise<NovelProject[]> {
  return apiInvoke<NovelProject[]>("import_project_family_upload", {
    input,
  });
}

export async function exportVideoStudioProjectData(
  input: ExportVideoStudioProjectDataInput,
): Promise<ExportProjectPackageData> {
  return apiInvoke<ExportProjectPackageData>("export_video_studio_project_data", {
    input,
  });
}

export async function importVideoStudioProjectUpload(
  input: ImportProjectPackageUploadInput,
): Promise<NovelProject> {
  return apiInvoke<NovelProject>("import_video_studio_project_upload", {
    input,
  });
}
