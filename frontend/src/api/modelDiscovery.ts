import { usesLocalRuntimeSettings } from "../utils/runtimeMode";
import { apiInvoke } from "./index";
import type { HostApiCommand } from "./generated/host";
export type {
  DiscoveredModel,
  ModelDiscoveryInput,
  ModelDiscoveryResult,
} from "./generated/config";

import type { ModelDiscoveryInput, ModelDiscoveryResult } from "./generated/config";

interface LocalRuntimeDiscoveryKeyOptions {
  systemConfigId: number;
  getSystemDefaultApiKey: () => string | null | undefined;
  getLocalConfigApiKey: (configId: number) => string | null | undefined;
}

export type ModelConfigScope = "server" | "local";

interface ModelDiscoveryFieldOptions {
  provider: string;
  baseUrl: string;
  apiKey: string;
  clearApiKey: boolean;
  configId?: number | null;
  hasApiKey: boolean;
  readonly: boolean;
  scope: ModelConfigScope;
}

export function buildModelDiscoveryField(options: ModelDiscoveryFieldOptions): {
  discoveryKey: string;
  input: ModelDiscoveryInput;
} {
  const apiKey = options.apiKey.trim();
  const configId = options.configId ?? null;
  return {
    discoveryKey: [
      options.provider,
      options.baseUrl.trim(),
      options.scope,
      configId ?? "new",
      options.clearApiKey ? "cleared-key" : apiKey ? "form-key" : "saved-key",
    ].join("|"),
    input: {
      provider: options.provider,
      base_url: options.baseUrl,
      api_key: options.clearApiKey ? null : apiKey || undefined,
      config_id: configId,
      use_saved_api_key: Boolean(
        !options.readonly &&
        configId != null &&
        configId > 0 &&
        options.hasApiKey &&
        !options.clearApiKey &&
        !apiKey,
      ),
    },
  };
}

export function detectConfigModels(
  command: HostApiCommand,
  input: ModelDiscoveryInput,
  scope: ModelConfigScope,
  options: LocalRuntimeDiscoveryKeyOptions,
): Promise<ModelDiscoveryResult> {
  return apiInvoke<ModelDiscoveryResult>(command, {
    input: resolveModelDiscoveryInput(input, scope, options),
  });
}

function resolveModelDiscoveryInput(
  input: ModelDiscoveryInput,
  scope: ModelConfigScope,
  options: LocalRuntimeDiscoveryKeyOptions,
): ModelDiscoveryInput {
  if (scope === "server" || input.api_key === null) return input;
  const apiKey = input.api_key?.trim();
  const useLocalRuntime = usesLocalRuntimeSettings();
  if (useLocalRuntime && input.config_id === options.systemConfigId) {
    return {
      ...input,
      api_key: apiKey || options.getSystemDefaultApiKey()?.trim() || undefined,
      use_saved_api_key: false,
    };
  }
  if (
    apiKey ||
    !useLocalRuntime ||
    input.config_id == null ||
    input.config_id <= 0
  ) {
    return input;
  }
  return {
    ...input,
    api_key: options.getLocalConfigApiKey(input.config_id)?.trim() || undefined,
    use_saved_api_key: false,
  };
}
