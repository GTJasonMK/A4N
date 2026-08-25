import React, { useCallback } from "react";
import {
  addVideoConfig,
  addServerVideoConfig,
  listVideoConfigs,
  listServerVideoConfigs,
  removeVideoConfig,
  removeServerVideoConfig,
  setDefaultServerVideoConfig,
  setDefaultVideoConfig,
  SYSTEM_VIDEO_CONFIG_ID,
  updateVideoConfig,
  updateServerVideoConfig,
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
import { useI18n } from "../../i18n";

interface VideoTabProps {
  scope?: ModelConfigScope;
}

export const VideoTab: React.FC<VideoTabProps> = ({ scope = "local" }) => {
  const { t } = useI18n();
  const serverScope = scope === "server";
  const loadItems = useCallback(async (): Promise<{
    items: ConfigItem<VideoForm>[];
    catalog: ModelProviderCatalog;
  }> => {
    const [list, providerCatalog] = await Promise.all([
      serverScope ? listServerVideoConfigs() : listVideoConfigs(),
      listModelProviderCatalog(),
    ]);
    const items = list.map((config) => {
      assertCatalogProvider("video", config.backend, providerCatalog);
      return {
        ...toVideoItem(config),
        deleteDisabled: !serverScope && config.id === SYSTEM_VIDEO_CONFIG_ID,
        deleteDisabledReason:
          t("settings.systemTemplateCannotDelete", { name: t("settings.videoConfig") }),
      };
    });
    return { items, catalog: providerCatalog };
  }, [serverScope, t]);
  const { items, catalog, loading, refresh } = useConfigItems(
    loadItems,
    t("settings.loadConfigFailed", { name: t("settings.videoConfig") }),
  );

  return (
    <ConfigListPanel<VideoForm>
      title={t("settings.videoConfig")}
      items={items}
      loading={loading}
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
        await (serverScope ? addServerVideoConfig : addVideoConfig)(
          toCreateVideoInput(
            form,
            catalog?.video.find((provider) => provider.id === form.provider),
            t,
          ),
        );
      }}
      onUpdate={async (id, form) => {
        await (serverScope ? updateServerVideoConfig : updateVideoConfig)(
          id,
          toUpdateVideoInput(
            form,
            catalog?.video.find((provider) => provider.id === form.provider),
            t,
          ),
        );
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
