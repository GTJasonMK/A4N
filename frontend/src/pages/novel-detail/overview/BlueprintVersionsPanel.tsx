import React, { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { Check } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  diffBlueprintVersions,
  listBlueprintVersions,
  restoreBlueprintVersion,
  type BlueprintVersion,
} from "../../../api/blueprint";
import { formatDateTime, useI18n, type MessageKey, type MessageTranslator } from "../../../i18n";

const blueprintVersionLabel = (version: string, t: MessageTranslator): string =>
  t("novel.blueprintVersions.versionLabel", { version });

interface BlueprintVersionsPanelProps {
  projectId: number;
  onMutated: () => Promise<boolean>;
}

export const BlueprintVersionsPanel: React.FC<BlueprintVersionsPanelProps> = ({
  projectId,
  onMutated,
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const [versions, setVersions] = useState<BlueprintVersion[]>([]);
  const [loading, setLoading] = useState(false);
  const [picked, setPicked] = useState<string[]>([]);
  const [diff, setDiff] = useState<unknown>(null);
  const [diffing, setDiffing] = useState(false);
  const [restoring, setRestoring] = useState<string | null>(null);
  const loadSeqRef = useRef(0);
  const diffSeqRef = useRef(0);
  const mountedRef = useRef(true);
  const activeProjectRef = useRef(projectId);
  const loadingRef = useRef(false);
  const diffingRef = useRef(false);
  const restoringRef = useRef<string | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      diffSeqRef.current += 1;
      loadingRef.current = false;
      diffingRef.current = false;
      restoringRef.current = null;
    };
  }, []);

  const load = useCallback(async (): Promise<boolean> => {
    if (loadingRef.current || restoringRef.current != null) return false;
    const requestProjectId = projectId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    loadingRef.current = true;
    setLoading(true);
    try {
      const data = await listBlueprintVersions(requestProjectId);
      if (
        !mountedRef.current ||
        activeProjectRef.current !== requestProjectId ||
        loadSeqRef.current !== requestSeq
      ) {
        return false;
      }
      setVersions(data);
      return true;
    } catch (err) {
      if (
        !mountedRef.current ||
        activeProjectRef.current !== requestProjectId ||
        loadSeqRef.current !== requestSeq
      ) {
        return false;
      }
      addToast(extractErrorMessage(err, t("novel.blueprintVersions.loadFailed")), "error");
      return false;
    } finally {
      if (loadSeqRef.current === requestSeq) loadingRef.current = false;
      if (
        mountedRef.current &&
        activeProjectRef.current === requestProjectId &&
        loadSeqRef.current === requestSeq
      ) {
        setLoading(false);
      }
    }
  }, [addToast, projectId, t]);

  useLayoutEffect(() => {
    activeProjectRef.current = projectId;
    loadSeqRef.current += 1;
    diffSeqRef.current += 1;
    loadingRef.current = false;
    diffingRef.current = false;
    restoringRef.current = null;
    setVersions([]);
    setPicked([]);
    setDiff(null);
    setLoading(false);
    setDiffing(false);
    setRestoring(null);
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const available = new Set(versions.map((version) => version.version));
    setPicked((prev) => {
      const next = prev.filter((version) => available.has(version));
      if (next.length !== prev.length) setDiff(null);
      return next;
    });
  }, [versions]);

  const togglePick = (version: string) => {
    diffSeqRef.current += 1;
    diffingRef.current = false;
    setDiffing(false);
    setPicked((prev) => {
      const has = prev.includes(version);
      if (has) return prev.filter((item) => item !== version);
      const next = [...prev, version];
      return next.length > 2 ? next.slice(-2) : next;
    });
    setDiff(null);
  };

  const handleDiff = async () => {
    if (diffingRef.current || restoringRef.current != null) return;
    if (picked.length !== 2) {
      addToast(t("novel.blueprintVersions.selectTwo"), "error");
      return;
    }
    const requestProjectId = projectId;
    const requestPair = [...picked] as [string, string];
    const requestSeq = diffSeqRef.current + 1;
    diffSeqRef.current = requestSeq;
    diffingRef.current = true;
    setDiffing(true);
    try {
      const data = await diffBlueprintVersions({
        project_id: requestProjectId,
        version_a: requestPair[0],
        version_b: requestPair[1],
      });
      if (
        !mountedRef.current ||
        activeProjectRef.current !== requestProjectId ||
        diffSeqRef.current !== requestSeq
      ) {
        return;
      }
      setDiff(data.diff);
    } catch (err) {
      if (
        !mountedRef.current ||
        activeProjectRef.current !== requestProjectId ||
        diffSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, t("novel.blueprintVersions.diffFailed")), "error");
    } finally {
      if (diffSeqRef.current === requestSeq) diffingRef.current = false;
      if (
        mountedRef.current &&
        activeProjectRef.current === requestProjectId &&
        diffSeqRef.current === requestSeq
      ) {
        setDiffing(false);
      }
    }
  };

  const handleRestore = async (version: BlueprintVersion) => {
    if (restoringRef.current != null) return;
    const requestProjectId = projectId;
    const versionLabel = blueprintVersionLabel(version.version, t);
    restoringRef.current = version.version;
    const ok = await confirmDialog({
      title: t("novel.blueprintVersions.restoreTitle"),
      message: t("novel.blueprintVersions.restoreMessage", { version: versionLabel }),
      confirmText: t("novel.blueprintVersions.restore"),
      dialogType: "warning",
    });
    if (!ok) {
      if (restoringRef.current === version.version) restoringRef.current = null;
      return;
    }
    if (!mountedRef.current || activeProjectRef.current !== requestProjectId) {
      if (restoringRef.current === version.version) restoringRef.current = null;
      return;
    }
    loadSeqRef.current += 1;
    diffSeqRef.current += 1;
    loadingRef.current = false;
    diffingRef.current = false;
    setLoading(false);
    setDiffing(false);
    setDiff(null);
    setRestoring(version.version);
    try {
      const result = await restoreBlueprintVersion({
        project_id: projectId,
        version: version.version,
      });
      if (!mountedRef.current || activeProjectRef.current !== requestProjectId) return;
      if (!await onMutated()) return;
      if (!mountedRef.current || activeProjectRef.current !== requestProjectId) return;
      if (!await load()) return;
      addToast(
        result.warnings.length > 0
          ? t("novel.blueprintVersions.restoredWithWarnings", {
              version: versionLabel,
              warnings: result.warnings.join(t("common.summarySeparator")),
            })
          : t("novel.blueprintVersions.restored", { version: versionLabel }),
        result.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (!mountedRef.current || activeProjectRef.current !== requestProjectId) return;
      addToast(extractErrorMessage(err, t("novel.blueprintVersions.restoreFailed")), "error");
    } finally {
      if (restoringRef.current === version.version) restoringRef.current = null;
      if (mountedRef.current && activeProjectRef.current === requestProjectId) setRestoring(null);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-small leading-relaxed text-book-text-muted">
          {versions.length > 0
            ? t("novel.blueprintVersions.summary", { count: versions.length })
            : t("novel.blueprintVersions.emptySummary")}
        </p>
        <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
          <Dropdown
            items={[
              {
                label: loading ? t("novel.blueprintVersions.refreshing") : t("novel.blueprintVersions.refresh"),
                icon: loading ? <LoadingSpinner size="sm" /> : undefined,
                onClick: load,
                disabled: loading || restoring != null,
              },
            ]}
          />
          {picked.length === 2 ? (
            <BookButton
              variant="secondary"
              size="sm"
              onClick={handleDiff}
              disabled={diffing}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {diffing ? (
                <span className="inline-flex items-center gap-2">
                  <LoadingSpinner size="sm" /> {t("novel.blueprintVersions.comparing")}
                </span>
              ) : (
                t("novel.blueprintVersions.compareSelected")
              )}
            </BookButton>
          ) : null}
        </div>
      </div>

      {loading && versions.length === 0 ? (
        <div className="text-xs text-book-text-muted inline-flex items-center gap-2">
          <LoadingSpinner size="sm" /> {t("novel.blueprintVersions.loading")}
        </div>
      ) : versions.length === 0 ? (
        <div className="text-xs text-book-text-muted py-3">
          {t("novel.blueprintVersions.empty")}
        </div>
      ) : (
        <>
          <ul className="a4n-flat-surface divide-y divide-book-border/35 overflow-hidden">
            {versions.map((version) => {
              const isPicked = picked.includes(version.version);
              const versionLabel = blueprintVersionLabel(version.version, t);
              return (
                <li
                  key={version.id}
                  className={`flex items-start gap-2.5 px-3 py-2.5 text-xs transition-colors ${
                    isPicked ? "bg-book-primary/10" : ""
                  }`}
                >
                  <button
                    type="button"
                    aria-pressed={isPicked}
                    aria-label={t("novel.blueprintVersions.select", { version: versionLabel })}
                    onClick={() => togglePick(version.version)}
                    className={`mt-0.5 inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-full border transition-colors focus-visible:outline-2 focus-visible:outline-book-primary/35 ${
                      isPicked
                        ? "border-book-primary bg-book-primary text-white"
                        : "border-book-border/70 bg-book-bg-paper/45 text-transparent hover:border-book-primary/55"
                    }`}
                  >
                    <Check size={12} />
                  </button>
                  <span className="min-w-0 flex-1" title={version.version}>
                    <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                      <span className="font-medium text-book-text-main">{versionLabel}</span>
                      <time className="text-caption tabular-nums text-book-text-muted">
                        {formatDateTime(version.created_at, locale)}
                      </time>
                      <span className="text-caption text-book-text-muted">
                        {version.created_by}
                      </span>
                    </span>
                    <span className="mt-0.5 block break-words text-small text-book-text-sub">
                      {version.change_description}
                    </span>
                  </span>
                  {restoring != null ? (
                    restoring === version.version ? (
                      <span className="inline-flex h-8 w-8 shrink-0 items-center justify-center text-book-text-muted">
                        <LoadingSpinner size="sm" />
                      </span>
                    ) : (
                      <span className="h-8 w-8 shrink-0" />
                    )
                  ) : (
                    <Dropdown
                      items={[
                        {
                          label: t("novel.blueprintVersions.restoreThis"),
                          danger: true,
                          onClick: () => void handleRestore(version),
                        },
                      ]}
                    />
                  )}
                </li>
              );
            })}
          </ul>
          {diff != null && (
            <div className="mt-3 pt-3 border-t border-book-border/30">
              <div className="text-xs text-book-text-muted mb-1.5">
                {t("novel.blueprintVersions.selectedDiff")}
              </div>
              <BlueprintDiffSummary diff={diff} />
            </div>
          )}
        </>
      )}
    </div>
  );
};

const BLUEPRINT_DIFF_LABELS: Record<string, MessageKey> = {
  title: "novel.blueprintVersions.diffLabels.title",
  genre: "novel.blueprintVersions.diffLabels.genre",
  style: "novel.blueprintVersions.diffLabels.style",
  tone: "novel.blueprintVersions.diffLabels.tone",
  target_audience: "novel.blueprintVersions.diffLabels.targetAudience",
  full_synopsis: "novel.blueprintVersions.diffLabels.fullSynopsis",
  world_setting: "novel.blueprintVersions.diffLabels.worldSetting",
  total_chapters: "novel.blueprintVersions.diffLabels.totalChapters",
  characters: "novel.blueprintVersions.diffLabels.characters",
  added: "novel.blueprintVersions.diffLabels.added",
  removed: "novel.blueprintVersions.diffLabels.removed",
  changed: "novel.blueprintVersions.diffLabels.changed",
  old: "novel.blueprintVersions.diffLabels.old",
  new: "novel.blueprintVersions.diffLabels.new",
};

const BlueprintDiffSummary: React.FC<{ diff: unknown }> = ({ diff }) => {
  const { t } = useI18n();
  if (typeof diff === "string") {
    return <div className="a4n-flat-surface whitespace-pre-wrap break-words p-3 text-small leading-relaxed text-book-text-sub">{diff}</div>;
  }
  const entries =
    diff && typeof diff === "object" && !Array.isArray(diff)
      ? Object.entries(diff as Record<string, unknown>)
      : [];
  if (entries.length === 0) {
    return <div className="text-small text-book-text-muted">{t("novel.blueprintVersions.noDiff")}</div>;
  }
  return (
    <div className="a4n-flat-surface divide-y divide-book-border/40">
        {entries.map(([key, value]) => (
          <div key={key} className="grid gap-1 px-3 py-2 text-small sm:grid-cols-[88px_minmax(0,1fr)] sm:gap-3">
            <span className="font-medium text-book-text-muted">
              {BLUEPRINT_DIFF_LABELS[key] ? t(BLUEPRINT_DIFF_LABELS[key]) : key}
            </span>
            <span className="min-w-0 whitespace-pre-wrap break-words text-book-text-sub">
              {formatBlueprintDiffValue(value, t)}
            </span>
          </div>
        ))}
    </div>
  );
};

function formatBlueprintDiffValue(
  value: unknown,
  t: MessageTranslator,
): string {
  if (value == null) return t("novel.blueprintVersions.notRecorded");
  if (typeof value === "string") return value.trim() || t("novel.blueprintVersions.notRecorded");
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value
      .map((item) => formatBlueprintDiffValue(item, t))
      .filter(Boolean)
      .join(t("common.listSeparator")) || t("novel.blueprintVersions.notRecorded");
  }
  if (typeof value === "object") {
    return Object.entries(value as Record<string, unknown>)
      .map(([key, item]) =>
        `${BLUEPRINT_DIFF_LABELS[key] ? t(BLUEPRINT_DIFF_LABELS[key]) : key}: ${formatBlueprintDiffValue(item, t)}`,
      )
      .join(t("common.summarySeparator"));
  }
  return String(value);
}
