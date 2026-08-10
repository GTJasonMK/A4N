import { apiInvoke } from "../index";
import type { PromptEntrySummary } from "../generated/host";

export async function promptList(): Promise<PromptEntrySummary[]> {
  return apiInvoke<PromptEntrySummary[]>("prompt_list");
}

export async function promptGetDependencies(name: string): Promise<string[]> {
  return apiInvoke<string[]>("prompt_get_dependencies", { name });
}

export async function promptGetReverseDependencies(name: string): Promise<string[]> {
  return apiInvoke<string[]>("prompt_get_reverse_dependencies", { name });
}
