import React, { useCallback } from "react";
import {
  addVideoConfig,
  addServerVideoConfig,
  loadVideoConfigList,
  listServerVideoConfigs,
  removeVideoConfig,
  removeServerVideoConfig,
  setDefaultServerVideoConfig,
  setDefaultVideoConfig,
  SYSTEM_VIDEO_CONFIG_ID,
  updateVideoConfig,
  updateServerVideoConfig,
  updateSystemDefaultVideoKey,
} from "../../api/video";
import { ConfigListPanel, type ConfigItem } from "./ConfigListPanel";
import { useConfigItems } from "./config-list/useConfigItems";
import { VideoConfigEditor } from "./video/VideoConfigEditor";
import {
  DEFAULT_VIDEO_FORM,
  formatVideoItem,
  toCreateVideoInput,
  toUpdateVideoInput,
  toVideoItem,
  validateVideoForm,
} from "./video/videoConfigUtils";
import type { VideoForm } from "./video/types";
import {
  assertCatalogProvider,
  listModelProviderCatalog,
} from "../../api/providerCatalog";
import type { ModelProviderCatalog } from "../../api/generated/config";
import type { ModelConfigScope } from "../../api/modelDiscovery";
import { getActiveLocale, translateMessage, useI18n } from "../../i18n";

interface VideoTabProps {
  scope?: ModelConfigScope;
}

export const VideoTab: React.FC<VideoTabProps> = ({ scope = "local" }) => {
  const { t } = useI18n();
  const serverScope = scope === "server";
  const loadItems = useCallback(async (): Promise<{
    items: ConfigItem<VideoForm>[];
    catalog: ModelProviderCatalog | null;
    warning?: unknown[] | null;
  }> => {
    const [listResult, catalogResult] = await Promise.allSettled([
      serverScope
        ? listServerVideoConfigs().then((configs) => ({ configs, warning: undefined }))
        : loadVideoConfigList(),
      listModelProviderCatalog(),
    ]);
    if (listResult.status === "rejected") throw listResult.reason;
    const list = listResult.value.configs;
    const providerCatalog = catalogResult.status === "fulfilled" ? catalogResult.value : null;
    const warnings = listResult.value.warning === undefined
      ? []
      : [listResult.value.warning];
    const items = list.map((config) => {
      if (providerCatalog) assertCatalogProvider("video", config.backend, providerCatalog);
      return {
        ...toVideoItem(config),
        deleteDisabled: !serverScope && config.id === SYSTEM_VIDEO_CONFIG_ID,
        deleteDisabledReason: translateMessage(getActiveLocale(), "settings.systemTemplateCannotDelete", { name: translateMessage(getActiveLocale(), "settings.videoConfig") }),
      };
    });
    return {
      items,
      catalog: providerCatalog,
      warning: catalogResult.status === "rejected"
        ? [...warnings, catalogResult.reason]
        : warnings.length ? warnings : null,
    };
  }, [serverScope]);
  const { items, setItems, catalog, loading, error, refresh } = useConfigItems(
    loadItems,
    () => t("settings.loadConfigFailed", { name: t("settings.videoConfig") }),
  );

  return (
    <ConfigListPanel<VideoForm>
      title={t("settings.videoConfig")}
      items={items}
      onItemsChange={setItems}
      loading={loading}
      loadError={error}
      onRefresh={refresh}
      defaultForm={DEFAULT_VIDEO_FORM}
      validate={(form) =>
        validateVideoForm(
          form,
          catalog?.video.find((provider) => provider.id === form.provider),
          t,
        )
      }
      activeLabel={serverScope ? t("settings.globalDefault") : t("settings.currentDefault")}
      activateText={serverScope ? t("settings.setGlobalDefault") : t("settings.setDefault")}
      activateSuccessMessage={
        serverScope ? t("settings.globalDefaultSuccess") : t("settings.setDefaultSuccess")
      }
      activateErrorMessage={
        serverScope ? t("settings.globalDefaultFailed") : t("settings.setDefaultFailed")
      }
      formatItem={(form) => formatVideoItem(form, t)}
      renderEditor={(form, setForm, context) => (
        <VideoConfigEditor
          form={form}
          setForm={setForm}
          configId={context.selectedId}
          scope={scope}
          readonly={context.readonly}
          templateLocked={
            !serverScope && context.selectedId === SYSTEM_VIDEO_CONFIG_ID
          }
          providers={catalog?.video ?? []}
        />
      )}
      onCreate={async (form) => {
        const created = await (serverScope ? addServerVideoConfig : addVideoConfig)(
          toCreateVideoInput(
            form,
            catalog?.video.find((provider) => provider.id === form.provider),
            t,
          ),
        );
        return toVideoItem(created);
      }}
      onUpdate={async (id, form) => {
        const input = toUpdateVideoInput(form, catalog?.video.find((provider) => provider.id === form.provider), t);
        if (!serverScope && id === SYSTEM_VIDEO_CONFIG_ID) {
          return toVideoItem(await updateSystemDefaultVideoKey({ api_key_encrypted: input.api_key_encrypted }));
        }
        return toVideoItem(await (serverScope ? updateServerVideoConfig : updateVideoConfig)(
          id,
          input,
        ));
      }}
      onDelete={serverScope ? removeServerVideoConfig : removeVideoConfig}
      onActivate={async (id) => {
        await (
          serverScope ? setDefaultServerVideoConfig : setDefaultVideoConfig
        )(id);
      }}
    />
  );
};
