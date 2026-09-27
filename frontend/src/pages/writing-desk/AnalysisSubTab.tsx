import React, { useEffect, useMemo, useState } from "react";
import { Eye, Sparkles } from "lucide-react";
import type {
  ChapterMaterialUpdate,
  ChapterMaterialSyncState,
  ChapterMaterialSyncStatus,
} from "../../api/chapter";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { FactFeedbackPanel } from "../novel-detail/FactFeedbackPanel";
import {
  listObjectProfiles,
  type ObjectProfile,
} from "../../api/objectProfiles";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import {
  analysisTextList,
  firstAnalysisText as firstText,
} from "../../utils/chapterAnalysisText";
import {
  asArray,
  asJsonRecord as asRecord,
  type JsonRecord,
} from "../../utils/jsonRecord";
import {
  getObjectProfileImportanceLabel,
  getObjectProfileTypeLabel,
} from "../../utils/objectProfilePresentation";
import {
  chapterEventImportanceLabel,
  chapterEventTypeLabel,
} from "../../utils/chapterAnalysisPresentation";
import { getChapterSyncFailurePresentation } from "../../utils/workflowPresentation";
import {
  useI18n,
  type MessageKey,
  type MessageTranslator,
} from "../../i18n";

const FORESHADOWING_CATEGORY_LABELS: Record<string, MessageKey> = {
  character_secret: "writing.analysisDetails.foreshadowingCharacterSecret",
  plot_twist: "writing.analysisDetails.foreshadowingPlotTwist",
  item_mystery: "writing.analysisDetails.foreshadowingItemMystery",
  world_rule: "writing.analysisDetails.foreshadowingWorldRule",
};

const knownLabel = (
  labels: Record<string, MessageKey>,
  value: string | null,
  t: MessageTranslator,
): string | null => {
  const key = value ? labels[value] : undefined;
  return key ? t(key) : null;
};

const ANALYSIS_CARD_CLASS =
  "a4n-flat-surface mb-2 break-inside-avoid p-3 sm:p-4";

const MATERIAL_SYNC_ROWS: ReadonlyArray<{
  key: keyof ChapterMaterialSyncState;
  label: MessageKey;
}> = [
  { key: "content_index", label: "writing.analysisDetails.syncRowContentIndex" },
  { key: "outline_and_foreshadowing", label: "writing.analysisDetails.syncRowOutlineForeshadowing" },
  { key: "objects", label: "writing.analysisDetails.syncRowObjects" },
  { key: "characters", label: "writing.analysisDetails.syncRowCharacters" },
  { key: "scenes", label: "writing.analysisDetails.syncRowScenes" },
];

const MATERIAL_SYNC_STATUS: Record<
  ChapterMaterialSyncStatus,
  { label: MessageKey; className: string }
> = {
  completed: { label: "writing.analysisDetails.statusCompleted", className: "bg-success/10 text-success" },
  not_analyzed: {
    label: "writing.analysisDetails.statusNotAnalyzed",
    className: "bg-book-bg-tertiary text-book-text-muted",
  },
  needs_analysis: {
    label: "writing.analysisDetails.statusNeedsAnalysis",
    className: "bg-warning/10 text-warning",
  },
  pending: { label: "writing.analysisDetails.statusPending", className: "bg-warning/10 text-warning" },
  running: { label: "writing.analysisDetails.statusRunning", className: "bg-info/10 text-info" },
  failed: { label: "writing.analysisDetails.statusFailed", className: "bg-error/10 text-error" },
  unknown: {
    label: "writing.analysisDetails.statusUnknown",
    className: "bg-book-bg-tertiary text-book-text-muted",
  },
};

const MATERIAL_UPDATE_LABELS: Record<string, MessageKey> = {
  "outline:created": "writing.analysisDetails.updateOutlineCreated",
  "outline:updated": "writing.analysisDetails.updateOutlineUpdated",
  "outline:existing": "writing.analysisDetails.updateOutlineExisting",
  "outline:preserved": "writing.analysisDetails.updateOutlinePreserved",
  "foreshadowing:created": "writing.analysisDetails.updateForeshadowingCreated",
  "foreshadowing:updated": "writing.analysisDetails.updateForeshadowingUpdated",
  "foreshadowing:existing": "writing.analysisDetails.updateForeshadowingExisting",
  "foreshadowing:resolved": "writing.analysisDetails.updateForeshadowingResolved",
  "object:created": "writing.analysisDetails.updateObjectCreated",
  "object:updated": "writing.analysisDetails.updateObjectUpdated",
  "character:created": "writing.analysisDetails.updateCharacterCreated",
  "character:existing": "writing.analysisDetails.updateCharacterExisting",
  "character:preserved": "writing.analysisDetails.updateCharacterPreserved",
  "character:synced": "writing.analysisDetails.updateCharacterSynced",
  "scene:synced": "writing.analysisDetails.updateSceneSynced",
};

export const materialUpdateDetails = (
  updates: ChapterMaterialUpdate[],
  t: MessageTranslator,
): string[] =>
  updates.map((update) => {
    const label = t(
      MATERIAL_UPDATE_LABELS[`${update.material_kind}:${update.action}`] ??
        "writing.analysisDetails.updateFallback",
    );
    return t("writing.analysisDetails.updateItem", { label, name: update.name });
  });

export const materialUpdateSummary = (
  updates: ChapterMaterialUpdate[],
  t: MessageTranslator,
): string | null => {
  const details = materialUpdateDetails(updates, t);
  if (details.length === 0) return null;
  const visible = details.slice(0, 8);
  const remaining = updates.length - visible.length;
    return t("writing.analysisDetails.updateSummaryValue", {
    details: visible.join(t("common.listSeparator")),
    remaining: remaining > 0 ? t("writing.analysisDetails.moreUpdates", { count: remaining }) : "",
  });
};

export const objectProfilesForChapter = (
  profiles: ObjectProfile[],
  chapterNumber: number,
): ObjectProfile[] =>
  profiles.filter(
    (profile) =>
      !(
        profile.ownership_kind === "user" &&
        asArray(profile.evidence_json).length === 0
      ) &&
      profile.first_seen_chapter <= chapterNumber &&
      profile.last_seen_chapter >= chapterNumber,
  );

const MaterialSyncPanel: React.FC<{
  state: ChapterMaterialSyncState;
  updates: ChapterMaterialUpdate[];
}> = ({ state, updates }) => {
  const { t } = useI18n();
  const updateDetails = materialUpdateDetails(updates, t);
  const updateSummary = materialUpdateSummary(updates, t);
  return (
    <section
      className="a4n-flat-surface p-3 sm:p-4"
      aria-label={t("writing.analysisDetails.syncAria")}
    >
      <div className="mb-2">
        <div className="text-small font-semibold text-book-text-main">
          {t("writing.analysisDetails.syncHeading")}
        </div>
        <p className="mt-0.5 text-caption text-book-text-muted">
          {t("writing.analysisDetails.syncDescription")}
        </p>
      </div>
      <dl className="grid grid-cols-2 gap-2 sm:grid-cols-5">
        {MATERIAL_SYNC_ROWS.map(({ key, label }) => {
          const presentation =
            MATERIAL_SYNC_STATUS[state[key]] ?? MATERIAL_SYNC_STATUS.unknown;
          return (
            <div
              key={key}
              className="flex min-w-0 flex-wrap items-center justify-between gap-x-2 gap-y-1 rounded-lg bg-book-bg/55 px-2.5 py-2"
            >
              <dt className="text-caption text-book-text-muted">
                {t(label)}
              </dt>
              <dd
                className={`w-fit shrink-0 rounded-full px-2 py-0.5 text-caption font-medium ${presentation.className}`}
              >
                {t(presentation.label)}
              </dd>
            </div>
          );
        })}
      </dl>
      {updateSummary && updateDetails.length > 8 ? (
        <details className="mt-2 text-caption leading-relaxed text-book-text-sub">
          <summary className="cursor-pointer">{t("writing.analysisDetails.updateSummary", { summary: updateSummary })}</summary>
          <ul
            className="mt-1.5 list-disc space-y-1 pl-5"
            aria-label={t("writing.analysisDetails.otherUpdatesAria")}
          >
            {updateDetails.slice(8).map((detail, index) => (
              <li key={`${detail}-${index}`}>{detail}</li>
            ))}
          </ul>
        </details>
      ) : updateSummary ? (
        <p className="mt-2 text-caption leading-relaxed text-book-text-sub">
          {t("writing.analysisDetails.updateSummary", { summary: updateSummary })}
        </p>
      ) : null}
    </section>
  );
};

interface AnalysisSubTabProps {
  projectId: number;
  chapterNumber: number;
  analysis: unknown;
  materialSync: ChapterMaterialSyncState;
  materialUpdates: ChapterMaterialUpdate[];
  analysisSyncStatus: string | null;
  analysisSyncError: string | null;
  analysisSyncHasSavedAnalysis: boolean;
  analysisBlockedReason: string | null;
  generating: boolean;
  canGenerate: boolean;
  canEditProject: boolean;
  onGenerate: () => void | Promise<void>;
  onPreviewGenerate: () => void | Promise<void>;
  canResumeSync: boolean;
  onResumeSync: () => void | Promise<void>;
  onPreviewResumeSync: () => void | Promise<void>;
}

export const AnalysisSubTab: React.FC<AnalysisSubTabProps> = ({
  projectId,
  chapterNumber,
  analysis,
  materialSync,
  materialUpdates,
  analysisSyncStatus,
  analysisSyncError,
  analysisSyncHasSavedAnalysis,
  analysisBlockedReason,
  generating,
  canGenerate,
  canEditProject,
  onGenerate,
  onPreviewGenerate,
  canResumeSync,
  onResumeSync,
  onPreviewResumeSync,
}) => {
  const { t } = useI18n();
  const [objectProfiles, setObjectProfiles] = useState<ObjectProfile[]>([]);
  const [objectProfilesProjectId, setObjectProfilesProjectId] = useState<number | null>(null);
  const [objectProfilesError, setObjectProfilesError] = useState<string | null>(
    null,
  );
  const [objectProfilesLoading, setObjectProfilesLoading] = useState(
    Boolean(analysis),
  );

  useEffect(() => {
    let cancelled = false;
    if (objectProfilesProjectId !== projectId) {
      setObjectProfiles([]);
      setObjectProfilesProjectId(projectId);
      setObjectProfilesError(null);
    }
    if (!analysis) {
      setObjectProfiles([]);
      setObjectProfilesError(null);
      setObjectProfilesLoading(false);
      return;
    }
    setObjectProfilesLoading(true);
    void listObjectProfiles(projectId)
      .then((rows) => {
        if (cancelled) return;
        setObjectProfiles(rows);
        setObjectProfilesProjectId(projectId);
        setObjectProfilesError(null);
      })
      .catch((err) => {
        if (cancelled) return;
        setObjectProfilesError(
          err instanceof Error ? err.message : String(err),
        );
      })
      .finally(() => {
        if (cancelled) return;
        setObjectProfilesLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [analysis, objectProfilesProjectId, projectId]);

  const chapterObjects = useMemo(
    () => objectProfilesProjectId === projectId
      ? objectProfilesForChapter(objectProfiles, chapterNumber)
      : [],
    [chapterNumber, objectProfiles, objectProfilesProjectId, projectId],
  );

  const root = asRecord(analysis);
  const syncFailure = ["failed", "interrupted", "cancelled"].includes(
    analysisSyncStatus ?? "",
  )
      ? getChapterSyncFailurePresentation(
        analysisSyncError,
        t("writing.analysisDetails.syncFailureFallback"),
        t,
      )
    : null;
  const needsReanalysis = Object.values(materialSync).includes("needs_analysis");
  if (!root) {
    const emptyStateTitle = generating
      ? t("writing.analysisDetails.emptyAnalyzing", { chapter: chapterNumber })
      : canResumeSync
        ? t("writing.analysisDetails.emptyCanResume", { chapter: chapterNumber })
        : analysisBlockedReason
          ? t("writing.analysisDetails.emptyBlocked", { chapter: chapterNumber })
          : needsReanalysis
            ? t("writing.analysisDetails.emptyNeedsReanalysis", { chapter: chapterNumber })
            : t("writing.analysisDetails.emptyNotGenerated", { chapter: chapterNumber });

    return (
      <div className="h-full min-h-0 overflow-y-auto p-4 text-xs">
        <div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
          <MaterialSyncPanel state={materialSync} updates={materialUpdates} />
          <div className="flex flex-col items-center py-4 text-center">
            {generating ? (
              <LoadingSpinner size="lg" className="mb-3" />
            ) : (
              <Sparkles size={20} className="mb-3 text-book-text-muted" />
            )}
            <h3 className="text-sm font-semibold text-book-text-main">
              {emptyStateTitle}
            </h3>
            <p
              className={`mt-1.5 max-w-sm leading-relaxed ${
                analysisBlockedReason || syncFailure
                  ? "text-error"
                  : "text-book-text-muted"
              }`}
            >
              {analysisBlockedReason ??
                syncFailure?.summary ??
                (needsReanalysis
                  ? t("writing.analysisDetails.staleDescription")
                  : t("writing.analysisDetails.generatedDescription"))}
            </p>
            {!analysisBlockedReason && syncFailure?.detail ? (
              <details className="mt-2 max-w-sm text-left text-caption text-book-text-muted">
                <summary className="w-fit cursor-pointer rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main">
                  {t("writing.diagnostics")}
                </summary>
                <pre className="mt-1.5 max-h-28 overflow-auto whitespace-pre-wrap break-all rounded-lg bg-book-bg-tertiary/70 p-2 font-mono leading-relaxed custom-scrollbar">
                  {syncFailure.detail}
                </pre>
              </details>
            ) : null}
            {!generating && (canResumeSync || canGenerate) ? (
              <div className="mt-4 flex w-full max-w-xs items-center gap-2">
                {canGenerate ? (
                  <Dropdown
                    items={[
                      {
                        label: canResumeSync
                          ? t("writing.analysisDetails.previewResumeRequest")
                          : needsReanalysis
                            ? t("writing.analysisDetails.previewResumeRequest")
                            : t("writing.analysisDetails.previewModelRequest"),
                        icon: <Eye size={12} />,
                        onClick: () => void onPreviewGenerate(),
                      },
                    ]}
                  />
                ) : null}
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={() => {
                    if (canResumeSync) {
                      void onResumeSync();
                      return;
                    }
                    void onGenerate();
                  }}
                  className="flex-1"
                >
                  <Sparkles size={13} />
                  {canResumeSync
                    ? t("writing.analysisDetails.continueAnalysis")
                    : needsReanalysis
                      ? t("writing.analysisDetails.regenerateAnalysis")
                      : t("writing.analysisDetails.generateAnalysis")}
                </BookButton>
              </div>
            ) : null}
          </div>
        </div>
      </div>
    );
  }

  const metadata = asRecord(root.metadata);
  const summaries = asRecord(root.summaries);
  const characterStates = asRecord(root.character_states);
  const foreshadowing = asRecord(root.foreshadowing);
  const keyEvents = asArray(root.key_events)
    .map(asRecord)
    .filter(Boolean) as JsonRecord[];

  const summary =
    firstText(summaries, ["compressed", "summary", "short"]) ||
    firstText(summaries, ["one_line"]);
  const keywords = analysisTextList(summaries?.keywords, { limit: 8 });
  const metadataRows = [
    {
      label: t("writing.analysisDetails.metadataCharacters"),
      value: analysisTextList(metadata?.characters, { limit: 12 }).join(t("common.listSeparator")),
    },
    {
      label: t("writing.analysisDetails.metadataLocations"),
      value: analysisTextList(metadata?.locations, { limit: 8 }).join(t("common.listSeparator")),
    },
    {
      label: t("writing.analysisDetails.metadataObjects"),
      value: analysisTextList(metadata?.items, { limit: 8 }).join(t("common.listSeparator")),
    },
    {
      label: t("writing.analysisDetails.metadataTags"),
      value: analysisTextList(metadata?.tags, { limit: 8 }).join(t("common.listSeparator")),
    },
    { label: t("writing.analysisDetails.metadataTone"), value: firstText(metadata, ["tone"]) },
    { label: t("writing.analysisDetails.metadataTime"), value: firstText(metadata, ["timeline_marker"]) },
  ].filter((row): row is { label: string; value: string } =>
    Boolean(row.value),
  );
  const stateRows = characterStates
    ? Object.entries(characterStates)
        .map(([name, raw]) => ({ name, data: asRecord(raw) }))
        .filter((row): row is { name: string; data: JsonRecord } =>
          Boolean(row.data),
        )
    : [];
  const planted = asArray(foreshadowing?.planted)
    .map(asRecord)
    .filter(Boolean) as JsonRecord[];
  const resolved = asArray(foreshadowing?.resolved)
    .map(asRecord)
    .filter(Boolean) as JsonRecord[];
  const tensions = analysisTextList(foreshadowing?.tensions, { limit: 10 });
  const hasForeshadowing =
    planted.length > 0 || resolved.length > 0 || tensions.length > 0;
  const secondaryActions: React.ComponentProps<typeof Dropdown>["items"] = [
    ...(canResumeSync &&
      analysisSyncHasSavedAnalysis &&
      analysisSyncStatus !== "running"
      ? [
          {
            label: t("writing.analysisDetails.previewSyncRequest"),
            icon: <Eye size={12} />,
            onClick: () => void onPreviewResumeSync(),
          },
        ]
      : []),
    ...(canGenerate
      ? [
          {
            label: canResumeSync
              ? t("writing.analysisDetails.previewResumeRequest")
              : t("writing.analysisDetails.previewAnalysisRequest"),
            icon: <Eye size={12} />,
            onClick: () => void onPreviewGenerate(),
          },
        ]
      : []),
    ...(canResumeSync && canGenerate
      ? [
          { type: "divider" as const },
          {
            label: t("writing.analysisDetails.regenerateAnalysis"),
            icon: <Sparkles size={12} />,
            onClick: () => void onGenerate(),
          },
        ]
      : []),
  ];

  return (
    <div className="h-full overflow-y-auto custom-scrollbar p-2 text-xs text-book-text-sub">
      <div className="mx-auto mb-2 w-full max-w-5xl space-y-2 sm:flex sm:items-center sm:justify-end sm:space-y-0 sm:gap-2">
        {analysisBlockedReason ? (
          <p className="min-w-0 text-small leading-relaxed text-error sm:mr-auto sm:flex-1">
            {analysisBlockedReason}
          </p>
        ) : null}
        {!analysisBlockedReason && syncFailure ? (
          <div className="min-w-0 sm:mr-auto sm:flex-1">
            <p className="text-small leading-relaxed text-error">
              {syncFailure.summary}
            </p>
            {syncFailure.detail ? (
              <details className="mt-1 text-caption text-book-text-muted">
                <summary className="w-fit cursor-pointer hover:text-book-text-main">
                  {t("writing.diagnostics")}
                </summary>
                <pre className="mt-1.5 max-h-28 overflow-auto whitespace-pre-wrap break-all rounded-lg bg-book-bg-tertiary/70 p-2 font-mono leading-relaxed custom-scrollbar">
                  {syncFailure.detail}
                </pre>
              </details>
            ) : null}
          </div>
        ) : null}
        {generating || canResumeSync || canGenerate ? (
          <div className="flex items-center gap-2 sm:contents">
            {!generating && secondaryActions.length > 0 ? (
              <Dropdown items={secondaryActions} />
            ) : null}
            <BookButton
              variant="primary"
              size="sm"
              className="flex-1 sm:flex-none"
              onClick={() => {
                if (canResumeSync) {
                  void onResumeSync();
                  return;
                }
                void onGenerate();
              }}
              disabled={generating}
            >
              <span className="inline-flex items-center gap-1.5">
                {generating ? (
                  <LoadingSpinner size="sm" />
                ) : (
                  <Sparkles size={13} />
                )}
                {generating
                  ? t("writing.analysisDetails.processing")
                  : canResumeSync
                    ? analysisSyncHasSavedAnalysis
                      ? t("writing.analysisDetails.continueSync")
                      : t("writing.analysisDetails.continueAnalysis")
                    : t("writing.analysisDetails.regenerateAnalysis")}
              </span>
            </BookButton>
          </div>
        ) : null}
      </div>
      <div className="mx-auto mb-2 w-full max-w-5xl">
        <MaterialSyncPanel state={materialSync} updates={materialUpdates} />
        <div className="mt-2"><FactFeedbackPanel key={projectId} projectId={projectId} canEdit={canEditProject} refreshKey={`${analysisSyncStatus}:${JSON.stringify(asRecord(analysis)?._meta)}`} /></div>
      </div>
      <div className="mx-auto w-full max-w-5xl columns-1 gap-2 sm:columns-2">
        {asArray(asRecord(analysis)?.world_facts).length > 0 ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">{t("novel.factFeedback.world")}</h3>
            <ul className="space-y-3">
              {asArray(asRecord(analysis)?.world_facts).map((raw, index) => {
                const fact = asRecord(raw);
                if (!fact || typeof fact.statement !== "string") return null;
                const assertion = fact.assertion === "observed" ? "novel.factFeedback.observed" : fact.assertion === "inferred" ? "novel.factFeedback.inferred" : fact.assertion === "reported" ? "novel.factFeedback.reported" : fact.assertion === "recollection" ? "novel.factFeedback.recollection" : "novel.factFeedback.hypothetical";
                return <li key={index}>
                  <p className="text-caption text-book-text-muted">{t(assertion)}</p>
                  <p className="mt-1 whitespace-pre-wrap leading-relaxed">{fact.statement}</p>
                  {analysisTextList(fact.evidence).map((quote, quoteIndex) => <blockquote key={quoteIndex} className="mt-1 border-l-2 border-book-border pl-2 text-caption text-book-text-muted">{quote}</blockquote>)}
                </li>;
              })}
            </ul>
          </section>
        ) : null}
        {summary || keywords.length > 0 ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">
              {t("writing.analysisDetails.summary")}
            </h3>
            {summary ? (
              <p className="whitespace-pre-wrap leading-relaxed">{summary}</p>
            ) : null}
            {keywords.length > 0 ? (
              <p
                className={`${summary ? "mt-2" : ""} text-small text-book-text-muted`}
              >
                {t("writing.analysisDetails.keywords", {
                  value: keywords.join(t("common.listSeparator")),
                })}
              </p>
            ) : null}
          </section>
        ) : null}

        {metadataRows.length > 0 ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">
              {t("writing.analysisDetails.chapterElements")}
            </h3>
            <dl className="space-y-2">
              {metadataRows.map(({ label, value }) => (
                <div
                  key={label}
                  className="grid grid-cols-[42px_minmax(0,1fr)] gap-2 leading-relaxed"
                >
                  <dt className="text-book-text-muted">{label}</dt>
                  <dd className="min-w-0 text-book-text-sub">{value}</dd>
                </div>
              ))}
            </dl>
          </section>
        ) : null}

        {keyEvents.length > 0 ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">
              {t("writing.analysisDetails.keyEvents")}
            </h3>
            <ul className="divide-y divide-book-border/35">
              {keyEvents.map((event, idx) => {
                const desc = firstText(event, [
                  "description",
                  "summary",
                  "event",
                ]);
                if (!desc) return null;
                const involved = analysisTextList(event.involved_characters, {
                  limit: 8,
                });
                const eventType = chapterEventTypeLabel(
                  firstText(event, ["type"]),
                  t,
                );
                const importance = chapterEventImportanceLabel(
                  firstText(event, ["importance"]),
                  t,
                );
                const detail = [eventType, importance, ...involved]
                  .filter(Boolean)
                  .join(" · ");
                return (
                  <li
                    key={`${desc}-${idx}`}
                    className="py-2 first:pt-0 last:pb-0"
                  >
                    <div className="font-medium leading-relaxed text-book-text-main">
                      {desc}
                    </div>
                    {detail ? (
                      <div className="mt-0.5 text-small text-book-text-muted">
                        {detail}
                      </div>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          </section>
        ) : null}

        <section className={ANALYSIS_CARD_CLASS}>
          <h3 className="mb-2 text-small font-semibold text-book-text-main">
            {t("writing.analysisDetails.identifiedObjects")}
          </h3>
          {objectProfilesLoading ? (
            <div className="flex items-center gap-2 text-small text-book-text-muted">
              <LoadingSpinner size="sm" />
              {t("writing.analysisDetails.objectProfilesLoading")}
            </div>
          ) : objectProfilesError ? (
            <div>
              <p className="text-small text-error">{t("writing.analysisDetails.objectProfilesLoadFailed")}</p>
              <details className="mt-2 text-caption text-book-text-muted">
                <summary className="w-fit cursor-pointer hover:text-book-text-main">
                  {t("writing.diagnostics")}
                </summary>
                <pre className="mt-1.5 whitespace-pre-wrap break-all rounded-lg bg-book-bg-tertiary/70 p-2 font-mono leading-relaxed">
                  {objectProfilesError}
                </pre>
              </details>
            </div>
          ) : chapterObjects.length > 0 ? (
            <ul className="divide-y divide-book-border/35">
              {chapterObjects.slice(0, 8).map((item) => (
                <li key={item.id} className="py-2 first:pt-0 last:pb-0">
                  <div className="font-medium text-book-text-main">
                    {item.name}
                  </div>
                  <div className="mt-0.5 text-small text-book-text-muted">
                    {getObjectProfileImportanceLabel(item.importance, t)} ·{" "}
                    {getObjectProfileTypeLabel(item.object_type, t)}
                  </div>
                  {item.story_usage ? (
                    <div className="mt-1 line-clamp-2 leading-relaxed text-book-text-sub">
                      {item.story_usage}
                    </div>
                  ) : null}
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-small text-book-text-muted">
              {t("writing.analysisDetails.noObjectProfiles")}
            </p>
          )}
        </section>

        {stateRows.length > 0 ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">
              {t("writing.analysisDetails.characterStates")}
            </h3>
            <ul className="divide-y divide-book-border/35">
              {stateRows.map(({ name, data }) => {
                const changes = analysisTextList(data.changes, { limit: 6 });
                const status = firstText(data, ["status"]);
                const context = [
                  firstText(data, ["location", "current_location"]),
                  firstText(data, ["emotional_state", "emotion"]),
                ]
                  .filter(Boolean)
                  .join(" · ");
                return (
                  <li
                    key={name}
                    className="grid grid-cols-[72px_minmax(0,1fr)] gap-2 py-2 first:pt-0 last:pb-0"
                  >
                    <span
                      className="truncate font-medium text-book-text-main"
                      title={name}
                    >
                      {name}
                    </span>
                    <div className="min-w-0">
                      {status && (
                        <div className="leading-relaxed">{status}</div>
                      )}
                      {context ? (
                        <div className="mt-0.5 text-small text-book-text-muted">
                          {context}
                        </div>
                      ) : null}
                      {changes.length > 0 && (
                        <div className="mt-1 text-small leading-relaxed text-book-text-sub">
                          {t("writing.analysisDetails.changes", {
                            value: changes.join(t("common.listSeparator")),
                          })}
                        </div>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
          </section>
        ) : null}

        {hasForeshadowing ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">
              {t("writing.analysisDetails.foreshadowing")}
            </h3>
            <div className="space-y-3">
              {planted.length > 0 && (
                <div>
                  <div className="mb-1 text-small text-book-text-muted">
                    {t("writing.analysisDetails.planted")}
                  </div>
                  <ul className="divide-y divide-book-border/35">
                    {planted.map((item, idx) => {
                      const desc = firstText(item, ["description", "content"]);
                      if (!desc) return null;
                      const tags = [
                        knownLabel(
                          FORESHADOWING_CATEGORY_LABELS,
                          firstText(item, ["category", "type"]),
                          t,
                        ),
                        chapterEventImportanceLabel(
                          firstText(item, ["priority", "importance"]),
                          t,
                        ),
                      ].filter((tag): tag is string => tag != null);
                      return (
                        <li
                          key={`${desc}-${idx}`}
                          className="py-2 first:pt-0 last:pb-0"
                        >
                          <div className="leading-relaxed text-book-text-main">
                            {desc}
                          </div>
                          {tags.length > 0 ? (
                            <div className="mt-0.5 text-small text-book-text-muted">
                              {tags.join(" · ")}
                            </div>
                          ) : null}
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
              {resolved.length > 0 && (
                <div>
                  <div className="mb-1 text-small text-book-text-muted">
                    {t("writing.analysisDetails.resolved")}
                  </div>
                  <ul className="divide-y divide-book-border/35">
                    {resolved.map((item, idx) => {
                      const desc = firstText(item, [
                        "resolution",
                        "description",
                        "content",
                      ]);
                      if (!desc) return null;
                      return (
                        <li
                          key={idx}
                          className="py-2 leading-relaxed first:pt-0 last:pb-0"
                        >
                          {desc}
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
              {tensions.length > 0 && (
                <div>
                  <div className="mb-1 text-small text-book-text-muted">
                    {t("writing.analysisDetails.tensions")}
                  </div>
                  <ul className="divide-y divide-book-border/35">
                    {tensions.map((item) => (
                      <li
                        key={item}
                        className="py-2 leading-relaxed first:pt-0 last:pb-0"
                      >
                        {item}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </section>
        ) : null}
      </div>
    </div>
  );
};
