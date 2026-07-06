import { apiInvoke } from "../index";
import type { PromptVersion } from "./types";

export async function promptVersionHistory(name: string): Promise<PromptVersion[]> {
  return apiInvoke<PromptVersion[]>("prompt_version_history", { name });
}

export async function promptVersionDiff(input: {
  name: string;
  version_a: string;
  version_b: string;
}): Promise<string> {
  return apiInvoke<string>("prompt_version_diff", { input });
}
