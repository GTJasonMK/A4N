import React, { useCallback, useEffect, useRef, useState } from "react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import {
  addProfileAttribute,
  attributeHistory,
  modifyProfileAttribute,
  type AttributeEvent,
} from "../../api/character";
import {
  ATTRIBUTE_CATEGORY_OPTIONS,
  ATTRIBUTE_OPERATION_OPTIONS,
  attributeKeyLabel,
  attributeValueText,
  categoryLabel,
  operationLabel,
  sourceKindLabel,
} from "./manageViewUtils";
import { formatUiDateTime } from "../../utils/dateTime";
import { nonEmptyText } from "../../utils/text";

const parseAttributeValue = (raw: string): unknown => {
  const text = raw.trim();
  if (!text) return "";
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
};

const nullableFilterValue = (value: string): string | null => {
  return value ? value : null;
};

const eventLoadKey = (
  profileId: number,
  chapterFrom: number | "",
  chapterTo: number | "",
  category: string,
  key: string,
  operation: string,
): string => [
  profileId,
  chapterFrom === "" ? "all" : chapterFrom,
  chapterTo === "" ? "all" : chapterTo,
  category,
  key,
  operation,
].join(":");

export const EventManager: React.FC<{
  profileId: number;
  defaultChapter: number;
  initialCategory?: string;
  initialKey?: string;
  onChanged?: () => void | Promise<void>;
}> = ({ profileId, defaultChapter, initialCategory = "", initialKey = "", onChanged }) => {
  const { addToast } = useToast();
  const [events, setEvents] = useState<AttributeEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadedScopeKey, setLoadedScopeKey] = useState<string | null>(null);
  const [filterChapterFrom, setFilterChapterFrom] = useState<number | "">("");
  const [filterChapterTo, setFilterChapterTo] = useState<number | "">("");
  const [filterCategory, setFilterCategory] = useState(initialCategory);
  const [filterKey, setFilterKey] = useState(initialKey);
  const [filterOperation, setFilterOperation] = useState("");
  const [mode, setMode] = useState<"add" | "modify">("add");
  const [chapterNumber, setChapterNumber] = useState(defaultChapter);
  const [category, setCategory] = useState("explicit");
  const [key, setKey] = useState("");
  const [valueText, setValueText] = useState("");
  const [cause, setCause] = useState("");
  const [evidence, setEvidence] = useState("");
  const [saving, setSaving] = useState(false);
  const mountedRef = useRef(true);
  const activeProfileIdRef = useRef(profileId);
  const filterChapterFromRef = useRef(filterChapterFrom);
  const filterChapterToRef = useRef(filterChapterTo);
  const filterCategoryRef = useRef(filterCategory);
  const filterKeyRef = useRef(filterKey);
  const filterOperationRef = useRef(filterOperation);
  const loadSeqRef = useRef(0);
  const saveSeqRef = useRef(0);
  const loadInFlightKeyRef = useRef<string | null>(null);
  const currentFilterScopeKeyRef = useRef("");
  const saveInFlightRef = useRef(false);

  activeProfileIdRef.current = profileId;
  filterChapterFromRef.current = filterChapterFrom;
  filterChapterToRef.current = filterChapterTo;
  filterCategoryRef.current = filterCategory;
  filterKeyRef.current = filterKey;
  filterOperationRef.current = filterOperation;
  const currentFilterScopeKey = eventLoadKey(
    profileId,
    filterChapterFrom,
    filterChapterTo,
    filterCategory,
    filterKey,
    filterOperation,
  );
  currentFilterScopeKeyRef.current = currentFilterScopeKey;
  const hasCurrentEvents = loadedScopeKey === currentFilterScopeKey;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      saveSeqRef.current += 1;
      loadInFlightKeyRef.current = null;
      saveInFlightRef.current = false;
    };
  }, []);

  const isCurrentProfile = useCallback((requestProfileId: number) => {
    return mountedRef.current && activeProfileIdRef.current === requestProfileId;
  }, []);

  const invalidateLoadedEvents = useCallback(() => {
    loadSeqRef.current += 1;
    loadInFlightKeyRef.current = null;
    setLoading(false);
    setLoadedScopeKey(null);
    setEvents([]);
  }, []);

  const isCurrentLoad = useCallback((requestProfileId: number, requestKey: string, seq: number) => {
    return (
      isCurrentProfile(requestProfileId) &&
      currentFilterScopeKeyRef.current === requestKey &&
      loadSeqRef.current === seq
    );
  }, [isCurrentProfile]);

  const load = useCallback(
    async (
      nextChapterFrom: number | "" = filterChapterFromRef.current,
      nextChapterTo: number | "" = filterChapterToRef.current,
      nextCategory = filterCategoryRef.current,
      nextKey = filterKeyRef.current,
      nextOperation = filterOperationRef.current,
    ) => {
      const requestProfileId = profileId;
      const requestKey = eventLoadKey(
        requestProfileId,
        nextChapterFrom,
        nextChapterTo,
        nextCategory,
        nextKey,
        nextOperation,
      );
      if (loadInFlightKeyRef.current === requestKey) return;
      const requestSeq = loadSeqRef.current + 1;
      loadSeqRef.current = requestSeq;
      loadInFlightKeyRef.current = requestKey;
      setLoading(true);
      try {
        const rows = await attributeHistory(requestProfileId, {
          chapter_from: nextChapterFrom === "" ? null : nextChapterFrom,
          chapter_to: nextChapterTo === "" ? null : nextChapterTo,
          category: nullableFilterValue(nextCategory),
          key: nonEmptyText(nextKey),
          operation: nullableFilterValue(nextOperation),
        });
        if (!isCurrentLoad(requestProfileId, requestKey, requestSeq)) return;
        setEvents(rows);
        setLoadedScopeKey(requestKey);
      } catch (err) {
        if (!isCurrentLoad(requestProfileId, requestKey, requestSeq)) return;
        addToast(extractErrorMessage(err, "加载事件失败"), "error");
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
    const normalizedCategory = initialCategory?.trim();
    const normalizedKey = initialKey?.trim();
    const nextCategory = normalizedCategory ? normalizedCategory : "";
    const nextKey = normalizedKey ? normalizedKey : "";
    const nextScopeKey = eventLoadKey(profileId, "", "", nextCategory, nextKey, "");
    currentFilterScopeKeyRef.current = nextScopeKey;
    setFilterChapterFrom("");
    setFilterChapterTo("");
    setFilterCategory(nextCategory);
    setFilterKey(nextKey);
    setFilterOperation("");
    invalidateLoadedEvents();
    void load(
      "",
      "",
      nextCategory,
      nextKey,
      "",
    );
  }, [initialCategory, initialKey, invalidateLoadedEvents, load, profileId]);

  useEffect(() => {
    saveSeqRef.current += 1;
    saveInFlightRef.current = false;
    setSaving(false);
    setChapterNumber(defaultChapter);
    setKey("");
    setValueText("");
    setCause("");
    setEvidence("");
  }, [defaultChapter, profileId]);

  const resetFilters = async () => {
    const nextScopeKey = eventLoadKey(profileId, "", "", "", "", "");
    currentFilterScopeKeyRef.current = nextScopeKey;
    setFilterChapterFrom("");
    setFilterChapterTo("");
    setFilterCategory("");
    setFilterKey("");
    setFilterOperation("");
    invalidateLoadedEvents();
    await load("", "", "", "", "");
  };

  const handleSubmit = async () => {
    const attrKey = key.trim();
    const attrCause = cause.trim();
    const attrEvidence = evidence.trim();
    if (!attrKey || !attrCause || !attrEvidence) {
      addToast("属性名称、原因和证据不能为空", "error");
      return;
    }
    if (saveInFlightRef.current) return;
    const requestProfileId = profileId;
    const requestMode = mode;
    const requestPayload = {
      chapter_number: chapterNumber,
      category,
      key: attrKey,
      value: parseAttributeValue(valueText),
      cause: attrCause,
      evidence: attrEvidence,
    };
    const requestSeq = saveSeqRef.current + 1;
    saveSeqRef.current = requestSeq;
    saveInFlightRef.current = true;
    setSaving(true);
    try {
      const result =
        requestMode === "add"
          ? await addProfileAttribute({
              profile_id: requestProfileId,
              chapter_number: requestPayload.chapter_number,
              category: requestPayload.category,
              key: requestPayload.key,
              value: requestPayload.value,
              cause: requestPayload.cause,
              evidence: requestPayload.evidence,
            })
          : await modifyProfileAttribute({
              profile_id: requestProfileId,
              chapter_number: requestPayload.chapter_number,
              category: requestPayload.category,
              key: requestPayload.key,
              new_value: requestPayload.value,
              cause: requestPayload.cause,
              evidence: requestPayload.evidence,
            });
      if (!isCurrentProfile(requestProfileId) || saveSeqRef.current !== requestSeq) return;
      const successMessage = requestMode === "add" ? "已新增属性" : "已修改属性";
      addToast(
        result.warnings.length > 0
          ? `${successMessage}；${result.warnings.join("；")}`
          : successMessage,
        result.warnings.length > 0 ? "info" : "success",
      );
      setKey("");
      setValueText("");
      setCause("");
      setEvidence("");
      await load();
      if (!isCurrentProfile(requestProfileId) || saveSeqRef.current !== requestSeq) return;
      await onChanged?.();
    } catch (err) {
      if (!isCurrentProfile(requestProfileId) || saveSeqRef.current !== requestSeq) return;
      addToast(
        extractErrorMessage(err, requestMode === "add" ? "新增属性失败" : "修改属性失败"),
        "error",
      );
    } finally {
      if (isCurrentProfile(requestProfileId) && saveSeqRef.current === requestSeq) {
        saveInFlightRef.current = false;
        setSaving(false);
      }
    }
  };

  const chapterFilterLabel =
    filterChapterFrom !== "" || filterChapterTo !== ""
      ? filterChapterFrom !== "" && filterChapterTo !== ""
        ? `第 ${filterChapterFrom}-${filterChapterTo} 章`
        : filterChapterFrom !== ""
          ? `第 ${filterChapterFrom} 章起`
          : `截至第 ${filterChapterTo} 章`
      : null;
  const filterSummary = [
    chapterFilterLabel,
    filterCategory ? categoryLabel(filterCategory) : null,
    filterOperation ? operationLabel(filterOperation) : null,
    filterKey.trim() ? attributeKeyLabel(filterKey) : null,
  ]
    .filter(Boolean)
    .join(" · ") || "全部记录";
  const submitReady =
    key.trim().length > 0 &&
    cause.trim().length > 0 &&
    evidence.trim().length > 0;

  return (
    <div className="flex flex-col gap-2">
      <div className="text-caption text-book-text-muted">
        属性事件记录每次角色属性变化，手动维护也会写入同一条事件溯源链路。
      </div>
      <details className="border-y border-book-border/35 py-1">
        <summary className="flex cursor-pointer list-none items-center gap-2 py-1 text-small [&::-webkit-details-marker]:hidden">
          <span className="font-medium text-book-text-main">筛选事件</span>
          <span className="min-w-0 truncate text-caption text-book-text-muted">
            {filterSummary}
            {hasCurrentEvents ? ` · ${events.length} 条` : ""}
          </span>
        </summary>
        <div className="mt-2 grid grid-cols-2 gap-2 text-small sm:grid-cols-3">
          <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
            起始章
            <input
              type="number"
              min={1}
              value={filterChapterFrom}
              onChange={(event) => {
                const next = Number(event.target.value);
                const nextValue = event.target.value
                  ? Math.max(1, Number.isFinite(next) ? Math.floor(next) : 1)
                  : "";
                if (filterChapterFromRef.current !== nextValue) invalidateLoadedEvents();
                setFilterChapterFrom(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
            />
          </label>
          <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
            结束章
            <input
              type="number"
              min={1}
              value={filterChapterTo}
              onChange={(event) => {
                const next = Number(event.target.value);
                const nextValue = event.target.value
                  ? Math.max(1, Number.isFinite(next) ? Math.floor(next) : 1)
                  : "";
                if (filterChapterToRef.current !== nextValue) invalidateLoadedEvents();
                setFilterChapterTo(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
            />
          </label>
          <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
            类别
            <select
              value={filterCategory}
              onChange={(event) => {
                const nextValue = event.target.value;
                if (filterCategoryRef.current !== nextValue) invalidateLoadedEvents();
                setFilterCategory(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
            >
              <option value="">全部</option>
              {ATTRIBUTE_CATEGORY_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
            操作
            <select
              value={filterOperation}
              onChange={(event) => {
                const nextValue = event.target.value;
                if (filterOperationRef.current !== nextValue) invalidateLoadedEvents();
                setFilterOperation(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
            >
              <option value="">全部</option>
              {ATTRIBUTE_OPERATION_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label className="col-span-2 flex min-w-0 flex-col gap-1 text-book-text-muted sm:col-span-2">
            属性名称
            <input
              value={filterKey}
              onChange={(event) => {
                const nextValue = event.target.value;
                if (filterKeyRef.current !== nextValue) invalidateLoadedEvents();
                setFilterKey(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
              placeholder="全部"
            />
          </label>
          <div className="col-span-2 flex items-center justify-end gap-2 sm:col-span-3">
            <BookButton variant="ghost" size="sm" onClick={() => void resetFilters()} disabled={loading}>
              清空条件
            </BookButton>
            <BookButton variant="secondary" size="sm" onClick={() => void load()} disabled={loading}>
              {loading ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> 筛选中…
                </span>
              ) : (
                "应用筛选"
              )}
            </BookButton>
          </div>
        </div>
      </details>
      <details className="border-b border-book-border/35 py-1">
        <summary className="cursor-pointer py-1 text-small font-medium text-book-text-main">
          手动维护属性
        </summary>
        <div className="mt-2 grid gap-2 text-small sm:grid-cols-2">
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">操作</span>
            <select
              value={mode}
              onChange={(event) => setMode(event.target.value === "modify" ? "modify" : "add")}
              disabled={saving}
              className="pyqt-input !px-2 !py-1"
            >
              <option value="add">新增</option>
              <option value="modify">修改</option>
            </select>
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">章节</span>
            <input
              type="number"
              min={1}
              value={chapterNumber}
              disabled={saving}
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
            <span className="text-caption text-book-text-muted">分类</span>
            <select
              value={category}
              onChange={(event) => setCategory(event.target.value)}
              disabled={saving}
              className="pyqt-input !px-2 !py-1"
            >
              {ATTRIBUTE_CATEGORY_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">属性名称</span>
            <input
              value={key}
              onChange={(event) => setKey(event.target.value)}
              disabled={saving}
              className="pyqt-input !px-2 !py-1"
              placeholder="例如：外貌"
            />
          </label>
          <label className="flex flex-col gap-1 sm:col-span-2">
            <span className="text-caption text-book-text-muted">属性值</span>
            <textarea
              value={valueText}
              onChange={(event) => setValueText(event.target.value)}
              disabled={saving}
              className="book-control min-h-[58px] resize-y rounded-xl px-2.5 py-2 text-small"
              placeholder="填写角色当前的属性内容"
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">原因</span>
            <textarea
              value={cause}
              onChange={(event) => setCause(event.target.value)}
              disabled={saving}
              className="pyqt-input min-h-[58px] !px-2 !py-1 text-small"
              placeholder="说明这次属性变化的原因"
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">证据</span>
            <textarea
              value={evidence}
              onChange={(event) => setEvidence(event.target.value)}
              disabled={saving}
              className="pyqt-input min-h-[58px] !px-2 !py-1 text-small"
              placeholder="引用章节原文或明确来源"
            />
          </label>
          <div className="flex justify-end sm:col-span-2">
            <BookButton
              size="sm"
              variant="primary"
              onClick={handleSubmit}
              disabled={saving || !submitReady}
            >
              {saving ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" />
                  {mode === "add" ? "新增中…" : "保存中…"}
                </span>
              ) : mode === "add" ? (
                "新增属性"
              ) : (
                "保存修改"
              )}
            </BookButton>
          </div>
        </div>
      </details>
      {loading ? (
        <div className="text-small text-book-text-muted">加载…</div>
      ) : !hasCurrentEvents ? (
        <div className="text-small text-book-text-muted italic">当前筛选条件尚未加载，请筛选</div>
      ) : events.length === 0 ? (
        <div className="text-small text-book-text-muted italic">尚无事件</div>
      ) : (
        <ul className="a4n-flat-surface divide-y divide-book-border/45">
          {events.map((event) => {
            const oldValue = attributeValueText(event.old_value);
            const newValue = attributeValueText(event.new_value);
            return (
              <li
                key={event.id}
                className="px-3 py-2.5 text-small"
              >
                <div className="mb-1 flex min-w-0 flex-wrap items-baseline gap-x-1.5 gap-y-0.5">
                  <span className="font-medium text-book-text-main">
                    {attributeKeyLabel(event.key)}
                  </span>
                  <span className="text-caption text-book-text-muted">
                    第 {event.chapter_number} 章 · {categoryLabel(event.category)} ·{" "}
                    {operationLabel(event.operation)} · {sourceKindLabel(event.source_kind)}
                  </span>
                  {event.timestamp && (
                    <span className="ml-auto shrink-0 text-caption text-book-text-muted">
                      {formatUiDateTime(event.timestamp)}
                    </span>
                  )}
                </div>
                {(oldValue || newValue) && (
                  <div className="grid divide-y divide-book-border/30 border-y border-book-border/30 text-caption sm:grid-cols-2 sm:divide-x sm:divide-y-0">
                    {oldValue && (
                      <div className="px-2 py-1.5">
                        <span className="text-book-text-muted">旧值：</span>
                        <span className="break-words text-book-text-sub">{oldValue}</span>
                      </div>
                    )}
                    {newValue && (
                      <div className="px-2 py-1.5">
                        <span className="text-book-text-muted">新值：</span>
                        <span className="break-words text-book-text-sub">{newValue}</span>
                      </div>
                    )}
                  </div>
                )}
                {event.change_description && (
                  <div className="mt-1 text-caption leading-relaxed text-book-text-main">
                    摘要：{event.change_description}
                  </div>
                )}
                {event.cause && (
                  <div className="mt-1 text-caption leading-relaxed text-book-text-sub">
                    原因：{event.cause}
                  </div>
                )}
                {event.evidence && (
                  <blockquote className="mt-1.5 border-l-2 border-book-border/70 pl-2 text-caption leading-relaxed text-book-text-muted">
                    {event.evidence}
                  </blockquote>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
};
