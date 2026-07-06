import { apiInvoke } from "../index";
import type { CategoryInfo, PromptDependencyGraph } from "./types";

export async function promptDependencyGraph(): Promise<PromptDependencyGraph> {
  return apiInvoke<PromptDependencyGraph>("prompt_dependency_graph");
}

export async function promptValidateVariables(input: {
  template_name: string;
  variables: string[];
}): Promise<string[]> {
  return apiInvoke<string[]>("prompt_validate_variables", { input });
}

export async function promptListCategories(): Promise<CategoryInfo[]> {
  return apiInvoke<CategoryInfo[]>("prompt_list_categories");
}
