import { hasRecentBlueprintGenerationPending } from "./blueprintPending";
import { getProjectCapabilities, resolveWorkflowStage } from "./projectStatus";

export interface RoutedProject {
  id?: number | string;
  status?: string | null;
  project_kind?: string | null;
  source_mode?: string | null;
}

export interface DerivedProjectPresentation {
  root_project_title?: string | null;
  derived_chapter_scope?: unknown | null;
  source_sync_status?: string | null;
}

const PROJECT_KINDS = ["text", "video", "comic", "galgame"] as const;
export type RoutedProjectKind = (typeof PROJECT_KINDS)[number];

export const PROJECT_KIND_LABELS: Record<RoutedProjectKind, string> = {
  text: "文本项目",
  video: "视频项目",
  comic: "漫画项目",
  galgame: "GALGame 项目",
};

export const PROJECT_ACCESS_ROLE_LABELS = {
  owner: "所有者",
  manager: "管理员",
  editor: "编辑者",
  viewer: "只读成员",
} as const;

const SOURCE_SYNC_STATUS_LABELS: Record<string, string> = {
  current: "已同步",
  root_updated: "源文本有更新",
  conflict: "存在冲突",
  detached: "已脱离源项目",
};

const projectId = (project: RoutedProject): string => {
  const id = String(project.id ?? "").trim();
  if (!id) throw new Error("项目 ID 缺失，无法计算项目路由");
  return id;
};

const isDraftLikeProject = (project: RoutedProject): boolean =>
  resolveWorkflowStage(project.status) === "draft";

const projectKind = (project: RoutedProject): RoutedProjectKind => {
  const kind = String(project.project_kind ?? "").trim();
  if (PROJECT_KINDS.includes(kind as RoutedProjectKind)) return kind as RoutedProjectKind;
  throw new Error(`项目族类型无效，无法计算项目路由: ${kind || "<empty>"}`);
};

export const getProjectKindLabel = (project: RoutedProject): string =>
  PROJECT_KIND_LABELS[projectKind(project)];

/** 兼容管理端仍返回的 legacy project_type（如 novel）。 */
export const getProjectTypeLabel = (value: string | null | undefined): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  if (normalized === "novel" || normalized === "text") return PROJECT_KIND_LABELS.text;
  if (normalized === "video") return PROJECT_KIND_LABELS.video;
  if (normalized === "comic" || normalized === "manga") return PROJECT_KIND_LABELS.comic;
  if (normalized === "galgame") return PROJECT_KIND_LABELS.galgame;
  return "未知类型";
};

export const getSourceSyncStatusLabel = (value: string | null | undefined): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  return SOURCE_SYNC_STATUS_LABELS[normalized] ?? (normalized || "未知");
};

export const formatDerivedChapterScope = (scope: unknown): string | null => {
  if (!scope || typeof scope !== "object") return null;
  const record = scope as Record<string, unknown>;
  const start = Number(record.start);
  const end = Number(record.end);
  if (!Number.isInteger(start) || !Number.isInteger(end)) return null;
  return start === end ? `第 ${start} 章` : `第 ${start}-${end} 章`;
};

export const getDerivedProjectSubtitle = (
  project: DerivedProjectPresentation,
  fallback: string,
): string => {
  const parts = [
    project.root_project_title ? `源项目：${project.root_project_title}` : null,
    formatDerivedChapterScope(project.derived_chapter_scope),
    getSourceSyncStatusLabel(project.source_sync_status),
  ].filter((part): part is string => Boolean(part));
  return parts.join(" · ") || fallback;
};

export const getProjectAccessRoleLabel = (value: string | null | undefined): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  return (
    PROJECT_ACCESS_ROLE_LABELS[
      normalized as keyof typeof PROJECT_ACCESS_ROLE_LABELS
    ] ?? (normalized || "成员")
  );
};

export const getProjectPrimaryRoute = (project: RoutedProject): string => {
  const id = projectId(project);
  if (projectKind(project) === "video" && project.source_mode === "freeform") {
    return `/video-project/${id}/studio`;
  }
  if (projectKind(project) === "video") return `/video-project/${id}`;
  if (projectKind(project) === "comic") return `/comic-project/${id}`;
  if (projectKind(project) === "galgame") return `/galgame-project/${id}`;
  return isDraftLikeProject(project) ? `/inspiration/${id}` : `/novel/${id}`;
};

export const getProjectPrimaryLabel = (project: RoutedProject): string =>
  projectKind(project) === "video"
    ? "进入视频项目"
    : projectKind(project) === "comic"
      ? "进入漫画项目"
      : projectKind(project) === "galgame"
        ? "进入 GALGame 项目"
        : isDraftLikeProject(project)
          ? "继续灵感对话"
          : "进入项目详情";

export const getProjectWorkspaceRoute = (project: RoutedProject): string | null => {
  const id = projectId(project);
  if (projectKind(project) === "video" && project.source_mode === "freeform") {
    return `/video-project/${id}/studio`;
  }
  if (projectKind(project) === "video") return `/video-project/${id}`;
  if (projectKind(project) === "comic") return `/comic-project/${id}`;
  if (projectKind(project) === "galgame") return `/galgame-project/${id}`;
  const capabilities = getProjectCapabilities(project.status);
  if (!capabilities.canGenerateChapterContent) return null;
  return `/write/${id}`;
};

export const getProjectWorkspaceLabel = (project: RoutedProject): string | null =>
  projectKind(project) === "video"
    ? "进入视频工作台"
    : projectKind(project) === "comic"
      ? "进入漫画工作台"
      : projectKind(project) === "galgame"
        ? "进入游戏工作台"
        : getProjectWorkspaceRoute(project)
          ? "进入写作台"
          : null;

export const getProjectHomeEntryRoute = (project: RoutedProject): string => {
  const id = projectId(project);
  if (projectKind(project) !== "text") return getProjectPrimaryRoute(project);
  if (isDraftLikeProject(project)) return `/inspiration/${id}`;
  if (hasRecentBlueprintGenerationPending("novel", id)) return `/inspiration/${id}`;

  const stage = resolveWorkflowStage(project.status);
  if (stage === "writing" || stage === "completed") {
    return getProjectWorkspaceRoute(project) || getProjectPrimaryRoute(project);
  }
  return getProjectPrimaryRoute(project);
};

export const getProjectHomeEntryLabel = (project: RoutedProject): string => {
  const id = projectId(project);
  if (projectKind(project) !== "text") return getProjectPrimaryLabel(project);
  if (isDraftLikeProject(project)) return "继续灵感对话";
  if (hasRecentBlueprintGenerationPending("novel", id)) return "检查蓝图结果";

  const stage = resolveWorkflowStage(project.status);
  if (stage === "writing" || stage === "completed") {
    return getProjectWorkspaceLabel(project) || getProjectPrimaryLabel(project);
  }
  return getProjectPrimaryLabel(project);
};
