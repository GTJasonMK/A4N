import { apiInvoke } from "../index";
import type { PromptEntry } from "./types";

export async function promptList(): Promise<PromptEntry[]> {
  return apiInvoke<PromptEntry[]>("prompt_list");
}

export async function promptGetDependencies(name: string): Promise<string[]> {
  return apiInvoke<string[]>("prompt_get_dependencies", { name });
}

export async function promptGetReverseDependencies(name: string): Promise<string[]> {
  return apiInvoke<string[]>("prompt_get_reverse_dependencies", { name });
}
