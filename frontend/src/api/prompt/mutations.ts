import { apiInvoke } from "../index";
import type { PromptEntrySummary, UpdatePromptInput } from "../generated/host";

export async function promptLoadDefaults(): Promise<number> {
  return apiInvoke<number>("prompt_load_defaults");
}

export async function promptUpdate(input: UpdatePromptInput): Promise<PromptEntrySummary> {
  return apiInvoke<PromptEntrySummary>("prompt_update", { input });
}

export async function promptReset(name: string): Promise<void> {
  return apiInvoke<void>("prompt_reset", { name });
}
