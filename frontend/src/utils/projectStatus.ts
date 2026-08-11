/**
 * 项目状态机
 *
 * 6 个进度摘要。非线性创作可以跳过不存在的蓝图或大纲阶段。
 */

import type { NovelProject } from "../api/project";
import {
  PROJECT_STATUSES,
  PROJECT_STATUS_LABELS,
  type ProjectStatus,
} from "../api/generated/host";

export { PROJECT_STATUSES };

/** 获取状态中文文案 */
export function getProjectStatusText(status: ProjectStatus): string {
  return PROJECT_STATUS_LABELS[status];
}

/** 获取项目列表/头部展示用阶段文案。文本项目使用小说状态机，派生项目使用对应工作台语义。 */
export function getProjectProgressText(project: Pick<
  NovelProject,
  "progress"
>): string {
  return project.progress.label;
}
