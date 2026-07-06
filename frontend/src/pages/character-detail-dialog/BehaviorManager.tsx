import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { listBehaviors, type BehaviorRecord } from "../../api/character";
import { jsonPreview, listPreview } from "./manageViewUtils";

const behaviorLoadKey = (
  profileId: number,
  chapterFilter: number | "",
  limit: number,
): string => `${profileId}:${chapterFilter === "" ? "all" : chapterFilter}:${limit}`;

export const BehaviorManager: React.FC<{ profileId: number }> = ({ profileId }) => {
  const { addToast } = useToast();
  const [items, setItems] = useState<BehaviorRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadedScopeKey, setLoadedScopeKey] = useState<string | null>(null);
  const [chapterFilter, setChapterFilter] = useState<number | "">("");
  const [limit, setLimit] = useState(20);
  const mountedRef = useRef(true);
  const activeProfileIdRef = useRef(profileId);
  const chapterFilterRef = useRef(chapterFilter);
  const limitRef = useRef(limit);
  const loadSeqRef = useRef(0);
  const loadInFlightKeyRef = useRef<string | null>(null);

  activeProfileIdRef.current = profileId;
  chapterFilterRef.current = chapterFilter;
  limitRef.current = limit;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      loadInFlightKeyRef.current = null;
    };
  }, []);

  const currentScopeKey = behaviorLoadKey(profileId, chapterFilter, limit);
  const hasCurrentResults = loadedScopeKey === currentScopeKey;

  const invalidateLoadedResults = useCallback(() => {
    loadSeqRef.current += 1;
    loadInFlightKeyRef.current = null;
    setLoading(false);
    setLoadedScopeKey(null);
    setItems([]);
  }, []);

  const isCurrentLoad = useCallback((
    requestProfileId: number,
    requestChapterFilter: number | "",
    requestLimit: number,
    seq: number,
  ) => {
    return (
      mountedRef.current &&
      activeProfileIdRef.current === requestProfileId &&
      chapterFilterRef.current === requestChapterFilter &&
      limitRef.current === requestLimit &&
      loadSeqRef.current === seq
    );
  }, []);

  const load = useCallback(async () => {
    const requestProfileId = profileId;
    const requestChapterFilter = chapterFilterRef.current;
    const requestLimit = limitRef.current;
    const requestKey = behaviorLoadKey(requestProfileId, requestChapterFilter, requestLimit);
    if (loadInFlightKeyRef.current === requestKey) return;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    loadInFlightKeyRef.current = requestKey;
    setLoading(true);
    try {
      const chapter = typeof requestChapterFilter === "number" ? requestChapterFilter : null;
      const rows = await listBehaviors({
        profile_id: requestProfileId,
        chapter_from: chapter,
        chapter_to: chapter,
        limit: requestLimit,
      });
      if (!isCurrentLoad(requestProfileId, requestChapterFilter, requestLimit, requestSeq)) return;
      setItems(rows);
      setLoadedScopeKey(requestKey);
    } catch (err) {
      if (!isCurrentLoad(requestProfileId, requestChapterFilter, requestLimit, requestSeq)) return;
      addToast(extractErrorMessage(err, "加载行为失败"), "error");
    } finally {
      if (loadInFlightKeyRef.current === requestKey) {
        loadInFlightKeyRef.current = null;
      }
      if (isCurrentLoad(requestProfileId, requestChapterFilter, requestLimit, requestSeq)) {
        setLoading(false);
      }
    }
  }, [addToast, isCurrentLoad, profileId]);

  useEffect(() => {
    invalidateLoadedResults();
  }, [invalidateLoadedResults, profileId]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="flex flex-col gap-2">
      <div className="text-[10px] text-book-text-muted">
        行为记录由章节同步生成，用于隐性属性复审和后续溯源。
      </div>
      <div className="flex flex-wrap items-center gap-2 text-[11px]">
        <label className="flex items-center gap-1 text-book-text-muted">
          章节
          <input
            type="number"
            min={1}
            value={chapterFilter}
            onChange={(event) => {
              const value = event.target.value;
              if (!value) {
                if (chapterFilterRef.current !== "") invalidateLoadedResults();
                setChapterFilter("");
                return;
              }
              const parsed = Number(value);
              if (Number.isFinite(parsed) && parsed > 0) {
                const next = Math.floor(parsed);
                if (chapterFilterRef.current !== next) invalidateLoadedResults();
                setChapterFilter(next);
              }
            }}
            className="pyqt-input !px-2 !py-1 w-20"
            placeholder="全部"
          />
        </label>
        <label className="flex items-center gap-1 text-book-text-muted">
          数量
          <input
            type="number"
            min={1}
            max={100}
            value={limit}
            onChange={(event) => {
              const parsed = Number(event.target.value);
              if (Number.isFinite(parsed) && parsed > 0) {
                const next = Math.max(1, Math.min(100, Math.floor(parsed)));
                if (limitRef.current !== next) invalidateLoadedResults();
                setLimit(next);
              }
            }}
            className="pyqt-input !px-2 !py-1 w-20"
          />
        </label>
        <BookButton variant="secondary" size="sm" onClick={load} disabled={loading}>
          {loading ? <Loader2 size={11} className="animate-spin" /> : "刷新"}
        </BookButton>
      </div>
      {loading ? (
        <div className="text-[11px] text-book-text-muted">加载…</div>
      ) : !hasCurrentResults ? (
        <div className="text-[11px] text-book-text-muted italic">当前筛选条件尚未加载，请刷新行为记录</div>
      ) : items.length === 0 ? (
        <div className="text-[11px] text-book-text-muted italic">尚无行为记录</div>
      ) : (
        <ul className="space-y-1 max-h-60 overflow-y-auto custom-scrollbar">
          {items.map((item) => (
            <li
              key={item.id}
              className="a4n-stat-tile px-2 py-1.5 text-[11px]"
            >
              <div className="flex items-center gap-1.5 mb-0.5">
                <span className="pyqt-pill !text-[10px] font-mono">
                  第{item.chapter_number}章
                </span>
                <span className="pyqt-pill !text-[10px]">{item.behavior_type}</span>
              </div>
              <div className="text-book-text-sub">{item.behavior_description}</div>
              {item.original_text && (
                <div className="mt-1 rounded bg-book-bg-tertiary/50 px-2 py-1 text-[10px] leading-relaxed text-book-text-muted">
                  原文：{item.original_text}
                </div>
              )}
              <div className="mt-1 flex flex-wrap gap-1 text-[10px] text-book-text-muted">
                {listPreview(item.behavior_tags) && (
                  <span>标签：{listPreview(item.behavior_tags)}</span>
                )}
                {listPreview(item.related_characters) && (
                  <span>相关：{listPreview(item.related_characters)}</span>
                )}
              </div>
              {jsonPreview(item.classification_results) && (
                <div className="mt-0.5 font-mono text-[10px] text-book-text-muted">
                  分类：{jsonPreview(item.classification_results)}
                </div>
              )}
              {item.impact_analysis && (
                <div className="text-book-text-muted italic mt-0.5">
                  影响：{item.impact_analysis}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
