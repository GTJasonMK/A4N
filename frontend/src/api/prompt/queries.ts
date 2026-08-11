import { apiInvoke } from "../index";
import type { PromptEntrySummary } from "../generated/host";

export async function promptList(): Promise<PromptEntrySummary[]> {
  return apiInvoke<PromptEntrySummary[]>("prompt_list");
}
