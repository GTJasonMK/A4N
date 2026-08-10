import { usesLocalRuntimeSettings } from "../utils/runtimeMode";
export type {
  DiscoveredModel,
  ModelDiscoveryInput,
  ModelDiscoveryResult,
} from "./generated/config";

import type {
  ModelDiscoveryInput,
} from "./generated/config";

interface LocalRuntimeDiscoveryKeyOptions {
  systemConfigId: number;
  getSystemDefaultApiKey: () => string | null | undefined;
  getLocalConfigApiKey: (configId: number) => string | null | undefined;
}

export function withLocalRuntimeDiscoveryKey(
  input: ModelDiscoveryInput,
  options: LocalRuntimeDiscoveryKeyOptions,
): ModelDiscoveryInput {
  if (input.api_key_encrypted === null) return input;
  const apiKey = input.api_key_encrypted?.trim();
  const useLocalRuntime = usesLocalRuntimeSettings();
  if (useLocalRuntime && input.config_id === options.systemConfigId) {
    return {
      ...input,
      api_key_encrypted: apiKey || options.getSystemDefaultApiKey()?.trim() || undefined,
      use_saved_api_key: false,
    };
  }
  if (apiKey || !useLocalRuntime || input.config_id == null || input.config_id <= 0) {
    return input;
  }
  return {
    ...input,
    api_key_encrypted: options.getLocalConfigApiKey(input.config_id)?.trim() || undefined,
    use_saved_api_key: false,
  };
}
