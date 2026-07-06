import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
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
  CATEGORY_LABELS,
  OPERATION_LABELS,
  compactJson,
} from "./manageViewUtils";
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
      addToast("属性 key、原因和证据不能为空", "error");
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
      if (requestMode === "add") {
        await addProfileAttribute({
          profile_id: requestProfileId,
          chapter_number: requestPayload.chapter_number,
          category: requestPayload.category,
          key: requestPayload.key,
          value: requestPayload.value,
          cause: requestPayload.cause,
          evidence: requestPayload.evidence,
        });
      } else {
        await modifyProfileAttribute({
          profile_id: requestProfileId,
          chapter_number: requestPayload.chapter_number,
          category: requestPayload.category,
          key: requestPayload.key,
          new_value: requestPayload.value,
          cause: requestPayload.cause,
          evidence: requestPayload.evidence,
        });
      }
      if (!isCurrentProfile(requestProfileId) || saveSeqRef.current !== requestSeq) return;
      addToast(requestMode === "add" ? "已新增属性" : "已修改属性", "success");
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

  return (
    <div className="flex flex-col gap-2">
      <div className="text-[10px] text-book-text-muted">
        属性事件记录每次角色属性变化，手动维护也会写入同一条事件溯源链路。
      </div>
      <div className="flex flex-wrap items-end gap-2 text-[11px]">
        <label className="flex flex-col gap-1 text-book-text-muted">
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
            className="pyqt-input !w-20 !px-2 !py-1"
          />
        </label>
        <label className="flex flex-col gap-1 text-book-text-muted">
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
            className="pyqt-input !w-20 !px-2 !py-1"
          />
        </label>
        <label className="flex flex-col gap-1 text-book-text-muted">
          类别
          <select
            value={filterCategory}
            onChange={(event) => {
              const nextValue = event.target.value;
              if (filterCategoryRef.current !== nextValue) invalidateLoadedEvents();
              setFilterCategory(nextValue);
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
        <label className="flex flex-col gap-1 text-book-text-muted">
          操作
          <select
            value={filterOperation}
            onChange={(event) => {
              const nextValue = event.target.value;
              if (filterOperationRef.current !== nextValue) invalidateLoadedEvents();
              setFilterOperation(nextValue);
            }}
            className="pyqt-input !px-2 !py-1"
          >
            <option value="">全部</option>
            {ATTRIBUTE_OPERATION_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-book-text-muted">
          属性 key
          <input
            value={filterKey}
            onChange={(event) => {
              const nextValue = event.target.value;
              if (filterKeyRef.current !== nextValue) invalidateLoadedEvents();
              setFilterKey(nextValue);
            }}
            className="pyqt-input !w-28 !px-2 !py-1"
            placeholder="全部"
          />
        </label>
        <BookButton variant="secondary" size="sm" onClick={() => void load()} disabled={loading}>
          {loading ? <Loader2 size={11} className="animate-spin" /> : "筛选"}
        </BookButton>
        <button
          type="button"
          onClick={() => void resetFilters()}
          className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[26px]"
          disabled={loading}
        >
          清空
        </button>
        <span className="pb-1 text-[10px] text-book-text-muted">
          当前 {hasCurrentEvents ? events.length : 0} 条
        </span>
      </div>
      <details className="a4n-page-surface px-2 py-1.5">
        <summary className="cursor-pointer text-[11px] font-medium text-book-text-main">
          手动维护属性
        </summary>
        <div className="mt-2 grid gap-2 text-[11px] md:grid-cols-6">
          <label className="flex flex-col gap-1">
            <span className="text-[10px] text-book-text-muted">操作</span>
            <select
              value={mode}
              onChange={(event) => setMode(event.target.value === "modify" ? "modify" : "add")}
              className="pyqt-input !px-2 !py-1"
            >
              <option value="add">新增</option>
              <option value="modify">修改</option>
            </select>
          </label>
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
          <label className="flex flex-col gap-1 md:col-span-3">
            <span className="text-[10px] text-book-text-muted">属性 key</span>
            <input
              value={key}
              onChange={(event) => setKey(event.target.value)}
              className="pyqt-input !px-2 !py-1"
              placeholder="例如 appearance"
            />
          </label>
          <label className="flex flex-col gap-1 md:col-span-6">
            <span className="text-[10px] text-book-text-muted">属性值</span>
            <textarea
              value={valueText}
              onChange={(event) => setValueText(event.target.value)}
              className="pyqt-input min-h-[58px] !px-2 !py-1 font-mono text-[11px]"
              placeholder='可填普通文本，也可填 JSON，例如 {"身份":"掌柜"}'
            />
          </label>
          <label className="flex flex-col gap-1 md:col-span-3">
            <span className="text-[10px] text-book-text-muted">原因</span>
            <textarea
              value={cause}
              onChange={(event) => setCause(event.target.value)}
              className="pyqt-input min-h-[58px] !px-2 !py-1 text-[11px]"
              placeholder="说明这次属性变化的原因"
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
            <BookButton size="sm" variant="secondary" onClick={handleSubmit} disabled={saving}>
              {saving ? <Loader2 size={12} className="animate-spin" /> : "写入事件"}
            </BookButton>
          </div>
        </div>
      </details>
      {loading ? (
        <div className="text-[11px] text-book-text-muted">加载…</div>
      ) : !hasCurrentEvents ? (
        <div className="text-[11px] text-book-text-muted italic">当前筛选条件尚未加载，请筛选</div>
      ) : events.length === 0 ? (
        <div className="text-[11px] text-book-text-muted italic">尚无事件</div>
      ) : (
        <ul className="max-h-72 space-y-1 overflow-y-auto custom-scrollbar">
          {events.map((event) => {
            const oldValue = compactJson(event.old_value);
            const newValue = compactJson(event.new_value);
            return (
              <li
                key={event.id}
                className="a4n-stat-tile px-2 py-1.5 text-[11px]"
              >
                <div className="mb-1 flex flex-wrap items-center gap-1.5">
                  <span className="pyqt-pill !text-[10px]">第{event.chapter_number}章</span>
                  <span className="pyqt-pill !text-[10px]">
                    {CATEGORY_LABELS[event.category] ?? event.category}
                  </span>
                  <span
                    className={`pyqt-pill !text-[10px] ${
                      event.operation === "delete"
                        ? "!border-error/40 !text-error"
                        : event.operation === "add"
                          ? "!border-success/40 !text-success"
                          : ""
                    }`}
                  >
                    {OPERATION_LABELS[event.operation] ?? event.operation}
                  </span>
                  <span className="font-mono text-book-text-sub">{event.key}</span>
                  {event.timestamp && (
                    <span className="ml-auto text-[10px] text-book-text-muted">
                      {event.timestamp}
                    </span>
                  )}
                </div>
                {(oldValue || newValue) && (
                  <div className="grid gap-1 text-[10px] md:grid-cols-2">
                    {oldValue && (
                      <div className="rounded bg-book-bg-tertiary/40 px-2 py-1">
                        <span className="text-book-text-muted">旧值：</span>
                        <span className="font-mono text-book-text-sub">{oldValue}</span>
                      </div>
                    )}
                    {newValue && (
                      <div className="rounded bg-book-bg-tertiary/40 px-2 py-1">
                        <span className="text-book-text-muted">新值：</span>
                        <span className="font-mono text-book-text-sub">{newValue}</span>
                      </div>
                    )}
                  </div>
                )}
                {event.change_description && (
                  <div className="mt-1 text-[10px] leading-relaxed text-book-text-main">
                    摘要：{event.change_description}
                  </div>
                )}
                {event.cause && (
                  <div className="mt-1 text-[10px] leading-relaxed text-book-text-sub">
                    原因：{event.cause}
                  </div>
                )}
                {event.evidence && (
                  <div className="mt-1 rounded bg-book-bg-tertiary/35 px-2 py-1 text-[10px] leading-relaxed text-book-text-muted">
                    证据：{event.evidence}
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
