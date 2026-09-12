import { apiInvoke } from "../index";
import {
  detectConfigModels,
  type ModelConfigScope,
  type ModelDiscoveryInput,
  type ModelDiscoveryResult,
} from "../modelDiscovery";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  CreateLlmInput,
  LlmConfigSummary,
  UpdateLlmInput,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";
import {
  createLocalLlmConfig,
  deleteLocalLlmConfig,
  getSystemDefaultLlmApiKey,
  getLocalLlmConfig,
  hasSystemDefaultLlmApiKey,
  listLocalLlmConfigs,
  setDefaultLocalLlmConfig,
  setSystemDefaultLlmConfig,
  updateLocalLlmConfig,
  updateSystemDefaultLlmApiKey,
  usesSystemDefaultLlmConfig,
} from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";
import {
  mergeLocalRuntimeConfigs,
  providerConfigIdOrSystem,
  requireSystemDefaultProviderConfig,
  rejectSystemDefaultMutation,
  systemDefaultProviderConfig,
} from "./shared";

export const SYSTEM_LLM_CONFIG_ID = -1;

function llmConfigId(configId: Id): number {
  return idNum(configId, "LLM 配置 ID");
}

function isSystemDefaultLlmConfigId(configId: Id): boolean {
  return (
    providerConfigIdOrSystem(configId, "LLM 配置 ID", SYSTEM_LLM_CONFIG_ID) ===
    SYSTEM_LLM_CONFIG_ID
  );
}

function systemDefaultLlmConfig(
  configs: LlmConfigSummary[],
): LlmConfigSummary | null {
  return systemDefaultProviderConfig(
    configs,
    SYSTEM_LLM_CONFIG_ID,
    hasSystemDefaultLlmApiKey(),
    usesSystemDefaultLlmConfig(),
  );
}

function requiredSystemDefaultLlmConfig(
  configs: LlmConfigSummary[],
): LlmConfigSummary {
  return requireSystemDefaultProviderConfig(
    systemDefaultLlmConfig(configs),
    "服务器尚未配置系统默认 LLM 模板",
  );
}

function systemDefaultRuntimeLlmConfig(
  config: LlmConfigSummary,
  apiKey?: string,
) {
  return {
    ...config,
    id: SYSTEM_LLM_CONFIG_ID,
    name: `系统默认：${config.name}`,
    ...(apiKey ? { api_key_encrypted: apiKey } : {}),
    has_api_key: apiKey ? true : config.has_api_key,
    is_default: true,
  };
}

async function loadSystemDefaultRuntimeLlmConfig(options?: {
  requireApiKey?: boolean;
}) {
  const requireApiKey = options?.requireApiKey ?? true;
  const apiKey = requireApiKey ? getSystemDefaultLlmApiKey() : undefined;
  if (apiKey === null) {
    throw new Error("使用系统默认 LLM 需要先填写并保存你的个人 API Key");
  }
  const config = requiredSystemDefaultLlmConfig(await listServerLlmConfig());
  return systemDefaultRuntimeLlmConfig(config, apiKey);
}

export async function listServerLlmConfig(): Promise<LlmConfigSummary[]> {
  return apiInvoke<LlmConfigSummary[]>("list_llm_config");
}

export async function listLlmConfig(): Promise<LlmConfigSummary[]> {
  if (!usesLocalRuntimeSettings()) return listServerLlmConfig();
  const [serverConfigs, localConfigs] = await Promise.all([
    listServerLlmConfig(),
    listLocalLlmConfigs(),
  ]);
  return mergeLocalRuntimeConfigs(
    localConfigs,
    systemDefaultLlmConfig(serverConfigs),
    usesSystemDefaultLlmConfig(),
  );
}

export async function createServerLlmConfig(
  input: CreateLlmInput,
): Promise<LlmConfigSummary> {
  return apiInvoke<LlmConfigSummary>("create_llm_config", { input });
}

export async function createLlmConfig(
  input: CreateLlmInput,
): Promise<LlmConfigSummary> {
  if (usesLocalRuntimeSettings()) return createLocalLlmConfig(input);
  return createServerLlmConfig(input);
}

export async function updateServerLlmConfig(
  configId: Id,
  input: UpdateLlmInput,
): Promise<LlmConfigSummary> {
  return apiInvoke<LlmConfigSummary>("update_llm_config", {
    configId: llmConfigId(configId),
    input,
  });
}

export async function updateLlmConfig(
  configId: Id,
  input: UpdateLlmInput,
): Promise<LlmConfigSummary> {
  if (isSystemDefaultLlmConfigId(configId)) {
    if (!usesLocalRuntimeSettings()) rejectSystemDefaultMutation();
    updateSystemDefaultLlmApiKey(input.api_key_encrypted);
    return requiredSystemDefaultLlmConfig(await listServerLlmConfig());
  }
  if (usesLocalRuntimeSettings()) {
    return updateLocalLlmConfig(llmConfigId(configId), input);
  }
  return updateServerLlmConfig(configId, input);
}

export async function setDefaultServerLlmConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("set_default_llm_config", {
    configId: llmConfigId(configId),
  });
}

export async function setDefaultLlmConfig(configId: Id): Promise<void> {
  if (usesLocalRuntimeSettings() && isSystemDefaultLlmConfigId(configId)) {
    requiredSystemDefaultLlmConfig(await listServerLlmConfig());
    setSystemDefaultLlmConfig();
    return;
  }
  if (usesLocalRuntimeSettings()) {
    setDefaultLocalLlmConfig(llmConfigId(configId));
    return;
  }
  return setDefaultServerLlmConfig(configId);
}

export async function deleteServerLlmConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("delete_llm_config", {
    configId: llmConfigId(configId),
  });
}

export async function deleteLlmConfig(configId: Id): Promise<void> {
  if (isSystemDefaultLlmConfigId(configId)) rejectSystemDefaultMutation();
  if (usesLocalRuntimeSettings()) {
    deleteLocalLlmConfig(llmConfigId(configId));
    return;
  }
  return deleteServerLlmConfig(configId);
}

async function resolveLlmConnectivityTarget(
  configId: Id,
  scope: ModelConfigScope,
  requireApiKey: boolean,
): Promise<{ configId: number; config: LlmConfigSummary | null }> {
  if (scope === "server") {
    return { configId: llmConfigId(configId), config: null };
  }
  if (isSystemDefaultLlmConfigId(configId)) {
    return {
      configId: SYSTEM_LLM_CONFIG_ID,
      config: await loadSystemDefaultRuntimeLlmConfig({ requireApiKey }),
    };
  }
  const id = llmConfigId(configId);
  return {
    configId: id,
    config: usesLocalRuntimeSettings() ? getLocalLlmConfig(id) : null,
  };
}

export async function testLlmConfig(
  configId: Id,
  scope: ModelConfigScope,
): Promise<void> {
  const target = await resolveLlmConnectivityTarget(configId, scope, true);
  return apiInvoke<void>("test_llm_config", {
    configId: target.configId,
    runtime_config_snapshot: target.config
      ? buildRuntimeConfigSnapshot({ llm: target.config })
      : null,
  });
}

export async function previewTestLlmConfigRequest(
  configId: Id,
  scope: ModelConfigScope,
): Promise<ModelRequestPreview> {
  const target = await resolveLlmConnectivityTarget(configId, scope, false);
  return apiInvoke<ModelRequestPreview>("preview_test_llm_config_request", {
    configId: target.configId,
    runtime_config_snapshot: target.config
      ? buildRuntimeConfigSnapshot(
          { llm: target.config },
          {
            includeLlmApiKeys: false,
            includeSystemLlmApiKey: false,
            includeProviderApiKeys: false,
          },
        )
      : null,
  });
}

export async function detectLlmConfigModels(
  input: ModelDiscoveryInput,
  scope: ModelConfigScope,
): Promise<ModelDiscoveryResult> {
  return detectConfigModels("detect_llm_config_models", input, scope, {
    systemConfigId: SYSTEM_LLM_CONFIG_ID,
    getSystemDefaultApiKey: getSystemDefaultLlmApiKey,
    getLocalConfigApiKey: (configId) =>
      getLocalLlmConfig(configId).api_key_encrypted,
  });
}
