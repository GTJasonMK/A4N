import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye, Loader2 } from "lucide-react";
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
import type { ModelRequestPreview } from "../../api/requestPreview";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { jsonPreview, listPreview } from "./manageViewUtils";

const formatPercent = (value: number): string => {
  if (!Number.isFinite(value)) {
    throw new Error("隐性属性复审比例不是有效数字");
  }
  return `${Math.round(value * 100)}%`;
};

const implicitReviewScopeKey = (
  profileId: number,
  windowChapters: number,
  threshold: number,
): string => `${profileId}:${windowChapters}:${threshold}`;

export const ImplicitReviewManager: React.FC<{
  profileId: number;
  defaultChapter: number;
  onChanged?: () => void | Promise<void>;
}> = ({ profileId, defaultChapter, onChanged }) => {
  const { addToast } = useToast();
  const [review, setReview] = useState<ImplicitReview | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadedScopeKey, setLoadedScopeKey] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [decidingKey, setDecidingKey] = useState<string | null>(null);
  const [applyingKey, setApplyingKey] = useState<string | null>(null);
  const [decisions, setDecisions] = useState<Record<string, ImplicitUpdateDecision>>({});
  const [windowChapters, setWindowChapters] = useState(10);
  const [threshold, setThreshold] = useState(5);
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
        evidence_limit: 12,
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
      addToast(extractErrorMessage(err, "加载隐性复审失败"), "error");
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
  }, [addToast, isCurrentParams, profileId]);

  useEffect(() => {
    invalidateReviewScope();
    void load();
  }, [invalidateReviewScope, load]);

  useEffect(() => {
    checkSeqRef.current += 1;
    applySeqRef.current += 1;
    previewSeqRef.current += 1;
    setDecidingKey(null);
    setApplyingKey(null);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    checkingRef.current = false;
    applyingRef.current = false;
    previewingRef.current = false;
  }, [profileId]);

  const attrs = review ? review.attributes : [];
  const needsReview = review ? review.needs_review : [];

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
        evidence_limit: 12,
      });
      if (
        !isCurrentParams(requestProfileId, requestWindow, requestThreshold) ||
        checkSeqRef.current !== requestSeq
      ) {
        return;
      }
      setDecisions((current) => ({ ...current, [attributeKey]: decision }));
      addToast("已生成复审建议", "success");
    } catch (err) {
      if (
        !isCurrentParams(requestProfileId, requestWindow, requestThreshold) ||
        checkSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, "生成复审建议失败"), "error");
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
        evidence_limit: 12,
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
      addToast(extractErrorMessage(err, "预览复审请求失败"), "error");
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
      addToast("复审建议缺少原因或证据，不能应用", "error");
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
        decision,
        chapter_number: requestDefaultChapter,
      });
      if (!isCurrentProfile(requestProfileId) || applySeqRef.current !== requestSeq) return;

      if (result.action === "modify") {
        addToast("已应用隐性属性修改", "success");
      } else if (result.action === "mark_deletion" && result.deletion_mark) {
        addToast(
          result.ready_for_deletion
            ? `删除标记已达到阈值 ${result.deletion_mark.consecutive_count}/${result.threshold}`
            : `已记录删除标记 ${result.deletion_mark.consecutive_count}/${result.threshold}`,
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
      addToast(extractErrorMessage(err, "应用复审建议失败"), "error");
    } finally {
      if (isCurrentProfile(requestProfileId) && applySeqRef.current === requestSeq) {
        setApplyingKey(null);
      }
      if (applySeqRef.current === requestSeq) {
        applyingRef.current = false;
      }
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2 text-xs flex-wrap">
        <label className="flex items-center gap-1 text-book-text-muted">
          窗口
          <input
            type="number"
            min={1}
            max={50}
            value={windowChapters}
            onChange={(event) => {
              const parsed = Number(event.target.value);
              if (Number.isFinite(parsed) && parsed > 0) {
                const nextValue = Math.max(1, Math.min(50, Math.floor(parsed)));
                if (windowChaptersRef.current !== nextValue) invalidateReviewScope();
                setWindowChapters(nextValue);
              }
            }}
            className="pyqt-input !py-1 !px-2 w-16"
          />
        </label>
        <label className="flex items-center gap-1 text-book-text-muted">
          阈值
          <input
            type="number"
            min={1}
            max={50}
            value={threshold}
            onChange={(event) => {
              const parsed = Number(event.target.value);
              if (Number.isFinite(parsed) && parsed > 0) {
                const nextValue = Math.max(1, Math.min(50, Math.floor(parsed)));
                if (thresholdRef.current !== nextValue) invalidateReviewScope();
                setThreshold(nextValue);
              }
            }}
            className="pyqt-input !py-1 !px-2 w-16"
          />
        </label>
        <BookButton variant="secondary" onClick={load} disabled={loading}>
          {loading ? <Loader2 size={11} className="animate-spin" /> : "刷新"}
        </BookButton>
        <div className="flex-1" />
        {review && (
          <span className="text-[11px] text-book-text-muted">
            {needsReview.length > 0
              ? `${needsReview.length} 个属性需要复审`
              : "暂无达到阈值的隐性属性"}
          </span>
        )}
      </div>
      {loading ? (
        <div className="text-[11px] text-book-text-muted">加载…</div>
      ) : !hasCurrentReview ? (
        <div className="text-[11px] text-book-text-muted italic">当前复审参数尚未加载，请刷新</div>
      ) : attrs.length === 0 ? (
        <div className="text-[11px] text-book-text-muted italic">
          尚无隐性属性或行为分类记录
        </div>
      ) : (
        <ul className="space-y-1 max-h-72 overflow-y-auto custom-scrollbar">
          {attrs.map((attr) => (
            <ImplicitReviewItem
              key={attr.attribute_key}
              attr={attr}
              decision={decisions[attr.attribute_key]}
              deciding={decidingKey === attr.attribute_key}
              applying={applyingKey === attr.attribute_key}
              actionsDisabled={
                (decidingKey != null && decidingKey !== attr.attribute_key) ||
                (applyingKey != null && applyingKey !== attr.attribute_key)
              }
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
            />
          ))}
        </ul>
      )}
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="隐性属性复审请求预览"
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
}) => (
  <li
    className={`a4n-stat-tile px-2 py-1.5 text-[11px] ${
      attr.threshold_reached ? "border-warning/40 bg-warning-bg/40" : ""
    }`}
  >
    <button
      type="button"
      onClick={onToggle}
      className="w-full text-left flex items-center gap-2"
    >
      <span className="pyqt-pill !text-[10px] font-mono shrink-0">
        {attr.attribute_key}
      </span>
      <span className="text-book-text-sub truncate flex-1">
        {jsonPreview(attr.current_value)}
      </span>
      <span className="text-book-text-muted shrink-0">
        符合 {attr.conform_count} / 不符合 {attr.non_conform_count}
      </span>
      <span className="pyqt-pill !text-[10px] shrink-0">
        {formatPercent(attr.conform_rate)}
      </span>
      {attr.threshold_reached && (
        <span className="pyqt-pill !text-[10px] !border-warning/50 !text-warning shrink-0">
          需复审
        </span>
      )}
    </button>
    {expanded && (
      <div className="mt-2 space-y-1">
        <div className="flex items-center gap-2">
          <BookButton
            variant="secondary"
            onClick={onPreviewCheck}
            disabled={actionsDisabled || deciding || applying || attr.evidence.length === 0}
          >
            <span className="inline-flex items-center gap-1">
              <Eye size={11} />
              预览请求
            </span>
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={onCheck}
            disabled={actionsDisabled || deciding || applying || attr.evidence.length === 0}
          >
            {deciding ? <Loader2 size={11} className="animate-spin" /> : "生成复审建议"}
          </BookButton>
          {decision && decision.decision !== "keep" && (
            <BookButton
              variant="secondary"
              onClick={onApply}
              disabled={actionsDisabled || applying}
            >
              {applying ? <Loader2 size={11} className="animate-spin" /> : "应用建议"}
            </BookButton>
          )}
          {attr.evidence.length === 0 && (
            <span className="text-[10px] text-book-text-muted">需要先有行为证据</span>
          )}
        </div>
        {decision && (
          <div className="a4n-stat-tile border-book-primary/30 bg-book-primary/5 px-2 py-1.5 text-[10px] leading-relaxed">
            <div className="flex items-center gap-1.5 mb-0.5">
              <span className="pyqt-pill !text-[10px]">建议：{decision.decision}</span>
              {decision.suggested_new_value != null && (
                <span className="font-mono text-book-text-sub truncate">
                  新值：{jsonPreview(decision.suggested_new_value)}
                </span>
              )}
            </div>
            <div className="text-book-text-sub">{decision.reasoning}</div>
            {decision.evidence_summary && (
              <div className="mt-0.5 text-book-text-muted">
                证据：{decision.evidence_summary}
              </div>
            )}
          </div>
        )}
        {attr.evidence.length === 0 ? (
          <div className="text-[10px] text-book-text-muted italic">暂无行为证据</div>
        ) : (
          attr.evidence.map((item) => (
            <div
              key={item.behavior_id}
              className="a4n-stat-tile bg-book-bg-tertiary/45 px-2 py-1 text-[10px] leading-relaxed"
            >
              <div className="flex items-center gap-1.5 mb-0.5">
                <span className="pyqt-pill !text-[10px]">第{item.chapter_number}章</span>
                <span className="pyqt-pill !text-[10px]">{item.behavior_type}</span>
                <span className="font-mono text-book-text-muted">
                  {jsonPreview(item.classification)}
                </span>
              </div>
              <div className="text-book-text-sub">{item.behavior_description}</div>
              {item.original_text && (
                <div className="mt-0.5 text-book-text-muted">原文：{item.original_text}</div>
              )}
              {listPreview(item.behavior_tags) && (
                <div className="mt-0.5 text-book-text-muted">
                  标签：{listPreview(item.behavior_tags)}
                </div>
              )}
            </div>
          ))
        )}
      </div>
    )}
  </li>
);
