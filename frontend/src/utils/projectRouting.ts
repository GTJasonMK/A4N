import type { ProjectKind, ProjectSourceMode, ProjectStatus } from "../api/project";

export interface RoutedProject {
  id: number | string;
  status: ProjectStatus;
  project_kind: ProjectKind;
  source_mode: ProjectSourceMode;
}

export interface DerivedProjectPresentation {
  root_project_title?: string | null;
  derived_chapter_scope?: unknown | null;
  source_sync_status?: string | null;
}

const PROJECT_TYPE_LABELS = {
  novel_long: "长篇小说",
  novel_short: "短篇故事",
  script: "剧本",
  prose: "散文",
} as const;

export const PROJECT_KIND_LABELS: Record<ProjectKind, string> = {
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

export const getProjectTypeLabel = (value: string): string =>
  PROJECT_TYPE_LABELS[value as keyof typeof PROJECT_TYPE_LABELS] ?? "未知类型";

export const getAdminProjectTypeLabel = (project: {
  project_kind: ProjectKind;
  project_type: string;
}): string =>
  project.project_kind === "text"
    ? getProjectTypeLabel(project.project_type)
    : PROJECT_KIND_LABELS[project.project_kind];

const getSourceSyncStatusLabel = (value: string | null | undefined): string => {
  const normalized = String(value ?? "").trim().toLowerCase();
  return SOURCE_SYNC_STATUS_LABELS[normalized] ?? (normalized || "未知");
};

const formatDerivedChapterScope = (scope: unknown): string | null => {
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

export const getProjectStudioRoute = (id: number | string): string =>
  `/video-project/${id}/studio`;

const projectRoute = (
  project: RoutedProject,
  textRoute: "novel" | "write",
): string => {
  const id = project.id;
  if (project.project_kind === "video") {
    return project.source_mode === "freeform"
      ? getProjectStudioRoute(id)
      : `/video-project/${id}`;
  }
  if (project.project_kind === "comic") return `/comic-project/${id}`;
  if (project.project_kind === "galgame") return `/galgame-project/${id}`;
  return `/${textRoute}/${id}`;
};

export const getProjectPrimaryRoute = (project: RoutedProject): string =>
  projectRoute(project, "novel");

export const getProjectPrimaryLabel = (project: RoutedProject): string =>
  project.project_kind === "video"
    ? "进入视频项目"
    : project.project_kind === "comic"
      ? "进入漫画项目"
      : project.project_kind === "galgame"
        ? "进入 GALGame 项目"
        : "进入项目详情";

export const getProjectWorkspaceRoute = (project: RoutedProject): string =>
  projectRoute(project, "write");

export const getProjectWorkspaceLabel = (project: RoutedProject): string =>
  project.project_kind === "video"
    ? "进入视频工作台"
    : project.project_kind === "comic"
      ? "进入漫画工作台"
      : project.project_kind === "galgame"
        ? "进入游戏工作台"
        : "进入写作台";

export const getProjectHomeEntryRoute = (project: RoutedProject): string => {
  if (project.project_kind !== "text") return getProjectPrimaryRoute(project);

  if (project.status === "writing" || project.status === "completed") {
    return getProjectWorkspaceRoute(project);
  }
  return getProjectPrimaryRoute(project);
};

export const getProjectHomeEntryLabel = (project: RoutedProject): string => {
  if (project.project_kind !== "text") return getProjectPrimaryLabel(project);

  if (project.status === "writing" || project.status === "completed") {
    return getProjectWorkspaceLabel(project);
  }
  return getProjectPrimaryLabel(project);
};
