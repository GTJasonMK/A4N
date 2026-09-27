import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { BookButton } from "../../components/ui/BookButton";
import {
  createWorkflowTaskId,
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
  listLocalEmbeddingModels,
  getEmbeddingModelDownloadInfo,
  downloadEmbeddingModel,
  cancelEmbeddingModelDownload,
  deleteLocalEmbeddingModel,
  type EmbeddingModelDownloadInfo,
  type LocalEmbeddingModel,
} from "../../api/config";
import { ConfigListPanel } from "./ConfigListPanel";
import { useConfigItems } from "./config-list/useConfigItems";
import { EmbeddingConfigEditor } from "./embedding/EmbeddingConfigEditor";
import {
  DEFAULT_FASTEMBED_DIMENSION,
  DEFAULT_FASTEMBED_MODEL,
  DEFAULT_FORM,
  formatBytesProgress,
  formatDownloadStartMessage,
  preferredFastembedModel,
  toItem,
} from "./embedding/embeddingTabUtils";
import type { EmbeddingForm } from "./embedding/types";
import {
  assertCatalogProvider,
  listModelProviderCatalog,
} from "../../api/providerCatalog";
import type { ModelConfigScope } from "../../api/modelDiscovery";
import { useI18n } from "../../i18n";

/**
 * Embedding 配置 Tab
 *
 * 当前真实接入的后端：
 * - api：OpenAI 兼容 `/embeddings` 接口
 * - fastembed：显式下载后的本地 ONNX 嵌入模型
 * - local_hash：本地确定性哈希向量，用于离线开发和基础检索验证
 */

interface EmbeddingTabProps {
  scope?: ModelConfigScope;
}

export const EmbeddingTab: React.FC<EmbeddingTabProps> = ({
  scope = "local",
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const usesRemoteBackend = scope === "server" || shouldUseHttpBackend();
  const usesLocalDesktopEmbedding =
    scope === "local" && canUseLocalDesktopEmbedding();
  const modelHostLabel = usesRemoteBackend && !usesLocalDesktopEmbedding
    ? t("settingsEmbedding.remoteServer")
    : t("settingsEmbedding.thisDevice");
  const { runProgressTask } = useProgressTask();
  const [localModels, setLocalModels] = useState<LocalEmbeddingModel[]>([]);
  const [modelLoadError, setModelLoadError] = useState<string | null>(null);
  const [busyModel, setBusyModel] = useState<string | null>(null);
  const [downloadInfoByModel, setDownloadInfoByModel] = useState<
    Record<string, EmbeddingModelDownloadInfo>
  >({});
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

  const loadConfigs = useCallback(async () => {
    const [listResult, catalogResult] = await Promise.allSettled([
      listEmbeddingConfig(scope),
      listModelProviderCatalog(),
    ]);
    if (listResult.status === "rejected") throw listResult.reason;
    const list = listResult.value;
    const catalog = catalogResult.status === "fulfilled" ? catalogResult.value : null;
    for (const config of list) {
      if (catalog) assertCatalogProvider("embedding", config.backend, catalog);
    }
    return {
      items: list.map(toItem),
      catalog,
      warning: catalogResult.status === "rejected"
        ? extractErrorMessage(catalogResult.reason, t("settings.loadConfigFailed", { name: t("settings.embedding") }))
        : null,
    };
  }, [scope, t]);
  const { items, setItems, catalog, loading, error: loadError, refresh: refreshConfigs } = useConfigItems(
    loadConfigs,
    t("settingsEmbedding.configLoadFailed", { host: modelHostLabel }),
  );

  const refreshModels = useCallback(async (): Promise<boolean> => {
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    setModelLoadError(null);
    try {
      const models = await listLocalEmbeddingModels(scope);
      if (!isCurrentRefresh(requestSeq)) return false;
      setLocalModels(models);
      return true;
    } catch (error) {
      if (!isCurrentRefresh(requestSeq)) return false;
      setModelLoadError(
        extractErrorMessage(error, t("settingsEmbedding.directoryReadFailed", { host: modelHostLabel })),
      );
      return false;
    }
  }, [isCurrentRefresh, modelHostLabel, scope, t]);

  useEffect(() => {
    void refreshModels();
  }, [refreshModels]);

  const refresh = useCallback(async () => {
    await Promise.all([refreshConfigs(), refreshModels()]);
  }, [refreshConfigs, refreshModels]);

  const pickFastembedModel = () => preferredFastembedModel(localModels);

  const selectableProviders = useMemo(
    () =>
      (catalog?.embedding ?? []).filter((provider) =>
        !usesLocalDesktopEmbedding || provider.id === "fastembed",
      ),
    [catalog, usesLocalDesktopEmbedding],
  );
  const embeddingProviderLabels = useMemo(
    () => new Map((catalog?.embedding ?? []).map((provider) => [provider.id, provider.display_name])),
    [catalog],
  );

  const defaultEmbeddingForm = useMemo((): EmbeddingForm => {
    if (!usesLocalDesktopEmbedding) {
      return { ...DEFAULT_FORM, name: t("settingsEmbedding.localHash") };
    }
    const model = preferredFastembedModel(localModels);
    return {
      ...DEFAULT_FORM,
      name: t("settingsEmbedding.localFastembed"),
      backend: "fastembed",
      model: model?.name ?? DEFAULT_FASTEMBED_MODEL,
      dimension: String(model?.dimension ?? DEFAULT_FASTEMBED_DIMENSION),
      batch_size: "32",
      api_base_url: "",
      api_key: "",
      clear_api_key: false,
      local_model_path: "",
    };
  }, [localModels, t, usesLocalDesktopEmbedding]);

  const resolveLocalModel = (modelName: string): LocalEmbeddingModel | null =>
    localModelByName.get(modelName.trim().toLowerCase()) ?? null;

  const resolveDownloadInfo = (modelName: string): EmbeddingModelDownloadInfo | null =>
    downloadInfoByModel[modelName.trim().toLowerCase()] ?? null;

  const rememberDownloadInfo = (info: EmbeddingModelDownloadInfo) => {
    setDownloadInfoByModel((prev) => ({
      ...prev,
      [info.model_name.toLowerCase()]: info,
      [info.model_code.toLowerCase()]: info,
    }));
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
      addToast(t("settingsEmbedding.chooseFastembed"), "error");
      return;
    }

    const actionKey = modelActionKey(model);
    if (!beginModelAction(actionKey)) return;
    setBusyModel(model);
    const cachedInfo = resolveDownloadInfo(model);
    const taskId = createWorkflowTaskId("embedding-download", 0);
    try {
      await runProgressTask(
        {
          cancelTaskId: taskId,
          title: t("settingsEmbedding.downloadTitle", { host: modelHostLabel }),
          startMessage: cachedInfo
            ? formatDownloadStartMessage(cachedInfo, locale, t)
            : t("settingsEmbedding.readingModelSize"),
          successMessage: t("settingsEmbedding.downloadSuccess"),
          errorMessage: t("settingsEmbedding.downloadFailed"),
          total: cachedInfo ? cachedInfo.total_size_bytes : 1,
          cancelLabel: t("settingsEmbedding.stopDownload"),
          runningHint: t("settingsEmbedding.keepWindowOpen"),
          formatProgress: (current, total, percent) =>
            formatBytesProgress(current, total, percent, locale, t),
          cancelTask: (taskId) => cancelEmbeddingModelDownload(taskId, scope),
        },
        async (control) => {
          if (cachedInfo) {
            control.setProgress(
              formatDownloadStartMessage(cachedInfo, locale, t),
              cachedInfo.downloaded_size_bytes,
              cachedInfo.total_size_bytes,
            );
          }
          await runTaskWithPolling(
            downloadEmbeddingModel(model, taskId, scope),
            () => getEmbeddingModelDownloadInfo(model, scope),
            (info) => {
              if (!mountedRef.current || modelActionRef.current !== actionKey) return;
              rememberDownloadInfo(info);
              control.setProgress(
                t("settingsEmbedding.downloading", { host: modelHostLabel }),
                info.downloaded_size_bytes,
                info.total_size_bytes,
              );
            },
          );
        },
      );
      if (!await refreshModels()) return;
      addToast(t("settingsEmbedding.downloadedTo", { host: modelHostLabel }), "success");
    } catch (err) {
      const message = extractErrorMessage(err, t("settingsEmbedding.downloadFailed"));
      if (isCancelledOperation(message)) {
        addToast(t("settingsEmbedding.downloadStopped"), "info");
      } else {
        addToast(message, "error");
      }
    } finally {
      if (modelActionRef.current === actionKey && mountedRef.current) {
        setDownloadInfoByModel({});
        await refreshModels();
        if (mountedRef.current) setBusyModel(null);
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
        title: t("settingsEmbedding.deleteTitle", { host: modelHostLabel }),
        message: t("settingsEmbedding.deleteMessage", {
          model,
          host: modelHostLabel,
        }),
        confirmText: t("common.delete"),
        dialogType: "danger",
      });
      if (!ok) return;

      setBusyModel(model);
      await deleteLocalEmbeddingModel(model, scope);
      setDownloadInfoByModel({});
      if (!await refreshModels()) return;
      addToast(t("settingsEmbedding.cacheDeleted", { host: modelHostLabel }), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("settingsEmbedding.deleteFailed")), "error");
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
            ? t("settingsEmbedding.remoteEmbeddingLocalDevice")
            : t("settingsEmbedding.remoteEmbeddingOnly")}
        </div>
      )}
      {modelLoadError ? (
        <div
          role="alert"
          className="flex flex-wrap items-center gap-2 rounded-xl border border-error/30 bg-error-bg px-3 py-2.5"
        >
          <p className="min-w-0 flex-1 text-small leading-relaxed text-book-text-main">
            {modelLoadError}
          </p>
          <BookButton
            variant="secondary"
            size="sm"
            onClick={() => void refresh()}
            disabled={loading}
          >
            <RefreshCw size={13} aria-hidden="true" />
            {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      <div className="flex-1 min-h-0">
        <ConfigListPanel<EmbeddingForm>
          title={t("settings.embeddingConfig")}
          items={items}
          onItemsChange={setItems}
          loading={loading}
          loadError={loadError}
          onRefresh={refresh}
          defaultForm={defaultEmbeddingForm}
          validate={(f) => {
            if (usesLocalDesktopEmbedding && f.backend !== "fastembed") {
              return t("settingsEmbedding.remoteFastembedOnly");
            }
            if (!f.name.trim()) return t("settingsForm.nameRequired");
            if (!f.model.trim()) return t("settingsForm.modelRequired");
            if (!Number.isInteger(Number(f.dimension)) || Number(f.dimension) <= 0) {
              return t("settingsForm.dimensionPositive");
            }
            if (!Number.isInteger(Number(f.batch_size)) || Number(f.batch_size) <= 0) {
              return t("settingsForm.batchPositive");
            }
            if (f.backend === "api" && !f.api_base_url.trim()) {
              return t("settingsForm.baseUrlRequired");
            }
            if (f.backend === "fastembed") {
              const model = resolveLocalModel(f.model);
              if (!model) return t("settingsForm.selectFastembedModel");
              if (Number(f.dimension) !== model.dimension) {
                return t("settingsEmbedding.modelDimensionMismatch", {
                  count: model.dimension,
                });
              }
              if (!model.downloaded) return t("settingsForm.fastembedNotDownloaded");
            }
            return null;
          }}
          formatItem={(c) => ({
            primary: c.name,
            secondary: `${embeddingProviderLabels.get(c.backend) ?? c.backend} · ${c.model}`,
            tertiary: `${t("settingsEmbedding.dimensionUnit", { count: c.dimension })} · ${t("settingsForm.batchSize")} ${c.batch_size}`,
          })}
          renderEditor={(form, setForm, context) => (
            <EmbeddingConfigEditor
              form={form}
              providers={selectableProviders}
              localModels={localModels}
              busyModel={busyModel}
              readonly={context.readonly}
              setForm={setForm}
              pickFastembedModel={pickFastembedModel}
              resolveLocalModel={resolveLocalModel}
              resolveDownloadInfo={resolveDownloadInfo}
              modelHostLabel={modelHostLabel}
              onDownloadModel={handleDownloadModel}
              onDeleteModel={handleDeleteModel}
            />
          )}
          onCreate={async (f) => {
            const apiKey = f.api_key.trim();
            const created = await createEmbeddingConfig(
              {
                name: f.name,
                backend: f.backend,
                base_url: f.backend === "api" ? f.api_base_url : null,
                api_key_encrypted: f.backend === "api" ? apiKey || null : null,
                model: f.model,
                dimension: Number(f.dimension),
                batch_size: Number(f.batch_size),
                local_model_path:
                  f.backend === "fastembed" ? f.local_model_path || null : null,
                pricing: f.pricing,
              },
              scope,
            );
            return toItem(created);
          }}
          onUpdate={async (id, f) => {
            const apiKey = f.api_key.trim();
            return toItem(await updateEmbeddingConfig(
              id,
              {
                name: f.name,
                backend: f.backend,
                base_url: f.backend === "api" ? f.api_base_url : null,
                api_key_encrypted:
                  f.backend === "api"
                    ? f.clear_api_key
                      ? null
                      : apiKey || undefined
                    : null,
                model: f.model,
                dimension: Number(f.dimension),
                batch_size: Number(f.batch_size),
                local_model_path:
                  f.backend === "fastembed" ? f.local_model_path || null : null,
                pricing: f.pricing,
              },
              scope,
            ));
          }}
          onDelete={(id) => deleteEmbeddingConfig(id, scope)}
          onActivate={(id) => setDefaultEmbeddingConfig(id, scope)}
          onTest={async (id) => {
            try {
              await testEmbeddingConfig(id, scope);
            } catch (err) {
              const message = extractErrorMessage(err, t("settings.connectionTestFailed"));
              if (
                usesRemoteBackend &&
                message.includes("本地嵌入模型") &&
                message.includes("尚未下载")
              ) {
                throw new Error(
                  usesLocalDesktopEmbedding
                    ? t("settingsEmbedding.remoteDownloadHintLocal", { message })
                    : t("settingsEmbedding.remoteDownloadHintServer", { message }),
                );
              }
              throw err;
            }
          }}
          onPreviewTest={(id) => previewTestEmbeddingConfigRequest(id, scope)}
        />
      </div>
    </div>
  );
};
