import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye, Loader2, RefreshCw } from "lucide-react";
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
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../components/ui/Dropdown";
import {
  attributeKeyLabel,
  behaviorTypeLabel,
  decisionLabel,
  jsonPreview,
  listPreview,
} from "./manageViewUtils";

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

  const reviewOperationBusy =
    loading ||
    decidingKey != null ||
    applyingKey != null ||
    requestPreviewLoading;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2 text-[11px] text-book-text-muted">
        <span className="min-w-0 flex-1">
          {review
            ? needsReview.length > 0
              ? `${needsReview.length} 个属性需要复审`
              : "当前没有需要复审的属性"
            : "检查隐性属性是否仍符合近期行为"}
        </span>
        <BookIconButton
          label="刷新复审结果"
          onClick={load}
          variant="secondary"
          size="sm"
          disabled={reviewOperationBusy}
        >
          <RefreshCw size={13} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      <details className="text-[11px] text-book-text-muted">
        <summary className="w-fit cursor-pointer select-none py-1 hover:text-book-text-main">
          复审范围 · 最近 {windowChapters} 章 · 连续 {threshold} 次后提醒
        </summary>
        <div className="mt-1.5 flex flex-wrap items-end gap-2 rounded-xl bg-book-bg-tertiary/40 p-2.5">
          <label className="flex flex-col gap-1">
            <span className="text-[10px]">检查章节数</span>
            <input
              type="number"
              min={1}
              max={50}
              value={windowChapters}
              disabled={reviewOperationBusy}
              onChange={(event) => {
                const parsed = Number(event.target.value);
                if (Number.isFinite(parsed) && parsed > 0) {
                  const nextValue = Math.max(1, Math.min(50, Math.floor(parsed)));
                  if (windowChaptersRef.current !== nextValue) invalidateReviewScope();
                  setWindowChapters(nextValue);
                }
              }}
              className="pyqt-input h-8 w-24 !px-2 !py-1"
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-[10px]">提醒阈值</span>
            <input
              type="number"
              min={1}
              max={50}
              value={threshold}
              disabled={reviewOperationBusy}
              onChange={(event) => {
                const parsed = Number(event.target.value);
                if (Number.isFinite(parsed) && parsed > 0) {
                  const nextValue = Math.max(1, Math.min(50, Math.floor(parsed)));
                  if (thresholdRef.current !== nextValue) invalidateReviewScope();
                  setThreshold(nextValue);
                }
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
                <Loader2 size={11} className="animate-spin" /> 更新中…
              </span>
            ) : (
              "更新结果"
            )}
          </BookButton>
        </div>
      </details>
      {loading ? (
        <div className="text-[11px] text-book-text-muted">加载…</div>
      ) : !hasCurrentReview ? (
        <div className="text-[11px] text-book-text-muted italic">当前复审参数尚未加载，请刷新</div>
      ) : attrs.length === 0 ? (
        <div className="text-[11px] text-book-text-muted italic">
          尚无隐性属性或行为分类记录
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
}) => {
  const hasEvidence = attr.evidence.length > 0;
  const menuItems: DropdownItem[] = [];
  if (hasEvidence) {
    menuItems.push({
      label: "查看模型请求",
      icon: <Eye size={12} />,
      disabled: actionsDisabled || deciding || applying,
      onClick: onPreviewCheck,
    });
    if (decision) {
      menuItems.push({
        label: "重新生成建议",
        icon: <RefreshCw size={12} />,
        disabled: actionsDisabled || deciding || applying,
        onClick: onCheck,
      });
    }
  }

  return (
    <li
      className={`a4n-flat-surface overflow-hidden px-3 py-2.5 text-[11px] ${
        attr.threshold_reached ? "bg-warning-bg/25" : ""
      }`}
    >
      <button type="button" onClick={onToggle} className="w-full text-left">
        <div className="flex min-w-0 items-start gap-2">
          <span className="min-w-0 flex-1 font-medium text-book-text-main">
            {attributeKeyLabel(attr.attribute_key)}
          </span>
          <span className="shrink-0 text-book-text-sub">{formatPercent(attr.conform_rate)} 符合</span>
        </div>
        <div className="mt-0.5 truncate text-book-text-sub">{jsonPreview(attr.current_value)}</div>
        <div className="mt-0.5 flex flex-wrap gap-x-1.5 gap-y-0.5 text-[10px] text-book-text-muted">
          <span>{attr.conform_count} 条符合</span>
          <span aria-hidden="true">·</span>
          <span>{attr.non_conform_count} 条不符合</span>
          {attr.threshold_reached ? (
            <>
              <span aria-hidden="true">·</span>
              <span className="text-warning">需要复审</span>
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
                      <Loader2 size={11} className="animate-spin" /> 生成中…
                    </span>
                  ) : (
                    "生成复审建议"
                  )}
                </BookButton>
              ) : decision.decision !== "keep" ? (
                <BookButton onClick={onApply} disabled={actionsDisabled || applying}>
                  {applying ? (
                    <span className="inline-flex items-center gap-1.5">
                      <Loader2 size={11} className="animate-spin" /> 应用中…
                    </span>
                  ) : (
                    "应用建议"
                  )}
                </BookButton>
              ) : null}
            </div>
          ) : (
            <div className="text-[10px] text-book-text-muted">暂无行为证据，暂时无法复审</div>
          )}
          {decision ? (
            <div className="rounded-xl border-l-2 border-book-primary/55 bg-book-primary/5 px-3 py-2.5 text-[10px] leading-relaxed">
              <div className="font-medium text-book-text-main">
                建议{decisionLabel(decision.decision)}
                {decision.suggested_new_value != null
                  ? ` · 新值：${jsonPreview(decision.suggested_new_value)}`
                  : ""}
              </div>
              <div className="mt-1 text-book-text-sub">{decision.reasoning}</div>
              {decision.evidence_summary ? (
                <div className="mt-1 text-book-text-muted">证据：{decision.evidence_summary}</div>
              ) : null}
            </div>
          ) : null}
          {hasEvidence ? (
            <ol className="divide-y divide-book-border/25 rounded-xl bg-book-bg-tertiary/35 px-3">
              {attr.evidence.map((item) => (
                <li key={item.behavior_id} className="py-2.5 text-[10px] leading-relaxed">
                  <div className="text-book-text-muted">
                    第 {item.chapter_number} 章 · {behaviorTypeLabel(item.behavior_type)}
                  </div>
                  <div className="mt-0.5 text-book-text-sub">{item.behavior_description}</div>
                  {item.original_text ? (
                    <div className="mt-0.5 text-book-text-muted">原文：{item.original_text}</div>
                  ) : null}
                  {listPreview(item.behavior_tags) ? (
                    <div className="mt-0.5 text-book-text-muted">
                      标签：{listPreview(item.behavior_tags)}
                    </div>
                  ) : null}
                  {jsonPreview(item.classification) ? (
                    <details className="mt-1 text-book-text-muted">
                      <summary className="w-fit cursor-pointer hover:text-book-text-main">
                        诊断信息
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
