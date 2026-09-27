import React, { useCallback, useMemo } from "react";
import { extractErrorMessage } from "../../api";
import {
  createLlmConfig,
  createServerLlmConfig,
  deleteLlmConfig,
  deleteServerLlmConfig,
  loadLlmConfigList,
  listServerLlmConfig,
  previewTestLlmConfigRequest,
  setDefaultLlmConfig,
  setDefaultServerLlmConfig,
  testLlmConfig,
  updateLlmConfig,
  updateServerLlmConfig,
  updateSystemDefaultLlmKey,
  SYSTEM_LLM_CONFIG_ID,
} from "../../api/config";
import { ConfigListPanel, type ConfigItem } from "./ConfigListPanel";
import { useConfigItems } from "./config-list/useConfigItems";
import { LlmConfigEditor } from "./llm/LlmConfigEditor";
import {
  DEFAULT_LLM_FORM,
  formatLlmItem,
  toCreateLlmInput,
  toLlmItem,
  toUpdateLlmInput,
  validateLlmForm,
} from "./llm/llmConfigUtils";
import type { LlmForm } from "./llm/types";
import {
  assertCatalogProvider,
  listModelProviderCatalog,
} from "../../api/providerCatalog";
import type { ModelProviderCatalog } from "../../api/generated/config";
import type { ModelConfigScope } from "../../api/modelDiscovery";
import { useI18n } from "../../i18n";

interface LlmTabProps {
  scope?: ModelConfigScope;
}

export const LlmTab: React.FC<LlmTabProps> = ({ scope = "local" }) => {
  const { t } = useI18n();
  const serverScope = scope === "server";
  const loadItems = useCallback(async (): Promise<{
    items: ConfigItem<LlmForm>[];
    catalog: ModelProviderCatalog | null;
    warning?: string | null;
  }> => {
    const [listResult, catalogResult] = await Promise.allSettled([
      serverScope
        ? listServerLlmConfig().then((configs) => ({ configs, warning: undefined }))
        : loadLlmConfigList(),
      listModelProviderCatalog(),
    ]);
    if (listResult.status === "rejected") throw listResult.reason;
    const list = listResult.value.configs;
    const providerCatalog = catalogResult.status === "fulfilled" ? catalogResult.value : null;
    const warnings = listResult.value.warning === undefined
      ? []
      : [extractErrorMessage(listResult.value.warning, t("settings.loadConfigFailed", { name: t("settings.llm") }))];
    const items = list.map((config) => {
      if (providerCatalog) assertCatalogProvider("llm", config.provider, providerCatalog);
      return {
        ...toLlmItem(config),
        deleteDisabled: !serverScope && config.id === SYSTEM_LLM_CONFIG_ID,
        deleteDisabledReason: t("settings.systemTemplateCannotDelete", { name: "LLM" }),
      };
    });
    return {
      items,
      catalog: providerCatalog,
      warning: catalogResult.status === "rejected"
        ? [...warnings, extractErrorMessage(catalogResult.reason, t("settings.loadConfigFailed", { name: t("settings.llm") }))].join("; ")
        : warnings.join("; ") || null,
    };
  }, [serverScope, t]);
  const { items, setItems, catalog, loading, error, refresh } = useConfigItems(
    loadItems,
    t("settings.loadConfigFailed", { name: t("settings.llmConfig") }),
  );

  const defaultForm = useMemo(() => ({
    ...DEFAULT_LLM_FORM,
    api_base_url: catalog?.llm.find((provider) => provider.id === DEFAULT_LLM_FORM.provider)?.default_base_url ?? "",
  }), [catalog]);

  return (
    <div className="flex h-auto min-h-0 flex-col gap-3 md:h-full">
      <div className="min-h-0 md:flex-1">
        <ConfigListPanel<LlmForm>
          title={t("settings.llmConfig")}
          items={items}
          onItemsChange={setItems}
          loading={loading}
          loadError={error}
          onRefresh={refresh}
          defaultForm={defaultForm}
          validate={(form) => validateLlmForm(form, t)}
          activeLabel={serverScope ? t("settings.globalDefault") : t("settings.currentDefault")}
          activateText={serverScope ? t("settings.setGlobalDefault") : t("settings.setDefault")}
          activateSuccessMessage={serverScope ? t("settings.globalDefaultSuccess") : t("settings.setDefaultSuccess")}
          activateErrorMessage={serverScope ? t("settings.globalDefaultFailed") : t("settings.setDefaultFailed")}
          formatItem={formatLlmItem}
          renderEditor={(form, setForm, context) => (
            <LlmConfigEditor
              form={form}
              setForm={setForm}
              configId={context.selectedId}
              scope={scope}
              readonly={context.readonly}
              templateLocked={!serverScope && context.selectedId === SYSTEM_LLM_CONFIG_ID}
              providers={catalog?.llm ?? []}
            />
          )}
          onCreate={async (form) => {
            const created = await (serverScope ? createServerLlmConfig : createLlmConfig)(toCreateLlmInput(form, t));
            return toLlmItem(created);
          }}
          onUpdate={async (id, form) => {
            const input = toUpdateLlmInput(form, t);
            if (!serverScope && id === SYSTEM_LLM_CONFIG_ID) {
              return toLlmItem(await updateSystemDefaultLlmKey({ api_key_encrypted: input.api_key_encrypted }));
            }
            return toLlmItem(await (serverScope ? updateServerLlmConfig : updateLlmConfig)(
              id,
              input,
            ));
          }}
          onDelete={serverScope ? deleteServerLlmConfig : deleteLlmConfig}
          onActivate={serverScope ? setDefaultServerLlmConfig : setDefaultLlmConfig}
          onTest={async (id) => {
            await testLlmConfig(id, scope);
          }}
          onPreviewTest={(id) => previewTestLlmConfigRequest(id, scope)}
        />
      </div>
    </div>
  );
};
