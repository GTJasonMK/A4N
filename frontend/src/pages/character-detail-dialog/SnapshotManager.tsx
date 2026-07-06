import React, { useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
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

const snapshotReferenceCountText = (snapshot: SnapshotReference): string =>
  `显性 ${snapshot.attribute_counts.explicit} / 隐性 ${snapshot.attribute_counts.implicit} / 社会 ${snapshot.attribute_counts.social}`;

const rollbackChapterLabel = (chapter: number): string =>
  chapter === 0 ? "初始状态" : `第${chapter}章`;

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
): string => `${profileId}:${startChapter === "" ? "all" : startChapter}:${endChapter === "" ? "all" : endChapter}`;

export const SnapshotManager: React.FC<{
  profileId: number;
  onChanged?: () => void | Promise<void>;
}> = ({ profileId, onChanged }) => {
  const { addToast } = useToast();
  const [snapshots, setSnapshots] = useState<SnapshotReference[]>([]);
  const [conflict, setConflict] = useState<ProfileSyncConflict | null>(null);
  const [loading, setLoading] = useState(false);
  const [diffing, setDiffing] = useState(false);
  const [rollingBack, setRollingBack] = useState(false);
  const [snapshotData, setSnapshotData] = useState<Snapshot | null>(null);
  const [chapterState, setChapterState] = useState<ProfileChapterState | null>(null);
  const [delta, setDelta] = useState<SnapshotDelta | null>(null);
  const [diffFromChapter, setDiffFromChapter] = useState(0);
  const [diffToChapter, setDiffToChapter] = useState<number | null>(null);
  const [rollbackChapter, setRollbackChapter] = useState<number | null>(null);
  const [snapshotStartChapter, setSnapshotStartChapter] = useState<number | "">("");
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
    const requestKey = snapshotLoadKey(requestProfileId, requestStartChapter, requestEndChapter);
    loadSeqRef.current = requestSeq;
    loadingRef.current = true;
    setLoading(true);
    setSnapshotData(null);
    setChapterState(null);
    setDelta(null);
    try {
      const [nextSnapshots, nextConflict] = await Promise.all([
        listSnapshots(requestProfileId, {
          start_chapter: requestStartChapter === "" ? null : requestStartChapter,
          end_chapter: requestEndChapter === "" ? null : requestEndChapter,
        }),
        checkProfileSyncConflict(requestProfileId),
      ]);
      if (!isCurrentLoad(
        requestProfileId,
        requestStartChapter,
        requestEndChapter,
        requestSeq,
      )) return;
      setSnapshots(nextSnapshots);
      setConflict(nextConflict);
      setLoadedScopeKey(requestKey);
      const snapshotChapters = nextSnapshots.map((snapshot) => snapshot.chapter_number);
      const latestDiffChapter = snapshotChapters[snapshotChapters.length - 1] ?? null;
      const previousDiffChapter =
        snapshotChapters.length > 1 ? snapshotChapters[snapshotChapters.length - 2] : 0;
      setDiffToChapter((current) =>
        current != null && snapshotChapters.includes(current) ? current : latestDiffChapter,
      );
      setDiffFromChapter((current) =>
        current === 0 || snapshotChapters.includes(current) ? current : previousDiffChapter,
      );
      const availableChapters = rollbackChapterOptions(nextConflict, nextSnapshots);
      const latestRollbackChapter = availableChapters[availableChapters.length - 1] ?? null;
      setRollbackChapter((current) => {
        if (availableChapters.length === 0) return null;
        if (nextConflict.has_conflict) return latestRollbackChapter;
        if (current != null && availableChapters.includes(current)) return current;
        return latestRollbackChapter;
      });
    } catch (err) {
      if (!isCurrentLoad(
        requestProfileId,
        requestStartChapter,
        requestEndChapter,
        requestSeq,
      )) return;
      addToast(extractErrorMessage(err, "加载快照失败"), "error");
    } finally {
      if (isCurrentLoad(
        requestProfileId,
        requestStartChapter,
        requestEndChapter,
        requestSeq,
      )) setLoading(false);
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
    try {
      const nextSnapshot = await getSnapshot(hash);
      if (!isCurrentRequest(requestProfileId, snapshotViewSeqRef, requestSeq)) return;
      setSnapshotData(nextSnapshot);
    } catch (err) {
      if (!isCurrentRequest(requestProfileId, snapshotViewSeqRef, requestSeq)) return;
      addToast(extractErrorMessage(err, "加载快照失败"), "error");
    }
  };

  const handleViewState = async (chapterNumber: number) => {
    const requestProfileId = profileId;
    const requestSeq = chapterStateSeqRef.current + 1;
    chapterStateSeqRef.current = requestSeq;
    try {
      const nextState = await getProfileStateAtChapter(requestProfileId, chapterNumber);
      if (!isCurrentRequest(requestProfileId, chapterStateSeqRef, requestSeq)) return;
      setChapterState(nextState);
    } catch (err) {
      if (!isCurrentRequest(requestProfileId, chapterStateSeqRef, requestSeq)) return;
      addToast(extractErrorMessage(err, "加载章节状态失败"), "error");
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
      addToast(extractErrorMessage(err, "对比失败"), "error");
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
    const targetText = rollbackChapterLabel(requestRollbackChapter);
    const ok = await confirmDialog({
      title: "回滚角色快照",
      message:
        requestRollbackChapter === 0
          ? "回滚到初始状态？全部属性事件、行为、状态索引、快照引用和删除保护计数都会被清空，档案会恢复为创建时的初始属性。"
          : `回滚到${targetText}？此后的事件、行为、状态索引和快照引用会被删除，跨过该章节的删除保护计数会截断到回滚点。`,
      confirmText: "回滚",
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
      const r = await rollbackToChapter(requestProfileId, requestRollbackChapter);
      if (!isCurrentRequest(requestProfileId, rollbackSeqRef, requestSeq)) return;
      addToast(
        `已删除 ${r.deleted_events} 个事件 / ${r.deleted_behaviors} 条行为 / ${r.deleted_state_indexes} 个状态索引 / ${r.deleted_state_rag} 条状态 RAG / ${r.deleted_attribute_event_rag} 条属性事件 RAG / ${r.deleted_snapshots} 个快照引用；删除标记 ${r.deleted_deletion_marks} 个删除 / ${r.rewound_deletion_marks} 个截断`,
        "success",
      );
      await load();
      if (!isCurrentRequest(requestProfileId, rollbackSeqRef, requestSeq)) return;
      await onChanged?.();
    } catch (err) {
      if (!isCurrentRequest(requestProfileId, rollbackSeqRef, requestSeq)) return;
      addToast(extractErrorMessage(err, "回滚失败"), "error");
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
  const canRollback = rollbackChapter != null && rollbackChapters.includes(rollbackChapter);
  const snapshotChapters = snapshots.map((snapshot) => snapshot.chapter_number);
  const canDiffChapters = diffToChapter != null && diffFromChapter <= diffToChapter;
  const diffToChapterValue = diffToChapter === null ? "" : diffToChapter;
  const rollbackChapterValue = rollbackChapter === null ? "" : rollbackChapter;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2 text-xs flex-wrap">
        <label className="flex items-center gap-1 text-book-text-muted">
          快照范围
          <input
            type="number"
            min={1}
            value={snapshotStartChapter}
            onChange={(event) => {
              const parsed = parsePositiveInput(event.target.value);
              if (parsed !== null) {
                if (snapshotStartChapterRef.current !== parsed) {
                  invalidateSnapshotScope();
                }
                setSnapshotStartChapter(parsed);
              }
            }}
            className="pyqt-input !py-1 !px-2 w-20"
            placeholder="起"
          />
        </label>
        <input
          type="number"
          min={1}
          value={snapshotEndChapter}
          onChange={(event) => {
            const parsed = parsePositiveInput(event.target.value);
            if (parsed !== null) {
              if (snapshotEndChapterRef.current !== parsed) {
                invalidateSnapshotScope();
              }
              setSnapshotEndChapter(parsed);
            }
          }}
          className="pyqt-input !py-1 !px-2 w-20"
          placeholder="止"
        />
        <BookButton variant="secondary" size="sm" onClick={handleReload} disabled={loading}>
          {loading ? <Loader2 size={11} className="animate-spin" /> : "刷新"}
        </BookButton>
        <label className="text-book-text-muted">状态差异</label>
        <select
          value={diffFromChapter}
          onChange={(event) => {
            const parsed = Number(event.target.value);
            if (Number.isFinite(parsed) && parsed >= 0) {
              setDiffFromChapter(parsed);
            }
          }}
          disabled={snapshotChapters.length === 0}
          className="pyqt-input !py-1 !px-2 w-24"
        >
          <option value={0}>初始</option>
          {snapshotChapters.map((chapter) => (
            <option key={chapter} value={chapter}>
              第{chapter}章
            </option>
          ))}
        </select>
        <span className="text-book-text-muted">→</span>
        <select
          value={diffToChapterValue}
          onChange={(event) => {
            const value = Number(event.target.value);
            setDiffToChapter(Number.isFinite(value) ? value : null);
          }}
          disabled={snapshotChapters.length === 0}
          className="pyqt-input !py-1 !px-2 w-24"
        >
          {snapshotChapters.length === 0 ? (
            <option value="">无快照</option>
          ) : (
            snapshotChapters.map((chapter) => (
              <option key={chapter} value={chapter}>
                第{chapter}章
              </option>
            ))
          )}
        </select>
        <BookButton
          variant="secondary"
          size="sm"
          onClick={handleChapterDiff}
          disabled={!canDiffChapters || diffing}
        >
          {diffing ? "生成中" : "生成 Diff"}
        </BookButton>
        <div className="flex-1" />
        <label className="text-book-text-muted">回滚快照</label>
        <select
          value={rollbackChapterValue}
          onChange={(event) => {
            if (!event.target.value) {
              setRollbackChapter(null);
              return;
            }
            const value = Number(event.target.value);
            setRollbackChapter(Number.isFinite(value) ? value : null);
          }}
          disabled={rollbackChapters.length === 0}
          className="pyqt-input !py-1 !px-2 w-24"
        >
          {rollbackChapters.length === 0 ? (
            <option value="">无快照</option>
          ) : (
            rollbackChapters.map((chapter) => (
              <option key={chapter} value={chapter}>
                {rollbackChapterLabel(chapter)}
              </option>
            ))
          )}
        </select>
        <button
          type="button"
          onClick={handleRollback}
          disabled={!canRollback || rollingBack}
          className="pyqt-btn-secondary !min-h-9 !px-2 text-[11px] !border-error/50 !text-error hover:!bg-error-bg sm:!min-h-[26px]"
        >
          {rollingBack ? "回滚中" : "回滚"}
        </button>
      </div>
      {loading ? (
        <div className="text-[11px] text-book-text-muted">加载…</div>
      ) : !hasCurrentSnapshots ? (
        <div className="text-[11px] text-book-text-muted italic">当前快照范围尚未加载，请刷新</div>
      ) : (
        <>
          {conflict && (
            <div
              className={`a4n-stat-tile px-2 py-1.5 text-[11px] ${
                conflict.has_conflict
                  ? "border-error/40 bg-error-bg/40 text-error"
                  : "text-book-text-muted"
              }`}
            >
              <div className="flex flex-wrap items-center gap-1.5">
                <span className="pyqt-pill !text-[10px]">
                  档案同步至第 {conflict.last_synced_chapter} 章
                </span>
                <span className="pyqt-pill !text-[10px]">
                  当前最大章节 {conflict.max_available_chapter}
                </span>
                {conflict.has_conflict && (
                  <span className="font-medium">
                    同步状态超过当前章节范围，请回滚到可用快照后再继续同步。
                  </span>
                )}
              </div>
              {conflict.available_snapshot_chapters.length > 0 && (
                <div className="mt-1 text-[10px]">
                  可回滚快照：第 {conflict.available_snapshot_chapters.join("、")} 章
                </div>
              )}
              {conflict.has_conflict && conflict.available_snapshot_chapters.length === 0 && (
                <div className="mt-1 text-[10px]">
                  没有可用快照，可回滚到初始状态后重新同步。
                </div>
              )}
            </div>
          )}
          {snapshots.length === 0 ? (
            <div className="text-[11px] text-book-text-muted italic">尚无快照引用</div>
          ) : (
            <ul className="space-y-1 max-h-56 overflow-y-auto custom-scrollbar">
              {snapshots.map((s) => {
                return (
                  <li
                    key={s.id}
                    className="a4n-stat-tile px-2 py-1.5 text-[11px]"
                  >
                    <div className="flex items-center gap-2">
                      <span className="pyqt-pill !text-[10px] font-mono">
                        第{s.chapter_number}章
                      </span>
                      <span className="font-mono text-book-text-muted shrink-0 text-[10px]">
                        {s.snapshot_hash.slice(0, 12)}
                      </span>
                      <span className="text-book-text-muted text-[10px]">
                        变化 {s.changes_in_chapter} / 行为 {s.behaviors_in_chapter}
                      </span>
                      <span className="flex-1" />
                      <button
                        type="button"
                        onClick={() => void handleViewState(s.chapter_number)}
                        className="text-[10px] text-book-primary px-1.5 py-0.5 rounded hover:bg-book-primary/10"
                      >
                        状态
                      </button>
                      <button
                        type="button"
                        onClick={() => void handleView(s.snapshot_hash)}
                        className="text-[10px] text-book-primary px-1.5 py-0.5 rounded hover:bg-book-primary/10"
                      >
                        详情
                      </button>
                    </div>
                    <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[10px] text-book-text-muted">
                      <span>{snapshotReferenceCountText(s)}</span>
                      {s.state_summary && (
                        <span className="min-w-0 flex-1 truncate text-book-text-sub">
                          {s.state_summary}
                        </span>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </>
      )}
      {chapterState != null && <ProfileChapterStateDetail state={chapterState} />}
      {snapshotData != null && <SnapshotDetail snapshot={snapshotData} />}
      {delta != null && <SnapshotDeltaView delta={delta} />}
    </div>
  );
};
