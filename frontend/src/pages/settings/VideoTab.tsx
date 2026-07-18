import React, { useCallback, useEffect, useRef, useState } from "react";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import {
  activateVideoConfig,
  activateServerVideoConfig,
  addVideoConfig,
  addServerVideoConfig,
  listVideoConfigs,
  listServerVideoConfigs,
  removeVideoConfig,
  removeServerVideoConfig,
  SYSTEM_VIDEO_CONFIG_ID,
  testVideoConfig,
  testServerVideoConfig,
  updateVideoConfig,
  updateServerVideoConfig,
} from "../../api/video";
import { ConfigListPanel, type ConfigItem } from "./ConfigListPanel";
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

interface VideoTabProps {
  scope?: "user" | "server";
}

export const VideoTab: React.FC<VideoTabProps> = ({ scope = "user" }) => {
  const addToast = useToast((state) => state.addToast);
  const [items, setItems] = useState<ConfigItem<VideoForm>[]>([]);
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
      const list = await (serverScope ? listServerVideoConfigs() : listVideoConfigs());
      if (!isCurrentRefresh(requestSeq)) return;
      setItems(
        list.map((config) => ({
          ...toVideoItem(config),
          deleteDisabled: !serverScope && config.id === SYSTEM_VIDEO_CONFIG_ID,
          deleteDisabledReason: "系统默认视频模板由管理员维护，不能在用户配置页删除。",
        })),
      );
    } catch (err) {
      if (!isCurrentRefresh(requestSeq)) return;
      addToast(extractErrorMessage(err, "加载视频配置失败"), "error");
    } finally {
      if (isCurrentRefresh(requestSeq)) setLoading(false);
    }
  }, [addToast, isCurrentRefresh, serverScope]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <ConfigListPanel<VideoForm>
      title="视频生成配置"
      items={items}
      loading={loading}
      onRefresh={refresh}
      defaultForm={DEFAULT_VIDEO_FORM}
      validate={validateVideoForm}
      activeLabel={serverScope ? "全局默认" : "当前默认"}
      activateText={serverScope ? "设为全局默认" : "设为默认"}
      activateSuccessMessage={serverScope ? "已设为系统全局默认" : "已设为当前默认"}
      activateErrorMessage={serverScope ? "设置系统全局默认失败" : "设置默认失败"}
      formatItem={formatVideoItem}
      renderEditor={(form, setForm, context) => (
        <VideoConfigEditor
          form={form}
          setForm={setForm}
          configId={context.selectedId}
          serverScope={serverScope}
          readonly={context.readonly}
          templateLocked={!serverScope && context.selectedId === SYSTEM_VIDEO_CONFIG_ID}
        />
      )}
      onCreate={async (form) => {
        await (serverScope ? addServerVideoConfig : addVideoConfig)(toCreateVideoInput(form));
      }}
      onUpdate={async (id, form) => {
        await (serverScope ? updateServerVideoConfig : updateVideoConfig)(
          id,
          toUpdateVideoInput(form),
        );
      }}
      onDelete={serverScope ? removeServerVideoConfig : removeVideoConfig}
      onActivate={async (id) => {
        await (serverScope ? activateServerVideoConfig : activateVideoConfig)(id);
      }}
      onTest={async (id) => {
        await (serverScope ? testServerVideoConfig : testVideoConfig)(id);
      }}
    />
  );
};
