import React, { useCallback } from "react";
import {
  activateImageConfig,
  activateServerImageConfig,
  addImageConfig,
  addServerImageConfig,
  listImageConfigs,
  listServerImageConfigs,
  previewTestImageConfigRequest,
  previewServerTestImageConfigRequest,
  removeImageConfig,
  removeServerImageConfig,
  SYSTEM_IMAGE_CONFIG_ID,
  testImageConfig,
  testServerImageConfig,
  updateImageConfig,
  updateServerImageConfig,
} from "../../api/image";
import { ConfigListPanel, type ConfigItem } from "./ConfigListPanel";
import { useConfigItems } from "./config-list/useConfigItems";
import { ImageConfigEditor } from "./image/ImageConfigEditor";
import {
  DEFAULT_IMAGE_FORM,
  formatImageItem,
  toCreateImageInput,
  toImageItem,
  toUpdateImageInput,
  validateImageForm,
} from "./image/imageConfigUtils";
import type { ImageForm } from "./image/types";
import {
  assertCatalogProvider,
  listModelProviderCatalog,
} from "../../api/providerCatalog";
import type { ModelProviderCatalog } from "../../api/generated/config";

/**
 * 图片生成配置 Tab
 *
 * 注意：A4N 后端 image_gen_config 只存 name / backend / base_url / api_key_encrypted /
 * model / max_concurrent / default_width / default_height —— 没有 sampling steps 等推理参数。
 * 调用方在每次 image_scene / image_page 生成时按需传递。
 */

interface ImageTabProps {
  scope?: "user" | "server";
}

export const ImageTab: React.FC<ImageTabProps> = ({ scope = "user" }) => {
  const serverScope = scope === "server";
  const loadItems = useCallback(async (): Promise<{
    items: ConfigItem<ImageForm>[];
    catalog: ModelProviderCatalog;
  }> => {
    const [list, providerCatalog] = await Promise.all([
      serverScope ? listServerImageConfigs() : listImageConfigs(),
      listModelProviderCatalog(),
    ]);
    const items = list.map((config) => {
      assertCatalogProvider("image", config.backend, providerCatalog);
      return {
        ...toImageItem(config),
        deleteDisabled: !serverScope && config.id === SYSTEM_IMAGE_CONFIG_ID,
        deleteDisabledReason: "系统默认图片模板由管理员维护，不能在用户配置页删除。",
      };
    });
    return { items, catalog: providerCatalog };
  }, [serverScope]);
  const { items, catalog, loading, refresh } = useConfigItems(
    loadItems,
    "加载图片配置失败",
  );

  return (
    <ConfigListPanel<ImageForm>
      title="图片生成配置"
      items={items}
      loading={loading}
      onRefresh={refresh}
      defaultForm={DEFAULT_IMAGE_FORM}
      validate={validateImageForm}
      activeLabel={serverScope ? "全局默认" : "当前默认"}
      activateText={serverScope ? "设为全局默认" : "设为默认"}
      activateSuccessMessage={serverScope ? "已设为系统全局默认" : "已设为当前默认"}
      activateErrorMessage={serverScope ? "设置系统全局默认失败" : "设置默认失败"}
      formatItem={formatImageItem}
      renderEditor={(form, setForm, context) => (
        <ImageConfigEditor
          form={form}
          setForm={setForm}
          configId={context.selectedId}
          serverScope={serverScope}
          readonly={context.readonly}
          templateLocked={!serverScope && context.selectedId === SYSTEM_IMAGE_CONFIG_ID}
          providers={catalog?.image ?? []}
        />
      )}
      onCreate={async (f) => {
        await (serverScope ? addServerImageConfig : addImageConfig)(toCreateImageInput(f));
      }}
      onUpdate={async (id, f) => {
        await (serverScope ? updateServerImageConfig : updateImageConfig)(
          id,
          toUpdateImageInput(f),
        );
      }}
      onDelete={serverScope ? removeServerImageConfig : removeImageConfig}
      onActivate={async (id) => {
        await (serverScope ? activateServerImageConfig : activateImageConfig)(id);
      }}
      onTest={async (id) => {
        await (serverScope ? testServerImageConfig : testImageConfig)(id);
      }}
      onPreviewTest={serverScope ? previewServerTestImageConfigRequest : previewTestImageConfigRequest}
    />
  );
};
