import { apiInvoke } from "../index";
import type { PromptEntry } from "./types";

export async function promptLoadDefaults(): Promise<number> {
  return apiInvoke<number>("prompt_load_defaults");
}

export async function promptUpdate(input: {
  name: string;
  content: string;
  author: string;
}): Promise<PromptEntry> {
  return apiInvoke<PromptEntry>("prompt_update", { input });
}

export async function promptReset(name: string): Promise<void> {
  return apiInvoke<void>("prompt_reset", { name });
}
