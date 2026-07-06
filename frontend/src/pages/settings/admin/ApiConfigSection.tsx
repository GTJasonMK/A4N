import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Bot,
  Brain,
  Film,
  Image as ImageIcon,
  Loader2,
  RefreshCw,
  ServerCog,
} from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  listEmbeddingConfig,
  getServerLlmMaxTokensConfig,
  listServerLlmConfig,
  type EmbeddingConfig,
  type LlmConfig,
  type LlmMaxTokensConfig,
} from "../../../api/config";
import { listServerImageConfigs, type ImageGenConfig } from "../../../api/image";
import { listServerVideoConfigs, type VideoGenConfig } from "../../../api/video";
import { useToast } from "../../../components/feedback/Toast";
import { BookIconButton } from "../../../components/ui/BookButton";
import { EmbeddingTab } from "../EmbeddingTab";
import { ImageTab } from "../ImageTab";
import { LlmTab } from "../LlmTab";
import { VideoTab } from "../VideoTab";

type ApiConfigPanel = "overview" | "llm" | "image" | "video" | "embedding";

interface ApiConfigSnapshot {
  llm: LlmConfig[];
  image: ImageGenConfig[];
  video: VideoGenConfig[];
  embedding: EmbeddingConfig[];
  maxTokens: LlmMaxTokensConfig;
}

const PANEL_DEFS: Array<{
  id: ApiConfigPanel;
  label: string;
  icon: React.ReactNode;
}> = [
  { id: "overview", label: "默认总览", icon: <ServerCog size={14} /> },
  { id: "llm", label: "LLM", icon: <Bot size={14} /> },
  { id: "image", label: "图片", icon: <ImageIcon size={14} /> },
  { id: "video", label: "视频", icon: <Film size={14} /> },
  { id: "embedding", label: "嵌入", icon: <Brain size={14} /> },
];

export const ApiConfigSection: React.FC = () => {
  const [panel, setPanel] = useState<ApiConfigPanel>("overview");

  return (
    <div className="flex min-h-0 flex-col gap-3">
      <div className="flex shrink-0 flex-wrap items-center gap-1">
        {PANEL_DEFS.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => setPanel(item.id)}
            data-active={panel === item.id ? "1" : undefined}
            className="pyqt-tab inline-flex items-center gap-1.5"
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </div>

      {panel === "overview" ? (
        <ApiConfigOverview />
      ) : (
        <div className="min-h-[60vh] min-w-0 rounded border border-book-border bg-book-bg-soft/40 p-2 sm:p-3 lg:h-[72vh] lg:min-h-[640px]">
          {panel === "llm" && <LlmTab scope="server" />}
          {panel === "image" && <ImageTab scope="server" />}
          {panel === "video" && <VideoTab scope="server" />}
          {panel === "embedding" && <EmbeddingTab />}
        </div>
      )}
    </div>
  );
};

const ApiConfigOverview: React.FC = () => {
  const addToast = useToast((state) => state.addToast);
  const [snapshot, setSnapshot] = useState<ApiConfigSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const mountedRef = useRef(true);
  const loadSeqRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
    };
  }, []);

  const load = useCallback(async () => {
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const [llm, image, video, embedding, maxTokens] = await Promise.all([
        listServerLlmConfig(),
        listServerImageConfigs(),
        listServerVideoConfigs(),
        listEmbeddingConfig(),
        getServerLlmMaxTokensConfig(),
      ]);
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      setSnapshot({ llm, image, video, embedding, maxTokens });
    } catch (err) {
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "加载 API 调用配置失败"), "error");
    } finally {
      if (mountedRef.current && loadSeqRef.current === requestSeq) setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    void load();
  }, [load]);

  const defaults = useMemo(() => {
    const llm = snapshot?.llm.find((item) => item.is_default) ?? null;
    const image = snapshot?.image.find((item) => item.is_default) ?? null;
    const video = snapshot?.video.find((item) => item.is_default) ?? null;
    const embedding = snapshot?.embedding.find((item) => item.is_default) ?? null;
    return { llm, image, video, embedding };
  }, [snapshot]);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h3 className="text-sm font-serif text-book-text-main">系统全局默认 API 调用配置</h3>
          <div className="text-[11px] text-book-text-muted">
            服务器数据库配置，所有账号共享，由管理员统一管理
          </div>
        </div>
        <BookIconButton label="刷新" variant="secondary" size="sm" onClick={load} disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>

      {loading && snapshot == null ? (
        <div className="inline-flex items-center gap-2 py-10 text-xs text-book-text-muted">
          <Loader2 size={14} className="animate-spin" />
          正在加载配置
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
          <ApiSummaryCard
            icon={<Bot size={16} />}
            title="LLM 默认调用"
            empty={!defaults.llm}
            rows={
              defaults.llm
                ? [
                    ["配置", configName(defaults.llm)],
                    ["模型", defaults.llm.model],
                    ["Provider", defaults.llm.provider],
                    ["Base URL", defaults.llm.base_url],
                    ["并发 / 超时", `${defaults.llm.max_concurrent} / ${defaults.llm.timeout_secs}s`],
                    [
                      "上下文 / 最大输出",
                      `${defaults.llm.max_context_tokens} / ${defaults.llm.max_output_tokens}`,
                    ],
                    ["API Key", defaults.llm.has_api_key ? "已配置" : "未配置"],
                    ["请求选项", compactJson(defaults.llm.request_options_json)],
                  ]
                : []
            }
            footer={
              snapshot
                ? `场景预算：章节 ${snapshot.maxTokens.chapter_generation}，分析 ${snapshot.maxTokens.chapter_analysis}，媒体 ${snapshot.maxTokens.media_prompt}`
                : undefined
            }
          />

          <ApiSummaryCard
            icon={<ImageIcon size={16} />}
            title="图片生成默认调用"
            empty={!defaults.image}
            rows={
              defaults.image
                ? [
                    ["配置", configName(defaults.image)],
                    ["模型", defaults.image.model],
                    ["Backend", defaults.image.backend],
                    ["Base URL", valueOrMissing(defaults.image.base_url)],
                    ["默认尺寸", `${defaults.image.default_width} x ${defaults.image.default_height}`],
                    ["并发", String(defaults.image.max_concurrent)],
                    ["API Key", defaults.image.has_api_key ? "已配置" : "未配置"],
                    ["Provider 选项", compactJson(defaults.image.provider_options_json)],
                  ]
                : []
            }
          />

          <ApiSummaryCard
            icon={<Film size={16} />}
            title="视频生成默认调用"
            empty={!defaults.video}
            rows={
              defaults.video
                ? [
                    ["配置", configName(defaults.video)],
                    ["模型", defaults.video.model],
                    ["Backend", defaults.video.backend],
                    ["Base URL", valueOrMissing(defaults.video.base_url)],
                    ["默认规格", `${defaults.video.default_width} x ${defaults.video.default_height}`],
                    [
                      "时长 / FPS",
                      `${defaults.video.default_duration_secs}s / ${defaults.video.default_fps}`,
                    ],
                    ["时长模式", videoDurationText(defaults.video)],
                    ["并发", String(defaults.video.max_concurrent)],
                    ["Provider 选项", compactJson(defaults.video.provider_options_json)],
                  ]
                : []
            }
          />

          <ApiSummaryCard
            icon={<Brain size={16} />}
            title="嵌入默认调用"
            empty={!defaults.embedding}
            rows={
              defaults.embedding
                ? [
                    ["配置", configName(defaults.embedding)],
                    ["模型", defaults.embedding.model],
                    ["Backend", defaults.embedding.backend],
                    ["Base URL", valueOrMissing(defaults.embedding.base_url)],
                    ["维度 / Batch", `${defaults.embedding.dimension} / ${defaults.embedding.batch_size}`],
                    ["本地模型路径", valueOrMissing(defaults.embedding.local_model_path)],
                    ["API Key", defaults.embedding.has_api_key ? "已配置" : "未配置"],
                  ]
                : []
            }
          />
        </div>
      )}
    </div>
  );
};

const ApiSummaryCard: React.FC<{
  icon: React.ReactNode;
  title: string;
  empty: boolean;
  rows: Array<[string, string]>;
  footer?: string;
}> = ({ icon, title, empty, rows, footer }) => (
  <section className="pyqt-card min-w-0 p-3">
    <div className="mb-3 flex items-center gap-2">
      <span className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded border border-book-border bg-book-bg-tertiary text-book-primary">
        {icon}
      </span>
      <h4 className="min-w-0 truncate text-sm font-medium text-book-text-main">{title}</h4>
    </div>
    {empty ? (
      <div className="py-6 text-center text-xs text-book-text-muted">未设置默认配置</div>
    ) : (
      <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
        {rows.map(([label, value]) => (
          <div key={label} className="min-w-0 rounded border border-book-border bg-book-bg-soft px-2.5 py-2">
            <div className="text-[11px] text-book-text-muted">{label}</div>
            <div className="mt-0.5 min-h-[18px] truncate text-xs text-book-text-main" title={value}>
              {value}
            </div>
          </div>
        ))}
      </div>
    )}
    {footer && !empty && (
      <div className="mt-3 rounded border border-book-border bg-book-bg-tertiary/40 px-2.5 py-2 text-[11px] text-book-text-muted">
        {footer}
      </div>
    )}
  </section>
);

function configName(config: { id: number; name: string }): string {
  return `#${config.id} ${config.name}`;
}

function valueOrMissing(value: string | null | undefined): string {
  const text = value?.trim();
  return text ? text : "未配置";
}

function compactJson(value: unknown): string {
  if (value == null) return "无";
  try {
    const text = JSON.stringify(value);
    return text.length > 90 ? `${text.slice(0, 87)}...` : text;
  } catch {
    return "无法显示";
  }
}

function videoDurationText(config: VideoGenConfig): string {
  if (config.duration_mode === "fixed") {
    return `固定 ${config.default_duration_secs}s`;
  }
  const supported = config.supported_durations_json;
  if (Array.isArray(supported) && supported.length > 0) {
    return supported.join(", ");
  }
  return `${config.min_duration_secs}-${config.max_duration_secs}s`;
}
