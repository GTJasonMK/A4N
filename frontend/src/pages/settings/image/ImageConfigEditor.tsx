import React from "react";
import { detectImageConfigModels } from "../../../api/image";
import { BookInput } from "../../../components/ui/BookInput";
import { FormField } from "../ConfigListPanel";
import { ClearSavedApiKeySwitch } from "../config-list/ClearSavedApiKeySwitch";
import { ModelDiscoveryField } from "../config-list/ModelDiscoveryField";
import { IMAGE_SIZE_OPTIONS } from "./imageConfigUtils";
import type { ImageProviderDescriptor } from "../../../api/generated/config";
import { buildModelDiscoveryField } from "../../../api/modelDiscovery";
import type { ModelConfigScope } from "../../../api/modelDiscovery";
import type { ImageForm } from "./types";
import { useI18n } from "../../../i18n";

interface ImageConfigEditorProps {
  form: ImageForm;
  setForm: (form: ImageForm) => void;
  configId?: number | null;
  scope: ModelConfigScope;
  readonly?: boolean;
  templateLocked?: boolean;
  providers: ImageProviderDescriptor[];
}

const IMAGE_SIZE_DATALIST_ID = "image-config-size-options";

export const ImageConfigEditor: React.FC<ImageConfigEditorProps> = ({
  form,
  setForm,
  configId = null,
  scope,
  readonly = false,
  templateLocked = false,
  providers,
}) => {
  const { t } = useI18n();
  const apiKeyInputDisabled = form.clear_api_key;
  const structureDisabled = readonly || templateLocked;
  const selectedProvider = providers.find(
    (provider) => provider.id === form.provider,
  );
  const modelDiscovery = buildModelDiscoveryField({
    provider: form.provider,
    baseUrl: form.api_base_url,
    apiKey: form.api_key,
    clearApiKey: form.clear_api_key,
    configId,
    hasApiKey: form.has_api_key,
    readonly,
    scope,
  });

  return (
    <>
      <FormField label={t("settingsForm.configName")} required>
        <BookInput
          value={form.name}
          onChange={(event) => setForm({ ...form, name: event.target.value })}
          placeholder={t("settingsForm.configNamePlaceholder")}
          disabled={structureDisabled}
        />
      </FormField>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <FormField label={t("settingsForm.imageProvider")}>
          <select
            value={form.provider}
            onChange={(event) =>
              setForm({ ...form, provider: event.target.value })
            }
            className="pyqt-input w-full"
            disabled={structureDisabled}
          >
            {providers.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.display_name}
              </option>
            ))}
          </select>
        </FormField>
        <FormField label={t("settingsForm.model")} required>
          <ModelDiscoveryField
            value={form.model}
            onChange={(model) => setForm({ ...form, model })}
            placeholder={t("settingsForm.imageModelPlaceholder")}
            disabled={readonly || !selectedProvider}
            inputDisabled={structureDisabled}
            discoveryKey={modelDiscovery.discoveryKey}
            detectModels={() => detectImageConfigModels(modelDiscovery.input, scope)}
          />
        </FormField>
      </div>

      <FormField label={t("settingsForm.endpoint")} required>
        <BookInput
          value={form.api_base_url}
          onChange={(event) =>
            setForm({ ...form, api_base_url: event.target.value })
          }
          placeholder={t("settingsForm.endpointPlaceholder")}
          disabled={structureDisabled}
        />
      </FormField>

      <FormField
        label={t("settingsForm.apiKey")}
        hint={
          templateLocked
            ? t("settingsForm.personalApiKeyHint")
            : t("settingsForm.localApiKeyHint")
        }
      >
        <BookInput
          type="password"
          value={form.api_key}
          onChange={(event) =>
            setForm({
              ...form,
              api_key: event.target.value,
              clear_api_key: false,
            })
          }
          autoComplete="off"
          disabled={apiKeyInputDisabled || readonly}
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

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <FormField
          label={t("settingsForm.size")}
          hint={t("settingsForm.sizeHint")}
        >
          <BookInput
            value={form.size}
            list={IMAGE_SIZE_DATALIST_ID}
            onChange={(event) => setForm({ ...form, size: event.target.value })}
            placeholder={t("settingsForm.sizePlaceholder")}
            disabled={structureDisabled}
          />
          <datalist id={IMAGE_SIZE_DATALIST_ID}>
            {IMAGE_SIZE_OPTIONS.map((size) => (
              <option key={size} value={size}>
                {size}
              </option>
            ))}
          </datalist>
        </FormField>
        <FormField
          label={t("settingsForm.concurrencyLimit")}
          hint={t("settingsForm.imageConcurrencyHint")}
        >
          <BookInput
            type="number"
            min={1}
            max={8}
            step={1}
            value={String(form.max_concurrent)}
            onChange={(event) =>
              setForm({ ...form, max_concurrent: Number(event.target.value) })
            }
            disabled={structureDisabled}
          />
        </FormField>
      </div>
    </>
  );
};
