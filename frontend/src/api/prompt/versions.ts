import { apiInvoke } from "../index";
import type { PromptVersionSummary, VersionDiffInput } from "../generated/host";

export async function promptVersionHistory(name: string): Promise<PromptVersionSummary[]> {
  return apiInvoke<PromptVersionSummary[]>("prompt_version_history", { name });
}

export async function promptVersionDiff(input: VersionDiffInput): Promise<string> {
  return apiInvoke<string>("prompt_version_diff", { input });
}
