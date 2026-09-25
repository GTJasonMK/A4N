import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React from "react";
import type {
  EmbeddingModelDownloadInfo,
  LocalEmbeddingModel,
} from "../../../api/config";
import { BookInput } from "../../../components/ui/BookInput";
import { FormField } from "../ConfigListPanel";
import { ClearSavedApiKeySwitch } from "../config-list/ClearSavedApiKeySwitch";
import { FastembedModelPanel } from "./FastembedModelPanel";
import {
  DEFAULT_FASTEMBED_DIMENSION,
  DEFAULT_FASTEMBED_MODEL,
  formatModelOptionLabel,
  formatSelectedModelHint,
} from "./embeddingTabUtils";
import type { EmbeddingForm } from "./types";
import type { EmbeddingProviderDescriptor } from "../../../api/generated/config";
import { useI18n } from "../../../i18n";
import { PricingPlanFields } from "../config-list/PricingPlanFields";

interface EmbeddingConfigEditorProps {
  form: EmbeddingForm;
  providers: EmbeddingProviderDescriptor[];
  localModels: LocalEmbeddingModel[];
  busyModel: string | null;
  readonly?: boolean;
  setForm: (form: EmbeddingForm) => void;
  pickFastembedModel: () => LocalEmbeddingModel | null;
  resolveLocalModel: (modelName: string) => LocalEmbeddingModel | null;
  resolveDownloadInfo: (modelName: string) => EmbeddingModelDownloadInfo | null;
  modelHostLabel: string;
  onDownloadModel: (modelName: string) => void | Promise<void>;
  onDeleteModel: (modelName: string) => void | Promise<void>;
}

export const EmbeddingConfigEditor: React.FC<EmbeddingConfigEditorProps> = ({
  form,
  providers,
  localModels,
  busyModel,
  readonly = false,
  setForm,
  pickFastembedModel,
  resolveLocalModel,
  resolveDownloadInfo,
  modelHostLabel,
  onDownloadModel,
  onDeleteModel,
}) => {
  const { locale, t } = useI18n();
  const apiKeyInputDisabled = form.clear_api_key || readonly;

  return (
    <>
      <FormField label={t("settingsForm.configName")} required>
        <BookInput
          value={form.name}
          onChange={(e) => setForm({ ...form, name: e.target.value })}
          placeholder={t("settingsForm.configNamePlaceholder")}
          disabled={readonly}
        />
      </FormField>

      <div className="a4n-config-form-grid">
        <FormField label={t("settingsForm.backend")}>
          <ThemedSelect
            value={form.backend}
            onChange={(backend) => {
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
                    ? "384"
                    : backend === "fastembed"
                      ? String(fastembedModel?.dimension ?? DEFAULT_FASTEMBED_DIMENSION)
                      : form.dimension,
                batch_size:
                  backend === "fastembed" && form.batch_size === "100" ? "32" : form.batch_size,
              });
            }}
            className="pyqt-input w-full"
            disabled={readonly}
          >
            {providers.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.display_name}
              </option>
            ))}
          </ThemedSelect>
        </FormField>
        {form.backend === "fastembed" ? (
          <FormField
            label={t("settingsForm.model")}
            required
            hint={formatSelectedModelHint(
              resolveLocalModel(form.model),
              resolveDownloadInfo(form.model),
              locale,
              t,
            )}
          >
            <ThemedSelect
              value={form.model}
              onChange={(modelName) => {
                const localModel = resolveLocalModel(modelName);
                setForm({
                  ...form,
                  model: modelName,
                  dimension: localModel ? String(localModel.dimension) : form.dimension,
                });
              }}
              className="pyqt-input w-full"
              disabled={readonly}
            >
              {localModels.length === 0 ? (
                <option value={form.model}>{form.model}</option>
              ) : (
                localModels.map((model) => (
                  <option key={model.name} value={model.name}>
                    {formatModelOptionLabel(model, locale, t)}
                  </option>
                ))
              )}
            </ThemedSelect>
          </FormField>
        ) : (
          <FormField label={t("settingsForm.model")} required>
            <BookInput
              value={form.model}
              onChange={(e) => setForm({ ...form, model: e.target.value })}
              placeholder={
                form.backend === "api"
                  ? "text-embedding-3-small"
                  : "local-sha256-hash"
              }
              disabled={readonly}
            />
          </FormField>
        )}
      </div>

      {form.backend === "fastembed" && (
        <FastembedModelPanel
          model={resolveLocalModel(form.model)}
          downloadInfo={resolveDownloadInfo(form.model)}
          modelName={form.model}
          busy={busyModel === form.model}
          modelHostLabel={modelHostLabel}
          onDownload={() => void onDownloadModel(form.model)}
          onDelete={() => void onDeleteModel(form.model)}
        />
      )}

      {form.backend === "api" && (
        <FormField label={t("settingsForm.endpoint")}>
          <BookInput
            value={form.api_base_url}
            onChange={(e) => setForm({ ...form, api_base_url: e.target.value })}
            placeholder="https://api.openai.com/v1"
            disabled={readonly}
          />
        </FormField>
      )}

      {form.backend === "api" && (
        <>
          <FormField label={t("settingsForm.apiKey")}>
            <BookInput
              type="password"
              value={form.api_key}
              onChange={(e) =>
                setForm({ ...form, api_key: e.target.value, clear_api_key: false })
              }
              autoComplete="off"
              disabled={apiKeyInputDisabled}
            />
          </FormField>
          {form.has_api_key ? (
            <ClearSavedApiKeySwitch
              checked={form.clear_api_key}
              disabled={readonly}
              onChange={(checked) =>
                setForm({
                  ...form,
                  api_key: checked ? "" : form.api_key,
                  clear_api_key: checked,
                })
              }
            />
          ) : null}
        </>
      )}

      <div className="a4n-config-form-grid">
        <FormField
          label={t("settingsForm.dimension")}
          required
          hint={t("settingsForm.dimensionHint")}
        >
          <BookInput
            type="number"
            min={32}
            max={8192}
            value={form.dimension}
            disabled={readonly || form.backend === "fastembed"}
            onChange={(e) => setForm({ ...form, dimension: e.target.value })}
          />
        </FormField>
        <FormField label={t("settingsForm.batchSize")} required>
          <BookInput
            type="number"
            min={1}
            max={2048}
            value={form.batch_size}
            onChange={(e) => setForm({ ...form, batch_size: e.target.value })}
            disabled={readonly}
          />
        </FormField>
      </div>
      <PricingPlanFields
        kind="embedding"
        value={form.pricing}
        onChange={(pricing) => setForm({ ...form, pricing })}
        readonly={readonly}
      />
    </>
  );
};
