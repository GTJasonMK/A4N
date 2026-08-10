/**
 * 项目状态机
 *
 * 6 个状态枚举（顺序即推进顺序）：
 *   draft → blueprint_ready → part_outlines_ready → chapter_outlines_ready → writing → completed
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
