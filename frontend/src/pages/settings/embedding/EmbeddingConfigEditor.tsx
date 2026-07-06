import React from "react";
import type {
  EmbeddingModelDownloadInfo,
  LocalEmbeddingModel,
} from "../../../api/config";
import { BookInput } from "../../../components/ui/BookInput";
import { FormField } from "../ConfigListPanel";
import { FastembedModelPanel } from "./FastembedModelPanel";
import {
  BACKEND_LABELS,
  DEFAULT_FASTEMBED_DIMENSION,
  DEFAULT_FASTEMBED_MODEL,
  formatModelOptionLabel,
  formatSelectedModelHint,
} from "./embeddingTabUtils";
import type { EmbeddingForm } from "./types";

interface EmbeddingConfigEditorProps {
  form: EmbeddingForm;
  backends: string[];
  localModels: LocalEmbeddingModel[];
  busyModel: string | null;
  setForm: (form: EmbeddingForm) => void;
  pickFastembedModel: () => LocalEmbeddingModel | null;
  resolveLocalModel: (modelName: string) => LocalEmbeddingModel | null;
  resolveDownloadInfo: (modelName: string) => EmbeddingModelDownloadInfo | null;
  resolveDownloadInfoError: (modelName: string) => string | null;
  modelHostLabel: string;
  onDownloadModel: (modelName: string) => void | Promise<void>;
  onDeleteModel: (modelName: string) => void | Promise<void>;
}

export const EmbeddingConfigEditor: React.FC<EmbeddingConfigEditorProps> = ({
  form,
  backends,
  localModels,
  busyModel,
  setForm,
  pickFastembedModel,
  resolveLocalModel,
  resolveDownloadInfo,
  resolveDownloadInfoError,
  modelHostLabel,
  onDownloadModel,
  onDeleteModel,
}) => {
  const apiKeyInputDisabled = form.clear_api_key;

  return (
    <>
    <FormField label="配置名称" required>
      <BookInput
        value={form.name}
        onChange={(e) => setForm({ ...form, name: e.target.value })}
        placeholder="例如：默认 / 中文优化"
      />
    </FormField>

    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <FormField label="后端">
        <select
          value={form.backend}
          onChange={(e) => {
            const backend = e.target.value;
            const fastembedModel = pickFastembedModel();
            setForm({
              ...form,
              backend,
              model:
                backend === "local_hash"
                  ? "local-sha256-hash"
                  : backend === "fastembed"
                    ? fastembedModel?.name ?? DEFAULT_FASTEMBED_MODEL
                    : form.model || "text-embedding-3-small",
              api_base_url: backend === "api" ? form.api_base_url : "",
              api_key: backend === "api" ? form.api_key : "",
              clear_api_key: backend === "api" ? form.clear_api_key : false,
              dimension:
                backend === "local_hash"
                  ? 384
                  : backend === "fastembed"
                    ? fastembedModel?.dimension ?? DEFAULT_FASTEMBED_DIMENSION
                    : form.dimension,
              batch_size:
                backend === "fastembed" && form.batch_size === 100 ? 32 : form.batch_size,
            });
          }}
          className="pyqt-input w-full"
        >
          {backends.map((backend) => (
            <option key={backend} value={backend}>
              {BACKEND_LABELS[backend] || backend}
            </option>
          ))}
        </select>
      </FormField>
      {form.backend === "fastembed" ? (
        <FormField
          label="模型"
          required
          hint={formatSelectedModelHint(
            resolveLocalModel(form.model),
            resolveDownloadInfo(form.model),
            resolveDownloadInfoError(form.model),
          )}
        >
          <select
            value={form.model}
            onChange={(e) => {
              const modelName = e.target.value;
              const localModel = resolveLocalModel(modelName);
              setForm({
                ...form,
                model: modelName,
                dimension: localModel?.dimension ?? form.dimension,
              });
            }}
            className="pyqt-input w-full"
          >
            {localModels.length === 0 ? (
              <option value={DEFAULT_FASTEMBED_MODEL}>{DEFAULT_FASTEMBED_MODEL}</option>
            ) : (
              localModels.map((model) => (
                <option key={model.name} value={model.name}>
                  {formatModelOptionLabel(
                    model,
                    resolveDownloadInfo(model.name),
                    resolveDownloadInfoError(model.name),
                  )}
                </option>
              ))
            )}
          </select>
        </FormField>
      ) : (
        <FormField label="模型" required>
          <BookInput
            value={form.model}
            onChange={(e) => setForm({ ...form, model: e.target.value })}
            placeholder={form.backend === "api" ? "text-embedding-3-small" : "local-sha256-hash"}
          />
        </FormField>
      )}
    </div>

    {form.backend === "fastembed" && (
      <FastembedModelPanel
        model={resolveLocalModel(form.model)}
        downloadInfo={resolveDownloadInfo(form.model)}
        downloadInfoError={resolveDownloadInfoError(form.model)}
        modelName={form.model}
        busy={busyModel === form.model}
        modelHostLabel={modelHostLabel}
        onDownload={() => void onDownloadModel(form.model)}
        onDelete={() => void onDeleteModel(form.model)}
      />
    )}

    {form.backend === "api" && (
      <FormField label="API Base URL">
        <BookInput
          value={form.api_base_url}
          onChange={(e) => setForm({ ...form, api_base_url: e.target.value })}
          placeholder="https://api.openai.com/v1"
        />
      </FormField>
    )}

    {form.backend === "api" && (
      <FormField label="API Key">
        <BookInput
          type="password"
          value={form.api_key}
          onChange={(e) =>
            setForm({ ...form, api_key: e.target.value, clear_api_key: false })
          }
          autoComplete="off"
          disabled={apiKeyInputDisabled}
        />
        {form.has_api_key ? (
          <label className="mt-2 flex items-center gap-2 text-xs text-book-text-muted">
            <input
              type="checkbox"
              checked={form.clear_api_key}
              onChange={(event) =>
                setForm({
                  ...form,
                  api_key: event.target.checked ? "" : form.api_key,
                  clear_api_key: event.target.checked,
                })
              }
            />
            清空已保存 API Key
          </label>
        ) : null}
      </FormField>
    )}

    <FormField label="向量维度" required hint="必须与模型输出维度一致">
      <BookInput
        type="number"
        min={32}
        max={8192}
        value={String(form.dimension)}
        disabled={form.backend === "fastembed"}
        onChange={(e) => setForm({ ...form, dimension: Number(e.target.value) })}
      />
    </FormField>
    <FormField label="批量大小" required>
      <BookInput
        type="number"
        min={1}
        max={2048}
        value={String(form.batch_size)}
        onChange={(e) => setForm({ ...form, batch_size: Number(e.target.value) })}
      />
    </FormField>
    </>
  );
};
