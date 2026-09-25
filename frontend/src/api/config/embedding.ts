import { apiInvoke } from "../index";
import { runtimeInvoke } from "../runtime";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  CreateEmbeddingInput,
  EmbeddingConfig,
  EmbeddingModelDownloadInfo,
  LocalEmbedTextsOutput,
  LocalEmbeddingConfigInput,
  LocalEmbeddingModel,
  UpdateEmbeddingInput,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";
import {
  createLocalEmbeddingConfig,
  deleteLocalEmbeddingConfig,
  getLocalEmbeddingConfig,
  listLocalEmbeddingConfigs,
  setDefaultLocalEmbeddingConfig,
  updateLocalEmbeddingConfig,
} from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import type { ModelConfigScope } from "../modelDiscovery";
import {
  canUseLocalDesktopEmbedding,
  usesLocalRuntimeSettings,
} from "../../utils/runtimeMode";

function usesScopedLocalEmbeddingSettings(scope: ModelConfigScope): boolean {
  return scope === "local" && usesLocalRuntimeSettings();
}

function usesScopedLocalDesktopEmbedding(scope: ModelConfigScope): boolean {
  return scope === "local" && canUseLocalDesktopEmbedding();
}

export async function listEmbeddingConfig(
  scope: ModelConfigScope,
): Promise<EmbeddingConfig[]> {
  if (usesScopedLocalEmbeddingSettings(scope)) return listLocalEmbeddingConfigs();
  return apiInvoke<EmbeddingConfig[]>("list_embedding_config");
}

export async function createEmbeddingConfig(
  input: CreateEmbeddingInput,
  scope: ModelConfigScope,
): Promise<EmbeddingConfig> {
  if (usesScopedLocalEmbeddingSettings(scope)) return createLocalEmbeddingConfig(input);
  return apiInvoke<EmbeddingConfig>("create_embedding_config", { input });
}

export async function setDefaultEmbeddingConfig(
  configId: Id,
  scope: ModelConfigScope,
): Promise<void> {
  if (usesScopedLocalEmbeddingSettings(scope)) {
    return setDefaultLocalEmbeddingConfig(idNum(configId, "Embedding 配置 ID"));
  }
  return apiInvoke<void>("set_default_embedding_config", {
    configId: idNum(configId, "Embedding 配置 ID"),
  });
}

export async function deleteEmbeddingConfig(
  configId: Id,
  scope: ModelConfigScope,
): Promise<void> {
  if (usesScopedLocalEmbeddingSettings(scope)) {
    return deleteLocalEmbeddingConfig(idNum(configId, "Embedding 配置 ID"));
  }
  return apiInvoke<void>("delete_embedding_config", {
    configId: idNum(configId, "Embedding 配置 ID"),
  });
}

export async function updateEmbeddingConfig(
  configId: Id,
  input: UpdateEmbeddingInput,
  scope: ModelConfigScope,
): Promise<EmbeddingConfig> {
  if (usesScopedLocalEmbeddingSettings(scope)) {
    return updateLocalEmbeddingConfig(
      idNum(configId, "Embedding 配置 ID"),
      input,
    );
  }
  return apiInvoke<EmbeddingConfig>("update_embedding_config", {
    configId: idNum(configId, "Embedding 配置 ID"),
    input,
  });
}

function resolveEmbeddingConnectivityTarget(
  configId: Id,
  scope: ModelConfigScope,
): {
  configId: number;
  config: EmbeddingConfig | null;
} {
  const id = idNum(configId, "Embedding 配置 ID");
  return {
    configId: id,
    config: usesScopedLocalEmbeddingSettings(scope)
      ? getLocalEmbeddingConfig(id)
      : null,
  };
}

export async function testEmbeddingConfig(
  configId: Id,
  scope: ModelConfigScope,
): Promise<void> {
  const target = resolveEmbeddingConnectivityTarget(configId, scope);
  if (target.config && usesScopedLocalDesktopEmbedding(scope)) {
    return runtimeInvoke<void>("local_test_embedding_config", {
      input: localEmbeddingConfigInput(target.config),
    });
  }
  return apiInvoke<void>("test_embedding_config", {
    configId: target.configId,
    runtime_config_snapshot: target.config
      ? buildRuntimeConfigSnapshot({ embedding: target.config })
      : null,
  });
}

export async function previewTestEmbeddingConfigRequest(
  configId: Id,
  scope: ModelConfigScope,
): Promise<ModelRequestPreview> {
  const target = resolveEmbeddingConnectivityTarget(configId, scope);
  return apiInvoke<ModelRequestPreview>(
    "preview_test_embedding_config_request",
    {
      configId: target.configId,
      runtime_config_snapshot: target.config
        ? buildRuntimeConfigSnapshot(
            { embedding: target.config },
            { includeProviderApiKeys: false },
          )
        : null,
    },
  );
}

export async function listLocalEmbeddingModels(
  scope: ModelConfigScope,
): Promise<LocalEmbeddingModel[]> {
  if (usesScopedLocalDesktopEmbedding(scope)) {
    return runtimeInvoke<LocalEmbeddingModel[]>("local_list_embedding_models");
  }
  return apiInvoke<LocalEmbeddingModel[]>("list_local_embedding_models");
}

export async function getEmbeddingModelDownloadInfo(
  model: string,
  scope: ModelConfigScope,
): Promise<EmbeddingModelDownloadInfo> {
  if (usesScopedLocalDesktopEmbedding(scope)) {
    return runtimeInvoke<EmbeddingModelDownloadInfo>(
      "local_get_embedding_model_download_info",
      {
        input: { model },
      },
    );
  }
  return apiInvoke<EmbeddingModelDownloadInfo>(
    "get_embedding_model_download_info",
    {
      input: { model },
    },
  );
}

export async function downloadEmbeddingModel(
  model: string,
  taskId: string,
  scope: ModelConfigScope,
): Promise<LocalEmbeddingModel> {
  if (usesScopedLocalDesktopEmbedding(scope)) {
    return runtimeInvoke<LocalEmbeddingModel>(
      "local_download_embedding_model",
      {
        input: { model, task_id: taskId },
      },
    );
  }
  return apiInvoke<LocalEmbeddingModel>("download_embedding_model", {
    input: { model, task_id: taskId },
  });
}

export async function cancelEmbeddingModelDownload(
  taskId: string,
  scope: ModelConfigScope,
): Promise<void> {
  if (usesScopedLocalDesktopEmbedding(scope)) {
    return runtimeInvoke<void>("local_cancel_embedding_model_download", {
      input: { task_id: taskId },
    });
  }
  return apiInvoke<void>("cancel_embedding_model_download", {
    input: { task_id: taskId },
  });
}

export async function deleteLocalEmbeddingModel(
  model: string,
  scope: ModelConfigScope,
): Promise<void> {
  if (usesScopedLocalDesktopEmbedding(scope)) {
    return runtimeInvoke<void>("local_delete_embedding_model", {
      input: { model },
    });
  }
  return apiInvoke<void>("delete_local_embedding_model", { input: { model } });
}

export async function embedTextsLocally(
  config: EmbeddingConfig,
  texts: string[],
): Promise<LocalEmbedTextsOutput> {
  if (!canUseLocalDesktopEmbedding()) {
    throw new Error("当前运行模式不支持本机桌面 Embedding");
  }
  return runtimeInvoke<LocalEmbedTextsOutput>("local_embed_texts", {
    input: { config: localEmbeddingConfigInput(config), texts },
  });
}

function localEmbeddingConfigInput(
  config: EmbeddingConfig,
): LocalEmbeddingConfigInput {
  return {
    backend: config.backend,
    model: config.model,
    dimension: config.dimension,
    batch_size: config.batch_size,
    local_model_path: config.local_model_path,
  };
}
