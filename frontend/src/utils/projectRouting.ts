import { hasRecentBlueprintGenerationPending } from "./blueprintPending";
import { getProjectCapabilities, resolveWorkflowStage } from "./projectStatus";

export interface RoutedProject {
  id?: number | string;
  status?: string | null;
  project_kind?: string | null;
  source_mode?: string | null;
}

const PROJECT_KINDS = ["text", "video", "comic", "galgame"] as const;
type ProjectKind = (typeof PROJECT_KINDS)[number];

const projectId = (project: RoutedProject): string => {
  const id = String(project.id ?? "").trim();
  if (!id) throw new Error("项目 ID 缺失，无法计算项目路由");
  return id;
};

const isDraftLikeProject = (project: RoutedProject): boolean =>
  resolveWorkflowStage(project.status) === "draft";

const projectKind = (project: RoutedProject): ProjectKind => {
  const kind = String(project.project_kind ?? "").trim();
  if (PROJECT_KINDS.includes(kind as ProjectKind)) return kind as ProjectKind;
  throw new Error(`项目族类型无效，无法计算项目路由: ${kind || "<empty>"}`);
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
