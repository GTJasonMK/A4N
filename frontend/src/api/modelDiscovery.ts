import { usesLocalRuntimeSettings } from "../utils/runtimeMode";
import type { ModelDiscoveryInput as ApiModelDiscoveryInput } from "./generated/config";

export type ModelDiscoveryInput = ApiModelDiscoveryInput;

export interface DiscoveredModel {
  id: string;
  name?: string | null;
  owned_by?: string | null;
}

export interface ModelDiscoveryResult {
  models: DiscoveredModel[];
}

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
    api_key_encrypted: options.getLocalConfigApiKey(input.config_id)?.trim() || input.api_key_encrypted,
    use_saved_api_key: false,
  };
}
