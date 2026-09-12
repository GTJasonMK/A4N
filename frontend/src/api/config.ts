export * from "./config/types";
export * from "./config/llm";
export * from "./config/embedding";
export * from "./config/webSearch";

import type { ImportAllConfigsInput } from "./generated/config";
import { apiInvoke } from "./index";

export type { ImportAllConfigsInput } from "./generated/config";

export async function exportAllConfigsJson(): Promise<string> {
  return apiInvoke<string>("export_all_configs_json");
}

export async function importAllConfigs(
  input: ImportAllConfigsInput,
): Promise<string[]> {
  return apiInvoke<string[]>("import_all_configs", {
    input,
  });
}
