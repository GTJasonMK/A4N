import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  executeAttributeDeletion,
  listDeletionMarks,
  requestAttributeDeletion,
  resetDeletionMarks,
  type DeletionMark,
} from "../../api/character";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { ATTRIBUTE_CATEGORY_OPTIONS } from "./manageViewUtils";

type DeletionMarkStatusFilter = "all" | "pending" | "ready" | "executed";

const DELETION_STATUS_OPTIONS: Array<{
  value: DeletionMarkStatusFilter;
  label: string;
}> = [
  { value: "all", label: "全部" },
  { value: "pending", label: "保护中" },
  { value: "ready", label: "可执行" },
  { value: "executed", label: "已执行" },
];

const deletionMarkLoadKey = (
  profileId: number,
  category: string,
  status: DeletionMarkStatusFilter,
): string => `${profileId}:${category.trim() || "all"}:${status}`;

export const DeletionManager: React.FC<{
  profileId: number;
  defaultChapter: number;
  onChanged?: () => void | Promise<void>;
}> = ({ profileId, defaultChapter, onChanged }) => {
  const { addToast } = useToast();
  const [marks, setMarks] = useState<DeletionMark[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadedScopeKey, setLoadedScopeKey] = useState<string | null>(null);
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
    ) => {
      const requestProfileId = profileId;
      const requestKey = deletionMarkLoadKey(requestProfileId, nextCategory, nextStatus);
      if (loadInFlightKeyRef.current === requestKey) return;
      const requestSeq = loadSeqRef.current + 1;
      loadSeqRef.current = requestSeq;
      loadInFlightKeyRef.current = requestKey;
      const categoryFilter = nextCategory.trim();
      setLoading(true);
      try {
        const rows = await listDeletionMarks(requestProfileId, {
          category: categoryFilter ? categoryFilter : null,
          status: nextStatus,
        });
        if (!isCurrentLoad(requestProfileId, requestKey, requestSeq)) return;
        setMarks(rows);
        setLoadedScopeKey(requestKey);
      } catch (err) {
        if (!isCurrentLoad(requestProfileId, requestKey, requestSeq)) return;
        addToast(extractErrorMessage(err, "加载删除标记失败"), "error");
      } finally {
        if (loadInFlightKeyRef.current === requestKey) {
          loadInFlightKeyRef.current = null;
        }
        if (isCurrentLoad(requestProfileId, requestKey, requestSeq)) setLoading(false);
      }
    },
    [addToast, isCurrentLoad, profileId],
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
      title: "重置删除标记",
      message: `重置 ${category}.${key} 的未执行删除标记？该操作只影响删除保护计数。`,
      confirmText: "重置",
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
      const r = await resetDeletionMarks(requestProfileId, category, key);
      if (!isCurrentProfile(requestProfileId) || resetSeqRef.current !== requestSeq) return;
      addToast(`已重置 ${category}.${key} 的 ${r.deleted_marks} 个标记`, "success");
      await load();
    } catch (err) {
      if (!isCurrentProfile(requestProfileId) || resetSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "重置失败"), "error");
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
      addToast("属性 key、删除原因和证据不能为空", "error");
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
      const result = await requestAttributeDeletion({
        profile_id: requestProfileId,
        chapter_number: requestPayload.chapter_number,
        category: requestPayload.category,
        key: requestPayload.key,
        reason: requestPayload.reason,
        evidence: requestPayload.evidence,
      });
      if (!isCurrentProfile(requestProfileId) || requestSeqRef.current !== requestSeq) return;
      addToast(
        result.ready_for_deletion
          ? `删除标记已达到阈值 ${result.mark.consecutive_count}/${result.threshold}`
          : `已记录删除标记 ${result.mark.consecutive_count}/${result.threshold}`,
        "success",
      );
      setKey("");
      setReason("");
      setEvidence("");
      await load();
    } catch (err) {
      if (!isCurrentProfile(requestProfileId) || requestSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "请求删除失败"), "error");
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
      title: "执行属性删除",
      message: `确认删除 ${mark.category}.${mark.key}？这会写入 delete 属性事件，并从当前角色状态中移除该属性。`,
      confirmText: "执行删除",
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
      const result = await executeAttributeDeletion({
        profile_id: requestProfileId,
        chapter_number: mark.last_marked_chapter,
        category: mark.category,
        key: mark.key,
        cause: `手动执行删除（连续${mark.consecutive_count}次标记）`,
        evidence: mark.evidence,
      });
      if (!isCurrentProfile(requestProfileId) || executeSeqRef.current !== requestSeq) return;
      addToast(
        `已删除 ${result.event.category}.${result.event.key}，连续标记 ${result.consecutive_count}/${result.threshold}`,
        "success",
      );
      await load();
      if (!isCurrentProfile(requestProfileId) || executeSeqRef.current !== requestSeq) return;
      await onChanged?.();
    } catch (err) {
      if (!isCurrentProfile(requestProfileId) || executeSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "执行删除失败"), "error");
    } finally {
      if (isCurrentProfile(requestProfileId) && executeSeqRef.current === requestSeq) {
        setExecutingId(null);
      }
      if (executeSeqRef.current === requestSeq) {
        executingRef.current = false;
      }
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="text-[10px] text-book-text-muted">
        删除标记用于保护角色属性，连续达到阈值后才允许真正删除。
      </div>
      <div className="flex flex-wrap items-center gap-2 text-[11px]">
        <label className="flex items-center gap-1 text-book-text-muted">
          类别
          <select
            value={filterCategory}
            onChange={(event) => {
              const next = event.target.value;
              const nextKey = deletionMarkLoadKey(profileId, next, filterStatus);
              currentLoadScopeKeyRef.current = nextKey;
              if (currentLoadScopeKey !== nextKey) invalidateLoadedMarks();
              setFilterCategory(next);
              void load(next, filterStatus);
            }}
            className="pyqt-input !px-2 !py-1"
          >
            <option value="">全部</option>
            {ATTRIBUTE_CATEGORY_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-1 text-book-text-muted">
          状态
          <select
            value={filterStatus}
            onChange={(event) => {
              const next = event.target.value as DeletionMarkStatusFilter;
              const nextKey = deletionMarkLoadKey(profileId, filterCategory, next);
              currentLoadScopeKeyRef.current = nextKey;
              if (currentLoadScopeKey !== nextKey) invalidateLoadedMarks();
              setFilterStatus(next);
              void load(filterCategory, next);
            }}
            className="pyqt-input !px-2 !py-1"
          >
            {DELETION_STATUS_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <BookButton variant="secondary" size="sm" onClick={() => void load()} disabled={loading}>
          {loading ? <Loader2 size={11} className="animate-spin" /> : "刷新"}
        </BookButton>
        <span className="text-[10px] text-book-text-muted">
          当前 {hasCurrentMarks ? marks.length : 0} 条
        </span>
      </div>
      <details className="a4n-page-surface px-2 py-1.5">
        <summary className="cursor-pointer text-[11px] font-medium text-book-text-main">
          请求删除属性
        </summary>
        <div className="mt-2 grid gap-2 text-[11px] md:grid-cols-6">
          <label className="flex flex-col gap-1">
            <span className="text-[10px] text-book-text-muted">章节</span>
            <input
              type="number"
              min={1}
              value={chapterNumber}
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
            <span className="text-[10px] text-book-text-muted">分类</span>
            <select
              value={category}
              onChange={(event) => setCategory(event.target.value)}
              className="pyqt-input !px-2 !py-1"
            >
              {ATTRIBUTE_CATEGORY_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 md:col-span-4">
            <span className="text-[10px] text-book-text-muted">属性 key</span>
            <input
              value={key}
              onChange={(event) => setKey(event.target.value)}
              className="pyqt-input !px-2 !py-1"
              placeholder="例如 old_identity"
            />
          </label>
          <label className="flex flex-col gap-1 md:col-span-3">
            <span className="text-[10px] text-book-text-muted">删除原因</span>
            <textarea
              value={reason}
              onChange={(event) => setReason(event.target.value)}
              className="pyqt-input min-h-[58px] !px-2 !py-1 text-[11px]"
              placeholder="说明为什么该属性应被删除"
            />
          </label>
          <label className="flex flex-col gap-1 md:col-span-3">
            <span className="text-[10px] text-book-text-muted">证据</span>
            <textarea
              value={evidence}
              onChange={(event) => setEvidence(event.target.value)}
              className="pyqt-input min-h-[58px] !px-2 !py-1 text-[11px]"
              placeholder="引用章节原文或明确来源"
            />
          </label>
          <div className="md:col-span-6 flex justify-end">
            <BookButton
              size="sm"
              variant="secondary"
              onClick={handleRequestDeletion}
              disabled={requesting}
            >
              {requesting ? <Loader2 size={12} className="animate-spin" /> : "记录删除标记"}
            </BookButton>
          </div>
        </div>
      </details>
      {loading ? (
        <div className="text-[11px] text-book-text-muted">加载…</div>
      ) : !hasCurrentMarks ? (
        <div className="text-[11px] text-book-text-muted italic">当前筛选条件尚未加载，请刷新</div>
      ) : marks.length === 0 ? (
        <div className="text-[11px] text-book-text-muted italic">尚无删除标记</div>
      ) : (
        <ul className="space-y-0.5 max-h-40 overflow-y-auto custom-scrollbar">
          {marks.map((m) => {
            const markResetKey = `${m.category}:${m.key}`;
            const resettingThis = resettingKey === markResetKey;
            return (
              <li
                key={m.id}
                className="a4n-stat-tile flex items-center gap-2 px-2 py-1 text-[11px]"
              >
                <span className="font-mono text-book-text-muted shrink-0">#{m.id}</span>
                <span className="text-book-text-muted text-[10px] shrink-0">
                  创建 {m.created_at}
                </span>
                <span className="pyqt-pill !text-[10px] shrink-0">
                  {m.category}.{m.key}
                </span>
                <span className="pyqt-pill !text-[10px] shrink-0">
                  首次第{m.chapter_number}章
                </span>
                <span className="pyqt-pill !text-[10px] shrink-0">
                  最近第{m.last_marked_chapter}章
                </span>
                <span className="text-book-text-muted text-[10px] shrink-0">
                  连续 {m.consecutive_count}/{m.threshold}
                </span>
                {m.is_executed && (
                  <span className="pyqt-pill !border-success/40 !text-[10px] !text-success shrink-0">
                    已执行
                  </span>
                )}
                {!m.is_executed && m.consecutive_count >= m.threshold && (
                  <span className="pyqt-pill !border-error/40 !text-[10px] !text-error shrink-0">
                    可执行
                  </span>
                )}
                {!m.is_executed && m.consecutive_count < m.threshold && (
                  <span className="pyqt-pill !text-[10px] shrink-0">保护中</span>
                )}
                <span className="text-book-text-sub truncate flex-1">{m.reason}</span>
                {!m.is_executed && (
                  <div className="flex shrink-0 flex-wrap items-center gap-1">
                    <button
                      type="button"
                      onClick={() => void handleExecute(m)}
                      disabled={m.consecutive_count < m.threshold || executingId != null}
                      className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] disabled:opacity-50 sm:!min-h-[22px] sm:!px-1.5"
                    >
                      {executingId === m.id ? "执行中" : "执行"}
                    </button>
                    <button
                      type="button"
                      onClick={() => void handleReset(m.category, m.key)}
                      disabled={resettingKey != null}
                      className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[22px] sm:!px-1.5"
                    >
                      {resettingThis ? "重置中" : "重置"}
                    </button>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
};
