import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye, RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../api";
import {
  applyImplicitUpdateDecision,
  checkImplicitUpdate,
  getImplicitReview,
  previewCheckImplicitUpdateRequest,
  type ImplicitAttributeReview,
  type ImplicitReview,
  type ImplicitUpdateDecision,
} from "../../api/character";
import { CHARACTER_DELETION_THRESHOLD } from "../../api/generated/host";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../components/ui/Dropdown";
import { formatNumber, useI18n, type Locale, type MessageTranslator } from "../../i18n";
import {
  attributeKeyLabel,
  behaviorTypeLabel,
  decisionLabel,
  jsonPreview,
  listPreview,
} from "./manageViewUtils";

const IMPLICIT_REVIEW_EVIDENCE_LIMIT = 12;
const IMPLICIT_REVIEW_DEFAULT_WINDOW_CHAPTERS = 10;
const IMPLICIT_REVIEW_DEFAULT_THRESHOLD = 5;
const IMPLICIT_REVIEW_PARAMETER_MAX = 50;

const parseReviewParameter = (value: string): number | null => {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return null;
  return Math.max(1, Math.min(IMPLICIT_REVIEW_PARAMETER_MAX, Math.floor(parsed)));
};

const formatPercent = (value: number, locale: Locale, t: MessageTranslator): string => {
  if (!Number.isFinite(value)) {
    throw new Error(t("characterDetail.implicit.invalidPercent"));
  }
  return `${formatNumber(Math.round(value * 100), locale)}%`;
};

const implicitReviewScopeKey = (
  profileId: number,
  windowChapters: number,
  threshold: number,
): string => `${profileId}:${windowChapters}:${threshold}`;

export const ImplicitReviewManager: React.FC<{
  profileId: number;
  defaultChapter: number;
  onChanged?: () => boolean | void | Promise<boolean | void>;
}> = ({ profileId, defaultChapter, onChanged }) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const [review, setReview] = useState<ImplicitReview | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadedScopeKey, setLoadedScopeKey] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [decidingKey, setDecidingKey] = useState<string | null>(null);
  const [applyingKey, setApplyingKey] = useState<string | null>(null);
  const [decisions, setDecisions] = useState<Record<string, ImplicitUpdateDecision>>({});
  const [windowChapters, setWindowChapters] = useState(IMPLICIT_REVIEW_DEFAULT_WINDOW_CHAPTERS);
  const [threshold, setThreshold] = useState(IMPLICIT_REVIEW_DEFAULT_THRESHOLD);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const mountedRef = useRef(true);
  const activeProfileIdRef = useRef(profileId);
  const windowChaptersRef = useRef(windowChapters);
  const thresholdRef = useRef(threshold);
  const loadSeqRef = useRef(0);
  const checkSeqRef = useRef(0);
  const applySeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const checkingRef = useRef(false);
  const applyingRef = useRef(false);
  const previewingRef = useRef(false);
  const loadingRef = useRef(false);

  activeProfileIdRef.current = profileId;
  windowChaptersRef.current = windowChapters;
  thresholdRef.current = threshold;
  const currentScopeKey = implicitReviewScopeKey(profileId, windowChapters, threshold);
  const hasCurrentReview = loadedScopeKey === currentScopeKey;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      checkSeqRef.current += 1;
      applySeqRef.current += 1;
      previewSeqRef.current += 1;
      checkingRef.current = false;
      applyingRef.current = false;
      previewingRef.current = false;
      loadingRef.current = false;
    };
  }, []);

  const isCurrentProfile = useCallback((requestProfileId: number) => {
    return mountedRef.current && activeProfileIdRef.current === requestProfileId;
  }, []);

  const isCurrentParams = useCallback(
    (requestProfileId: number, requestWindow: number, requestThreshold: number) => {
      return (
        isCurrentProfile(requestProfileId) &&
        windowChaptersRef.current === requestWindow &&
        thresholdRef.current === requestThreshold
      );
    },
    [isCurrentProfile],
  );

  const invalidateReviewScope = useCallback(() => {
    loadSeqRef.current += 1;
    checkSeqRef.current += 1;
    applySeqRef.current += 1;
    previewSeqRef.current += 1;
    checkingRef.current = false;
    applyingRef.current = false;
    previewingRef.current = false;
    loadingRef.current = false;
    setLoading(false);
    setLoadedScopeKey(null);
    setReview(null);
    setExpanded(null);
    setDecidingKey(null);
    setApplyingKey(null);
    setDecisions({});
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, []);

  const load = useCallback(async () => {
    if (loadingRef.current) return;
    const requestProfileId = profileId;
    const requestWindow = windowChaptersRef.current;
    const requestThreshold = thresholdRef.current;
    const requestScopeKey = implicitReviewScopeKey(
      requestProfileId,
      requestWindow,
      requestThreshold,
    );
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    loadingRef.current = true;
    setLoading(true);
    try {
      const nextReview = await getImplicitReview({
        profile_id: requestProfileId,
        window_chapters: requestWindow,
        threshold: requestThreshold,
        evidence_limit: IMPLICIT_REVIEW_EVIDENCE_LIMIT,
      });
      if (
        !isCurrentParams(requestProfileId, requestWindow, requestThreshold) ||
        loadSeqRef.current !== requestSeq
      ) {
        return;
      }
      setReview(nextReview);
      setLoadedScopeKey(requestScopeKey);
    } catch (err) {
      if (
        !isCurrentParams(requestProfileId, requestWindow, requestThreshold) ||
        loadSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, t("characterDetail.implicit.loadFailed")), "error");
    } finally {
      if (
        isCurrentParams(requestProfileId, requestWindow, requestThreshold) &&
        loadSeqRef.current === requestSeq
      ) {
        setLoading(false);
      }
      if (loadSeqRef.current === requestSeq) {
        loadingRef.current = false;
      }
    }
  }, [addToast, isCurrentParams, profileId, t]);

  useEffect(() => {
    invalidateReviewScope();
    void load();
  }, [invalidateReviewScope, load]);

  const attrs = review ? review.attributes : [];
  const needsReviewCount = attrs.filter((attr) => attr.threshold_reached).length;

  useEffect(() => {
    const visibleKeys = new Set(attrs.map((attr) => attr.attribute_key));
    setDecisions((current) => {
      const next = Object.fromEntries(
        Object.entries(current).filter(([key]) => visibleKeys.has(key)),
      );
      return Object.keys(next).length === Object.keys(current).length ? current : next;
    });
    setExpanded((current) => (current && visibleKeys.has(current) ? current : null));
  }, [attrs]);

  const handleCheck = async (attributeKey: string) => {
    if (decidingKey != null || applyingKey != null) return;
    if (checkingRef.current || applyingRef.current) return;
    checkingRef.current = true;
    const requestProfileId = profileId;
    const requestWindow = windowChapters;
    const requestThreshold = threshold;
    const requestSeq = checkSeqRef.current + 1;
    checkSeqRef.current = requestSeq;
    setDecidingKey(attributeKey);
    try {
      const decision = await checkImplicitUpdate({
        profile_id: requestProfileId,
        attribute_key: attributeKey,
        window_chapters: requestWindow,
        threshold: requestThreshold,
        evidence_limit: IMPLICIT_REVIEW_EVIDENCE_LIMIT,
      });
      if (
        !isCurrentParams(requestProfileId, requestWindow, requestThreshold) ||
        checkSeqRef.current !== requestSeq
      ) {
        return;
      }
      setDecisions((current) => ({ ...current, [attributeKey]: decision }));
      addToast(t("characterDetail.implicit.generated"), "success");
    } catch (err) {
      if (
        !isCurrentParams(requestProfileId, requestWindow, requestThreshold) ||
        checkSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, t("characterDetail.implicit.generateFailed")), "error");
    } finally {
      if (
        isCurrentParams(requestProfileId, requestWindow, requestThreshold) &&
        checkSeqRef.current === requestSeq
      ) {
        setDecidingKey(null);
      }
      if (checkSeqRef.current === requestSeq) {
        checkingRef.current = false;
      }
    }
  };

  const handlePreviewCheck = async (attributeKey: string) => {
    if (previewingRef.current) return;
    previewingRef.current = true;
    const requestProfileId = profileId;
    const requestWindow = windowChapters;
    const requestThreshold = threshold;
    const requestSeq = previewSeqRef.current + 1;
    previewSeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewCheckImplicitUpdateRequest({
        profile_id: requestProfileId,
        attribute_key: attributeKey,
        window_chapters: requestWindow,
        threshold: requestThreshold,
        evidence_limit: IMPLICIT_REVIEW_EVIDENCE_LIMIT,
      });
      if (
        !isCurrentParams(requestProfileId, requestWindow, requestThreshold) ||
        previewSeqRef.current !== requestSeq
      ) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (
        !isCurrentParams(requestProfileId, requestWindow, requestThreshold) ||
        previewSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, t("characterDetail.implicit.previewFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (
        isCurrentParams(requestProfileId, requestWindow, requestThreshold) &&
        previewSeqRef.current === requestSeq
      ) {
        setRequestPreviewLoading(false);
      }
      if (previewSeqRef.current === requestSeq) {
        previewingRef.current = false;
      }
    }
  };

  const handleApply = async (decision: ImplicitUpdateDecision) => {
    if (decidingKey != null || applyingKey != null) return;
    if (checkingRef.current || applyingRef.current) return;
    if (decision.decision === "keep") return;
    const reason = decision.reasoning.trim();
    const evidence = decision.evidence_summary.trim();
    if (!reason || !evidence) {
      addToast(t("characterDetail.implicit.missingReasonEvidence"), "error");
      return;
    }

    applyingRef.current = true;
    const requestProfileId = profileId;
    const requestDefaultChapter = defaultChapter;
    const requestSeq = applySeqRef.current + 1;
    applySeqRef.current = requestSeq;
    setApplyingKey(decision.attribute_key);
    try {
      const result = await applyImplicitUpdateDecision({
        profile_id: requestProfileId,
        decision,
        chapter_number: requestDefaultChapter,
      });
      if (!isCurrentProfile(requestProfileId) || applySeqRef.current !== requestSeq) return;

      if (result.action === "modify") {
        addToast(t("characterDetail.implicit.applied"), "success");
      } else if (result.action === "mark_deletion" && result.deletion_mark) {
        addToast(
          result.deletion_mark.consecutive_count >= CHARACTER_DELETION_THRESHOLD
            ? t("characterDetail.implicit.thresholdReached", {
                count: result.deletion_mark.consecutive_count,
                threshold: CHARACTER_DELETION_THRESHOLD,
              })
            : t("characterDetail.implicit.marked", {
                count: result.deletion_mark.consecutive_count,
                threshold: CHARACTER_DELETION_THRESHOLD,
              }),
          "success",
        );
      }
      setDecisions((current) => {
        const next = { ...current };
        delete next[decision.attribute_key];
        return next;
      });
      await load();
      if (!isCurrentProfile(requestProfileId) || applySeqRef.current !== requestSeq) return;
      await onChanged?.();
    } catch (err) {
      if (!isCurrentProfile(requestProfileId) || applySeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("characterDetail.implicit.applyFailed")), "error");
    } finally {
      if (isCurrentProfile(requestProfileId) && applySeqRef.current === requestSeq) {
        setApplyingKey(null);
      }
      if (applySeqRef.current === requestSeq) {
        applyingRef.current = false;
      }
    }
  };

  const reviewOperationBusy =
    loading ||
    decidingKey != null ||
    applyingKey != null ||
    requestPreviewLoading;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2 text-small text-book-text-muted">
        <span className="min-w-0 flex-1">
          {review
            ? needsReviewCount > 0
              ? t("characterDetail.implicit.needsReview", { count: needsReviewCount })
              : t("characterDetail.implicit.noNeedsReview")
            : t("characterDetail.implicit.hint")}
        </span>
        <BookIconButton
          label={t("characterDetail.implicit.refresh")}
          onClick={load}
          variant="secondary"
          size="sm"
          disabled={reviewOperationBusy}
        >
          <RefreshCw size={13} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      <details className="text-small text-book-text-muted">
        <summary className="w-fit cursor-pointer select-none py-1 hover:text-book-text-main">
          {t("characterDetail.implicit.scope", { window: windowChapters, threshold })}
        </summary>
        <div className="mt-1.5 flex flex-wrap items-end gap-2 rounded-xl bg-book-bg-tertiary/40 p-2.5">
          <label className="flex flex-col gap-1">
            <span className="text-caption">{t("characterDetail.implicit.window")}</span>
            <input
              type="number"
              min={1}
                max={IMPLICIT_REVIEW_PARAMETER_MAX}
              value={windowChapters}
              disabled={reviewOperationBusy}
              onChange={(event) => {
                const nextValue = parseReviewParameter(event.target.value);
                if (nextValue == null) return;
                if (windowChaptersRef.current !== nextValue) invalidateReviewScope();
                setWindowChapters(nextValue);
              }}
              className="pyqt-input h-8 w-24 !px-2 !py-1"
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-caption">{t("characterDetail.implicit.threshold")}</span>
            <input
              type="number"
              min={1}
                max={IMPLICIT_REVIEW_PARAMETER_MAX}
              value={threshold}
              disabled={reviewOperationBusy}
              onChange={(event) => {
                const nextValue = parseReviewParameter(event.target.value);
                if (nextValue == null) return;
                if (thresholdRef.current !== nextValue) invalidateReviewScope();
                setThreshold(nextValue);
              }}
              className="pyqt-input h-8 w-24 !px-2 !py-1"
            />
          </label>
          <BookButton
            variant="secondary"
            size="sm"
            onClick={load}
            disabled={reviewOperationBusy}
          >
            {loading ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" /> {t("characterDetail.implicit.updating")}
              </span>
            ) : (
              t("characterDetail.implicit.update")
            )}
          </BookButton>
        </div>
      </details>
      {loading ? (
        <div className="text-small text-book-text-muted">{t("characterDetail.implicit.loading")}</div>
      ) : !hasCurrentReview ? (
        <div className="text-small text-book-text-muted italic">{t("characterDetail.implicit.notLoaded")}</div>
      ) : attrs.length === 0 ? (
        <div className="text-small text-book-text-muted italic">
          {t("characterDetail.implicit.empty")}
        </div>
      ) : (
        <ul className="grid gap-2">
          {attrs.map((attr) => (
            <ImplicitReviewItem
              key={attr.attribute_key}
              attr={attr}
              decision={decisions[attr.attribute_key]}
              deciding={decidingKey === attr.attribute_key}
              applying={applyingKey === attr.attribute_key}
              actionsDisabled={reviewOperationBusy}
              expanded={expanded === attr.attribute_key}
              onPreviewCheck={() => void handlePreviewCheck(attr.attribute_key)}
              onCheck={() => void handleCheck(attr.attribute_key)}
              onApply={() => {
                const decision = decisions[attr.attribute_key];
                if (decision) void handleApply(decision);
              }}
              onToggle={() =>
                setExpanded((current) =>
                  current === attr.attribute_key ? null : attr.attribute_key,
                )
              }
              locale={locale}
              t={t}
            />
          ))}
        </ul>
      )}
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("characterDetail.implicit.previewTitle")}
        onClose={() => {
          previewSeqRef.current += 1;
          previewingRef.current = false;
          setRequestPreviewOpen(false);
        }}
      />
    </div>
  );
};

const ImplicitReviewItem: React.FC<{
  attr: ImplicitAttributeReview;
  decision?: ImplicitUpdateDecision;
  deciding: boolean;
  applying: boolean;
  actionsDisabled: boolean;
  expanded: boolean;
  onPreviewCheck: () => void;
  onCheck: () => void;
  onApply: () => void;
  onToggle: () => void;
  locale: Locale;
  t: MessageTranslator;
}> = ({
  attr,
  decision,
  deciding,
  applying,
  actionsDisabled,
  expanded,
  onPreviewCheck,
  onCheck,
  onApply,
  onToggle,
  locale,
  t,
}) => {
  const hasEvidence = attr.evidence.length > 0;
  const menuItems: DropdownItem[] = [];
  if (hasEvidence) {
    menuItems.push({
      label: t("characterDetail.implicit.viewRequest"),
      icon: <Eye size={12} />,
      disabled: actionsDisabled || deciding || applying,
      onClick: onPreviewCheck,
    });
    if (decision) {
      menuItems.push({
        label: t("characterDetail.implicit.regenerate"),
        icon: <RefreshCw size={12} />,
        disabled: actionsDisabled || deciding || applying,
        onClick: onCheck,
      });
    }
  }

  return (
    <li
      className={`a4n-flat-surface overflow-hidden px-3 py-2.5 text-small ${
        attr.threshold_reached ? "bg-warning-bg/25" : ""
      }`}
    >
      <button type="button" onClick={onToggle} className="w-full text-left">
        <div className="flex min-w-0 items-start gap-2">
          <span className="min-w-0 flex-1 font-medium text-book-text-main">
            {attributeKeyLabel(attr.attribute_key, t)}
          </span>
          <span className="shrink-0 text-book-text-sub">{formatPercent(attr.conform_rate, locale, t)} {t("characterDetail.implicit.conform")}</span>
        </div>
        <div className="mt-0.5 truncate text-book-text-sub">{jsonPreview(attr.current_value)}</div>
        <div className="mt-0.5 flex flex-wrap gap-x-1.5 gap-y-0.5 text-caption text-book-text-muted">
          <span>{t("characterDetail.implicit.conformCount", { count: attr.conform_count })}</span>
          <span aria-hidden="true">·</span>
          <span>{t("characterDetail.implicit.nonConformCount", { count: attr.non_conform_count })}</span>
          {attr.threshold_reached ? (
            <>
              <span aria-hidden="true">·</span>
              <span className="text-warning">{t("characterDetail.implicit.needsReviewShort")}</span>
            </>
          ) : null}
        </div>
      </button>
      {expanded ? (
        <div className="mt-2.5 space-y-2 border-t border-book-border/30 pt-2.5">
          {hasEvidence ? (
            <div className="flex items-center justify-end gap-2">
              {menuItems.length > 0 ? <Dropdown items={menuItems} /> : null}
              {!decision ? (
                <BookButton
                  onClick={onCheck}
                  disabled={actionsDisabled || deciding || applying}
                >
                  {deciding ? (
                    <span className="inline-flex items-center gap-1.5">
                      <LoadingSpinner size="sm" /> {t("characterDetail.implicit.generating")}
                    </span>
                  ) : (
                    t("characterDetail.implicit.generate")
                  )}
                </BookButton>
              ) : decision.decision !== "keep" ? (
                <BookButton onClick={onApply} disabled={actionsDisabled || applying}>
                  {applying ? (
                    <span className="inline-flex items-center gap-1.5">
                      <LoadingSpinner size="sm" /> {t("characterDetail.implicit.applying")}
                    </span>
                  ) : (
                    t("characterDetail.implicit.apply")
                  )}
                </BookButton>
              ) : null}
            </div>
          ) : (
            <div className="text-caption text-book-text-muted">{t("characterDetail.implicit.noEvidence")}</div>
          )}
          {decision ? (
            <div className="rounded-xl border-l-2 border-book-primary/55 bg-book-primary/5 px-3 py-2.5 text-caption leading-relaxed">
              <div className="font-medium text-book-text-main">
                {t("characterDetail.implicit.suggest")}{decisionLabel(decision.decision, t)}
                {decision.suggested_new_value != null
                  ? t("characterDetail.implicit.newValue", { value: jsonPreview(decision.suggested_new_value) })
                  : ""}
              </div>
              <div className="mt-1 text-book-text-sub">{decision.reasoning}</div>
              {decision.evidence_summary ? (
                <div className="mt-1 text-book-text-muted">{t("characterDetail.implicit.evidence")}{decision.evidence_summary}</div>
              ) : null}
            </div>
          ) : null}
          {hasEvidence ? (
            <ol className="divide-y divide-book-border/25 rounded-xl bg-book-bg-tertiary/35 px-3">
              {attr.evidence.map((item) => (
                <li key={item.behavior_id} className="py-2.5 text-caption leading-relaxed">
                  <div className="text-book-text-muted">
                    {t("characterDetail.implicit.chapter", { chapter: item.chapter_number })} · {behaviorTypeLabel(item.behavior_type, t)}
                  </div>
                  <div className="mt-0.5 text-book-text-sub">{item.behavior_description}</div>
                  {item.original_text ? (
                    <div className="mt-0.5 text-book-text-muted">{t("characterDetail.implicit.originalText")}{item.original_text}</div>
                  ) : null}
                  {listPreview(item.behavior_tags, t) ? (
                    <div className="mt-0.5 text-book-text-muted">
                      {t("characterDetail.implicit.tags")}{listPreview(item.behavior_tags, t)}
                    </div>
                  ) : null}
                  {jsonPreview(item.classification) ? (
                    <details className="mt-1 text-book-text-muted">
                      <summary className="w-fit cursor-pointer hover:text-book-text-main">
                        {t("characterDetail.implicit.diagnostics")}
                      </summary>
                      <div className="mt-1 break-words font-mono">
                        {jsonPreview(item.classification)}
                      </div>
                    </details>
                  ) : null}
                </li>
              ))}
            </ol>
          ) : null}
        </div>
      ) : null}
    </li>
  );
};
