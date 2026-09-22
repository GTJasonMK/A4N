import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { listBehaviors, type BehaviorRecord } from "../../api/character";
import { useI18n } from "../../i18n";
import {
  behaviorTypeLabel,
  jsonPreview,
  listPreview,
  sourceKindLabel,
} from "./manageViewUtils";

const behaviorLoadKey = (
  profileId: number,
  chapterFilter: number | "",
  limit: number,
): string => `${profileId}:${chapterFilter === "" ? "all" : chapterFilter}:${limit}`;

export const BehaviorManager: React.FC<{ profileId: number }> = ({ profileId }) => {
  const { addToast } = useToast();
  const { t } = useI18n();
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
      addToast(extractErrorMessage(err, t("characterDetail.behavior.loadFailed")), "error");
    } finally {
      if (loadInFlightKeyRef.current === requestKey) {
        loadInFlightKeyRef.current = null;
      }
      if (isCurrentLoad(requestProfileId, requestChapterFilter, requestLimit, requestSeq)) {
        setLoading(false);
      }
    }
  }, [addToast, isCurrentLoad, profileId, t]);

  useEffect(() => {
    invalidateLoadedResults();
  }, [invalidateLoadedResults, profileId]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="flex flex-col gap-2">
      <div className="text-caption text-book-text-muted">
        {t("characterDetail.behavior.description")}
      </div>
      <div className="grid grid-cols-2 items-end gap-2 text-small sm:flex sm:flex-wrap">
        <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
          <span>{t("characterDetail.behavior.chapter")}</span>
          <input
            type="number"
            min={1}
            value={chapterFilter}
            disabled={loading}
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
            className="pyqt-input w-full !px-2 !py-1 sm:w-24"
            placeholder={t("characterDetail.behavior.all")}
          />
        </label>
        <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
          <span>{t("characterDetail.behavior.displayCount")}</span>
          <ThemedSelect
            value={limit}
            disabled={loading}
            onChange={(selectedValue) => {
              const parsed = Number(selectedValue);
              if (parsed === 20 || parsed === 50 || parsed === 100) {
                const next = parsed;
                if (limitRef.current !== next) invalidateLoadedResults();
                setLimit(next);
              }
            }}
            className="pyqt-input w-full !px-2 !py-1 sm:w-24"
          >
            <option value={20}>{t("characterDetail.behavior.count", { count: 20 })}</option>
            <option value={50}>{t("characterDetail.behavior.count", { count: 50 })}</option>
            <option value={100}>{t("characterDetail.behavior.count", { count: 100 })}</option>
          </ThemedSelect>
        </label>
        <BookButton
          variant="secondary"
          size="sm"
          onClick={load}
          disabled={loading}
          className="col-span-2 justify-self-end sm:col-auto sm:justify-self-auto"
        >
          {loading ? (
            <span className="inline-flex items-center gap-1.5">
              <LoadingSpinner size="sm" /> {t("characterDetail.behavior.refreshing")}
            </span>
          ) : (
            t("characterDetail.behavior.refresh")
          )}
        </BookButton>
      </div>
      {loading ? (
        <div className="text-small text-book-text-muted">{t("characterDetail.behavior.loading")}</div>
      ) : !hasCurrentResults ? (
        <div className="text-small text-book-text-muted italic">{t("characterDetail.behavior.notLoaded")}</div>
      ) : items.length === 0 ? (
        <div className="text-small text-book-text-muted italic">{t("characterDetail.behavior.empty")}</div>
      ) : (
        <ul className="a4n-flat-surface divide-y divide-book-border/45">
          {items.map((item) => (
            <li
              key={item.id}
              className="px-3 py-2 text-small"
            >
              <div className="mb-1 text-caption text-book-text-muted">
                {t("characterDetail.chapter", { chapter: item.chapter_number })} · {behaviorTypeLabel(item.behavior_type, t)} ·{" "}
                {sourceKindLabel(item.source_kind, t)}
              </div>
              <div className="leading-relaxed text-book-text-main">{item.behavior_description}</div>
              {item.original_text && (
                <blockquote className="mt-1.5 border-l-2 border-book-border/70 pl-2 text-caption leading-relaxed text-book-text-muted">
                  {item.original_text}
                </blockquote>
              )}
              <div className="mt-1.5 flex flex-wrap gap-x-3 gap-y-1 text-caption text-book-text-muted">
                {listPreview(item.behavior_tags, t) && (
                  <span>{t("characterDetail.behavior.tags")}{listPreview(item.behavior_tags, t)}</span>
                )}
                {listPreview(item.related_characters, t) && (
                  <span>{t("characterDetail.behavior.related")}{listPreview(item.related_characters, t)}</span>
                )}
              </div>
              {jsonPreview(item.classification_results) && (
                <details className="mt-1 text-caption text-book-text-muted">
                  <summary className="w-fit cursor-pointer hover:text-book-text-main">{t("characterDetail.diagnostics")}</summary>
                  <div className="mt-1 break-words font-mono">
                    {jsonPreview(item.classification_results)}
                  </div>
                </details>
              )}
              {item.impact_analysis && (
                <div className="text-book-text-muted italic mt-0.5">
                  {t("characterDetail.behavior.impact")}{item.impact_analysis}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
