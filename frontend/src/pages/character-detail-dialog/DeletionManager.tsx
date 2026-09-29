import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { RefreshCw, RotateCcw, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../api";
import {
  executeAttributeDeletion,
  listDeletionMarks,
  requestAttributeDeletion,
  resetDeletionMarks,
  type DeletionMark,
} from "../../api/character";
import {
  CHARACTER_DELETION_THRESHOLD,
  type CharacterAttributeCategory,
} from "../../api/generated/host";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import {
  ATTRIBUTE_CATEGORY_OPTIONS,
  attributeKeyLabel,
  categoryLabel,
  characterDetailDraftKey,
  sourceKindLabel,
} from "./manageViewUtils";
import { formatDateTime, useI18n } from "../../i18n";
import { useDirty } from "../../components/navigation/dirtyRegistry";

type DeletionMarkStatusFilter = "all" | "pending" | "ready" | "executed";

const deletionMarkLoadKey = (
  profileId: number,
  category: string,
  status: DeletionMarkStatusFilter,
): string => `${profileId}:${category.trim() || "all"}:${status}`;

export const DeletionManager: React.FC<{
  profileId: number;
  defaultChapter: number;
  onChanged?: () => boolean | void | Promise<boolean | void>;
}> = ({ profileId, defaultChapter, onChanged }) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const deletionStatusOptions: Array<{
    value: DeletionMarkStatusFilter;
    label: string;
  }> = [
    { value: "all", label: t("characterDetail.deletion.all") },
    { value: "pending", label: t("characterDetail.deletion.pending") },
    { value: "ready", label: t("characterDetail.deletion.ready") },
    { value: "executed", label: t("characterDetail.deletion.executed") },
  ];
  const [marks, setMarks] = useState<DeletionMark[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadedScopeKey, setLoadedScopeKey] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [chapterNumber, setChapterNumber] = useState(defaultChapter);
  const [category, setCategory] = useState("explicit");
  const [filterCategory, setFilterCategory] = useState("");
  const [filterStatus, setFilterStatus] = useState<DeletionMarkStatusFilter>("all");
  const [key, setKey] = useState("");
  const [reason, setReason] = useState("");
  const [evidence, setEvidence] = useState("");
  const [requesting, setRequesting] = useState(false);
  const [executingId, setExecutingId] = useState<number | null>(null);
  const [resettingKey, setResettingKey] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const activeProfileIdRef = useRef(profileId);
  const filterCategoryRef = useRef(filterCategory);
  const filterStatusRef = useRef(filterStatus);
  const marksRef = useRef(marks);
  const loadSeqRef = useRef(0);
  const requestSeqRef = useRef(0);
  const executeSeqRef = useRef(0);
  const resetSeqRef = useRef(0);
  const loadInFlightKeyRef = useRef<string | null>(null);
  const currentLoadScopeKeyRef = useRef("");
  const requestingRef = useRef(false);
  const executingRef = useRef(false);
  const resettingRef = useRef<string | null>(null);

  activeProfileIdRef.current = profileId;
  filterCategoryRef.current = filterCategory;
  filterStatusRef.current = filterStatus;
  marksRef.current = marks;
  const currentLoadScopeKey = deletionMarkLoadKey(profileId, filterCategory, filterStatus);
  currentLoadScopeKeyRef.current = currentLoadScopeKey;
  const hasCurrentMarks = loadedScopeKey === currentLoadScopeKey;
  const formDirty =
    chapterNumber !== defaultChapter ||
    category !== "explicit" ||
    key.trim().length > 0 ||
    reason.trim().length > 0 ||
    evidence.trim().length > 0;

  const discardFormDraft = useCallback(() => {
    if (requesting) return false;
    setChapterNumber(defaultChapter);
    setCategory("explicit");
    setKey("");
    setReason("");
    setEvidence("");
    return true;
  }, [defaultChapter, requesting]);

  useDirty(
    characterDetailDraftKey(profileId, "deletion"),
    formDirty,
    discardFormDraft,
    () => !requesting,
  );

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      requestSeqRef.current += 1;
      executeSeqRef.current += 1;
      resetSeqRef.current += 1;
      loadInFlightKeyRef.current = null;
      requestingRef.current = false;
      executingRef.current = false;
      resettingRef.current = null;
    };
  }, []);

  const isCurrentProfile = useCallback((requestProfileId: number) => {
    return mountedRef.current && activeProfileIdRef.current === requestProfileId;
  }, []);

  const invalidateLoadedMarks = useCallback(() => {
    loadSeqRef.current += 1;
    loadInFlightKeyRef.current = null;
    setLoading(false);
    setLoadedScopeKey(null);
    setMarks([]);
    setLoadError(null);
  }, []);

  const isCurrentLoad = useCallback((requestProfileId: number, requestKey: string, seq: number) => {
    return (
      isCurrentProfile(requestProfileId) &&
      currentLoadScopeKeyRef.current === requestKey &&
      loadSeqRef.current === seq
    );
  }, [isCurrentProfile]);

  const load = useCallback(
    async (
      nextCategory = filterCategoryRef.current,
      nextStatus: DeletionMarkStatusFilter = filterStatusRef.current,
    ): Promise<boolean> => {
      const requestProfileId = profileId;
      const requestKey = deletionMarkLoadKey(requestProfileId, nextCategory, nextStatus);
      if (loadInFlightKeyRef.current === requestKey) return false;
      const requestSeq = loadSeqRef.current + 1;
      loadSeqRef.current = requestSeq;
      loadInFlightKeyRef.current = requestKey;
      const categoryFilter = nextCategory.trim();
      setLoading(true);
      setLoadError(null);
      try {
        const rows = await listDeletionMarks(requestProfileId, {
          category: categoryFilter ? categoryFilter : null,
          status: nextStatus,
        });
        if (!isCurrentLoad(requestProfileId, requestKey, requestSeq)) return false;
        setMarks(rows);
        setLoadedScopeKey(requestKey);
        setLoadError(null);
        return true;
      } catch (err) {
        if (!isCurrentLoad(requestProfileId, requestKey, requestSeq)) return false;
        setLoadError(extractErrorMessage(err, t("characterDetail.deletion.loadFailed")));
        return false;
      } finally {
        if (loadInFlightKeyRef.current === requestKey) {
          loadInFlightKeyRef.current = null;
        }
        if (isCurrentLoad(requestProfileId, requestKey, requestSeq)) setLoading(false);
      }
    },
    [addToast, isCurrentLoad, profileId, t],
  );

  useEffect(() => {
    invalidateLoadedMarks();
    void load();
  }, [invalidateLoadedMarks, load]);
  useEffect(() => {
    requestSeqRef.current += 1;
    executeSeqRef.current += 1;
    resetSeqRef.current += 1;
    setRequesting(false);
    setExecutingId(null);
    setResettingKey(null);
    requestingRef.current = false;
    executingRef.current = false;
    resettingRef.current = null;
    setChapterNumber(defaultChapter);
    setKey("");
    setReason("");
    setEvidence("");
  }, [defaultChapter, profileId]);

  const handleReset = async (category: string, key: string) => {
    const resetKey = `${category}:${key}`;
    if (resettingRef.current != null) return;
    resettingRef.current = resetKey;
    setResettingKey(resetKey);
    const ok = await confirmDialog({
      title: t("characterDetail.deletion.resetTitle"),
      message: t("characterDetail.deletion.resetMessage", {
        category: categoryLabel(category, t),
        attribute: attributeKeyLabel(key, t),
      }),
      confirmText: t("characterDetail.deletion.reset"),
      dialogType: "warning",
    });
    if (!ok) {
      resettingRef.current = null;
      setResettingKey(null);
      return;
    }
    const requestProfileId = profileId;
    const requestSeq = resetSeqRef.current + 1;
    resetSeqRef.current = requestSeq;
    if (!isCurrentProfile(requestProfileId)) {
      resettingRef.current = null;
      setResettingKey(null);
      return;
    }
    try {
      const deletedMarks = await resetDeletionMarks(requestProfileId, category, key);
      if (!isCurrentProfile(requestProfileId) || resetSeqRef.current !== requestSeq) return;
      await load();
      addToast(
        t("characterDetail.deletion.resetSuccess", {
          attribute: attributeKeyLabel(key, t),
          count: deletedMarks,
        }),
        "success",
      );
    } catch (err) {
      if (!isCurrentProfile(requestProfileId) || resetSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("characterDetail.deletion.resetFailed")), "error");
    } finally {
      if (resetSeqRef.current === requestSeq) {
        resettingRef.current = null;
        setResettingKey(null);
      }
    }
  };

  const handleRequestDeletion = async () => {
    if (requestingRef.current) return;
    const attrKey = key.trim();
    const deletionReason = reason.trim();
    const deletionEvidence = evidence.trim();
    if (!attrKey || !deletionReason || !deletionEvidence) {
      addToast(t("characterDetail.deletion.required"), "error");
      return;
    }
    requestingRef.current = true;
    const requestProfileId = profileId;
    const requestPayload = {
      chapter_number: chapterNumber,
      category,
      key: attrKey,
      reason: deletionReason,
      evidence: deletionEvidence,
    };
    const requestSeq = requestSeqRef.current + 1;
    requestSeqRef.current = requestSeq;
    setRequesting(true);
    try {
      const mark = await requestAttributeDeletion({
        profile_id: requestProfileId,
        chapter_number: requestPayload.chapter_number,
        category: requestPayload.category as CharacterAttributeCategory,
        key: requestPayload.key,
        reason: requestPayload.reason,
        evidence: requestPayload.evidence,
      });
      if (!isCurrentProfile(requestProfileId) || requestSeqRef.current !== requestSeq) return;
      await load();
      addToast(
        mark.consecutive_count >= CHARACTER_DELETION_THRESHOLD
          ? t("characterDetail.deletion.thresholdReached", {
              count: mark.consecutive_count,
              threshold: CHARACTER_DELETION_THRESHOLD,
            })
          : t("characterDetail.deletion.marked", {
              count: mark.consecutive_count,
              threshold: CHARACTER_DELETION_THRESHOLD,
            }),
        "success",
      );
      setKey("");
      setReason("");
      setEvidence("");
    } catch (err) {
      if (!isCurrentProfile(requestProfileId) || requestSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("characterDetail.deletion.requestFailed")), "error");
    } finally {
      if (isCurrentProfile(requestProfileId) && requestSeqRef.current === requestSeq) {
        setRequesting(false);
      }
      if (requestSeqRef.current === requestSeq) {
        requestingRef.current = false;
      }
    }
  };

  const handleExecute = async (mark: DeletionMark) => {
    if (executingRef.current) return;
    executingRef.current = true;
    const ok = await confirmDialog({
      title: t("characterDetail.deletion.executeTitle"),
      message: t("characterDetail.deletion.executeMessage", {
        category: categoryLabel(mark.category, t),
        attribute: attributeKeyLabel(mark.key, t),
      }),
      confirmText: t("characterDetail.deletion.execute"),
      dialogType: "danger",
    });
    if (!ok) {
      executingRef.current = false;
      return;
    }
    const requestProfileId = profileId;
    const requestSeq = executeSeqRef.current + 1;
    executeSeqRef.current = requestSeq;
    if (
      !isCurrentProfile(requestProfileId) ||
      !marksRef.current.some((item) => item.id === mark.id)
    ) {
      executingRef.current = false;
      return;
    }
    setExecutingId(mark.id);
    try {
      await executeAttributeDeletion({
        profile_id: requestProfileId,
        chapter_number: mark.last_marked_chapter,
        category: mark.category,
        key: mark.key,
        cause: t("characterDetail.deletion.executeCause", {
          count: mark.consecutive_count,
        }),
        evidence: mark.evidence,
      });
      if (!isCurrentProfile(requestProfileId) || executeSeqRef.current !== requestSeq) return;
      await load();
      try {
        if (await onChanged?.() === false) {
          addToast(t("characterDetail.refreshFailed"), "error");
        }
      } catch (refreshError) {
        addToast(extractErrorMessage(refreshError, t("characterDetail.refreshFailed")), "error");
      }
      addToast(
        t("characterDetail.deletion.executeSuccess", {
          attribute: attributeKeyLabel(mark.key, t),
        }),
        "success",
      );
      if (!isCurrentProfile(requestProfileId) || executeSeqRef.current !== requestSeq) return;
    } catch (err) {
      if (!isCurrentProfile(requestProfileId) || executeSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("characterDetail.deletion.executeFailed")), "error");
    } finally {
      if (isCurrentProfile(requestProfileId) && executeSeqRef.current === requestSeq) {
        setExecutingId(null);
      }
      if (executeSeqRef.current === requestSeq) {
        executingRef.current = false;
      }
    }
  };

  const deletionRequestReady =
    key.trim().length > 0 &&
    reason.trim().length > 0 &&
    evidence.trim().length > 0;

  return (
    <div className="flex flex-col gap-2">
      <div className="text-small leading-relaxed text-book-text-muted">
        {t("characterDetail.deletion.hint")}
      </div>
      {loadError ? (
        <div role="alert" className="flex flex-wrap items-center gap-2 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{loadError}</span>
          <BookButton variant="secondary" size="sm" onClick={() => void load()} disabled={loading}>
            <RefreshCw size={13} className={loading ? "animate-spin" : undefined} />
            {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      <div className="flex flex-wrap items-end gap-2 text-small">
        <label className="flex items-center gap-1 text-book-text-muted">
          {t("characterDetail.deletion.category")}
          <ThemedSelect
            value={filterCategory}
            onChange={(next) => {
              const nextKey = deletionMarkLoadKey(profileId, next, filterStatus);
              currentLoadScopeKeyRef.current = nextKey;
              if (currentLoadScopeKey !== nextKey) invalidateLoadedMarks();
              setFilterCategory(next);
              void load(next, filterStatus);
            }}
            className="pyqt-input !px-2 !py-1"
          >
            <option value="">{t("characterDetail.deletion.all")}</option>
            {ATTRIBUTE_CATEGORY_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {categoryLabel(option.value, t)}
              </option>
            ))}
          </ThemedSelect>
        </label>
        <label className="flex items-center gap-1 text-book-text-muted">
          {t("characterDetail.deletion.status")}
          <ThemedSelect
            value={filterStatus}
            onChange={(selectedValue) => {
              const next = selectedValue as DeletionMarkStatusFilter;
              const nextKey = deletionMarkLoadKey(profileId, filterCategory, next);
              currentLoadScopeKeyRef.current = nextKey;
              if (currentLoadScopeKey !== nextKey) invalidateLoadedMarks();
              setFilterStatus(next);
              void load(filterCategory, next);
            }}
            className="pyqt-input !px-2 !py-1"
          >
            {deletionStatusOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </ThemedSelect>
        </label>
        {loading ? <LoadingSpinner size="sm" className="text-book-text-muted" /> : null}
        {hasCurrentMarks && marks.length > 0 ? (
          <span className="text-caption text-book-text-muted">{t("characterDetail.deletion.total", { count: marks.length })}</span>
        ) : null}
      </div>
      <details className="text-small text-book-text-muted">
        <summary className="w-fit cursor-pointer select-none py-1 font-medium text-book-text-main">
          {t("characterDetail.deletion.request")}
        </summary>
        <div className="mt-1.5 grid gap-2 rounded-xl bg-book-bg-tertiary/40 p-3 text-small md:grid-cols-6">
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">{t("characterDetail.deletion.chapter")}</span>
            <input
              type="number"
              min={1}
              value={chapterNumber}
              disabled={requesting}
              onChange={(event) => {
                const parsed = Number(event.target.value);
                if (Number.isFinite(parsed) && parsed > 0) {
                  setChapterNumber(Math.floor(parsed));
                }
              }}
              className="pyqt-input !px-2 !py-1"
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">{t("characterDetail.deletion.category")}</span>
            <ThemedSelect
              value={category}
              onChange={setCategory}
              disabled={requesting}
              className="pyqt-input !px-2 !py-1"
            >
              {ATTRIBUTE_CATEGORY_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {categoryLabel(option.value, t)}
                </option>
              ))}
            </ThemedSelect>
          </label>
          <label className="flex flex-col gap-1 md:col-span-4">
            <span className="text-caption text-book-text-muted">{t("characterDetail.deletion.attribute")}</span>
            <input
              value={key}
              onChange={(event) => setKey(event.target.value)}
              disabled={requesting}
              className="pyqt-input !px-2 !py-1"
              placeholder={t("characterDetail.deletion.attributePlaceholder")}
            />
          </label>
          <label className="flex flex-col gap-1 md:col-span-3">
            <span className="text-caption text-book-text-muted">{t("characterDetail.deletion.reason")}</span>
            <textarea
              value={reason}
              onChange={(event) => setReason(event.target.value)}
              disabled={requesting}
              className="pyqt-input min-h-[58px] !px-2 !py-1 text-small"
              placeholder={t("characterDetail.deletion.reasonPlaceholder")}
            />
          </label>
          <label className="flex flex-col gap-1 md:col-span-3">
            <span className="text-caption text-book-text-muted">{t("characterDetail.deletion.evidence")}</span>
            <textarea
              value={evidence}
              onChange={(event) => setEvidence(event.target.value)}
              disabled={requesting}
              className="pyqt-input min-h-[58px] !px-2 !py-1 text-small"
              placeholder={t("characterDetail.deletion.evidencePlaceholder")}
            />
          </label>
          <div className="md:col-span-6 flex justify-end">
            <BookButton
              size="sm"
              onClick={handleRequestDeletion}
              disabled={requesting || !deletionRequestReady}
            >
              {requesting ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> {t("characterDetail.deletion.recording")}
                </span>
              ) : (
                t("characterDetail.deletion.record")
              )}
            </BookButton>
          </div>
        </div>
      </details>
      {loading ? (
        <div className="text-small text-book-text-muted">{t("characterDetail.deletion.loading")}</div>
      ) : loadError && !hasCurrentMarks ? null : !hasCurrentMarks ? (
        <div className="text-small text-book-text-muted italic">{t("characterDetail.deletion.notLoaded")}</div>
      ) : marks.length === 0 ? (
        <div className="text-small text-book-text-muted italic">{t("characterDetail.deletion.empty")}</div>
      ) : (
        <ul className="a4n-flat-surface divide-y divide-book-border/45 overflow-hidden">
          {marks.map((m) => {
            const markResetKey = `${m.category}:${m.key}`;
            const resettingThis = resettingKey === markResetKey;
            const executingThis = executingId === m.id;
            const actionItems: React.ComponentProps<typeof Dropdown>["items"] = [
              ...(m.consecutive_count >= CHARACTER_DELETION_THRESHOLD
                ? [
                    {
                      label: t("characterDetail.deletion.execute"),
                      icon: <Trash2 size={12} />,
                      danger: true,
                      onClick: () => void handleExecute(m),
                    },
                    { type: "divider" as const },
                  ]
                : []),
              {
                label: t("characterDetail.deletion.resetTitle"),
                icon: <RotateCcw size={12} />,
                onClick: () => void handleReset(m.category, m.key),
              },
            ];
            return (
              <li
                key={m.id}
                className="px-3 py-2.5 text-small"
              >
                <div className="flex min-w-0 items-start gap-2">
                  <span className="min-w-0 flex-1 font-medium text-book-text-main">
                    {categoryLabel(m.category, t)} · {attributeKeyLabel(m.key, t)}
                  </span>
                  {!m.is_executed ? (
                    <div className="flex shrink-0 items-center justify-end">
                      {executingThis || resettingThis ? (
                        <span
                          className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted"
                          title={executingThis ? t("characterDetail.deletion.executePending") : t("characterDetail.deletion.resetPending")}
                        >
                          <LoadingSpinner size="sm" />
                        </span>
                      ) : executingId == null && resettingKey == null ? (
                        <Dropdown items={actionItems} />
                      ) : null}
                    </div>
                  ) : null}
                </div>
                <div className="mt-0.5 flex flex-wrap gap-x-1.5 gap-y-0.5 text-caption text-book-text-muted">
                  <span>{t("characterDetail.chapter", { chapter: m.last_marked_chapter })}</span>
                  <span aria-hidden="true">·</span>
                  <span>{t("characterDetail.deletion.consecutive", { count: m.consecutive_count, threshold: CHARACTER_DELETION_THRESHOLD })}</span>
                  <span aria-hidden="true">·</span>
                  <span>{sourceKindLabel(m.source_kind, t)}</span>
                  <span aria-hidden="true">·</span>
                  <span className={deletionStatusTone(m)}>{deletionStatusLabel(m, t)}</span>
                  <span aria-hidden="true">·</span>
                  <span>{formatDateTime(m.created_at, locale)}</span>
                </div>
                <div className="mt-1.5 text-book-text-sub">{m.reason}</div>
                {m.evidence ? (
                  <div className="mt-0.5 line-clamp-2 text-caption leading-relaxed text-book-text-muted">
                    {t("characterDetail.deletion.evidence")}: {m.evidence}
                  </div>
                ) : null}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
};

function deletionStatusLabel(mark: DeletionMark, t: ReturnType<typeof useI18n>["t"]): string {
  if (mark.is_executed) return t("characterDetail.deletion.executed");
  if (mark.consecutive_count >= CHARACTER_DELETION_THRESHOLD) return t("characterDetail.deletion.statusCanDelete");
  return t("characterDetail.deletion.pending");
}

function deletionStatusTone(mark: DeletionMark): string {
  if (mark.is_executed) return "text-success";
  if (mark.consecutive_count >= CHARACTER_DELETION_THRESHOLD) return "text-error";
  return "text-book-text-muted";
}
