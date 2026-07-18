import React, { useCallback, useEffect, useRef, useState } from "react";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import {
  createLlmConfig,
  createServerLlmConfig,
  deleteLlmConfig,
  deleteServerLlmConfig,
  getServerLlmMaxTokensConfig,
  listLlmConfig,
  listServerLlmConfig,
  previewTestLlmConfigRequest,
  previewServerTestLlmConfigRequest,
  setDefaultLlmConfig,
  setDefaultServerLlmConfig,
  testLlmConfig,
  testServerLlmConfig,
  updateLlmConfig,
  updateServerLlmConfig,
  updateServerLlmMaxTokensConfig,
  SYSTEM_LLM_CONFIG_ID,
} from "../../api/config";
import { ConfigListPanel, type ConfigItem } from "./ConfigListPanel";
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

/**
 * LLM 配置 Tab
 *
 * A4N 后端的 llm_config 只存连接信息（name / provider / base_url / api_key /
 * model / max_concurrent）；各生成场景的 max_tokens 统一存放在
 * system_settings.llm_max_tokens。
 */
interface LlmTabProps {
  scope?: "user" | "server";
}

export const LlmTab: React.FC<LlmTabProps> = ({ scope = "user" }) => {
  const addToast = useToast((state) => state.addToast);
  const [items, setItems] = useState<ConfigItem<LlmForm>[]>([]);
  const [loading, setLoading] = useState(false);
  const mountedRef = useRef(true);
  const refreshSeqRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
    };
  }, []);

  const isCurrentRefresh = useCallback((seq: number) => {
    return mountedRef.current && refreshSeqRef.current === seq;
  }, []);

  const serverScope = scope === "server";

  const refresh = useCallback(async () => {
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const list = await (serverScope ? listServerLlmConfig() : listLlmConfig());
      if (!isCurrentRefresh(requestSeq)) return;
      setItems(
        list.map((config) => ({
          ...toLlmItem(config),
          deleteDisabled: !serverScope && config.id === SYSTEM_LLM_CONFIG_ID,
          deleteDisabledReason: "系统默认 LLM 模板由管理员维护，不能在用户配置页删除。",
        })),
      );
    } catch (err) {
      if (!isCurrentRefresh(requestSeq)) return;
      addToast(extractErrorMessage(err, "加载 LLM 配置失败"), "error");
    } finally {
      if (isCurrentRefresh(requestSeq)) setLoading(false);
    }
  }, [addToast, isCurrentRefresh, serverScope]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

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
          title="LLM 配置"
          items={items}
          loading={loading}
          onRefresh={refresh}
          defaultForm={DEFAULT_LLM_FORM}
          validate={validateLlmForm}
          activeLabel={serverScope ? "全局默认" : "当前默认"}
          activateText={serverScope ? "设为全局默认" : "设为默认"}
          activateSuccessMessage={serverScope ? "已设为系统全局默认" : "已设为当前默认"}
          activateErrorMessage={serverScope ? "设置系统全局默认失败" : "设置默认失败"}
          formatItem={formatLlmItem}
          renderEditor={(form, setForm, context) => (
            <LlmConfigEditor
              form={form}
              setForm={setForm}
              configId={context.selectedId}
              serverScope={serverScope}
              readonly={context.readonly}
              templateLocked={!serverScope && context.selectedId === SYSTEM_LLM_CONFIG_ID}
            />
          )}
          onCreate={async (form) => {
            await (serverScope ? createServerLlmConfig : createLlmConfig)(toCreateLlmInput(form));
          }}
          onUpdate={async (id, form) => {
            await (serverScope ? updateServerLlmConfig : updateLlmConfig)(
              id,
              toUpdateLlmInput(form),
            );
          }}
          onDelete={serverScope ? deleteServerLlmConfig : deleteLlmConfig}
          onActivate={serverScope ? setDefaultServerLlmConfig : setDefaultLlmConfig}
          onTest={async (id) => {
            await (serverScope ? testServerLlmConfig : testLlmConfig)(id);
          }}
          onPreviewTest={serverScope ? previewServerTestLlmConfigRequest : previewTestLlmConfigRequest}
        />
      </div>
    </div>
  );
};
