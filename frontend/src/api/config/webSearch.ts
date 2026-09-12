import { apiInvoke } from "../index";
import type {
  UpdateWebSearchConfigInput,
  WebSearchConfigSummary,
} from "../generated/config";

export type { UpdateWebSearchConfigInput, WebSearchConfigSummary } from "../generated/config";

export async function getServerWebSearchConfig(): Promise<WebSearchConfigSummary> {
  return apiInvoke<WebSearchConfigSummary>("get_web_search_config");
}

export async function updateServerWebSearchConfig(
  input: UpdateWebSearchConfigInput,
): Promise<WebSearchConfigSummary> {
  return apiInvoke<WebSearchConfigSummary>("update_web_search_config", { input });
}
