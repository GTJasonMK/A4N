import { apiInvoke } from "../index";
import type { CategoryInfo, ValidateVarsInput } from "../generated/host";

export async function promptValidateVariables(input: ValidateVarsInput): Promise<string[]> {
  return apiInvoke<string[]>("prompt_validate_variables", { input });
}

export async function promptListCategories(): Promise<CategoryInfo[]> {
  return apiInvoke<CategoryInfo[]>("prompt_list_categories");
}
