/**
 * 项目状态机
 *
 * 6 个状态枚举（顺序即推进顺序）：
 *   draft → blueprint_ready → part_outlines_ready → chapter_outlines_ready → writing → completed
 */

import type { NovelProject } from "../api/project";

export const PROJECT_STATUSES = [
  "draft",
  "blueprint_ready",
  "part_outlines_ready",
  "chapter_outlines_ready",
  "writing",
  "completed",
] as const;

export type ProjectStatus = (typeof PROJECT_STATUSES)[number];

type WorkflowStage = ProjectStatus | "unknown";

interface WorkflowCapabilities {
  canGenerateBlueprint: boolean;
  canEditBlueprint: boolean;
  canGeneratePartOutlines: boolean;
  canGenerateChapterOutlines: boolean;
  canGenerateChapterContent: boolean;
  canUseStoryAssets: boolean;
}

export interface ProjectForwardArtifacts {
  partOutlines: number;
  chapterOutlines: number;
}

const STATUS_TEXT: Record<ProjectStatus, string> = {
  draft: "草稿",
  blueprint_ready: "蓝图就绪",
  part_outlines_ready: "部分大纲就绪",
  chapter_outlines_ready: "章节大纲就绪",
  writing: "写作中",
  completed: "已完成",
};

const STATUS_NEXT: Record<ProjectStatus, ProjectStatus | null> = {
  draft: "blueprint_ready",
  blueprint_ready: "part_outlines_ready",
  part_outlines_ready: "chapter_outlines_ready",
  chapter_outlines_ready: "writing",
  writing: "completed",
  completed: null,
};

const STATUS_NEXT_HINT: Record<ProjectStatus, string> = {
  draft: "生成故事蓝图",
  blueprint_ready: "生成分部大纲",
  part_outlines_ready: "生成章节大纲",
  chapter_outlines_ready: "开始写作",
  writing: "标记完成",
  completed: "—",
};

/** 解析为前端工作流阶段 */
export function resolveWorkflowStage(status: unknown): WorkflowStage {
  if (typeof status !== "string" || status.length === 0) return "unknown";
  if (PROJECT_STATUSES.includes(status as ProjectStatus)) {
    return status as ProjectStatus;
  }
  return "unknown";
}

/** 获取状态中文文案 */
export function getProjectStatusText(status: string | undefined | null): string {
  const stage = resolveWorkflowStage(status);
  if (stage === "unknown") return status ? "未知状态" : "未知";
  return STATUS_TEXT[stage];
}

/** 获取项目列表/头部展示用阶段文案。文本项目使用小说状态机，派生项目使用对应工作台语义。 */
export function getProjectProgressText(project: Pick<
  NovelProject,
  "progress"
>): string {
  return project.progress.label;
}

/** 基于当前状态和已存在产物计算用户实际可推进的下一阶段。 */
export function getProjectForwardTarget(
  status: string | undefined | null,
  artifacts?: ProjectForwardArtifacts | null,
): ProjectStatus | null {
  const stage = resolveWorkflowStage(status);
  if (stage === "unknown" || stage === "completed") return null;
  if (stage === "blueprint_ready") {
    if (!artifacts) return null;
    if (artifacts.chapterOutlines > 0) return "chapter_outlines_ready";
    if (artifacts.partOutlines > 0) return "part_outlines_ready";
    return null;
  }
  return STATUS_NEXT[stage];
}

/** 推进下一阶段的中文操作提示 */
function getNextStatusHint(status: string | undefined | null): string {
  const stage = resolveWorkflowStage(status);
  if (stage === "unknown") return "—";
  return STATUS_NEXT_HINT[stage];
}

export function getProjectForwardStatusHint(
  status: string | undefined | null,
  target: ProjectStatus | null,
): string {
  const stage = resolveWorkflowStage(status);
  if (stage === "blueprint_ready" && target === "chapter_outlines_ready") {
    return "生成章节大纲";
  }
  return getNextStatusHint(stage);
}

/** 前端能力计算：用于按钮禁用、入口分流与提示文案。 */
export function getProjectCapabilities(status: string | undefined | null): WorkflowCapabilities {
  const stage = resolveWorkflowStage(status);
  return {
    canGenerateBlueprint: stage === "draft" || stage === "blueprint_ready",
    canEditBlueprint: stage === "draft" || stage === "blueprint_ready",
    canGeneratePartOutlines: stage === "blueprint_ready",
    canGenerateChapterOutlines:
      stage === "blueprint_ready" ||
      stage === "part_outlines_ready" ||
      stage === "chapter_outlines_ready" ||
      stage === "writing",
    canGenerateChapterContent:
      stage === "chapter_outlines_ready" ||
      stage === "writing" ||
      stage === "completed",
    canUseStoryAssets:
      stage === "chapter_outlines_ready" ||
      stage === "writing" ||
      stage === "completed",
  };
}
