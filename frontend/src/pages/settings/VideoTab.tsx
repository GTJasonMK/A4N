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

interface VideoTabProps {
  scope?: "user" | "server";
}

export const VideoTab: React.FC<VideoTabProps> = ({ scope = "user" }) => {
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
          "系统默认视频模板由管理员维护，不能在用户配置页删除。",
      };
    });
    return { items, catalog: providerCatalog };
  }, [serverScope]);
  const { items, catalog, loading, refresh } = useConfigItems(
    loadItems,
    "加载视频配置失败",
  );

  return (
    <ConfigListPanel<VideoForm>
      title="视频生成配置"
      items={items}
      loading={loading}
      onRefresh={refresh}
      defaultForm={DEFAULT_VIDEO_FORM}
      validate={(form) =>
        validateVideoForm(
          form,
          catalog?.video.find((provider) => provider.id === form.provider),
        )
      }
      activeLabel={serverScope ? "全局默认" : "当前默认"}
      activateText={serverScope ? "设为全局默认" : "设为默认"}
      activateSuccessMessage={
        serverScope ? "已设为系统全局默认" : "已设为当前默认"
      }
      activateErrorMessage={
        serverScope ? "设置系统全局默认失败" : "设置默认失败"
      }
      formatItem={formatVideoItem}
      renderEditor={(form, setForm, context) => (
        <VideoConfigEditor
          form={form}
          setForm={setForm}
          configId={context.selectedId}
          serverScope={serverScope}
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
          ),
        );
      }}
      onUpdate={async (id, form) => {
        await (serverScope ? updateServerVideoConfig : updateVideoConfig)(
          id,
          toUpdateVideoInput(
            form,
            catalog?.video.find((provider) => provider.id === form.provider),
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
