import { apiInvoke } from "../index";
import { requireCurrentUserId } from "../authStorage";
import {
  withLocalRuntimeDiscoveryKey,
  type ModelDiscoveryInput,
  type ModelDiscoveryResult,
} from "../modelDiscovery";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  CreateLlmInput,
  LlmConfigSummary,
  LlmMaxTokensConfig,
  UpdateLlmInput,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";
import {
  createLocalLlmConfig,
  deleteLocalLlmConfig,
  getSystemDefaultLlmApiKey,
  getLocalLlmConfig,
  hasSystemDefaultLlmApiKey,
  getLocalLlmMaxTokensConfig,
  listLocalLlmConfigs,
  setDefaultLocalLlmConfig,
  setSystemDefaultLlmConfig,
  updateLocalLlmConfig,
  updateLocalLlmMaxTokensConfig,
  updateSystemDefaultLlmApiKey,
  usesSystemDefaultLlmConfig,
} from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";
import {
  mergeLocalRuntimeConfigs,
  providerConfigIdOrSystem,
  rejectSystemDefaultMutation,
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
  const config = configs.find((item) => item.is_default) ?? null;
  if (!config) return null;
  return {
    ...config,
    id: SYSTEM_LLM_CONFIG_ID,
    name: `系统默认：${config.name}`,
    has_api_key: hasSystemDefaultLlmApiKey(),
    is_default: usesSystemDefaultLlmConfig(),
  };
}

function requiredSystemDefaultLlmConfig(
  configs: LlmConfigSummary[],
): LlmConfigSummary {
  const config = systemDefaultLlmConfig(configs);
  if (!config) throw new Error("服务器尚未配置系统默认 LLM 模板");
  return config;
}

function systemDefaultRuntimeLlmConfig(
  config: LlmConfigSummary,
  apiKey?: string,
) {
  return {
    ...config,
    id: SYSTEM_LLM_CONFIG_ID,
    name: `系统默认：${config.name.replace(/^系统默认：/, "")}`,
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

export async function testServerLlmConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("test_llm_config", {
    userId: requireCurrentUserId(),
    configId: llmConfigId(configId),
    runtime_config_snapshot: null,
  });
}

export async function testLlmConfig(configId: Id): Promise<void> {
  if (isSystemDefaultLlmConfigId(configId)) {
    const llm = await loadSystemDefaultRuntimeLlmConfig();
    return apiInvoke<void>("test_llm_config", {
      userId: requireCurrentUserId(),
      configId: SYSTEM_LLM_CONFIG_ID,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({ llm }),
    });
  }
  if (usesLocalRuntimeSettings()) {
    const id = llmConfigId(configId);
    return apiInvoke<void>("test_llm_config", {
      userId: requireCurrentUserId(),
      configId: id,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({
        llm: getLocalLlmConfig(id),
      }),
    });
  }
  return testServerLlmConfig(configId);
}

export async function previewServerTestLlmConfigRequest(
  configId: Id,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_test_llm_config_request", {
    userId: requireCurrentUserId(),
    configId: llmConfigId(configId),
    runtime_config_snapshot: null,
  });
}

export async function previewTestLlmConfigRequest(
  configId: Id,
): Promise<ModelRequestPreview> {
  if (isSystemDefaultLlmConfigId(configId)) {
    const llm = await loadSystemDefaultRuntimeLlmConfig({
      requireApiKey: false,
    });
    return apiInvoke<ModelRequestPreview>("preview_test_llm_config_request", {
      userId: requireCurrentUserId(),
      configId: SYSTEM_LLM_CONFIG_ID,
      runtime_config_snapshot: buildRuntimeConfigSnapshot(
        { llm },
        {
          includeLlmApiKeys: false,
          includeSystemLlmApiKey: false,
          includeProviderApiKeys: false,
        },
      ),
    });
  }
  if (usesLocalRuntimeSettings()) {
    const id = llmConfigId(configId);
    return apiInvoke<ModelRequestPreview>("preview_test_llm_config_request", {
      userId: requireCurrentUserId(),
      configId: id,
      runtime_config_snapshot: buildRuntimeConfigSnapshot(
        { llm: getLocalLlmConfig(id) },
        {
          includeLlmApiKeys: false,
          includeSystemLlmApiKey: false,
          includeProviderApiKeys: false,
        },
      ),
    });
  }
  return previewServerTestLlmConfigRequest(configId);
}

export async function getServerLlmMaxTokensConfig(): Promise<LlmMaxTokensConfig> {
  return apiInvoke<LlmMaxTokensConfig>("get_llm_max_tokens_config");
}

export async function getLlmMaxTokensConfig(): Promise<LlmMaxTokensConfig> {
  if (usesLocalRuntimeSettings()) return getLocalLlmMaxTokensConfig();
  return getServerLlmMaxTokensConfig();
}

export async function updateServerLlmMaxTokensConfig(
  input: LlmMaxTokensConfig,
): Promise<LlmMaxTokensConfig> {
  return apiInvoke<LlmMaxTokensConfig>("update_llm_max_tokens_config", {
    input,
  });
}

export async function updateLlmMaxTokensConfig(
  input: LlmMaxTokensConfig,
): Promise<LlmMaxTokensConfig> {
  if (usesLocalRuntimeSettings()) return updateLocalLlmMaxTokensConfig(input);
  return updateServerLlmMaxTokensConfig(input);
}

export async function detectServerLlmConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return apiInvoke<ModelDiscoveryResult>("detect_llm_config_models", {
    userId: requireCurrentUserId(),
    input,
  });
}

export async function detectLlmConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return detectServerLlmConfigModels(
    withLocalRuntimeDiscoveryKey(input, {
      systemConfigId: SYSTEM_LLM_CONFIG_ID,
      getSystemDefaultApiKey: getSystemDefaultLlmApiKey,
      getLocalConfigApiKey: (configId) =>
        getLocalLlmConfig(configId).api_key_encrypted,
    }),
  );
}
