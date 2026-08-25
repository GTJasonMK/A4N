import React, { useCallback } from "react";
import {
  createLlmConfig,
  createServerLlmConfig,
  deleteLlmConfig,
  deleteServerLlmConfig,
  getServerLlmMaxTokensConfig,
  listLlmConfig,
  listServerLlmConfig,
  previewTestLlmConfigRequest,
  setDefaultLlmConfig,
  setDefaultServerLlmConfig,
  testLlmConfig,
  updateLlmConfig,
  updateServerLlmConfig,
  updateServerLlmMaxTokensConfig,
  SYSTEM_LLM_CONFIG_ID,
} from "../../api/config";
import { ConfigListPanel, type ConfigItem } from "./ConfigListPanel";
import { useConfigItems } from "./config-list/useConfigItems";
import { LlmConfigEditor } from "./llm/LlmConfigEditor";
import { LlmMaxTokensPanel } from "./llm/LlmMaxTokensPanel";
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

/**
 * LLM 配置 Tab
 *
 * A4N 后端的 llm_config 只存连接信息（name / provider / base_url / api_key /
 * model / max_concurrent）；各生成场景的 max_tokens 统一存放在
 * system_settings.llm_max_tokens。
 */
interface LlmTabProps {
  scope?: ModelConfigScope;
}

export const LlmTab: React.FC<LlmTabProps> = ({ scope = "local" }) => {
  const { t } = useI18n();
  const serverScope = scope === "server";
  const loadItems = useCallback(async (): Promise<{
    items: ConfigItem<LlmForm>[];
    catalog: ModelProviderCatalog;
  }> => {
    const [list, providerCatalog] = await Promise.all([
      serverScope ? listServerLlmConfig() : listLlmConfig(),
      listModelProviderCatalog(),
    ]);
    const items = list.map((config) => {
      assertCatalogProvider("llm", config.provider, providerCatalog);
      return {
        ...toLlmItem(config),
        deleteDisabled: !serverScope && config.id === SYSTEM_LLM_CONFIG_ID,
        deleteDisabledReason: t("settings.systemTemplateCannotDelete", { name: "LLM" }),
      };
    });
    return { items, catalog: providerCatalog };
  }, [serverScope, t]);
  const { items, catalog, loading, refresh } = useConfigItems(
    loadItems,
    t("settings.loadConfigFailed", { name: t("settings.llmConfig") }),
  );

  const defaultModelOutputCap =
    items.find((item) => item.isActive)?.config.max_output_tokens ?? null;

  return (
    <div className="flex h-auto min-h-0 flex-col gap-3 md:h-full">
      <LlmMaxTokensPanel
        modelMaxOutputTokens={defaultModelOutputCap}
        getConfig={serverScope ? getServerLlmMaxTokensConfig : undefined}
        updateConfig={serverScope ? updateServerLlmMaxTokensConfig : undefined}
      />

      <div className="min-h-0 md:flex-1">
        <ConfigListPanel<LlmForm>
          title={t("settings.llmConfig")}
          items={items}
          loading={loading}
          onRefresh={refresh}
          defaultForm={DEFAULT_LLM_FORM}
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
            await (serverScope ? createServerLlmConfig : createLlmConfig)(toCreateLlmInput(form, t));
          }}
          onUpdate={async (id, form) => {
            await (serverScope ? updateServerLlmConfig : updateLlmConfig)(
              id,
              toUpdateLlmInput(form, t),
            );
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
