import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import {
  allocateHostTaskId,
  runTaskWithPolling,
  useProgressTask,
} from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import {
  canUseLocalDesktopEmbedding,
  shouldUseHttpBackend,
} from "../../utils/runtimeMode";
import { extractErrorMessage } from "../../api";
import {
  listEmbeddingConfig,
  createEmbeddingConfig,
  updateEmbeddingConfig,
  deleteEmbeddingConfig,
  setDefaultEmbeddingConfig,
  testEmbeddingConfig,
  previewTestEmbeddingConfigRequest,
  listEmbeddingBackends,
  listLocalEmbeddingModels,
  getEmbeddingModelDownloadInfo,
  downloadEmbeddingModel,
  cancelEmbeddingModelDownload,
  deleteLocalEmbeddingModel,
  type EmbeddingModelDownloadInfo,
  type LocalEmbeddingModel,
} from "../../api/config";
import { ConfigItem, ConfigListPanel } from "./ConfigListPanel";
import { EmbeddingConfigEditor } from "./embedding/EmbeddingConfigEditor";
import {
  BACKEND_LABELS,
  DEFAULT_FASTEMBED_DIMENSION,
  DEFAULT_FASTEMBED_MODEL,
  DEFAULT_FORM,
  formatBytesProgress,
  formatDownloadStartMessage,
  toItem,
} from "./embedding/embeddingTabUtils";
import type { EmbeddingForm } from "./embedding/types";

/**
 * Embedding 配置 Tab
 *
 * 当前真实接入的后端：
 * - api：OpenAI 兼容 `/embeddings` 接口
 * - fastembed：显式下载后的本地 ONNX 嵌入模型
 * - local_hash：本地确定性哈希向量，用于离线开发和基础检索验证
 */

export const EmbeddingTab: React.FC = () => {
  const { addToast } = useToast();
  const usesRemoteBackend = shouldUseHttpBackend();
  const usesLocalDesktopEmbedding = canUseLocalDesktopEmbedding();
  const modelHostLabel = usesLocalDesktopEmbedding
    ? "本机"
    : usesRemoteBackend
      ? "远程服务器"
      : "本机";
  const { runProgressTask } = useProgressTask();
  const [items, setItems] = useState<ConfigItem<EmbeddingForm>[]>([]);
  const [localModels, setLocalModels] = useState<LocalEmbeddingModel[]>([]);
  const [loading, setLoading] = useState(false);
  const [backends, setBackends] = useState<string[]>([]);
  const [busyModel, setBusyModel] = useState<string | null>(null);
  const [downloadTaskId, setDownloadTaskId] = useState<string | null>(null);
  const [downloadInfoByModel, setDownloadInfoByModel] = useState<
    Record<string, EmbeddingModelDownloadInfo>
  >({});
  const [downloadInfoErrors, setDownloadInfoErrors] = useState<Record<string, string>>(
    {},
  );
  const mountedRef = useRef(true);
  const refreshSeqRef = useRef(0);
  const modelActionRef = useRef<string | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
      modelActionRef.current = null;
    };
  }, []);

  const isCurrentRefresh = useCallback((seq: number) => {
    return mountedRef.current && refreshSeqRef.current === seq;
  }, []);

  const localModelByName = useMemo(() => {
    const map = new Map<string, LocalEmbeddingModel>();
    for (const model of localModels) {
      map.set(model.name.toLowerCase(), model);
      map.set(model.model_code.toLowerCase(), model);
    }
    return map;
  }, [localModels]);

  const refresh = useCallback(async () => {
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const [list, b, models] = await Promise.all([
        listEmbeddingConfig(),
        listEmbeddingBackends(),
        listLocalEmbeddingModels(),
      ]);
      if (!isCurrentRefresh(requestSeq)) return;
      setItems(list.map(toItem));
      setBackends(b);
      setLocalModels(models);
    } catch (err) {
      if (!isCurrentRefresh(requestSeq)) return;
      addToast(extractErrorMessage(err, "加载 Embedding 配置失败"), "error");
    } finally {
      if (isCurrentRefresh(requestSeq)) setLoading(false);
    }
  }, [addToast, isCurrentRefresh]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (downloadTaskId) return;

    let cancelled = false;
    const models = localModels.filter(
      (model) => !downloadInfoByModel[model.name.toLowerCase()],
    );
    if (models.length === 0) return;

    void (async () => {
      const queue = [...models];
      const workerCount = 1;
      const workers = Array.from({ length: workerCount }, async () => {
        while (!cancelled) {
          const model = queue.shift();
          if (!model) return;
          try {
            const info = await getEmbeddingModelDownloadInfo(model.name);
            if (cancelled) return;
            rememberDownloadInfo(info);
          } catch (err) {
            if (cancelled) return;
            rememberDownloadInfoError(
              model,
              extractErrorMessage(err, "模型大小获取失败"),
            );
          }
        }
      });
      await Promise.all(workers);
    })();

    return () => {
      cancelled = true;
    };
    // 后台补齐模型大小；downloadInfoByModel 只用作本轮筛选，避免每次写入都重启循环。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [localModels, downloadTaskId]);

  const pickFastembedModel = () =>
    localModels.find((model) => model.name === DEFAULT_FASTEMBED_MODEL) ??
    localModels.find((model) => model.downloaded) ??
    localModels[0] ??
    null;

  const selectableBackends = useMemo(
    () => (usesLocalDesktopEmbedding ? ["fastembed"] : backends),
    [backends, usesLocalDesktopEmbedding],
  );

  const defaultEmbeddingForm = useMemo((): EmbeddingForm => {
    if (!usesLocalDesktopEmbedding) return DEFAULT_FORM;
    const model = localModels.find((item) => item.name === DEFAULT_FASTEMBED_MODEL)
      ?? localModels.find((item) => item.downloaded)
      ?? localModels[0]
      ?? null;
    return {
      ...DEFAULT_FORM,
      name: "本机 fastembed",
      backend: "fastembed",
      model: model?.name ?? DEFAULT_FASTEMBED_MODEL,
      dimension: model?.dimension ?? DEFAULT_FASTEMBED_DIMENSION,
      batch_size: 32,
      api_base_url: "",
      api_key: "",
      clear_api_key: false,
      local_model_path: "",
    };
  }, [localModels, usesLocalDesktopEmbedding]);

  const resolveLocalModel = (modelName: string): LocalEmbeddingModel | null =>
    localModelByName.get(modelName.trim().toLowerCase()) ?? null;

  const resolveDownloadInfo = (modelName: string): EmbeddingModelDownloadInfo | null =>
    downloadInfoByModel[modelName.trim().toLowerCase()] ?? null;

  const resolveDownloadInfoError = (modelName: string): string | null =>
    downloadInfoErrors[modelName.trim().toLowerCase()] ?? null;

  const rememberDownloadInfo = (info: EmbeddingModelDownloadInfo) => {
    setDownloadInfoByModel((prev) => ({
      ...prev,
      [info.model_name.toLowerCase()]: info,
      [info.model_code.toLowerCase()]: info,
    }));
    setDownloadInfoErrors((prev) => {
      const next = { ...prev };
      delete next[info.model_name.toLowerCase()];
      delete next[info.model_code.toLowerCase()];
      return next;
    });
  };

  const rememberDownloadInfoError = (model: LocalEmbeddingModel | string, message: string) => {
    const modelName = typeof model === "string" ? model : model.name;
    const modelCode = typeof model === "string" ? model : model.model_code;
    setDownloadInfoErrors((prev) => ({
      ...prev,
      [modelName.toLowerCase()]: message,
      [modelCode.toLowerCase()]: message,
    }));
  };

  const fetchDownloadInfo = async (
    model: LocalEmbeddingModel | string,
  ): Promise<EmbeddingModelDownloadInfo> => {
    const modelName = typeof model === "string" ? model : model.name;
    const info = await getEmbeddingModelDownloadInfo(modelName);
    rememberDownloadInfo(info);
    return info;
  };

  const modelActionKey = (model: string) => model.trim().toLowerCase();

  const beginModelAction = (actionKey: string): boolean => {
    if (!mountedRef.current || modelActionRef.current) return false;
    modelActionRef.current = actionKey;
    return true;
  };

  const finishModelAction = (actionKey: string) => {
    if (modelActionRef.current === actionKey) {
      modelActionRef.current = null;
    }
  };

  const handleDownloadModel = async (modelName: string) => {
    const model = modelName.trim();
    if (!model) {
      addToast("请先选择 fastembed 模型", "error");
      return;
    }

    const actionKey = modelActionKey(model);
    if (!beginModelAction(actionKey)) return;
    setBusyModel(model);
    const cachedInfo = resolveDownloadInfo(model);
    const taskId = allocateHostTaskId("embedding-download", 0);
    setDownloadTaskId(taskId);
    try {
      await runProgressTask(
        {
          cancelTaskId: taskId,
          title: `下载${modelHostLabel}嵌入模型`,
          startMessage: cachedInfo
            ? formatDownloadStartMessage(cachedInfo)
            : "正在读取模型文件大小…",
          successMessage: "模型已下载并可用于 RAG",
          errorMessage: "模型下载失败",
          total: cachedInfo ? Math.max(1, cachedInfo.total_size_bytes) : 1,
          cancelLabel: "停止下载",
          runningHint: "下载过程中请保持当前窗口打开",
          formatProgress: formatBytesProgress,
          cancelTask: cancelEmbeddingModelDownload,
        },
        async (control) => {
          if (cachedInfo) {
            control.setProgress(
              formatDownloadStartMessage(cachedInfo),
              cachedInfo.downloaded_size_bytes,
              Math.max(1, cachedInfo.total_size_bytes),
            );
          }
          await runTaskWithPolling(
            downloadEmbeddingModel(model, taskId),
            () => getEmbeddingModelDownloadInfo(model),
            (info) => {
              if (!mountedRef.current || modelActionRef.current !== actionKey) return;
              rememberDownloadInfo(info);
              control.setProgress(
                `正在下载${modelHostLabel}嵌入模型…`,
                info.downloaded_size_bytes,
                Math.max(1, info.total_size_bytes),
              );
            },
          );
          const latestInfo = await fetchDownloadInfo(model);
          control.setProgress(
            `正在刷新${modelHostLabel}模型状态…`,
            latestInfo.downloaded_size_bytes,
            Math.max(1, latestInfo.total_size_bytes),
          );
          await refresh();
        },
      );
      addToast(`模型已下载到${modelHostLabel}`, "success");
    } catch (err) {
      const message = extractErrorMessage(err, "模型下载失败");
      if (isCancelledOperation(message)) {
        addToast("模型下载已停止", "info");
      } else {
        addToast(message, "error");
      }
    } finally {
      if (modelActionRef.current === actionKey && mountedRef.current) {
        setBusyModel(null);
        setDownloadTaskId(null);
      }
      finishModelAction(actionKey);
    }
  };

  const handleDeleteModel = async (modelName: string) => {
    const model = modelName.trim();
    if (!model) return;
    const actionKey = modelActionKey(model);
    if (!beginModelAction(actionKey)) return;

    try {
      const ok = await confirmDialog({
        title: `删除${modelHostLabel}嵌入模型`,
        message: `确定删除 ${model} 在${modelHostLabel}上的缓存？使用该模型的配置将无法通过连接测试，直到重新下载。`,
        confirmText: "删除",
        dialogType: "danger",
      });
      if (!ok) return;

      setBusyModel(model);
      await deleteLocalEmbeddingModel(model);
      try {
        const info = await getEmbeddingModelDownloadInfo(model);
        setDownloadInfoByModel((prev) => ({
          ...prev,
          [info.model_name.toLowerCase()]: info,
          [info.model_code.toLowerCase()]: info,
        }));
      } catch {
        setDownloadInfoByModel((prev) => {
          const next = { ...prev };
          delete next[model.toLowerCase()];
          return next;
        });
      }
      await refresh();
      addToast(`${modelHostLabel}模型缓存已删除`, "success");
    } catch (err) {
      addToast(extractErrorMessage(err, "删除模型失败"), "error");
    } finally {
      if (modelActionRef.current === actionKey && mountedRef.current) {
        setBusyModel(null);
      }
      finishModelAction(actionKey);
    }
  };

  return (
    <div className="h-full flex flex-col gap-4">
      {usesRemoteBackend && (
        <div className="rounded border border-warning/30 bg-warning-bg px-3 py-2 text-xs leading-relaxed text-warning">
          {usesLocalDesktopEmbedding
            ? `项目数据存放在服务器，嵌入模型由当前设备管理。`
            : "嵌入模型由远程服务器管理，本机缓存不会被直接使用。"}
        </div>
      )}
      <div className="flex-1 min-h-0">
        <ConfigListPanel<EmbeddingForm>
          title="Embedding 配置"
          items={items}
          loading={loading}
          onRefresh={refresh}
          defaultForm={defaultEmbeddingForm}
          validate={(f) => {
            if (usesLocalDesktopEmbedding && f.backend !== "fastembed") {
              return "远程后端 + 本机 Embedding 模式仅支持 fastembed";
            }
            if (!f.name.trim()) return "请填写配置名称";
            if (!f.model.trim()) return "请填写模型名";
            if (!Number.isInteger(f.dimension) || f.dimension <= 0) {
              return "向量维度必须为正整数";
            }
            if (!Number.isInteger(f.batch_size) || f.batch_size <= 0) {
              return "批量大小必须为正整数";
            }
            if (f.backend === "api" && !f.api_base_url.trim()) {
              return "API 后端必须填写 Base URL";
            }
            if (f.backend === "fastembed") {
              const model = resolveLocalModel(f.model);
              if (!model) return "请选择支持的 fastembed 模型";
              if (f.dimension !== model.dimension) {
                return `该模型输出 ${model.dimension} 维，配置维度必须一致`;
              }
              if (!model.downloaded) return "请先下载该 fastembed 模型";
            }
            return null;
          }}
          formatItem={(c) => ({
            primary: c.name,
            secondary: `${BACKEND_LABELS[c.backend] || c.backend} · ${c.model}`,
            tertiary: `维度 ${c.dimension} · 批量 ${c.batch_size}`,
          })}
          renderEditor={(form, setForm) => (
            <EmbeddingConfigEditor
              form={form}
              backends={selectableBackends}
              localModels={localModels}
              busyModel={busyModel}
              setForm={setForm}
              pickFastembedModel={pickFastembedModel}
              resolveLocalModel={resolveLocalModel}
              resolveDownloadInfo={resolveDownloadInfo}
              resolveDownloadInfoError={resolveDownloadInfoError}
              modelHostLabel={modelHostLabel}
              onDownloadModel={handleDownloadModel}
              onDeleteModel={handleDeleteModel}
            />
          )}
          onCreate={async (f) => {
            const apiKey = f.api_key.trim();
            await createEmbeddingConfig({
              name: f.name,
              backend: f.backend,
              base_url: f.backend === "api" ? f.api_base_url : null,
              api_key_encrypted: f.backend === "api" ? apiKey || null : null,
              model: f.model,
              dimension: f.dimension,
              batch_size: f.batch_size,
              local_model_path: f.backend === "fastembed" ? f.local_model_path || null : null,
            });
          }}
          onUpdate={async (id, f) => {
            const apiKey = f.api_key.trim();
            await updateEmbeddingConfig(id, {
              name: f.name,
              backend: f.backend,
              base_url: f.backend === "api" ? f.api_base_url : null,
              api_key_encrypted:
                f.backend === "api" ? (f.clear_api_key ? null : apiKey || undefined) : null,
              model: f.model,
              dimension: f.dimension,
              batch_size: f.batch_size,
              local_model_path: f.backend === "fastembed" ? f.local_model_path || null : null,
            });
          }}
          onDelete={(id) => deleteEmbeddingConfig(id)}
          onActivate={(id) => setDefaultEmbeddingConfig(id)}
          onTest={async (id) => {
            try {
              await testEmbeddingConfig(id);
            } catch (err) {
              const message = extractErrorMessage(err, "连接测试失败");
              if (
                usesRemoteBackend &&
                message.includes("本地嵌入模型") &&
                message.includes("尚未下载")
              ) {
                throw new Error(
                  usesLocalDesktopEmbedding
                    ? `${message}。当前为远程后端 + 本机 Embedding 模式，请在此页面下载模型到本机后重试。`
                    : `${message}。当前为远程后端模式，请在服务器侧准备对应模型，或切换到服务器可用的 Embedding 后端。`,
                );
              }
              throw err;
            }
          }}
          onPreviewTest={previewTestEmbeddingConfigRequest}
        />
      </div>
    </div>
  );
};
