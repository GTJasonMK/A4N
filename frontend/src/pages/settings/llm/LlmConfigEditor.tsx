import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React from "react";
import { BookInput, BookTextarea } from "../../../components/ui/BookInput";
import { detectLlmConfigModels } from "../../../api/config";
import { FormField } from "../ConfigListPanel";
import { ClearSavedApiKeySwitch } from "../config-list/ClearSavedApiKeySwitch";
import { ModelDiscoveryField } from "../config-list/ModelDiscoveryField";
import type { LlmForm } from "./types";
import { selectLlmProvider } from "./llmConfigUtils";
import {
  AGENT_PROTOCOL_JSON_COMPAT,
  AGENT_PROTOCOL_NATIVE_TOOL_CALLING,
  type LlmProviderDescriptor,
} from "../../../api/generated/config";
import { buildModelDiscoveryField } from "../../../api/modelDiscovery";
import type { ModelConfigScope } from "../../../api/modelDiscovery";
import { useI18n } from "../../../i18n";
import { PricingPlanFields } from "../config-list/PricingPlanFields";

interface LlmConfigEditorProps {
  form: LlmForm;
  setForm: (form: LlmForm) => void;
  configId?: number | null;
  scope: ModelConfigScope;
  readonly?: boolean;
  templateLocked?: boolean;
  providers: LlmProviderDescriptor[];
}

export const LlmConfigEditor: React.FC<LlmConfigEditorProps> = ({
  form,
  setForm,
  configId = null,
  scope,
  readonly = false,
  templateLocked = false,
  providers,
}) => {
  const { t } = useI18n();
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
        <FormField label={t("settingsForm.providerPreset")} hint={t("settingsForm.providerPresetHint")}>
          <ThemedSelect
            value={form.provider}
            aria-label={t("settingsForm.providerPreset")}
            onChange={(selectedValue) => setForm(selectLlmProvider(form, selectedValue, providers))}
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
            placeholder={t("settingsForm.llmModelPlaceholder")}
            disabled={structureDisabled || !selectedProvider}
            inputDisabled={structureDisabled}
            discoveryKey={modelDiscovery.discoveryKey}
            detectModels={() => detectLlmConfigModels(modelDiscovery.input, scope)}
          />
        </FormField>
      </div>

      <FormField
        label={t("settingsForm.agentProtocol")}
        hint={t("settingsForm.agentProtocolHint")}
      >
        <ThemedSelect
          value={form.agent_protocol}
          aria-label={t("settingsForm.agentProtocol")}
          onChange={(selectedValue) =>
            setForm({ ...form, agent_protocol: selectedValue })
          }
          className="pyqt-input w-full"
          disabled={structureDisabled}
        >
          <option value={AGENT_PROTOCOL_NATIVE_TOOL_CALLING}>
            {t("settingsForm.agentProtocolNative")}
          </option>
          <option value={AGENT_PROTOCOL_JSON_COMPAT}>
            {t("settingsForm.agentProtocolJsonCompat")}
          </option>
        </ThemedSelect>
      </FormField>

      <FormField label={t("settingsForm.endpoint")} required
        hint={selectedProvider ? t("settingsForm.protocolHint", { protocol: selectedProvider.protocol }) : undefined}>
        <BookInput
          value={form.api_base_url}
          onChange={(event) =>
            setForm({ ...form, api_base_url: event.target.value })
          }
          placeholder={
            selectedProvider?.default_base_url ?? "https://api.example.com/v1"
          }
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
          placeholder="sk-..."
          autoComplete="off"
          disabled={readonly}
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

      <div className="a4n-config-form-grid">
        <FormField
          label={t("settingsForm.concurrencyLimit")}
          hint={t("settingsForm.concurrencyHint")}
        >
          <BookInput
            type="number"
            min={1}
            max={32}
            step={1}
            value={form.max_concurrent}
            onChange={(event) =>
              setForm({ ...form, max_concurrent: event.target.value })
            }
            disabled={structureDisabled}
          />
        </FormField>

        <FormField
          label={t("settingsForm.timeoutSeconds")}
          hint={t("settingsForm.timeoutHint")}
        >
          <BookInput
            type="number"
            min={1}
            max={1800}
            step={1}
            value={form.timeout_secs}
            onChange={(event) =>
              setForm({ ...form, timeout_secs: event.target.value })
            }
            disabled={structureDisabled}
          />
        </FormField>
      </div>

      <div className="a4n-config-form-grid">
        <FormField
          label={t("settingsForm.contextWindow")}
          hint={t("settingsForm.contextWindowHint")}
        >
          <BookInput
            type="number"
            min={1024}
            max={1048576}
            step={1024}
            value={form.max_context_tokens}
            onChange={(event) =>
              setForm({
                ...form,
                max_context_tokens: event.target.value,
              })
            }
            disabled={structureDisabled}
          />
        </FormField>
        <FormField
          label={t("settingsForm.maxOutput")}
          hint={t("settingsForm.maxOutputHint")}
        >
          <BookInput
            type="number"
            min={1}
            max={Math.max(1, Number(form.max_context_tokens) - 1)}
            step={256}
            value={form.max_output_tokens}
            onChange={(event) =>
              setForm({
                ...form,
                max_output_tokens: event.target.value,
              })
            }
            disabled={structureDisabled}
          />
        </FormField>
      </div>

      <details className="border-y border-book-border/35 py-2" open={Boolean(form.extra_body_text.trim())}>
        <summary className="cursor-pointer text-sm font-semibold text-book-text-main">{t("settingsForm.advancedParams")}</summary>
        <p className="my-2 text-small text-book-text-muted">{t("settingsForm.advancedParamsHint")}</p>
        <BookTextarea
          aria-label={t("settingsForm.advancedParams")}
          rows={6}
          value={form.extra_body_text}
          onChange={(event) =>
            setForm({ ...form, extra_body_text: event.target.value })
          }
          placeholder={t("settingsForm.advancedParamsPlaceholder")}
          spellCheck={false}
          disabled={structureDisabled}
        />
      </details>
      <PricingPlanFields
        kind="llm"
        value={form.pricing}
        onChange={(pricing) => setForm({ ...form, pricing })}
        readonly={structureDisabled}
      />
    </>
  );
};
