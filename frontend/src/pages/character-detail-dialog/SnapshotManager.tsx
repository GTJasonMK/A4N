import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, { useEffect, useRef, useState } from "react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../api";
import {
  checkProfileSyncConflict,
  diffProfileChapters,
  getProfileStateAtChapter,
  getSnapshot,
  listSnapshots,
  rollbackToChapter,
  type ProfileChapterState,
  type ProfileSyncConflict,
  type Snapshot,
  type SnapshotDelta,
  type SnapshotReference,
} from "../../api/character";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import {
  ProfileChapterStateDetail,
  SnapshotDeltaView,
  SnapshotDetail,
} from "./SnapshotViews";
import { useI18n, type MessageTranslator } from "../../i18n";

const snapshotReferenceCountText = (
  snapshot: SnapshotReference,
  t: MessageTranslator,
): string =>
  t("characterDetail.snapshot.attributeCounts", {
    explicit: t("characterDetail.labels.explicit"),
    explicitCount: snapshot.attribute_counts.explicit,
    implicit: t("characterDetail.labels.implicit"),
    implicitCount: snapshot.attribute_counts.implicit,
    social: t("characterDetail.labels.social"),
    socialCount: snapshot.attribute_counts.social,
  });

const rollbackChapterLabel = (chapter: number, t: MessageTranslator): string =>
  chapter === 0
    ? t("characterDetail.snapshot.initialState")
    : t("characterDetail.snapshot.chapterCompact", { chapter });

const rollbackChapterOptions = (
  conflict: ProfileSyncConflict | null,
  snapshots: SnapshotReference[],
): number[] => {
  if (conflict?.available_snapshot_chapters.length) {
    return conflict.available_snapshot_chapters;
  }
  if (conflict?.has_conflict) {
    return [0];
  }
  return snapshots.map((snapshot) => snapshot.chapter_number);
};

const parsePositiveInput = (raw: string): number | "" | null => {
  if (!raw) return "";
  const parsed = Number(raw);
  return Number.isFinite(parsed) && parsed > 0 ? Math.floor(parsed) : null;
};

const snapshotLoadKey = (
  profileId: number,
  startChapter: number | "",
  endChapter: number | "",
): string =>
  `${profileId}:${startChapter === "" ? "all" : startChapter}:${endChapter === "" ? "all" : endChapter}`;

export const SnapshotManager: React.FC<{
  profileId: number;
  onChanged?: () => void | Promise<void>;
}> = ({ profileId, onChanged }) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const [snapshots, setSnapshots] = useState<SnapshotReference[]>([]);
  const [conflict, setConflict] = useState<ProfileSyncConflict | null>(null);
  const [loading, setLoading] = useState(false);
  const [diffing, setDiffing] = useState(false);
  const [rollingBack, setRollingBack] = useState(false);
  const [snapshotData, setSnapshotData] = useState<Snapshot | null>(null);
  const [chapterState, setChapterState] = useState<ProfileChapterState | null>(
    null,
  );
  const [delta, setDelta] = useState<SnapshotDelta | null>(null);
  const [diffFromChapter, setDiffFromChapter] = useState(0);
  const [diffToChapter, setDiffToChapter] = useState<number | null>(null);
  const [rollbackChapter, setRollbackChapter] = useState<number | null>(null);
  const [snapshotStartChapter, setSnapshotStartChapter] = useState<number | "">(
    "",
  );
  const [snapshotEndChapter, setSnapshotEndChapter] = useState<number | "">("");
  const [loadedScopeKey, setLoadedScopeKey] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const activeProfileIdRef = useRef(profileId);
  const snapshotStartChapterRef = useRef(snapshotStartChapter);
  const snapshotEndChapterRef = useRef(snapshotEndChapter);
  const loadSeqRef = useRef(0);
  const snapshotViewSeqRef = useRef(0);
  const chapterStateSeqRef = useRef(0);
  const diffSeqRef = useRef(0);
  const rollbackSeqRef = useRef(0);
  const loadingRef = useRef(false);
  const diffingRef = useRef(false);
  const rollingBackRef = useRef(false);

  activeProfileIdRef.current = profileId;
  snapshotStartChapterRef.current = snapshotStartChapter;
  snapshotEndChapterRef.current = snapshotEndChapter;

  const currentSnapshotScopeKey = snapshotLoadKey(
    profileId,
    snapshotStartChapter,
    snapshotEndChapter,
  );
  const hasCurrentSnapshots = loadedScopeKey === currentSnapshotScopeKey;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      snapshotViewSeqRef.current += 1;
      chapterStateSeqRef.current += 1;
      diffSeqRef.current += 1;
      rollbackSeqRef.current += 1;
      loadingRef.current = false;
      diffingRef.current = false;
      rollingBackRef.current = false;
    };
  }, []);

  const isCurrentProfile = (requestProfileId: number) =>
    mountedRef.current && activeProfileIdRef.current === requestProfileId;

  const isCurrentRequest = (
    requestProfileId: number,
    seqRef: React.MutableRefObject<number>,
    requestSeq: number,
  ) => isCurrentProfile(requestProfileId) && seqRef.current === requestSeq;

  const isCurrentLoad = (
    requestProfileId: number,
    requestStartChapter: number | "",
    requestEndChapter: number | "",
    requestSeq: number,
  ) =>
    isCurrentProfile(requestProfileId) &&
    snapshotStartChapterRef.current === requestStartChapter &&
    snapshotEndChapterRef.current === requestEndChapter &&
    loadSeqRef.current === requestSeq;

  const invalidateSnapshotScope = () => {
    loadSeqRef.current += 1;
    snapshotViewSeqRef.current += 1;
    chapterStateSeqRef.current += 1;
    diffSeqRef.current += 1;
    loadingRef.current = false;
    diffingRef.current = false;
    setLoading(false);
    setDiffing(false);
    setLoadedScopeKey(null);
    setSnapshots([]);
    setConflict(null);
    setSnapshotData(null);
    setChapterState(null);
    setDelta(null);
  };

  const load = async () => {
    const requestProfileId = profileId;
    const requestSeq = loadSeqRef.current + 1;
    const requestStartChapter = snapshotStartChapter;
    const requestEndChapter = snapshotEndChapter;
    const requestKey = snapshotLoadKey(
      requestProfileId,
      requestStartChapter,
      requestEndChapter,
    );
    loadSeqRef.current = requestSeq;
    loadingRef.current = true;
    setLoading(true);
    setSnapshotData(null);
    setChapterState(null);
    setDelta(null);
    try {
      const [nextSnapshots, nextConflict] = await Promise.all([
        listSnapshots(requestProfileId, {
          start_chapter:
            requestStartChapter === "" ? null : requestStartChapter,
          end_chapter: requestEndChapter === "" ? null : requestEndChapter,
        }),
        checkProfileSyncConflict(requestProfileId),
      ]);
      if (
        !isCurrentLoad(
          requestProfileId,
          requestStartChapter,
          requestEndChapter,
          requestSeq,
        )
      )
        return;
      setSnapshots(nextSnapshots);
      setConflict(nextConflict);
      setLoadedScopeKey(requestKey);
      const snapshotChapters = nextSnapshots.map(
        (snapshot) => snapshot.chapter_number,
      );
      const latestDiffChapter =
        snapshotChapters[snapshotChapters.length - 1] ?? null;
      const previousDiffChapter =
        snapshotChapters.length > 1
          ? snapshotChapters[snapshotChapters.length - 2]
          : 0;
      setDiffToChapter((current) =>
        current != null && snapshotChapters.includes(current)
          ? current
          : latestDiffChapter,
      );
      setDiffFromChapter((current) =>
        current === 0 || snapshotChapters.includes(current)
          ? current
          : previousDiffChapter,
      );
      const availableChapters = rollbackChapterOptions(
        nextConflict,
        nextSnapshots,
      );
      const latestRollbackChapter =
        availableChapters[availableChapters.length - 1] ?? null;
      setRollbackChapter((current) => {
        if (availableChapters.length === 0) return null;
        if (nextConflict.has_conflict) return latestRollbackChapter;
        if (current != null && availableChapters.includes(current))
          return current;
        return latestRollbackChapter;
      });
    } catch (err) {
      if (
        !isCurrentLoad(
          requestProfileId,
          requestStartChapter,
          requestEndChapter,
          requestSeq,
        )
      )
        return;
      addToast(extractErrorMessage(err, t("characterDetail.snapshot.loadFailed")), "error");
    } finally {
      if (
        isCurrentLoad(
          requestProfileId,
          requestStartChapter,
          requestEndChapter,
          requestSeq,
        )
      )
        setLoading(false);
      if (loadSeqRef.current === requestSeq) {
        loadingRef.current = false;
      }
    }
  };

  const handleReload = () => {
    if (loadingRef.current) return;
    void load();
  };

  useEffect(() => {
    loadSeqRef.current += 1;
    snapshotViewSeqRef.current += 1;
    chapterStateSeqRef.current += 1;
    diffSeqRef.current += 1;
    rollbackSeqRef.current += 1;
    setSnapshots([]);
    setConflict(null);
    setSnapshotData(null);
    setChapterState(null);
    setDelta(null);
    setLoadedScopeKey(null);
    setLoading(false);
    setDiffing(false);
    setRollingBack(false);
    loadingRef.current = false;
    diffingRef.current = false;
    rollingBackRef.current = false;
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [profileId]);

  const handleView = async (hash: string) => {
    const requestProfileId = profileId;
    const requestSeq = snapshotViewSeqRef.current + 1;
    snapshotViewSeqRef.current = requestSeq;
    chapterStateSeqRef.current += 1;
    diffSeqRef.current += 1;
    setSnapshotData(null);
    setChapterState(null);
    setDelta(null);
    try {
      const nextSnapshot = await getSnapshot(hash);
      if (!isCurrentRequest(requestProfileId, snapshotViewSeqRef, requestSeq))
        return;
      setSnapshotData(nextSnapshot);
    } catch (err) {
      if (!isCurrentRequest(requestProfileId, snapshotViewSeqRef, requestSeq))
        return;
      addToast(extractErrorMessage(err, t("characterDetail.snapshot.loadFailed")), "error");
    }
  };

  const handleViewState = async (chapterNumber: number) => {
    const requestProfileId = profileId;
    const requestSeq = chapterStateSeqRef.current + 1;
    chapterStateSeqRef.current = requestSeq;
    snapshotViewSeqRef.current += 1;
    diffSeqRef.current += 1;
    setSnapshotData(null);
    setChapterState(null);
    setDelta(null);
    try {
      const nextState = await getProfileStateAtChapter(
        requestProfileId,
        chapterNumber,
      );
      if (!isCurrentRequest(requestProfileId, chapterStateSeqRef, requestSeq))
        return;
      setChapterState(nextState);
    } catch (err) {
      if (!isCurrentRequest(requestProfileId, chapterStateSeqRef, requestSeq))
        return;
      addToast(extractErrorMessage(err, t("characterDetail.snapshot.chapterStateFailed")), "error");
    }
  };

  const handleChapterDiff = async () => {
    if (diffToChapter == null) return;
    if (diffingRef.current) return;
    diffingRef.current = true;
    const requestProfileId = profileId;
    const requestFromChapter = diffFromChapter;
    const requestToChapter = diffToChapter;
    const requestSeq = diffSeqRef.current + 1;
    diffSeqRef.current = requestSeq;
    snapshotViewSeqRef.current += 1;
    chapterStateSeqRef.current += 1;
    setSnapshotData(null);
    setChapterState(null);
    setDelta(null);
    setDiffing(true);
    try {
      const nextDelta = await diffProfileChapters(
        requestProfileId,
        requestFromChapter,
        requestToChapter,
      );
      if (!isCurrentRequest(requestProfileId, diffSeqRef, requestSeq)) return;
      setDelta(nextDelta);
    } catch (err) {
      if (!isCurrentRequest(requestProfileId, diffSeqRef, requestSeq)) return;
      addToast(extractErrorMessage(err, t("characterDetail.snapshot.compareFailed")), "error");
    } finally {
      if (isCurrentRequest(requestProfileId, diffSeqRef, requestSeq)) {
        setDiffing(false);
      }
      if (diffSeqRef.current === requestSeq) {
        diffingRef.current = false;
      }
    }
  };

  const handleRollback = async () => {
    if (rollbackChapter == null) return;
    if (rollingBackRef.current) return;
    rollingBackRef.current = true;
    const requestProfileId = profileId;
    const requestRollbackChapter = rollbackChapter;
    const targetText = rollbackChapterLabel(requestRollbackChapter, t);
    const ok = await confirmDialog({
      title: t("characterDetail.snapshot.rollbackTitle"),
      message:
        requestRollbackChapter === 0
          ? t("characterDetail.snapshot.rollbackInitialMessage")
          : t("characterDetail.snapshot.rollbackMessage", { target: targetText }),
      confirmText: t("characterDetail.snapshot.rollback"),
      dialogType: "danger",
    });
    if (!ok) {
      rollingBackRef.current = false;
      return;
    }
    if (!isCurrentProfile(requestProfileId)) {
      rollingBackRef.current = false;
      return;
    }
    const requestSeq = rollbackSeqRef.current + 1;
    rollbackSeqRef.current = requestSeq;
    setRollingBack(true);
    try {
      const r = await rollbackToChapter(
        requestProfileId,
        requestRollbackChapter,
      );
      if (!isCurrentRequest(requestProfileId, rollbackSeqRef, requestSeq))
        return;
      addToast(
        t("characterDetail.snapshot.rollbackSummary", {
          events: r.deleted_events,
          behaviors: r.deleted_behaviors,
          indexes: r.deleted_state_indexes,
          stateRag: r.deleted_state_rag,
          eventRag: r.deleted_attribute_event_rag,
          snapshots: r.deleted_snapshots,
          deletedMarks: r.deleted_deletion_marks,
          rewoundMarks: r.rewound_deletion_marks,
        }),
        "success",
      );
      await load();
      if (!isCurrentRequest(requestProfileId, rollbackSeqRef, requestSeq))
        return;
      await onChanged?.();
    } catch (err) {
      if (!isCurrentRequest(requestProfileId, rollbackSeqRef, requestSeq))
        return;
      addToast(extractErrorMessage(err, t("characterDetail.snapshot.rollbackFailed")), "error");
    } finally {
      if (isCurrentRequest(requestProfileId, rollbackSeqRef, requestSeq)) {
        setRollingBack(false);
      }
      if (rollbackSeqRef.current === requestSeq) {
        rollingBackRef.current = false;
      }
    }
  };

  const rollbackChapters = rollbackChapterOptions(conflict, snapshots);
  const canRollback =
    rollbackChapter != null && rollbackChapters.includes(rollbackChapter);
  const snapshotChapters = snapshots.map((snapshot) => snapshot.chapter_number);
  const canDiffChapters =
    diffToChapter != null && diffFromChapter <= diffToChapter;
  const diffToChapterValue = diffToChapter === null ? "" : diffToChapter;
  const rollbackChapterValue = rollbackChapter === null ? "" : rollbackChapter;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-end gap-2 text-xs">
        <label className="flex flex-col gap-1 text-book-text-muted">
          {t("characterDetail.snapshot.startChapter")}
          <input
            type="number"
            min={1}
            value={snapshotStartChapter}
            disabled={loading}
            onChange={(event) => {
              const parsed = parsePositiveInput(event.target.value);
              if (parsed !== null) {
                if (snapshotStartChapterRef.current !== parsed)
                  invalidateSnapshotScope();
                setSnapshotStartChapter(parsed);
              }
            }}
            className="pyqt-input !w-20 !px-2 !py-1"
            placeholder={t("characterDetail.behavior.all")}
          />
        </label>
        <label className="flex flex-col gap-1 text-book-text-muted">
          {t("characterDetail.snapshot.endChapter")}
          <input
            type="number"
            min={1}
            value={snapshotEndChapter}
            disabled={loading}
            onChange={(event) => {
              const parsed = parsePositiveInput(event.target.value);
              if (parsed !== null) {
                if (snapshotEndChapterRef.current !== parsed)
                  invalidateSnapshotScope();
                setSnapshotEndChapter(parsed);
              }
            }}
            className="pyqt-input !w-20 !px-2 !py-1"
            placeholder={t("characterDetail.behavior.all")}
          />
        </label>
        <BookButton
          variant="secondary"
          size="sm"
          onClick={handleReload}
          disabled={loading}
        >
          {loading ? (
            <span className="inline-flex items-center gap-1.5">
              <LoadingSpinner size="sm" /> {t("characterDetail.snapshot.refreshing")}
            </span>
          ) : (
            t("characterDetail.snapshot.refresh")
          )}
        </BookButton>
        {hasCurrentSnapshots && snapshots.length > 0 ? (
          <span className="pb-1 text-caption text-book-text-muted">
            {t("characterDetail.snapshot.count", { count: snapshots.length })}
          </span>
        ) : null}
      </div>
      <details className="a4n-flat-surface px-3 py-2">
        <summary className="cursor-pointer text-small font-medium text-book-text-main">
          {t("characterDetail.snapshot.compareRollback")}
        </summary>
        <div className="mt-3 grid gap-3 lg:grid-cols-2">
          <section>
            <div className="mb-1.5 text-caption font-medium text-book-text-muted">
              {t("characterDetail.snapshot.compareChapterState")}
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <ThemedSelect
                value={diffFromChapter}
                onChange={(selectedValue) => {
                  const parsed = Number(selectedValue);
                  if (Number.isFinite(parsed) && parsed >= 0)
                    setDiffFromChapter(parsed);
                }}
                disabled={snapshotChapters.length === 0}
                className="pyqt-input !w-24 !px-2 !py-1"
              >
                <option value={0}>{t("characterDetail.snapshot.initial")}</option>
                {snapshotChapters.map((chapter) => (
                  <option key={chapter} value={chapter}>
                    {t("characterDetail.snapshot.chapterCompact", { chapter })}
                  </option>
                ))}
              </ThemedSelect>
              <span className="text-book-text-muted">→</span>
              <ThemedSelect
                value={diffToChapterValue}
                onChange={(selectedValue) => {
                  const value = Number(selectedValue);
                  setDiffToChapter(Number.isFinite(value) ? value : null);
                }}
                disabled={snapshotChapters.length === 0}
                className="pyqt-input !w-24 !px-2 !py-1"
              >
                {snapshotChapters.length === 0 ? (
                  <option value="">{t("characterDetail.snapshot.noSnapshot")}</option>
                ) : (
                  snapshotChapters.map((chapter) => (
                    <option key={chapter} value={chapter}>
                      {t("characterDetail.snapshot.chapterCompact", { chapter })}
                    </option>
                  ))
                )}
              </ThemedSelect>
              <BookButton
                variant="secondary"
                size="sm"
                onClick={handleChapterDiff}
                disabled={!canDiffChapters || diffing}
              >
                {diffing ? t("characterDetail.snapshot.generating") : t("characterDetail.snapshot.generateDiff")}
              </BookButton>
            </div>
          </section>
          <section className="border-l-2 border-error/35 pl-3">
            <div className="mb-1.5 text-caption font-medium text-book-text-muted">
              {t("characterDetail.snapshot.rollbackState")}
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <ThemedSelect
                value={rollbackChapterValue}
                onChange={(selectedValue) => {
                  if (!selectedValue) {
                    setRollbackChapter(null);
                    return;
                  }
                  const value = Number(selectedValue);
                  setRollbackChapter(Number.isFinite(value) ? value : null);
                }}
                disabled={rollbackChapters.length === 0}
                className="pyqt-input !w-28 !px-2 !py-1"
              >
                {rollbackChapters.length === 0 ? (
                  <option value="">{t("characterDetail.snapshot.noSnapshot")}</option>
                ) : (
                  rollbackChapters.map((chapter) => (
                    <option key={chapter} value={chapter}>
                      {rollbackChapterLabel(chapter, t)}
                    </option>
                  ))
                )}
              </ThemedSelect>
              <button
                type="button"
                onClick={handleRollback}
                disabled={!canRollback || rollingBack}
                className="pyqt-btn-secondary !min-h-9 !border-error/50 !px-2 text-small !text-error hover:!bg-error-bg sm:!min-h-[26px]"
              >
                {rollingBack ? t("characterDetail.snapshot.rollingBack") : t("characterDetail.snapshot.rollback")}
              </button>
            </div>
          </section>
        </div>
      </details>
      {loading ? (
        <div className="text-small text-book-text-muted">{t("characterDetail.snapshot.loading")}</div>
      ) : !hasCurrentSnapshots ? (
        <div className="text-small text-book-text-muted italic">
          {t("characterDetail.snapshot.notLoaded")}
        </div>
      ) : (
        <>
          {conflict && (
            <div
              className={`rounded-xl border-l-2 px-3 py-2.5 text-small ${
                conflict.has_conflict
                  ? "border-error/60 bg-error-bg/35 text-error"
                  : "border-book-border bg-book-bg-tertiary/25 text-book-text-muted"
              }`}
            >
              <div className="leading-relaxed">
                {t("characterDetail.snapshot.syncStatus", {
                  synced: conflict.last_synced_chapter,
                  max: conflict.max_available_chapter,
                })}
              </div>
              {conflict.has_conflict ? (
                <div className="mt-1 font-medium">
                  {t("characterDetail.snapshot.syncOutOfRange")}
                </div>
              ) : null}
              {conflict.available_snapshot_chapters.length > 0 && (
                <div className="mt-1 text-caption">
                  {t("characterDetail.snapshot.availableSnapshots", {
                    chapters: conflict.available_snapshot_chapters
                      .map((chapter) => t("characterDetail.snapshot.chapterCompact", { chapter }))
                      .join(t("common.listSeparator")),
                  })}
                </div>
              )}
              {conflict.has_conflict &&
                conflict.available_snapshot_chapters.length === 0 && (
                  <div className="mt-1 text-caption">
                    {t("characterDetail.snapshot.noAvailableSnapshot")}
                  </div>
                )}
            </div>
          )}
          {snapshots.length === 0 ? (
            <div className="text-small text-book-text-muted italic">
              {t("characterDetail.snapshot.empty")}
            </div>
          ) : (
            <ul className="a4n-flat-surface divide-y divide-book-border/45">
              {snapshots.map((s) => {
                return (
                  <li key={s.id} className="px-3 py-2 text-small">
                    <div className="flex items-start gap-2">
                      <div className="min-w-0 flex-1">
                        <div className="font-medium text-book-text-main">
                          {t("characterDetail.chapter", { chapter: s.chapter_number })}
                          <span className="ml-2 text-caption font-normal text-book-text-muted">
                            {t("characterDetail.snapshot.changes", { count: s.changes_in_chapter })} · {t("characterDetail.snapshot.behaviors", { count: s.behaviors_in_chapter })}
                          </span>
                        </div>
                        <div className="mt-1 text-caption leading-relaxed text-book-text-muted">
                          {snapshotReferenceCountText(s, t)}
                          {s.state_summary ? ` · ${s.state_summary}` : ""}
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => void handleViewState(s.chapter_number)}
                        className="shrink-0 rounded-lg px-1.5 py-0.5 text-caption text-book-primary hover:bg-book-primary/10"
                      >
                        {t("characterDetail.snapshot.state")}
                      </button>
                      <button
                        type="button"
                        onClick={() => void handleView(s.snapshot_hash)}
                        className="shrink-0 rounded-lg px-1.5 py-0.5 text-caption text-book-primary hover:bg-book-primary/10"
                      >
                        {t("characterDetail.snapshot.details")}
                      </button>
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </>
      )}
      {chapterState != null && (
        <ProfileChapterStateDetail state={chapterState} />
      )}
      {snapshotData != null && <SnapshotDetail snapshot={snapshotData} />}
      {delta != null && <SnapshotDeltaView delta={delta} />}
    </div>
  );
};
