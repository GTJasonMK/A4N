import { apiInvoke } from "../index";
import {
  withLocalRuntimeDiscoveryKey,
  type ModelDiscoveryInput,
  type ModelDiscoveryResult,
} from "../modelDiscovery";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  CreateLlmInput,
  LlmConfig,
  LlmMaxTokensConfig,
  UpdateLlmInput,
} from "./types";
import { type ApiId as Id, idNum, integerNum } from "../shared";
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

export const SYSTEM_LLM_CONFIG_ID = -1;

function llmConfigId(configId: Id): number {
  return idNum(configId, "LLM 配置 ID");
}

function llmConfigIdOrSystem(configId: Id): number {
  const value = integerNum(configId, "LLM 配置 ID");
  return value === SYSTEM_LLM_CONFIG_ID ? value : idNum(value, "LLM 配置 ID");
}

function isSystemDefaultLlmConfigId(configId: Id): boolean {
  return llmConfigIdOrSystem(configId) === SYSTEM_LLM_CONFIG_ID;
}

function systemDefaultLlmConfig(configs: LlmConfig[]): LlmConfig | null {
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

function rejectSystemDefaultMutation(): never {
  throw new Error("系统默认配置模板由管理员维护，不能在用户配置页修改");
}

function requiredSystemDefaultLlmConfig(configs: LlmConfig[]): LlmConfig {
  const config = systemDefaultLlmConfig(configs);
  if (!config) throw new Error("服务器尚未配置系统默认 LLM 模板");
  return config;
}

function systemDefaultRuntimeLlmConfig(config: LlmConfig, apiKey?: string | null) {
  const normalizedApiKey = apiKey?.trim();
  return {
    ...config,
    id: SYSTEM_LLM_CONFIG_ID,
    name: `系统默认：${config.name.replace(/^系统默认：/, "")}`,
    ...(normalizedApiKey ? { api_key_encrypted: normalizedApiKey } : {}),
    has_api_key: normalizedApiKey ? true : config.has_api_key,
    is_default: true,
  };
}

async function loadSystemDefaultRuntimeLlmConfig(options?: { requireApiKey?: boolean }) {
  const requireApiKey = options?.requireApiKey ?? true;
  const apiKey = getSystemDefaultLlmApiKey()?.trim();
  if (requireApiKey && !apiKey) {
    throw new Error("使用系统默认 LLM 需要先填写并保存你的个人 API Key");
  }
  const config = requiredSystemDefaultLlmConfig(await listServerLlmConfig());
  return systemDefaultRuntimeLlmConfig(config, requireApiKey ? apiKey : undefined);
}

export async function listServerLlmConfig(): Promise<LlmConfig[]> {
  return apiInvoke<LlmConfig[]>("list_llm_config");
}

export async function listLlmConfig(): Promise<LlmConfig[]> {
  if (!usesLocalRuntimeSettings()) return listServerLlmConfig();
  const [serverConfigs, localConfigs] = await Promise.all([
    listServerLlmConfig(),
    Promise.resolve(listLocalLlmConfigs()),
  ]);
  const systemDefault = systemDefaultLlmConfig(serverConfigs);
  const localItems = usesSystemDefaultLlmConfig()
    ? localConfigs.map((item) => ({ ...item, is_default: false }))
    : localConfigs;
  return systemDefault ? [systemDefault, ...localItems] : localItems;
}

export async function createServerLlmConfig(input: CreateLlmInput): Promise<LlmConfig> {
  return apiInvoke<LlmConfig>("create_llm_config", { input });
}

export async function createLlmConfig(input: CreateLlmInput): Promise<LlmConfig> {
  if (usesLocalRuntimeSettings()) return createLocalLlmConfig(input);
  return createServerLlmConfig(input);
}

export async function updateServerLlmConfig(
  configId: Id,
  input: UpdateLlmInput,
): Promise<LlmConfig> {
  return apiInvoke<LlmConfig>("update_llm_config", {
    configId: llmConfigId(configId),
    input,
  });
}

export async function updateLlmConfig(
  configId: Id,
  input: UpdateLlmInput,
): Promise<LlmConfig> {
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
  return apiInvoke<void>("delete_llm_config", { configId: llmConfigId(configId) });
}

export async function deleteLlmConfig(configId: Id): Promise<void> {
  if (isSystemDefaultLlmConfigId(configId)) rejectSystemDefaultMutation();
  if (usesLocalRuntimeSettings()) {
    deleteLocalLlmConfig(llmConfigId(configId));
    return;
  }
  return deleteServerLlmConfig(configId);
}

export async function testServerLlmConfig(configId: Id): Promise<boolean> {
  return apiInvoke<boolean>("test_llm_config", {
    configId: llmConfigId(configId),
    runtime_config_snapshot: null,
  });
}

export async function testLlmConfig(configId: Id): Promise<boolean> {
  if (isSystemDefaultLlmConfigId(configId)) {
    const llm = await loadSystemDefaultRuntimeLlmConfig();
    return apiInvoke<boolean>("test_llm_config", {
      configId: SYSTEM_LLM_CONFIG_ID,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({ llm }),
    });
  }
  if (usesLocalRuntimeSettings()) {
    const id = llmConfigId(configId);
    return apiInvoke<boolean>("test_llm_config", {
      configId: id,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({ llm: getLocalLlmConfig(id) }),
    });
  }
  return testServerLlmConfig(configId);
}

export async function previewServerTestLlmConfigRequest(configId: Id): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_test_llm_config_request", {
    configId: llmConfigId(configId),
    runtime_config_snapshot: null,
  });
}

export async function previewTestLlmConfigRequest(configId: Id): Promise<ModelRequestPreview> {
  if (isSystemDefaultLlmConfigId(configId)) {
    const llm = await loadSystemDefaultRuntimeLlmConfig({ requireApiKey: false });
    return apiInvoke<ModelRequestPreview>("preview_test_llm_config_request", {
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
  return apiInvoke<LlmMaxTokensConfig>("update_llm_max_tokens_config", { input });
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
  return apiInvoke<ModelDiscoveryResult>("detect_llm_config_models", { input });
}

export async function detectLlmConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return detectServerLlmConfigModels(
    withLocalRuntimeDiscoveryKey(input, {
      systemConfigId: SYSTEM_LLM_CONFIG_ID,
      getSystemDefaultApiKey: getSystemDefaultLlmApiKey,
      getLocalConfigApiKey: (configId) => getLocalLlmConfig(configId).api_key_encrypted,
    }),
  );
}
