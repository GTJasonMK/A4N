import React, { useCallback, useMemo, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { Bot,
  Brain,
  Film,
  Image as ImageIcon,
  RefreshCw,
  ServerCog, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import {
  listEmbeddingConfig,
  listServerLlmConfig,
  type EmbeddingConfig,
} from "../../../api/config";
import { listServerImageConfigs, type ImageGenConfig } from "../../../api/image";
import { listServerVideoConfigs, type VideoGenConfig } from "../../../api/video";
import { listModelProviderCatalog } from "../../../api/providerCatalog";
import type { ModelProviderCatalog } from "../../../api/generated/config";
import type { LlmConfigSummary } from "../../../api/generated/config";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookIconButton } from "../../../components/ui/BookButton";
import { EmbeddingTab } from "../EmbeddingTab";
import { ImageTab } from "../ImageTab";
import { LlmTab } from "../LlmTab";
import { VideoTab } from "../VideoTab";
import { useI18n, type MessageKey } from "../../../i18n";
import { useAdminSectionLoader } from "./useAdminSectionLoader";
import { useDirtyRegistry, useDirtyRegistryVersion } from "../../../components/navigation/dirtyRegistry";

type ApiConfigPanel = "overview" | "llm" | "image" | "video" | "embedding";
type ApiConfigReadKey = "llm" | "image" | "video" | "embedding" | "providers";

interface ApiConfigSnapshot {
  llm: LlmConfigSummary[] | null;
  image: ImageGenConfig[] | null;
  video: VideoGenConfig[] | null;
  embedding: EmbeddingConfig[] | null;
  providers: ModelProviderCatalog | null;
  errors: Partial<Record<ApiConfigReadKey, unknown>>;
}

const PANEL_DEFS: Array<{
  id: ApiConfigPanel;
  labelKey: MessageKey;
  icon: React.ReactNode;
}> = [
  { id: "overview", labelKey: "settingsAdmin.defaultOverview", icon: <ServerCog size={14} /> },
  { id: "llm", labelKey: "settings.llm", icon: <Bot size={14} /> },
  { id: "image", labelKey: "settingsAdmin.image", icon: <ImageIcon size={14} /> },
  { id: "video", labelKey: "settingsAdmin.video", icon: <Film size={14} /> },
  { id: "embedding", labelKey: "settingsAdmin.embedding", icon: <Brain size={14} /> },
];

export const ApiConfigSection: React.FC = () => {
  const { t } = useI18n();
  const dirtyRegistry = useDirtyRegistry();
  useDirtyRegistryVersion();
  const canDiscardDirty = dirtyRegistry.canDiscardDirty();
  const [panel, setPanel] = useState<ApiConfigPanel>("overview");
  const navigationGuardRef = useRef(false);

  const selectPanel = async (nextPanel: ApiConfigPanel) => {
    if (nextPanel === panel) return;
    if (dirtyRegistry.hasDirty()) {
      if (!canDiscardDirty) {
        useToast.getState().addToast(t("common.operationInProgress"), "info");
        return;
      }
      if (navigationGuardRef.current) return;
      navigationGuardRef.current = true;
      try {
        const ok = await confirmDialog({
          title: t("common.unsavedChangesTitle"),
          message: t("common.unsavedChangesMessage", {
            name: t("settingsAdmin.configuration"),
            action: t("settings.category"),
          }),
          confirmText: t("common.discardDraft"),
          dialogType: "warning",
        });
        if (!ok) return;
        if (!await dirtyRegistry.discardDirty()) {
          useToast.getState().addToast(t("common.operationInProgress"), "info");
          return;
        }
      } finally {
        navigationGuardRef.current = false;
      }
    }
    setPanel(nextPanel);
  };

  return (
    <div className="flex min-h-0 flex-col gap-3">
      <div
        className="tab-bar shrink-0 overflow-x-auto border-b border-book-border/45"
        aria-label={t("settingsAdmin.apiConfigType")}
      >
        {PANEL_DEFS.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => void selectPanel(item.id)}
            data-active={panel === item.id ? "1" : undefined}
            className="pyqt-tab inline-flex items-center gap-1.5 whitespace-nowrap"
          >
            {item.icon}
            {t(item.labelKey)}
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
  const { t } = useI18n();
  const [snapshot, setSnapshot] = useState<ApiConfigSnapshot | null>(null);
  const loadSnapshot = useCallback(async (): Promise<ApiConfigSnapshot> => {
    const [llm, image, video, embedding, providers] = await Promise.allSettled([
      listServerLlmConfig(), listServerImageConfigs(), listServerVideoConfigs(),
      listEmbeddingConfig("server"), listModelProviderCatalog(),
    ]);
    const errors: ApiConfigSnapshot["errors"] = {};
    if (llm.status === "rejected") errors.llm = llm.reason;
    if (image.status === "rejected") errors.image = image.reason;
    if (video.status === "rejected") errors.video = video.reason;
    if (embedding.status === "rejected") errors.embedding = embedding.reason;
    if (providers.status === "rejected") errors.providers = providers.reason;
    return {
      llm: llm.status === "fulfilled" ? llm.value : null,
      image: image.status === "fulfilled" ? image.value : null,
      video: video.status === "fulfilled" ? video.value : null,
      embedding: embedding.status === "fulfilled" ? embedding.value : null,
      providers: providers.status === "fulfilled" ? providers.value : null,
      errors,
    };
  }, []);
  const applySnapshot = useCallback((loaded: ApiConfigSnapshot) => setSnapshot(loaded), []);
  const { loading, error, load } = useAdminSectionLoader(
    "admin-api-defaults", loadSnapshot, applySnapshot, "settingsAdmin.loadApiConfigFailed",
  );

  const defaults = useMemo(() => {
    const llm = snapshot?.llm?.find((item) => item.is_default) ?? null;
    const image = snapshot?.image?.find((item) => item.is_default) ?? null;
    const video = snapshot?.video?.find((item) => item.is_default) ?? null;
    const embedding = snapshot?.embedding?.find((item) => item.is_default) ?? null;
    return { llm, image, video, embedding };
  }, [snapshot]);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <ServerCog size={13} className="shrink-0 text-book-primary" />
        <span>{t("settingsAdmin.sharedDefaultsHint")}</span>
        <span className="flex-1" />
        <span className="shrink-0">
          <BookIconButton
            label={t("settingsAdmin.refresh")}
            variant="secondary"
            size="sm"
            onClick={load}
            disabled={loading}
          >
            <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
          </BookIconButton>
        </span>
      </div>

      {error ? <p role="alert" className="text-xs text-error">{error}</p> : null}
      {snapshot && Object.keys(snapshot.errors).length > 0 ? (
        <div role="alert" className="space-y-1 text-xs text-error">
          {Object.entries(snapshot.errors).map(([scope, reason]) => (
            <p key={scope}>
              {extractErrorMessage(reason, t("settingsAdmin.loadApiConfigFailed"))}
            </p>
          ))}
        </div>
      ) : null}
      {loading && snapshot == null ? (
        <div className="inline-flex items-center gap-2 py-10 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" />
          {t("settingsAdmin.loadingConfig")}
        </div>
      ) : snapshot ? (
        <section className="grid grid-cols-1 items-start gap-3 xl:grid-cols-2" aria-label={t("settingsAdmin.sharedDefaults")}>
          <ApiSummaryCard
            icon={<Bot size={16} />}
            title={t("settingsAdmin.llmDefault")}
            empty={snapshot.llm !== null && !defaults.llm}
            unavailable={snapshot.llm === null}
            primaryRows={
              defaults.llm
                ? [
                    [t("settingsForm.configName"), configName(defaults.llm)],
                    [t("settingsForm.model"), defaults.llm.model],
                    [t("settingsForm.provider"), providerLabel(snapshot?.providers?.llm, defaults.llm.provider, t)],
                    [t("settingsForm.apiKey"), defaults.llm.has_api_key ? t("settingsAdmin.configured") : t("settingsAdmin.notConfigured")],
                  ]
                : []
            }
            diagnosticRows={
              defaults.llm
                ? [
                    [t("settingsForm.endpoint"), defaults.llm.base_url],
                    [
                      t("settingsAdmin.concurrencyTimeout"),
                      `${defaults.llm.max_concurrent} / ${defaults.llm.timeout_secs}s`,
                    ],
                    [
                      t("settingsAdmin.contextOutput"),
                      `${defaults.llm.max_context_tokens} / ${defaults.llm.max_output_tokens}`,
                    ],
                    [t("settingsForm.advancedParams"), configuredLabel(defaults.llm.extra_body_json, t)],
                  ]
                : []
            }
          />

          <ApiSummaryCard
            icon={<ImageIcon size={16} />}
            title={t("settingsAdmin.imageDefault")}
            empty={snapshot.image !== null && !defaults.image}
            unavailable={snapshot.image === null}
            primaryRows={
              defaults.image
                ? [
                    [t("settingsForm.configName"), configName(defaults.image)],
                    [t("settingsForm.model"), defaults.image.model],
                    [t("settingsAdmin.runtimeMode"), providerLabel(snapshot?.providers?.image, defaults.image.backend, t)],
                    [t("settingsForm.apiKey"), defaults.image.has_api_key ? t("settingsAdmin.configured") : t("settingsAdmin.notConfigured")],
                  ]
                : []
            }
            diagnosticRows={
              defaults.image
                ? [
                    [t("settingsForm.endpoint"), valueOrMissing(defaults.image.base_url, t)],
                    [
                      t("settingsAdmin.supportedSizes"),
                      defaults.image.supported_sizes
                        .map((size) => `${size.width} × ${size.height}`)
                        .join(" · "),
                    ],
                    [t("settingsForm.concurrency"), String(defaults.image.max_concurrent)],
                  ]
                : []
            }
          />

          <ApiSummaryCard
            icon={<Film size={16} />}
            title={t("settingsAdmin.videoDefault")}
            empty={snapshot.video !== null && !defaults.video}
            unavailable={snapshot.video === null}
            primaryRows={
              defaults.video
                ? [
                    [t("settingsForm.configName"), configName(defaults.video)],
                    [t("settingsForm.model"), defaults.video.model],
                    [t("settingsAdmin.runtimeMode"), providerLabel(snapshot?.providers?.video, defaults.video.backend, t)],
                  ]
                : []
            }
            diagnosticRows={
              defaults.video
                ? [
                    [t("settingsForm.endpoint"), valueOrMissing(defaults.video.base_url, t)],
                    [
                      t("settingsAdmin.defaultSpec"),
                      `${defaults.video.default_width} × ${defaults.video.default_height}`,
                    ],
                    [
                      t("settingsAdmin.durationFps"),
                      `${defaults.video.default_duration_secs}s / ${defaults.video.default_fps}`,
                    ],
                    [t("settingsForm.durationMode"), videoDurationText(defaults.video, t)],
                    [t("settingsForm.concurrency"), String(defaults.video.max_concurrent)],
                  ]
                : []
            }
          />

          <ApiSummaryCard
            icon={<Brain size={16} />}
            title={t("settingsAdmin.embeddingDefault")}
            empty={snapshot.embedding !== null && !defaults.embedding}
            unavailable={snapshot.embedding === null}
            primaryRows={
              defaults.embedding
                ? [
                    [t("settingsForm.configName"), configName(defaults.embedding)],
                    [t("settingsForm.model"), modelLabel(defaults.embedding.model, t)],
                    [
                      t("settingsAdmin.runtimeMode"),
                      providerLabel(snapshot?.providers?.embedding, defaults.embedding.backend, t),
                    ],
                    [t("settingsForm.apiKey"), defaults.embedding.has_api_key ? t("settingsAdmin.configured") : t("settingsAdmin.notConfigured")],
                  ]
                : []
            }
            diagnosticRows={
              defaults.embedding
                ? [
                    [t("settingsForm.endpoint"), valueOrMissing(defaults.embedding.base_url, t)],
                    [
                      t("settingsAdmin.dimensionBatch"),
                      `${defaults.embedding.dimension} / ${defaults.embedding.batch_size}`,
                    ],
                    [t("settingsAdmin.localModelPath"), valueOrMissing(defaults.embedding.local_model_path, t)],
                  ]
                : []
            }
          />

        </section>
      ) : null}
    </div>
  );
};

const ApiSummaryCard: React.FC<{
  icon: React.ReactNode;
  title: string;
  empty: boolean;
  unavailable: boolean;
  primaryRows: Array<[string, string]>;
  diagnosticRows?: Array<[string, string]>;
  diagnosticFooter?: string;
}> = ({ icon, title, empty, unavailable, primaryRows, diagnosticRows = [], diagnosticFooter }) => {
  const { t } = useI18n();
  return (
  <article className="a4n-flat-surface min-w-0 overflow-hidden">
    <div className="flex items-center gap-2 px-3 py-2.5">
      <span className="inline-flex h-6 w-6 shrink-0 items-center justify-center text-book-primary">
        {icon}
      </span>
      <h4 className="min-w-0 truncate text-xs font-medium text-book-text-main">
        {title}
      </h4>
    </div>
    {unavailable ? (
      <div role="status" className="border-t border-book-border/25 px-3 py-3 text-xs text-book-text-muted">
        {t("settingsAdmin.loadApiConfigFailed")}
      </div>
    ) : empty ? (
      <div className="border-t border-book-border/25 px-3 py-3 text-xs text-book-text-muted">
        {t("settingsAdmin.noDefault")}
      </div>
    ) : (
      <>
        <ConfigRows rows={primaryRows} />
        {(diagnosticRows.length > 0 || diagnosticFooter) && (
          <details className="border-t border-book-border/35">
            <summary className="cursor-pointer px-3 py-2.5 text-small text-book-text-muted transition-colors hover:bg-book-bg-tertiary/40 hover:text-book-text-sub">
              {t("settingsAdmin.diagnostics")}
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
};

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

function valueOrMissing(
  value: string | null | undefined,
  t: ReturnType<typeof useI18n>["t"],
): string {
  const text = value?.trim();
  return text ? text : t("settingsAdmin.notConfigured");
}

function configuredLabel(value: unknown, t: ReturnType<typeof useI18n>["t"]): string {
  if (value == null) return t("settingsAdmin.notConfigured");
  if (typeof value === "object" && Object.keys(value as Record<string, unknown>).length === 0) {
    return t("settingsAdmin.notConfigured");
  }
  return t("settingsAdmin.configured");
}

function providerLabel(
  providers: Array<{ id: string; display_name: string }> | undefined,
  value: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  return providers?.find((provider) => provider.id === value)?.display_name ?? t("settingsAdmin.unregistered", { value });
}

function modelLabel(value: string, t: ReturnType<typeof useI18n>["t"]): string {
  if (value === "local-sha256-hash") return t("settingsAdmin.builtinHash");
  return value;
}

function videoDurationText(config: VideoGenConfig, t: ReturnType<typeof useI18n>["t"]): string {
  if (config.duration_mode === "fixed") {
    return t("settingsAdmin.fixedDurationValue", { seconds: config.default_duration_secs });
  }
  return t("settingsAdmin.durationRange", {
    min: config.min_duration_secs,
    max: config.max_duration_secs,
  });
}
