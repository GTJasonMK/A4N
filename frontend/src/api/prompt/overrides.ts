import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import { type ApiId as Id, idNum } from "../shared";
import type { ProjectPromptOverrideSummary } from "./types";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法管理项目提示词覆盖");
  return id;
};

export async function promptSetProjectOverride(input: {
  project_id: number;
  name: string;
  content: string;
}): Promise<void> {
  return apiInvoke<void>("prompt_set_project_override", { userId: currentUserId(), input });
}

export async function promptRemoveProjectOverride(input: {
  project_id: number;
  name: string;
}): Promise<void> {
  return apiInvoke<void>("prompt_remove_project_override", { userId: currentUserId(), input });
}

export async function promptListProjectOverrides(projectId: Id): Promise<ProjectPromptOverrideSummary[]> {
  return apiInvoke<ProjectPromptOverrideSummary[]>("prompt_list_project_overrides", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}
