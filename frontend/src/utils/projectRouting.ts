import { canEditProjectMaterials, type NovelProject, type ProjectKind, type ProjectSourceMode, type ProjectStatus } from "../api/project";
import type { MessageKey, MessageTranslator } from "../i18n";

export interface RoutedProject {
  id: number | string;
  status: ProjectStatus;
  project_kind: ProjectKind;
  source_mode: ProjectSourceMode;
}

export type HomeEntryProject = RoutedProject & Pick<NovelProject, "my_role" | "lifecycle_gate">;

const entersWritingDesk = (project: HomeEntryProject): boolean =>
  project.project_kind === "text" && canEditProjectMaterials(project) &&
  (project.status === "writing" || project.status === "completed");

export interface DerivedProjectPresentation {
  root_project_title?: string | null;
  derived_chapter_scope?: unknown | null;
  source_sync_status?: string | null;
}

const PROJECT_TYPE_LABEL_KEYS: Record<string, MessageKey> = {
  novel_long: "novel.longNovel",
  novel_short: "novel.shortStory",
  script: "novel.script",
  prose: "novel.prose",
} as const;

export const PROJECT_KIND_LABEL_KEYS: Record<ProjectKind, MessageKey> = {
  text: "novel.textProject",
  video: "novel.videoProject",
  canvas: "video.canvasProject",
  comic: "novel.comicProject",
  galgame: "novel.galgameProject",
};

const PROJECT_ACCESS_ROLE_LABEL_KEYS = {
  owner: "novel.ownerRole",
  manager: "novel.managerRole",
  editor: "novel.editorRole",
  viewer: "novel.viewerRole",
} as const;

export const getProjectTypeLabel = (
  value: string,
  t: MessageTranslator,
): string => t(PROJECT_TYPE_LABEL_KEYS[value] ?? "novel.unknownType");

export const getProjectKindLabel = (
  value: ProjectKind,
  t: MessageTranslator,
): string => t(PROJECT_KIND_LABEL_KEYS[value]);

export const getProjectAccessRoleLabel = (
  value: keyof typeof PROJECT_ACCESS_ROLE_LABEL_KEYS,
  t: MessageTranslator,
): string => t(PROJECT_ACCESS_ROLE_LABEL_KEYS[value]);

export const getAdminProjectTypeLabel = (project: {
  project_kind: ProjectKind;
  project_type: string;
}, t: MessageTranslator): string =>
  project.project_kind === "text"
    ? getProjectTypeLabel(project.project_type, t)
    : getProjectKindLabel(project.project_kind, t);

const formatDerivedChapterScope = (
  scope: unknown,
  format: (start: number, end: number) => string,
): string | null => {
  if (!scope || typeof scope !== "object") return null;
  const record = scope as Record<string, unknown>;
  const start = Number(record.start);
  const end = Number(record.end);
  if (!Number.isInteger(start) || !Number.isInteger(end)) return null;
  return format(start, end);
};

export interface DerivedProjectSubtitleLabels {
  fallback: string;
  sourceProject: (title: string) => string;
  chapterScope: (start: number, end: number) => string;
  sourceSyncStatus: (value: string | null | undefined) => string;
}

export const getDerivedProjectSubtitle = (
  project: DerivedProjectPresentation,
  labels: DerivedProjectSubtitleLabels,
): string => {
  const parts = [
    project.root_project_title ? labels.sourceProject(project.root_project_title) : null,
    formatDerivedChapterScope(project.derived_chapter_scope, labels.chapterScope),
    labels.sourceSyncStatus(project.source_sync_status),
  ].filter((part): part is string => Boolean(part));
  return parts.join(" · ") || labels.fallback;
};

export const getCanvasProjectRoute = (id: number | string): string =>
  `/canvas-project/${id}`;

const projectRoute = (
  project: RoutedProject,
  textRoute: "novel" | "write",
): string => {
  const id = project.id;
  if (project.project_kind === "canvas") return getCanvasProjectRoute(id);
  if (project.project_kind === "video") return `/video-project/${id}`;
  if (project.project_kind === "comic") return `/comic-project/${id}`;
  if (project.project_kind === "galgame") return `/galgame-project/${id}`;
  return `/${textRoute}/${id}`;
};

export const getProjectPrimaryRoute = (project: RoutedProject): string =>
  projectRoute(project, "novel");

export const getProjectStudioBackRoute = (project: RoutedProject): string =>
  project.project_kind === "canvas"
    ? "/"
    : getProjectPrimaryRoute(project);

export const getProjectPrimaryLabel = (
  project: RoutedProject,
  t: MessageTranslator,
): string =>
  project.project_kind === "canvas"
    ? t("video.canvasProject")
    : project.project_kind === "video"
      ? t("novel.enterVideoProject")
    : project.project_kind === "comic"
      ? t("novel.enterComicProject")
      : project.project_kind === "galgame"
        ? t("novel.enterGalgameProject")
        : t("novel.enterProjectDetails");

export const getProjectWorkspaceRoute = (project: RoutedProject): string =>
  projectRoute(project, "write");

export const getProjectWorkspaceLabel = (
  project: RoutedProject,
  t: MessageTranslator,
): string =>
  project.project_kind === "canvas"
    ? t("video.canvasProject")
    : project.project_kind === "video"
      ? t("novel.enterVideoWorkspace")
    : project.project_kind === "comic"
      ? t("novel.enterComicWorkspace")
      : project.project_kind === "galgame"
        ? t("novel.enterGameWorkspace")
        : t("novel.enterWritingDesk");

export const getProjectHomeEntryRoute = (project: HomeEntryProject): string =>
  entersWritingDesk(project) ? getProjectWorkspaceRoute(project) : getProjectPrimaryRoute(project);

export const getProjectHomeEntryLabel = (
  project: HomeEntryProject,
  t: MessageTranslator,
): string => entersWritingDesk(project) ? getProjectWorkspaceLabel(project, t) : getProjectPrimaryLabel(project, t);
