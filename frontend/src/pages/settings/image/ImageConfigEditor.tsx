import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React from "react";
import { Plus, Trash2 } from "lucide-react";
import { DEFAULT_IMAGE_SIZES, detectImageConfigModels } from "../../../api/image";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { FormField } from "../ConfigListPanel";
import { ClearSavedApiKeySwitch } from "../config-list/ClearSavedApiKeySwitch";
import { ModelDiscoveryField } from "../config-list/ModelDiscoveryField";
import type { ImageProviderDescriptor } from "../../../api/generated/config";
import { buildModelDiscoveryField } from "../../../api/modelDiscovery";
import type { ModelConfigScope } from "../../../api/modelDiscovery";
import type { ImageForm } from "./types";
import { useI18n } from "../../../i18n";
import { PricingPlanFields } from "../config-list/PricingPlanFields";

interface ImageConfigEditorProps {
  form: ImageForm;
  setForm: (form: ImageForm) => void;
  configId?: number | null;
  scope: ModelConfigScope;
  readonly?: boolean;
  templateLocked?: boolean;
  providers: ImageProviderDescriptor[];
}

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

      <div className="a4n-config-form-grid">
        <FormField label={t("settingsForm.imageProvider")}>
          <ThemedSelect
            value={form.provider}
            onChange={(selectedValue) =>
              setForm({ ...form, provider: selectedValue })
            }
            className="pyqt-input w-full"
            disabled={structureDisabled}
          >
            {providers.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.display_name}
              </option>
            ))}
          </ThemedSelect>
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

      <fieldset className="min-w-0">
        <legend className="mb-2 text-xs font-semibold text-book-text-sub">{t("settingsForm.supportedSizes")}</legend>
          <div className="space-y-2">
            {form.supported_sizes.map((size, index) => (
              <div
                key={index}
                className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] items-center gap-2"
              >
                <BookInput
                  label={t("settingsForm.width")}
                  type="number"
                  min={1}
                  value={size.width}
                  aria-label={`${t("settingsForm.width")} ${index + 1}`}
                  onChange={(event) =>
                    setForm({
                      ...form,
                      supported_sizes: form.supported_sizes.map((item, itemIndex) =>
                        itemIndex === index
                          ? { ...item, width: event.target.value }
                          : item,
                      ),
                    })
                  }
                  placeholder={t("settingsForm.width")}
                  disabled={structureDisabled}
                />
                <BookInput
                  label={t("settingsForm.height")}
                  type="number"
                  min={1}
                  value={size.height}
                  aria-label={`${t("settingsForm.height")} ${index + 1}`}
                  onChange={(event) =>
                    setForm({
                      ...form,
                      supported_sizes: form.supported_sizes.map((item, itemIndex) =>
                        itemIndex === index
                          ? { ...item, height: event.target.value }
                          : item,
                      ),
                    })
                  }
                  placeholder={t("settingsForm.height")}
                  disabled={structureDisabled}
                />
                <BookIconButton
                  label={t("settingsForm.removeSize")}
                  variant="ghost"
                  size="sm"
                  disabled={structureDisabled || form.supported_sizes.length <= 1}
                  onClick={() =>
                    setForm({
                      ...form,
                      supported_sizes: form.supported_sizes.filter(
                        (_, itemIndex) => itemIndex !== index,
                      ),
                    })
                  }
                  className="!h-9 !w-9 self-end"
                >
                  <Trash2 size={13} />
                </BookIconButton>
              </div>
            ))}
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() =>
                setForm({
                  ...form,
                  supported_sizes: [
                    ...form.supported_sizes,
                    { width: String(DEFAULT_IMAGE_SIZES[0].width), height: String(DEFAULT_IMAGE_SIZES[0].height) },
                  ],
                })
              }
              disabled={structureDisabled}
            >
              <Plus size={13} />
              {t("settingsForm.addSize")}
            </BookButton>
          </div>
        <p className="mt-2 text-small text-book-text-muted">{t("settingsForm.supportedSizesHint")}</p>
      </fieldset>
        <FormField
          label={t("settingsForm.concurrencyLimit")}
          hint={t("settingsForm.imageConcurrencyHint")}
        >
          <BookInput
            type="number"
            min={1}
            max={8}
            step={1}
            value={form.max_concurrent}
            onChange={(event) =>
              setForm({ ...form, max_concurrent: event.target.value })
            }
            disabled={structureDisabled}
          />
        </FormField>
      <PricingPlanFields
        kind="image"
        value={form.pricing}
        onChange={(pricing) => setForm({ ...form, pricing })}
        readonly={structureDisabled}
      />
    </>
  );
};
