import React, { useCallback } from "react";
import { extractErrorMessage } from "../../api";
import {
  addImageConfig,
  addServerImageConfig,
  loadImageConfigList,
  listServerImageConfigs,
  previewTestImageConfigRequest,
  removeImageConfig,
  removeServerImageConfig,
  setDefaultImageConfig,
  setDefaultServerImageConfig,
  SYSTEM_IMAGE_CONFIG_ID,
  testImageConfig,
  updateImageConfig,
  updateServerImageConfig,
  updateSystemDefaultImageKey,
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
import type { ModelConfigScope } from "../../api/modelDiscovery";
import { useI18n } from "../../i18n";

/**
 * 图片生成配置 Tab
 *
 * 注意：A4N 后端 image_gen_config 只存 name / backend / base_url / api_key_encrypted /
 * model / max_concurrent / supported_sizes —— 没有 sampling steps 等推理参数。
 * 调用方在每次 image_scene / image_page 生成时按需传递。
 */

interface ImageTabProps {
  scope?: ModelConfigScope;
}

export const ImageTab: React.FC<ImageTabProps> = ({ scope = "local" }) => {
  const { t } = useI18n();
  const serverScope = scope === "server";
  const loadItems = useCallback(async (): Promise<{
    items: ConfigItem<ImageForm>[];
    catalog: ModelProviderCatalog | null;
    warning?: string | null;
  }> => {
    const [listResult, catalogResult] = await Promise.allSettled([
      serverScope
        ? listServerImageConfigs().then((configs) => ({ configs, warning: undefined }))
        : loadImageConfigList(),
      listModelProviderCatalog(),
    ]);
    if (listResult.status === "rejected") throw listResult.reason;
    const list = listResult.value.configs;
    const providerCatalog = catalogResult.status === "fulfilled" ? catalogResult.value : null;
    const warnings = listResult.value.warning === undefined
      ? []
      : [extractErrorMessage(listResult.value.warning, t("settings.loadConfigFailed", { name: t("settings.image") }))];
    const items = list.map((config) => {
      if (providerCatalog) assertCatalogProvider("image", config.backend, providerCatalog);
      return {
        ...toImageItem(config),
        deleteDisabled: !serverScope && config.id === SYSTEM_IMAGE_CONFIG_ID,
        deleteDisabledReason:
          t("settings.systemTemplateCannotDelete", { name: t("settings.imageConfig") }),
      };
    });
    return {
      items,
      catalog: providerCatalog,
      warning: catalogResult.status === "rejected"
        ? [...warnings, extractErrorMessage(catalogResult.reason, t("settings.loadConfigFailed", { name: t("settings.image") }))].join("; ")
        : warnings.join("; ") || null,
    };
  }, [serverScope, t]);
  const { items, setItems, catalog, loading, error, refresh } = useConfigItems(
    loadItems,
    t("settings.loadConfigFailed", { name: t("settings.imageConfig") }),
  );

  return (
    <ConfigListPanel<ImageForm>
      title={t("settings.imageConfig")}
      items={items}
      onItemsChange={setItems}
      loading={loading}
      loadError={error}
      onRefresh={refresh}
      defaultForm={DEFAULT_IMAGE_FORM}
      validate={(form) => validateImageForm(form, t)}
      activeLabel={serverScope ? t("settings.globalDefault") : t("settings.currentDefault")}
      activateText={serverScope ? t("settings.setGlobalDefault") : t("settings.setDefault")}
      activateSuccessMessage={
        serverScope ? t("settings.globalDefaultSuccess") : t("settings.setDefaultSuccess")
      }
      activateErrorMessage={
        serverScope ? t("settings.globalDefaultFailed") : t("settings.setDefaultFailed")
      }
      formatItem={formatImageItem}
      renderEditor={(form, setForm, context) => (
        <ImageConfigEditor
          form={form}
          setForm={setForm}
          configId={context.selectedId}
          scope={scope}
          readonly={context.readonly}
          templateLocked={
            !serverScope && context.selectedId === SYSTEM_IMAGE_CONFIG_ID
          }
          providers={catalog?.image ?? []}
        />
      )}
      onCreate={async (f) => {
        const created = await (serverScope ? addServerImageConfig : addImageConfig)(
          toCreateImageInput(f),
        );
        return toImageItem(created);
      }}
      onUpdate={async (id, f) => {
        const input = toUpdateImageInput(f);
        if (!serverScope && id === SYSTEM_IMAGE_CONFIG_ID) {
          return toImageItem(await updateSystemDefaultImageKey({ api_key_encrypted: input.api_key_encrypted }));
        }
        return toImageItem(await (serverScope ? updateServerImageConfig : updateImageConfig)(
          id,
          input,
        ));
      }}
      onDelete={serverScope ? removeServerImageConfig : removeImageConfig}
      onActivate={async (id) => {
        await (
          serverScope ? setDefaultServerImageConfig : setDefaultImageConfig
        )(id);
      }}
      onTest={async (id) => {
        await testImageConfig(id, scope);
      }}
      onPreviewTest={(id) => previewTestImageConfigRequest(id, scope)}
    />
  );
};
