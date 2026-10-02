import { ThemedSelect } from "../../components/ui/ThemedSelect";
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
  characterDetailDraftKey,
  operationLabel,
  parsePositiveChapterNumber,
  sourceKindLabel,
} from "./manageViewUtils";
import { nonEmptyText } from "../../utils/text";
import type { CharacterAttributeCategory } from "../../api/generated/host";
import { formatDateTime, useI18n } from "../../i18n";
import { useDirty } from "../../components/navigation/dirtyRegistry";

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
  chapterFrom: string,
  chapterTo: string,
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
  onChanged?: () => boolean | void | Promise<boolean | void>;
}> = ({ profileId, defaultChapter, initialCategory = "", initialKey = "", onChanged }) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const [events, setEvents] = useState<AttributeEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadedScopeKey, setLoadedScopeKey] = useState<string | null>(null);
  const [filterError, setFilterError] = useState<string | null>(null);
  const [filterChapterFrom, setFilterChapterFrom] = useState("");
  const [filterChapterTo, setFilterChapterTo] = useState("");
  const [filterCategory, setFilterCategory] = useState(initialCategory);
  const [filterKey, setFilterKey] = useState(initialKey);
  const [filterOperation, setFilterOperation] = useState("");
  const [mode, setMode] = useState<"add" | "modify">("add");
  const [chapterNumber, setChapterNumber] = useState(String(defaultChapter));
  const [category, setCategory] = useState("explicit");
  const [key, setKey] = useState("");
  const [valueText, setValueText] = useState("");
  const [cause, setCause] = useState("");
  const [evidence, setEvidence] = useState("");
  const [chapterError, setChapterError] = useState<string | null>(null);
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
  const defaultCategory = initialCategory.trim() || "explicit";
  const defaultKey = initialKey.trim();
  const defaultMode = defaultKey ? "modify" : "add";
  const formDirty =
    mode !== defaultMode ||
    chapterNumber !== String(defaultChapter) ||
    category !== defaultCategory ||
    key.trim().length > 0 ||
    valueText.trim().length > 0 ||
    cause.trim().length > 0 ||
    evidence.trim().length > 0;

  const discardFormDraft = useCallback(() => {
    if (saving) return false;
    setMode(defaultMode);
    setChapterNumber(String(defaultChapter));
    setChapterError(null);
    setCategory(defaultCategory);
    setKey(defaultKey);
    setValueText("");
    setCause("");
    setEvidence("");
    return true;
  }, [defaultCategory, defaultChapter, defaultKey, defaultMode, saving]);

  useDirty(
    characterDetailDraftKey(profileId, "event"),
    formDirty,
    discardFormDraft,
    () => !saving,
  );

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
      nextChapterFrom: string = filterChapterFromRef.current,
      nextChapterTo: string = filterChapterToRef.current,
      nextCategory = filterCategoryRef.current,
      nextKey = filterKeyRef.current,
      nextOperation = filterOperationRef.current,
    ): Promise<boolean> => {
      const requestProfileId = profileId;
      const requestKey = eventLoadKey(
        requestProfileId,
        nextChapterFrom,
        nextChapterTo,
        nextCategory,
        nextKey,
        nextOperation,
      );
      const chapterFrom = nextChapterFrom ? parsePositiveChapterNumber(nextChapterFrom) : null;
      const chapterTo = nextChapterTo ? parsePositiveChapterNumber(nextChapterTo) : null;
      if ((nextChapterFrom && chapterFrom == null) || (nextChapterTo && chapterTo == null)) {
        setFilterError(t("characterDetail.invalidChapterInteger", {
          label: t("characterDetail.event.chapter"),
        }));
        return false;
      }
      if (chapterFrom != null && chapterTo != null && chapterFrom > chapterTo) {
        setFilterError(t("characterDetail.invalidChapterRange"));
        return false;
      }
      setFilterError(null);
      if (loadInFlightKeyRef.current === requestKey) return false;
      const requestSeq = loadSeqRef.current + 1;
      loadSeqRef.current = requestSeq;
      loadInFlightKeyRef.current = requestKey;
      setLoading(true);
      try {
        const rows = await attributeHistory(requestProfileId, {
          chapter_from: chapterFrom,
          chapter_to: chapterTo,
          category: nullableFilterValue(nextCategory),
          key: nonEmptyText(nextKey),
          operation: nullableFilterValue(nextOperation),
        });
        if (!isCurrentLoad(requestProfileId, requestKey, requestSeq)) return false;
        setEvents(rows);
        setLoadedScopeKey(requestKey);
        return true;
      } catch (err) {
        if (!isCurrentLoad(requestProfileId, requestKey, requestSeq)) return false;
        addToast(extractErrorMessage(err, t("characterDetail.event.loadFailed")), "error");
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
    setFilterError(null);
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
    setChapterNumber(String(defaultChapter));
    setChapterError(null);
    const nextCategory = initialCategory.trim() || "explicit";
    const nextKey = initialKey.trim();
    setMode(nextKey ? "modify" : "add");
    setCategory(nextCategory);
    setKey(nextKey);
    setValueText("");
    setCause("");
    setEvidence("");
  }, [defaultChapter, initialCategory, initialKey, profileId]);

  const resetFilters = async () => {
    const nextScopeKey = eventLoadKey(profileId, "", "", "", "", "");
    currentFilterScopeKeyRef.current = nextScopeKey;
    setFilterChapterFrom("");
    setFilterChapterTo("");
    setFilterCategory("");
    setFilterKey("");
    setFilterOperation("");
    setFilterError(null);
    invalidateLoadedEvents();
    await load("", "", "", "", "");
  };

  const handleSubmit = async () => {
    const attrKey = key.trim();
    const attrCause = cause.trim();
    const attrEvidence = evidence.trim();
    if (!attrKey || !attrCause || !attrEvidence) {
      addToast(t("characterDetail.event.required"), "error");
      return;
    }
    const parsedChapterNumber = parsePositiveChapterNumber(chapterNumber);
    if (parsedChapterNumber == null) {
      setChapterError(t("characterDetail.invalidChapterInteger", {
        label: t("characterDetail.event.chapter"),
      }));
      return;
    }
    setChapterError(null);
    if (saveInFlightRef.current) return;
    const requestProfileId = profileId;
    const requestMode = mode;
    const requestPayload = {
      chapter_number: parsedChapterNumber,
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
      await (
        requestMode === "add"
          ? addProfileAttribute({
              profile_id: requestProfileId,
              chapter_number: requestPayload.chapter_number,
              category: requestPayload.category as CharacterAttributeCategory,
              key: requestPayload.key,
              value: requestPayload.value,
              cause: requestPayload.cause,
              evidence: requestPayload.evidence,
            })
          : modifyProfileAttribute({
              profile_id: requestProfileId,
              chapter_number: requestPayload.chapter_number,
              category: requestPayload.category as CharacterAttributeCategory,
              key: requestPayload.key,
              new_value: requestPayload.value,
              cause: requestPayload.cause,
              evidence: requestPayload.evidence,
            })
      );
      if (!isCurrentProfile(requestProfileId) || saveSeqRef.current !== requestSeq) return;
      const successMessage = requestMode === "add"
        ? t("characterDetail.event.added")
        : t("characterDetail.event.modified");
      const refreshed = await load();
      if (!refreshed) addToast(t("characterDetail.refreshFailed"), "error");
      try {
        if (await onChanged?.() === false) {
          addToast(t("characterDetail.refreshFailed"), "error");
        }
      } catch (refreshError) {
        addToast(extractErrorMessage(refreshError, t("characterDetail.refreshFailed")), "error");
      }
      addToast(successMessage, "success");
      setKey("");
      setValueText("");
      setCause("");
      setEvidence("");
      if (!isCurrentProfile(requestProfileId) || saveSeqRef.current !== requestSeq) return;
    } catch (err) {
      if (!isCurrentProfile(requestProfileId) || saveSeqRef.current !== requestSeq) return;
      addToast(
        extractErrorMessage(
          err,
          requestMode === "add"
            ? t("characterDetail.event.addFailed")
            : t("characterDetail.event.modifyFailed"),
        ),
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
        ? t("characterDetail.event.chapterRange", {
            start: filterChapterFrom,
            end: filterChapterTo,
          })
        : filterChapterFrom !== ""
          ? t("characterDetail.event.fromChapter", { chapter: filterChapterFrom })
          : t("characterDetail.event.toChapter", { chapter: filterChapterTo })
      : null;
  const filterSummary = [
    chapterFilterLabel,
    filterCategory ? categoryLabel(filterCategory, t) : null,
    filterOperation ? operationLabel(filterOperation, t) : null,
    filterKey.trim() ? attributeKeyLabel(filterKey, t) : null,
  ]
    .filter(Boolean)
    .join(" · ") || t("characterDetail.event.allRecords");
  const submitReady =
    key.trim().length > 0 &&
    cause.trim().length > 0 &&
    evidence.trim().length > 0;

  return (
    <div className="flex flex-col gap-2">
      <div className="text-caption text-book-text-muted">
        {t("characterDetail.event.description")}
      </div>
      {filterError ? (
        <p id="event-filter-error" role="alert" className="text-small text-error">{filterError}</p>
      ) : null}
      <details className="border-y border-book-border/35 py-1">
        <summary className="flex cursor-pointer list-none items-center gap-2 py-1 text-small [&::-webkit-details-marker]:hidden">
          <span className="font-medium text-book-text-main">{t("characterDetail.event.filter")}</span>
          <span className="min-w-0 truncate text-caption text-book-text-muted">
            {filterSummary}
            {hasCurrentEvents ? ` · ${t("characterDetail.event.count", { count: events.length })}` : ""}
          </span>
        </summary>
        <div className="mt-2 grid grid-cols-2 gap-2 text-small sm:grid-cols-3">
          <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
            {t("characterDetail.event.startChapter")}
            <input
              type="number"
              min={1}
              step={1}
              aria-invalid={filterError != null}
              aria-describedby={filterError ? "event-filter-error" : undefined}
              value={filterChapterFrom}
              onChange={(event) => {
                const nextValue = event.target.value;
                if (filterChapterFromRef.current !== nextValue) invalidateLoadedEvents();
                setFilterError(null);
                setFilterChapterFrom(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
            />
          </label>
          <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
            {t("characterDetail.event.endChapter")}
            <input
              type="number"
              min={1}
              step={1}
              aria-invalid={filterError != null}
              aria-describedby={filterError ? "event-filter-error" : undefined}
              value={filterChapterTo}
              onChange={(event) => {
                const nextValue = event.target.value;
                if (filterChapterToRef.current !== nextValue) invalidateLoadedEvents();
                setFilterError(null);
                setFilterChapterTo(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
            />
          </label>
          <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
            {t("characterDetail.event.category")}
            <ThemedSelect
              value={filterCategory}
              onChange={(nextValue) => {
                if (filterCategoryRef.current !== nextValue) invalidateLoadedEvents();
                setFilterError(null);
                setFilterCategory(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
            >
              <option value="">{t("characterDetail.behavior.all")}</option>
              {ATTRIBUTE_CATEGORY_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {categoryLabel(option.value, t)}
                </option>
              ))}
            </ThemedSelect>
          </label>
          <label className="flex min-w-0 flex-col gap-1 text-book-text-muted">
            {t("characterDetail.event.operation")}
            <ThemedSelect
              value={filterOperation}
              onChange={(nextValue) => {
                if (filterOperationRef.current !== nextValue) invalidateLoadedEvents();
                setFilterError(null);
                setFilterOperation(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
            >
              <option value="">{t("characterDetail.behavior.all")}</option>
              {ATTRIBUTE_OPERATION_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {operationLabel(option.value, t)}
                </option>
              ))}
            </ThemedSelect>
          </label>
          <label className="col-span-2 flex min-w-0 flex-col gap-1 text-book-text-muted sm:col-span-2">
            {t("characterDetail.event.attribute")}
            <input
              value={filterKey}
              onChange={(event) => {
                const nextValue = event.target.value;
                if (filterKeyRef.current !== nextValue) invalidateLoadedEvents();
                setFilterError(null);
                setFilterKey(nextValue);
              }}
              className="pyqt-input w-full !px-2 !py-1"
              placeholder={t("characterDetail.behavior.all")}
            />
          </label>
          <div className="col-span-2 flex items-center justify-end gap-2 sm:col-span-3">
            <BookButton variant="ghost" size="sm" onClick={() => void resetFilters()} disabled={loading}>
              {t("characterDetail.event.clear")}
            </BookButton>
            <BookButton variant="secondary" size="sm" onClick={() => void load()} disabled={loading}>
              {loading ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> {t("characterDetail.event.filtering")}
                </span>
              ) : (
                t("characterDetail.event.apply")
              )}
            </BookButton>
          </div>
        </div>
      </details>
      <details className="border-b border-book-border/35 py-1">
        <summary className="cursor-pointer py-1 text-small font-medium text-book-text-main">
          {t("characterDetail.event.maintain")}
        </summary>
        <div className="mt-2 grid gap-2 text-small sm:grid-cols-2">
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">{t("characterDetail.event.operation")}</span>
            <ThemedSelect
              value={mode}
              onChange={(selectedValue) => setMode(selectedValue === "modify" ? "modify" : "add")}
              disabled={saving}
              className="pyqt-input !px-2 !py-1"
            >
              <option value="add">{t("characterDetail.event.add")}</option>
              <option value="modify">{t("characterDetail.event.modify")}</option>
            </ThemedSelect>
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">{t("characterDetail.event.chapter")}</span>
            <input
              type="number"
              min={1}
              step={1}
              aria-invalid={chapterError != null}
              aria-describedby={chapterError ? "event-chapter-error" : undefined}
              value={chapterNumber}
              disabled={saving}
              onChange={(event) => {
                const nextValue = event.target.value;
                setChapterNumber(nextValue);
                if (parsePositiveChapterNumber(nextValue) != null) setChapterError(null);
              }}
              className="pyqt-input !px-2 !py-1"
            />
            {chapterError ? (
              <p id="event-chapter-error" role="alert" className="text-small text-error">{chapterError}</p>
            ) : null}
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">{t("characterDetail.event.category")}</span>
            <ThemedSelect
              value={category}
              onChange={setCategory}
              disabled={saving}
              className="pyqt-input !px-2 !py-1"
            >
              {ATTRIBUTE_CATEGORY_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {categoryLabel(option.value, t)}
                </option>
              ))}
            </ThemedSelect>
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">{t("characterDetail.event.attribute")}</span>
            <input
              value={key}
              onChange={(event) => setKey(event.target.value)}
              disabled={saving}
              className="pyqt-input !px-2 !py-1"
              placeholder={t("characterDetail.event.attributePlaceholder")}
            />
          </label>
          <label className="flex flex-col gap-1 sm:col-span-2">
            <span className="text-caption text-book-text-muted">{t("characterDetail.event.value")}</span>
            <textarea
              value={valueText}
              onChange={(event) => setValueText(event.target.value)}
              disabled={saving}
              className="book-control min-h-[58px] resize-y rounded-xl px-2.5 py-2 text-small"
              placeholder={t("characterDetail.event.valuePlaceholder")}
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">{t("characterDetail.event.cause")}</span>
            <textarea
              value={cause}
              onChange={(event) => setCause(event.target.value)}
              disabled={saving}
              className="pyqt-input min-h-[58px] !px-2 !py-1 text-small"
              placeholder={t("characterDetail.event.causePlaceholder")}
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption text-book-text-muted">{t("characterDetail.event.evidence")}</span>
            <textarea
              value={evidence}
              onChange={(event) => setEvidence(event.target.value)}
              disabled={saving}
              className="pyqt-input min-h-[58px] !px-2 !py-1 text-small"
              placeholder={t("characterDetail.event.evidencePlaceholder")}
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
                  {mode === "add" ? t("characterDetail.event.adding") : t("characterDetail.event.saving")}
                </span>
              ) : mode === "add" ? (
                t("characterDetail.event.addAttribute")
              ) : (
                t("characterDetail.event.saveChanges")
              )}
            </BookButton>
          </div>
        </div>
      </details>
      {loading ? (
        <div className="text-small text-book-text-muted">{t("characterDetail.event.loading")}</div>
      ) : !hasCurrentEvents ? (
        <div className="text-small text-book-text-muted italic">{t("characterDetail.event.notLoaded")}</div>
      ) : events.length === 0 ? (
        <div className="text-small text-book-text-muted italic">{t("characterDetail.event.empty")}</div>
      ) : (
        <ul className="a4n-flat-surface divide-y divide-book-border/45">
          {events.map((event) => {
            const oldValue = attributeValueText(event.old_value, t);
            const newValue = attributeValueText(event.new_value, t);
            return (
              <li
                key={event.id}
                className="px-3 py-2.5 text-small"
              >
                <div className="mb-1 flex min-w-0 flex-wrap items-baseline gap-x-1.5 gap-y-0.5">
                  <span className="font-medium text-book-text-main">
                    {attributeKeyLabel(event.key, t)}
                  </span>
                  <span className="text-caption text-book-text-muted">
                    {t("characterDetail.chapter", { chapter: event.chapter_number })} · {categoryLabel(event.category, t)} ·{" "}
                    {operationLabel(event.operation, t)} · {sourceKindLabel(event.source_kind, t)}
                  </span>
                  {event.timestamp && (
                    <span className="ml-auto shrink-0 text-caption text-book-text-muted">
                      {formatDateTime(event.timestamp, locale)}
                    </span>
                  )}
                </div>
                {(oldValue || newValue) && (
                  <div className="grid divide-y divide-book-border/30 border-y border-book-border/30 text-caption sm:grid-cols-2 sm:divide-x sm:divide-y-0">
                    {oldValue && (
                      <div className="px-2 py-1.5">
                        <span className="text-book-text-muted">{t("characterDetail.event.oldValue")}</span>
                        <span className="break-words text-book-text-sub">{oldValue}</span>
                      </div>
                    )}
                    {newValue && (
                      <div className="px-2 py-1.5">
                        <span className="text-book-text-muted">{t("characterDetail.event.newValue")}</span>
                        <span className="break-words text-book-text-sub">{newValue}</span>
                      </div>
                    )}
                  </div>
                )}
                {event.change_description && (
                  <div className="mt-1 text-caption leading-relaxed text-book-text-main">
                    {t("characterDetail.event.summary")}{event.change_description}
                  </div>
                )}
                {event.cause && (
                  <div className="mt-1 text-caption leading-relaxed text-book-text-sub">
                    {t("characterDetail.event.reason")}{event.cause}
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
