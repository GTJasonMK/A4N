import { apiInvoke } from "../index";
import { type ApiId as Id, idNum } from "../shared";
import type {
  ProjectPromptOverrideSummary,
  RemoveOverrideInput,
  SetOverrideInput,
} from "../generated/host";

export async function promptSetProjectOverride(input: SetOverrideInput): Promise<void> {
  return apiInvoke<void>("prompt_set_project_override", { input });
}

export async function promptRemoveProjectOverride(input: RemoveOverrideInput): Promise<void> {
  return apiInvoke<void>("prompt_remove_project_override", { input });
}

export async function promptListProjectOverrides(projectId: Id): Promise<ProjectPromptOverrideSummary[]> {
  return apiInvoke<ProjectPromptOverrideSummary[]>("prompt_list_project_overrides", {
    projectId: idNum(projectId),
  });
}
