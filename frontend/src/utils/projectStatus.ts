/**
 * 项目状态机
 *
 * 6 个进度摘要。非线性创作可以跳过不存在的蓝图或大纲阶段。
 */

import type {
  NovelProject,
  ProjectLifecycleBlockedReason,
} from "../api/project";
import type { MessageTranslator } from "../i18n";
import { projectStatusLabel } from "../pages/status-machine-dialog/statusLabels";

/** 获取项目列表/头部展示用阶段文案。文本项目使用小说状态机，派生项目使用对应工作台语义。 */
export function getProjectProgressText(
  project: Pick<NovelProject, "project_kind" | "status">,
  t: MessageTranslator,
): string {
  return project.project_kind === "text"
    ? projectStatusLabel(project.status, t)
    : t("workflow.progress");
}

export function getProjectLifecycleBlockedReason(
  project: Pick<NovelProject, "lifecycle_gate">,
  t: MessageTranslator,
): string | null {
  const reason: ProjectLifecycleBlockedReason | null =
    project.lifecycle_gate === "active" ? null : project.lifecycle_gate;
  if (reason === "resetting") {
    return t("novel.resetFinishingBlocked");
  }
  return reason === "deleting" ? t("novel.deleting") : null;
}
