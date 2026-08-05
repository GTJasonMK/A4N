import { apiInvoke } from "../index";
import { runtimeInvoke } from "../runtime";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  CreateEmbeddingInput,
  EmbeddingConfig,
  EmbeddingModelDownloadInfo,
  LocalEmbedTextsOutput,
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
import {
  canUseLocalDesktopEmbedding,
  usesLocalRuntimeSettings,
} from "../../utils/runtimeMode";

export async function listEmbeddingConfig(): Promise<EmbeddingConfig[]> {
  if (usesLocalRuntimeSettings()) return listLocalEmbeddingConfigs();
  return apiInvoke<EmbeddingConfig[]>("list_embedding_config");
}

export async function createEmbeddingConfig(
  input: CreateEmbeddingInput,
): Promise<EmbeddingConfig> {
  if (usesLocalRuntimeSettings()) return createLocalEmbeddingConfig(input);
  return apiInvoke<EmbeddingConfig>("create_embedding_config", { input });
}

export async function setDefaultEmbeddingConfig(configId: Id): Promise<void> {
  if (usesLocalRuntimeSettings()) {
    setDefaultLocalEmbeddingConfig(idNum(configId, "Embedding 配置 ID"));
    return;
  }
  return apiInvoke<void>("set_default_embedding_config", {
    configId: idNum(configId, "Embedding 配置 ID"),
  });
}

export async function deleteEmbeddingConfig(configId: Id): Promise<void> {
  if (usesLocalRuntimeSettings()) {
    deleteLocalEmbeddingConfig(idNum(configId, "Embedding 配置 ID"));
    return;
  }
  return apiInvoke<void>("delete_embedding_config", {
    configId: idNum(configId, "Embedding 配置 ID"),
  });
}

export async function updateEmbeddingConfig(
  configId: Id,
  input: UpdateEmbeddingInput,
): Promise<EmbeddingConfig> {
  if (usesLocalRuntimeSettings()) {
    return updateLocalEmbeddingConfig(idNum(configId, "Embedding 配置 ID"), input);
  }
  return apiInvoke<EmbeddingConfig>("update_embedding_config", {
    configId: idNum(configId, "Embedding 配置 ID"),
    input,
  });
}

export async function testEmbeddingConfig(configId: Id): Promise<boolean> {
  if (usesLocalRuntimeSettings()) {
    const id = idNum(configId, "Embedding 配置 ID");
    if (canUseLocalDesktopEmbedding()) {
      return runtimeInvoke<boolean>("local_test_embedding_config", {
        input: getLocalEmbeddingConfig(id),
      });
    }
    return apiInvoke<boolean>("test_embedding_config", {
      configId: id,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({
        embedding: getLocalEmbeddingConfig(id),
      }),
    });
  }
  return apiInvoke<boolean>("test_embedding_config", {
    configId: idNum(configId, "Embedding 配置 ID"),
  });
}

export async function previewTestEmbeddingConfigRequest(configId: Id): Promise<ModelRequestPreview> {
  if (usesLocalRuntimeSettings()) {
    const id = idNum(configId, "Embedding 配置 ID");
    return apiInvoke<ModelRequestPreview>("preview_test_embedding_config_request", {
      configId: id,
      runtime_config_snapshot: buildRuntimeConfigSnapshot(
        {
          embedding: getLocalEmbeddingConfig(id),
        },
        { includeProviderApiKeys: false },
      ),
    });
  }
  return apiInvoke<ModelRequestPreview>("preview_test_embedding_config_request", {
    configId: idNum(configId, "Embedding 配置 ID"),
  });
}

export async function listLocalEmbeddingModels(): Promise<LocalEmbeddingModel[]> {
  if (canUseLocalDesktopEmbedding()) {
    return runtimeInvoke<LocalEmbeddingModel[]>("local_list_embedding_models");
  }
  return apiInvoke<LocalEmbeddingModel[]>("list_local_embedding_models");
}

export async function getEmbeddingModelDownloadInfo(
  model: string,
): Promise<EmbeddingModelDownloadInfo> {
  if (canUseLocalDesktopEmbedding()) {
    return runtimeInvoke<EmbeddingModelDownloadInfo>("local_get_embedding_model_download_info", {
      input: { model },
    });
  }
  return apiInvoke<EmbeddingModelDownloadInfo>("get_embedding_model_download_info", {
    input: { model },
  });
}

export async function downloadEmbeddingModel(
  model: string,
  taskId?: string | null,
): Promise<LocalEmbeddingModel> {
  if (canUseLocalDesktopEmbedding()) {
    return runtimeInvoke<LocalEmbeddingModel>("local_download_embedding_model", {
      input: { model, task_id: taskId ?? null },
    });
  }
  return apiInvoke<LocalEmbeddingModel>("download_embedding_model", {
    input: { model, task_id: taskId ?? null },
  });
}

export async function cancelEmbeddingModelDownload(taskId: string): Promise<void> {
  if (canUseLocalDesktopEmbedding()) {
    return runtimeInvoke<void>("local_cancel_embedding_model_download", {
      input: { task_id: taskId },
    });
  }
  return apiInvoke<void>("cancel_embedding_model_download", {
    input: { task_id: taskId },
  });
}

export async function deleteLocalEmbeddingModel(model: string): Promise<void> {
  if (canUseLocalDesktopEmbedding()) {
    return runtimeInvoke<void>("local_delete_embedding_model", { input: { model } });
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
    input: { config, texts },
  });
}
