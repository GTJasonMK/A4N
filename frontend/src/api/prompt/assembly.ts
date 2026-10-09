import { apiInvoke } from "../index";
import type {
  PromptAssemblyInput,
  PromptAssemblyOutput,
  PromptAssemblySummary,
  SavePromptAssemblyInput,
} from "../generated/host";

export async function promptAssemblyList(): Promise<PromptAssemblySummary[]> {
  return apiInvoke<PromptAssemblySummary[]>("prompt_assembly_list");
}

export async function promptAssemblyGet(input: PromptAssemblyInput): Promise<PromptAssemblyOutput> {
  return apiInvoke<PromptAssemblyOutput>("prompt_assembly_get", { input });
}

export async function promptAssemblySave(input: SavePromptAssemblyInput): Promise<void> {
  return apiInvoke<void>("prompt_assembly_save", { input });
}

export async function promptAssemblyReset(input: PromptAssemblyInput): Promise<void> {
  return apiInvoke<void>("prompt_assembly_reset", { input });
}
