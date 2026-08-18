import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Bot,
  Brain,
  Film,
  Image as ImageIcon,
  RefreshCw,
  ServerCog, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  listEmbeddingConfig,
  getServerLlmMaxTokensConfig,
  listServerLlmConfig,
  type EmbeddingConfig,
  type LlmMaxTokensConfig,
} from "../../../api/config";
import { listServerImageConfigs, type ImageGenConfig } from "../../../api/image";
import { listServerVideoConfigs, type VideoGenConfig } from "../../../api/video";
import { listModelProviderCatalog } from "../../../api/providerCatalog";
import type { ModelProviderCatalog } from "../../../api/generated/config";
import type { LlmConfigSummary } from "../../../api/generated/config";
import { useToast } from "../../../components/feedback/Toast";
import { BookIconButton } from "../../../components/ui/BookButton";
import { EmbeddingTab } from "../EmbeddingTab";
import { ImageTab } from "../ImageTab";
import { LlmTab } from "../LlmTab";
import { VideoTab } from "../VideoTab";

type ApiConfigPanel = "overview" | "llm" | "image" | "video" | "embedding";

interface ApiConfigSnapshot {
  llm: LlmConfigSummary[];
  image: ImageGenConfig[];
  video: VideoGenConfig[];
  embedding: EmbeddingConfig[];
  maxTokens: LlmMaxTokensConfig;
  providers: ModelProviderCatalog;
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
      <div
        className="tab-bar shrink-0 overflow-x-auto border-b border-book-border/45"
        aria-label="API 配置类型"
      >
        {PANEL_DEFS.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => setPanel(item.id)}
            data-active={panel === item.id ? "1" : undefined}
            className="pyqt-tab inline-flex items-center gap-1.5 whitespace-nowrap"
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </div>

      {panel === "overview" ? (
        <ApiConfigOverview />
      ) : (
        <div className="min-h-[60vh] min-w-0 lg:h-[72vh] lg:min-h-[640px]">
          {panel === "llm" && <LlmTab scope="server" />}
          {panel === "image" && <ImageTab scope="server" />}
          {panel === "video" && <VideoTab scope="server" />}
          {panel === "embedding" && <EmbeddingTab scope="server" />}
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
      const [llm, image, video, embedding, maxTokens, providers] = await Promise.all([
        listServerLlmConfig(),
        listServerImageConfigs(),
        listServerVideoConfigs(),
        listEmbeddingConfig("server"),
        getServerLlmMaxTokensConfig(),
        listModelProviderCatalog(),
      ]);
      if (!mountedRef.current || loadSeqRef.current !== requestSeq) return;
      setSnapshot({ llm, image, video, embedding, maxTokens, providers });
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
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <ServerCog size={13} className="shrink-0 text-book-primary" />
        <span>这里的默认配置由管理员维护，所有账号共享。</span>
        <span className="flex-1" />
        <span className="shrink-0">
          <BookIconButton
            label="刷新"
            variant="secondary"
            size="sm"
            onClick={load}
            disabled={loading}
          >
            <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
          </BookIconButton>
        </span>
      </div>

      {loading && snapshot == null ? (
        <div className="inline-flex items-center gap-2 py-10 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" />
          正在加载配置
        </div>
      ) : (
        <section className="grid grid-cols-1 items-start gap-3 xl:grid-cols-2" aria-label="共享默认配置">
          <ApiSummaryCard
            icon={<Bot size={16} />}
            title="语言模型默认配置"
            empty={!defaults.llm}
            primaryRows={
              defaults.llm
                ? [
                    ["配置", configName(defaults.llm)],
                    ["模型", defaults.llm.model],
                    ["服务商", providerLabel(snapshot?.providers.llm, defaults.llm.provider)],
                    ["访问密钥", defaults.llm.has_api_key ? "已配置" : "未配置"],
                  ]
                : []
            }
            diagnosticRows={
              defaults.llm
                ? [
                    ["接口地址", defaults.llm.base_url],
                    [
                      "并发 / 超时",
                      `${defaults.llm.max_concurrent} / ${defaults.llm.timeout_secs}s`,
                    ],
                    [
                      "上下文 / 最大输出",
                      `${defaults.llm.max_context_tokens} / ${defaults.llm.max_output_tokens}`,
                    ],
                    ["高级参数", configuredLabel(defaults.llm.extra_body_json)],
                  ]
                : []
            }
            diagnosticFooter={
              snapshot
                ? `场景预算：章节 ${snapshot.maxTokens.chapter_generation}，分析 ${snapshot.maxTokens.chapter_analysis}，媒体 ${snapshot.maxTokens.media_prompt}`
                : undefined
            }
          />

          <ApiSummaryCard
            icon={<ImageIcon size={16} />}
            title="图片生成默认调用"
            empty={!defaults.image}
            primaryRows={
              defaults.image
                ? [
                    ["配置", configName(defaults.image)],
                    ["模型", defaults.image.model],
                    ["运行方式", providerLabel(snapshot?.providers.image, defaults.image.backend)],
                    ["访问密钥", defaults.image.has_api_key ? "已配置" : "未配置"],
                  ]
                : []
            }
            diagnosticRows={
              defaults.image
                ? [
                    ["接口地址", valueOrMissing(defaults.image.base_url)],
                    [
                      "默认尺寸",
                      `${defaults.image.default_width} × ${defaults.image.default_height}`,
                    ],
                    ["并发", String(defaults.image.max_concurrent)],
                  ]
                : []
            }
          />

          <ApiSummaryCard
            icon={<Film size={16} />}
            title="视频生成默认调用"
            empty={!defaults.video}
            primaryRows={
              defaults.video
                ? [
                    ["配置", configName(defaults.video)],
                    ["模型", defaults.video.model],
                    ["运行方式", providerLabel(snapshot?.providers.video, defaults.video.backend)],
                  ]
                : []
            }
            diagnosticRows={
              defaults.video
                ? [
                    ["接口地址", valueOrMissing(defaults.video.base_url)],
                    [
                      "默认规格",
                      `${defaults.video.default_width} × ${defaults.video.default_height}`,
                    ],
                    [
                      "时长 / 帧率",
                      `${defaults.video.default_duration_secs}s / ${defaults.video.default_fps}`,
                    ],
                    ["时长模式", videoDurationText(defaults.video)],
                    ["并发", String(defaults.video.max_concurrent)],
                  ]
                : []
            }
          />

          <ApiSummaryCard
            icon={<Brain size={16} />}
            title="嵌入默认调用"
            empty={!defaults.embedding}
            primaryRows={
              defaults.embedding
                ? [
                    ["配置", configName(defaults.embedding)],
                    ["模型", modelLabel(defaults.embedding.model)],
                    [
                      "运行方式",
                      providerLabel(snapshot?.providers.embedding, defaults.embedding.backend),
                    ],
                    ["访问密钥", defaults.embedding.has_api_key ? "已配置" : "未配置"],
                  ]
                : []
            }
            diagnosticRows={
              defaults.embedding
                ? [
                    ["接口地址", valueOrMissing(defaults.embedding.base_url)],
                    [
                      "维度 / 批大小",
                      `${defaults.embedding.dimension} / ${defaults.embedding.batch_size}`,
                    ],
                    ["本地模型路径", valueOrMissing(defaults.embedding.local_model_path)],
                  ]
                : []
            }
          />
        </section>
      )}
    </div>
  );
};

const ApiSummaryCard: React.FC<{
  icon: React.ReactNode;
  title: string;
  empty: boolean;
  primaryRows: Array<[string, string]>;
  diagnosticRows?: Array<[string, string]>;
  diagnosticFooter?: string;
}> = ({ icon, title, empty, primaryRows, diagnosticRows = [], diagnosticFooter }) => (
  <article className="a4n-flat-surface min-w-0 overflow-hidden">
    <div className="flex items-center gap-2 px-3 py-2.5">
      <span className="inline-flex h-6 w-6 shrink-0 items-center justify-center text-book-primary">
        {icon}
      </span>
      <h4 className="min-w-0 truncate text-xs font-medium text-book-text-main">
        {title}
      </h4>
    </div>
    {empty ? (
      <div className="border-t border-book-border/25 px-3 py-3 text-xs text-book-text-muted">
        未设置默认配置
      </div>
    ) : (
      <>
        <ConfigRows rows={primaryRows} />
        {(diagnosticRows.length > 0 || diagnosticFooter) && (
          <details className="border-t border-book-border/35">
            <summary className="cursor-pointer px-3 py-2.5 text-small text-book-text-muted transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-text-sub">
              诊断信息
            </summary>
            <div className="border-t border-book-border/25 bg-book-bg-soft/35">
              <ConfigRows rows={diagnosticRows} />
              {diagnosticFooter && (
                <div className="border-t border-book-border/30 px-3 py-2 text-small leading-relaxed text-book-text-muted">
                  {diagnosticFooter}
                </div>
              )}
            </div>
          </details>
        )}
      </>
    )}
  </article>
);

const ConfigRows: React.FC<{ rows: Array<[string, string]> }> = ({ rows }) => (
  <dl className="divide-y divide-book-border/30 border-t border-book-border/25">
    {rows.map(([label, value]) => (
      <div
        key={label}
        className="grid min-w-0 grid-cols-[minmax(5rem,0.7fr)_minmax(0,1.3fr)] gap-3 px-3 py-2.5 text-xs"
      >
        <dt className="text-book-text-muted">{label}</dt>
        <dd className="min-w-0 truncate text-right text-book-text-main" title={value}>
          {value}
        </dd>
      </div>
    ))}
  </dl>
);

function configName(config: { id: number; name: string }): string {
  return config.name;
}

function valueOrMissing(value: string | null | undefined): string {
  const text = value?.trim();
  return text ? text : "未配置";
}

function configuredLabel(value: unknown): string {
  if (value == null) return "未配置";
  if (typeof value === "object" && Object.keys(value as Record<string, unknown>).length === 0) {
    return "未配置";
  }
  return "已配置";
}

function providerLabel(
  providers: Array<{ id: string; display_name: string }> | undefined,
  value: string,
): string {
  return providers?.find((provider) => provider.id === value)?.display_name ?? `未注册（${value}）`;
}

function modelLabel(value: string): string {
  if (value === "local-sha256-hash") return "内置哈希向量";
  return value;
}

function videoDurationText(config: VideoGenConfig): string {
  if (config.duration_mode === "fixed") {
    return `固定 ${config.default_duration_secs}s`;
  }
  return `${config.min_duration_secs}-${config.max_duration_secs}s`;
}
