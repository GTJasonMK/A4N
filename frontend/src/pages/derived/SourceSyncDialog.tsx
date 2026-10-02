import React, { useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, GitCompare, RefreshCw, Unplug } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { applySourceSyncDecisions, detachDerivedProject, previewSourceSync, type SourceSyncConflictSummary, type SourceSyncPlanSummary } from "../../api/sourceSync";
import { extractErrorMessage } from "../../api";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { Modal } from "../../components/ui/Modal";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { formatNumber, useI18n, type MessageKey, type MessageTranslator, type Locale } from "../../i18n";

interface SourceSyncDialogProps {
  open: boolean;
  projectId: number;
  projectTitle: string;
  sourceSyncStatus: string;
  canEditSourceSync: boolean;
  onClose: () => void;
  onChanged: () => boolean | void | Promise<boolean | void>;
}

const SEVERITY_KEYS: Record<string, MessageKey> = {
  info: "novel.sourceSyncDialog.severityInfo",
  warning: "novel.sourceSyncDialog.severityWarning",
  blocking: "novel.sourceSyncDialog.severityBlocking",
};

const STATUS_KEYS: Record<string, MessageKey> = {
  current: "novel.sourceSyncDialog.statusCurrent",
  root_updated: "novel.sourceSyncDialog.statusRootUpdated",
  conflict: "novel.sourceSyncDialog.statusConflict",
  detached: "novel.sourceSyncDialog.statusDetached",
};

const TARGET_KIND_KEYS: Record<string, MessageKey> = {
  chapter: "novel.sourceSyncDialog.targetChapter",
  segment: "novel.sourceSyncDialog.targetSegment",
  galgame_scene: "novel.sourceSyncDialog.targetGalgameScene",
  galgame_line: "novel.sourceSyncDialog.targetGalgameLine",
  character: "novel.sourceSyncDialog.targetCharacter",
  object: "novel.sourceSyncDialog.targetObject",
  scene: "novel.sourceSyncDialog.targetScene",
};

const CONFLICT_STATUS_KEYS: Record<string, MessageKey> = {
  pending: "novel.sourceSyncDialog.conflictPending",
  resolved: "novel.sourceSyncDialog.conflictResolved",
  ignored: "novel.sourceSyncDialog.conflictIgnored",
  obsolete: "novel.sourceSyncDialog.conflictObsolete",
};

const SOURCE_CHANGE_KEYS: Record<string, MessageKey> = {
  added: "novel.sourceSyncDialog.sourceAdded",
  updated: "novel.sourceSyncDialog.sourceUpdated",
  reordered: "novel.sourceSyncDialog.sourceReordered",
  deleted: "novel.sourceSyncDialog.sourceDeleted",
  moved: "novel.sourceSyncDialog.sourceMoved",
  split: "novel.sourceSyncDialog.sourceSplit",
};

const LOCAL_STATE_KEYS: Record<string, MessageKey> = {
  user_edited: "novel.sourceSyncDialog.localUserEdited",
  generated: "novel.sourceSyncDialog.localGenerated",
  imported: "novel.sourceSyncDialog.localImported",
  assembled: "novel.sourceSyncDialog.localAssembled",
  clean: "novel.sourceSyncDialog.localClean",
  draft: "novel.sourceSyncDialog.localDraft",
};

const SUMMARY_KEY_KEYS: Record<string, MessageKey> = {
  title: "novel.sourceSyncDialog.summaryTitle",
  visual_plan_changed: "novel.sourceSyncDialog.summaryVisualPlanChanged",
  shooting_script_changed: "novel.sourceSyncDialog.summaryShootingScriptChanged",
  text: "novel.sourceSyncDialog.summaryText",
  content: "novel.sourceSyncDialog.summaryText",
  summary: "novel.sourceSyncDialog.summarySummary",
  chapter_number: "novel.sourceSyncDialog.summaryChapter",
  scene_index: "novel.sourceSyncDialog.summaryScene",
  line_index: "novel.sourceSyncDialog.summaryLine",
  status: "novel.sourceSyncDialog.summaryStatus",
  source_text: "novel.sourceSyncDialog.summarySourceText",
  local_text: "novel.sourceSyncDialog.summaryLocalText",
  name: "novel.sourceSyncDialog.summaryName",
  chapter_title: "novel.sourceSyncDialog.summaryChapterTitle",
  scene_title: "novel.sourceSyncDialog.summarySceneTitle",
  old_text: "novel.sourceSyncDialog.summaryOldText",
  new_text: "novel.sourceSyncDialog.summaryNewText",
  reason: "novel.sourceSyncDialog.summaryReason",
  page_number: "novel.sourceSyncDialog.summaryPage",
  panel_number: "novel.sourceSyncDialog.summaryPanel",
  segment_index: "novel.sourceSyncDialog.summarySegment",
};

const labelFrom = (
  keys: Record<string, MessageKey>,
  value: string,
  t: MessageTranslator,
  fallback: MessageKey = "novel.sourceSyncDialog.unknown",
): string => t(keys[value] ?? fallback);

export const SourceSyncDialog: React.FC<SourceSyncDialogProps> = ({
  open,
  projectId,
  projectTitle,
  sourceSyncStatus,
  canEditSourceSync,
  onClose,
  onChanged,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const sourceSyncAvailable = sourceSyncStatus !== "detached";
  const [plan, setPlan] = useState<SourceSyncPlanSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [applyingConflictId, setApplyingConflictId] = useState<number | null>(null);
  const [detachConfirming, setDetachConfirming] = useState(false);
  const [detaching, setDetaching] = useState(false);
  const mountedRef = useRef(false);

  const refreshParent = async (): Promise<boolean> => {
    try {
      const refreshed = await onChanged();
      if (refreshed === false) addToast(t("novel.refreshFailed"), "error");
      return refreshed !== false;
    } catch (error) {
      addToast(extractErrorMessage(error, t("novel.refreshFailed")), "error");
      return false;
    }
  };
  const scopeRef = useRef({ open, projectId, token: 0 });
  const activeProjectIdRef = useRef(projectId);
  const previewSeqRef = useRef(0);
  const applySeqRef = useRef(0);
  const detachSeqRef = useRef(0);
  const previewBusyRef = useRef(false);
  const applyBusyRef = useRef(false);
  const detachBusyRef = useRef(false);

  const invalidatePreview = () => {
    previewSeqRef.current += 1;
    previewBusyRef.current = false;
  };

  useEffect(() => {
    const token = scopeRef.current.token + 1;
    scopeRef.current = { open, projectId, token };
    invalidatePreview();
    setPlan(null);
    setLoading(false);
  }, [open, projectId]);

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    applySeqRef.current += 1;
    detachSeqRef.current += 1;
    applyBusyRef.current = false;
    detachBusyRef.current = false;
    setApplying(false);
    setApplyingConflictId(null);
    setDetachConfirming(false);
    setDetaching(false);
  }, [projectId]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      scopeRef.current = {
        ...scopeRef.current,
        open: false,
        token: scopeRef.current.token + 1,
      };
      invalidatePreview();
      applySeqRef.current += 1;
      detachSeqRef.current += 1;
      applyBusyRef.current = false;
      detachBusyRef.current = false;
      setApplyingConflictId(null);
    };
  }, []);

  const isCurrentScope = (target: { projectId: number; token: number }) =>
    mountedRef.current &&
    scopeRef.current.open &&
    scopeRef.current.projectId === target.projectId &&
    scopeRef.current.token === target.token;

  const isCurrentAction = (
    target: { projectId: number; token: number },
    ref: React.MutableRefObject<number>,
    sequence: number,
  ) => isCurrentScope(target) && ref.current === sequence;

  const isCurrentMutation = (
    projectIdToMatch: number,
    ref: React.MutableRefObject<number>,
    sequence: number,
  ) =>
    mountedRef.current &&
    activeProjectIdRef.current === projectIdToMatch &&
    ref.current === sequence;

  const conflictCounts = useMemo(() => {
    const counts = { info: 0, warning: 0, blocking: 0 };
    for (const conflict of plan?.conflicts ?? []) {
      if (conflict.status !== "pending") continue;
      if (conflict.severity in counts) {
        counts[conflict.severity as keyof typeof counts] += 1;
      }
    }
    return counts;
  }, [plan]);
  const planCanResolveConflicts = canMutateSourceSyncRunStatus(plan?.run.status);
  const canApplyUnconflicted =
    plan != null &&
    planCanResolveConflicts &&
    (plan.run.status === "planned" || hasPendingInfoConflicts(plan));

  const handlePreview = async () => {
    if (!canEditSourceSync) {
      addToast(t("novel.sourceSyncDialog.permissionSync"), "error");
      return;
    }
    if (previewBusyRef.current) {
      addToast(t("novel.sourceSyncDialog.checkingUpdates"), "error");
      return;
    }
    previewBusyRef.current = true;
    const targetProjectId = projectId;
    const targetScope = { projectId: targetProjectId, token: scopeRef.current.token };
    const requestSeq = previewSeqRef.current + 1;
    previewSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const data = await previewSourceSync(targetProjectId);
      if (!isCurrentAction(targetScope, previewSeqRef, requestSeq)) return;
      setPlan(data);
      addToast(t("novel.sourceSyncDialog.planGenerated"), "success");
    } catch (err) {
      if (isCurrentAction(targetScope, previewSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.sourceSyncDialog.previewFailed")), "error");
      }
    } finally {
      if (isCurrentAction(targetScope, previewSeqRef, requestSeq)) {
        previewBusyRef.current = false;
        setLoading(false);
      }
    }
  };

  const handleApplyUnconflicted = async () => {
    if (!canEditSourceSync) {
      addToast(t("novel.sourceSyncDialog.permissionSync"), "error");
      return;
    }
    if (applyBusyRef.current) {
      addToast(t("novel.sourceSyncDialog.applyingUnconflicted"), "error");
      return;
    }
    applyBusyRef.current = true;
    const requestPlan = plan;
    if (!requestPlan) {
      applyBusyRef.current = false;
      return;
    }
    if (!canApplyUnconflictedForPlan(requestPlan)) {
      applyBusyRef.current = false;
      addToast(
        requestPlan.run.status === "completed"
          ? t("novel.sourceSyncDialog.planCompleted")
          : t("novel.sourceSyncDialog.noUnconflictedItems"),
        "error",
      );
      return;
    }
    const targetProjectId = projectId;
    const requestSeq = applySeqRef.current + 1;
    applySeqRef.current = requestSeq;
    const targetSyncRunId = requestPlan.run.id;
    setApplying(true);
    try {
      const updatedPlan = await applySourceSyncDecisions({
        sync_run_id: targetSyncRunId,
        decisions: [],
      });
      if (!isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) return;
      setPlan(updatedPlan);
      await refreshParent();
      if (!isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) return;
      addToast(t("novel.sourceSyncDialog.unconflictedApplied"), "success");
    } catch (err) {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.sourceSyncDialog.applyUnconflictedFailed")), "error");
      }
    } finally {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        applyBusyRef.current = false;
        setApplying(false);
        setApplyingConflictId(null);
      }
    }
  };

  const handleKeepLocal = async (conflict: SourceSyncConflictSummary) => {
    if (!canEditSourceSync) {
      addToast(t("novel.sourceSyncDialog.permissionSync"), "error");
      return;
    }
    if (conflict.status !== "pending") return;
    if (applyBusyRef.current) {
      addToast(t("novel.sourceSyncDialog.applyingDecision"), "error");
      return;
    }
    applyBusyRef.current = true;
    const requestPlan = plan;
    if (!requestPlan) {
      applyBusyRef.current = false;
      return;
    }
    if (!canMutateSourceSyncRunStatus(requestPlan.run.status)) {
      applyBusyRef.current = false;
      addToast(t("novel.sourceSyncDialog.planCompleted"), "error");
      return;
    }
    const targetProjectId = projectId;
    const requestSeq = applySeqRef.current + 1;
    applySeqRef.current = requestSeq;
    const decisionPayload = {
      user_action: "keep_local",
      conflict_scope: conflict.conflict_scope,
      target_kind: conflict.target_kind,
      target_id: conflict.target_id,
    };
    setApplying(true);
    setApplyingConflictId(conflict.id);
    try {
      const updatedPlan = await applySourceSyncDecisions({
        sync_run_id: requestPlan.run.id,
        decisions: [
          {
            conflict_id: conflict.id,
            strategy: "keep_local",
            decision_payload_json: decisionPayload,
          },
        ],
      });
      if (!isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) return;
      setPlan(updatedPlan);
      await refreshParent();
      if (!isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) return;
      addToast(t("novel.sourceSyncDialog.keptLocal"), "success");
    } catch (err) {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.sourceSyncDialog.applyDecisionFailed")), "error");
      }
    } finally {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        applyBusyRef.current = false;
        setApplying(false);
        setApplyingConflictId(null);
      }
    }
  };

  const handleRegenerateScene = async (conflict: SourceSyncConflictSummary) => {
    if (!canEditSourceSync) {
      addToast(t("novel.sourceSyncDialog.permissionSync"), "error");
      return;
    }
    if (conflict.status !== "pending") return;
    const sceneId = conflict.regenerate_scene_id;
    if (!sceneId) {
      addToast(t("novel.sourceSyncDialog.cannotLocateScene"), "error");
      return;
    }
    if (applyBusyRef.current) {
      addToast(t("novel.sourceSyncDialog.applyingDecision"), "error");
      return;
    }
    const ok = await confirmDialog({
      title: t("novel.sourceSyncDialog.regenerateTitle"),
      message: t("novel.sourceSyncDialog.regenerateMessage"),
      confirmText: t("novel.sourceSyncDialog.regenerateConfirm"),
      dialogType: "warning",
    });
    if (!ok) return;
    applyBusyRef.current = true;
    const requestPlan = plan;
    if (!requestPlan) {
      applyBusyRef.current = false;
      return;
    }
    if (!canMutateSourceSyncRunStatus(requestPlan.run.status)) {
      applyBusyRef.current = false;
      addToast(t("novel.sourceSyncDialog.planCompleted"), "error");
      return;
    }
    const targetProjectId = projectId;
    const requestSeq = applySeqRef.current + 1;
    applySeqRef.current = requestSeq;
    const decisionPayload = {
      galgame_scene_id: sceneId,
      reason: "user_requested_source_sync_regenerate",
    };
    setApplying(true);
    setApplyingConflictId(conflict.id);
    try {
      const updatedPlan = await applySourceSyncDecisions({
        sync_run_id: requestPlan.run.id,
        decisions: [
          {
            conflict_id: conflict.id,
            strategy: "regenerate_scene",
            decision_payload_json: decisionPayload,
          },
        ],
      });
      if (!isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) return;
      setPlan(updatedPlan);
      await refreshParent();
      if (!isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) return;
      addToast(t("novel.sourceSyncDialog.sceneRegenerated"), "success");
    } catch (err) {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.sourceSyncDialog.regenerateFailed")), "error");
      }
    } finally {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        applyBusyRef.current = false;
        setApplying(false);
        setApplyingConflictId(null);
      }
    }
  };

  const handleDetach = async () => {
    if (!canEditSourceSync) {
      addToast(t("novel.sourceSyncDialog.detachPermission"), "error");
      return;
    }
    if (detachBusyRef.current) {
      addToast(t("novel.sourceSyncDialog.detaching"), "error");
      return;
    }
    detachBusyRef.current = true;
    const targetProjectId = projectId;
    const requestSeq = detachSeqRef.current + 1;
    detachSeqRef.current = requestSeq;
    setDetachConfirming(true);
    const ok = await confirmDialog({
      title: t("novel.sourceSyncDialog.detachTitle"),
      message: t("novel.sourceSyncDialog.detachMessage"),
      confirmText: t("novel.sourceSyncDialog.detachConfirm"),
      dialogType: "warning",
    });
    if (!ok) {
      detachBusyRef.current = false;
      if (isCurrentMutation(targetProjectId, detachSeqRef, requestSeq)) {
        setDetachConfirming(false);
      }
      return;
    }
    if (!isCurrentMutation(targetProjectId, detachSeqRef, requestSeq)) {
      detachBusyRef.current = false;
      return;
    }
    setDetachConfirming(false);
    setDetaching(true);
    try {
      await detachDerivedProject(targetProjectId);
      if (!isCurrentMutation(targetProjectId, detachSeqRef, requestSeq)) return;
      await refreshParent();
      if (!isCurrentMutation(targetProjectId, detachSeqRef, requestSeq)) return;
      setPlan(null);
      addToast(t("novel.sourceSyncDialog.detached"), "success");
    } catch (err) {
      if (isCurrentMutation(targetProjectId, detachSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.sourceSyncDialog.detachFailed")), "error");
      }
    } finally {
      if (isCurrentMutation(targetProjectId, detachSeqRef, requestSeq)) {
        detachBusyRef.current = false;
        setDetachConfirming(false);
        setDetaching(false);
      }
    }
  };

  const busy = loading || applying || detachConfirming || detaching;
  const handleClose = () => {
    if (busy) return;
    onClose();
  };

  return (
    <Modal
      isOpen={open}
      onClose={handleClose}
      title={t("novel.sourceSyncDialog.title")}
      subtitle={projectTitle}
      icon={<GitCompare size={15} />}
      maxWidthClassName="max-w-5xl"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
      footer={canEditSourceSync && sourceSyncAvailable ? (
        <div className="flex w-full items-center gap-2">
          {!busy ? (
            <Dropdown
              items={[
                {
                  label: t("novel.sourceSyncDialog.detachSourceProject"),
                  onClick: handleDetach,
                  danger: true,
                  icon: <Unplug size={12} />,
                },
              ]}
            />
          ) : null}
          <span className="min-w-0 flex-1" />
          <BookButton
            variant={plan ? "secondary" : "primary"}
            size="sm"
            onClick={handlePreview}
            disabled={busy}
            className="!min-h-8 !text-xs"
          >
            {loading ? <LoadingSpinner size="sm" /> : <RefreshCw size={13} />}
            {plan ? t("novel.sourceSyncDialog.recheck") : t("novel.sourceSyncDialog.checkUpdate")}
          </BookButton>
          {canApplyUnconflicted ? (
            <BookButton
              variant="primary"
              size="sm"
              onClick={handleApplyUnconflicted}
              disabled={busy}
              className="!min-h-8 !text-xs"
            >
              {applying ? (
                <LoadingSpinner size="sm" />
              ) : (
                <CheckCircle2 size={13} />
              )}
              {t("novel.sourceSyncDialog.applySafeUpdates")}
            </BookButton>
          ) : null}
        </div>
      ) : undefined}
    >
      <div className="space-y-4">
        <section className="flex flex-wrap gap-x-5 gap-y-1 text-xs leading-relaxed">
          <span>
            <span className="text-book-text-muted">{t("novel.sourceSyncDialog.sourceStatus")}</span>{" "}
            <span className="font-medium text-book-text-main">
              {labelFrom(STATUS_KEYS, sourceSyncStatus, t)}
            </span>
          </span>
          <span>
            <span className="text-book-text-muted">{t("novel.sourceSyncDialog.updateCheck")}</span>{" "}
            <span className="font-medium text-book-text-main">
              {plan ? t("novel.sourceSyncDialog.checked") : t("novel.sourceSyncDialog.notChecked")}
            </span>
          </span>
          {plan ? (
            <span>
              <span className="text-book-text-muted">{t("novel.sourceSyncDialog.needsAttention")}</span>{" "}
              <span className="font-medium text-book-text-main">
                {[
                  conflictCounts.info > 0
                    ? t("novel.sourceSyncDialog.severityCount", {
                        label: t("novel.sourceSyncDialog.severityInfo"),
                        count: conflictCounts.info,
                      })
                    : null,
                  conflictCounts.warning > 0
                    ? t("novel.sourceSyncDialog.severityCount", {
                        label: t("novel.sourceSyncDialog.severityWarning"),
                        count: conflictCounts.warning,
                      })
                    : null,
                  conflictCounts.blocking > 0
                    ? t("novel.sourceSyncDialog.severityCount", {
                        label: t("novel.sourceSyncDialog.severityBlocking"),
                        count: conflictCounts.blocking,
                      })
                    : null,
                ].filter(Boolean).join(t("novel.sourceSyncDialog.countSeparator")) || t("novel.sourceSyncDialog.noAttention")}
              </span>
            </span>
          ) : null}
        </section>

        {!plan ? (
          <p className="text-sm leading-relaxed text-book-text-muted">
            {t(sourceSyncAvailable ? "novel.sourceSyncDialog.explanation" : "novel.sourceSyncDialog.detached")}
          </p>
        ) : plan.conflicts.length === 0 ? (
          <div className="rounded-[var(--pyqt-radius-lg)] border border-success/30 bg-success/10 px-4 py-4 text-sm text-book-text-main">
            {planCanResolveConflicts
              ? t("novel.sourceSyncDialog.noManualConflicts")
              : t("novel.sourceSyncDialog.completed")}
          </div>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-xs font-semibold text-book-text-main">
              <AlertTriangle size={13} className="text-warning" />
              {t("novel.sourceSyncDialog.conflictList")}
            </div>
            <div className="space-y-2">
              {plan.conflicts.map((conflict) => (
                <ConflictRow
                  key={conflict.id}
                  conflict={conflict}
                  disabled={busy || !canEditSourceSync || !sourceSyncAvailable || !planCanResolveConflicts}
                  applying={applyingConflictId === conflict.id}
                  onKeepLocal={() => handleKeepLocal(conflict)}
                  onRegenerateScene={() => handleRegenerateScene(conflict)}
                />
              ))}
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
};

function canMutateSourceSyncRunStatus(status: string | undefined): boolean {
  return status === "planned" || status === "applying";
}

function hasPendingInfoConflicts(plan: SourceSyncPlanSummary): boolean {
  return plan.conflicts.some((conflict) => conflict.status === "pending" && conflict.severity === "info");
}

function canApplyUnconflictedForPlan(plan: SourceSyncPlanSummary): boolean {
  return canMutateSourceSyncRunStatus(plan.run.status) && (plan.run.status === "planned" || hasPendingInfoConflicts(plan));
}

const ConflictRow: React.FC<{
  conflict: SourceSyncConflictSummary;
  disabled: boolean;
  applying: boolean;
  onKeepLocal: () => void;
  onRegenerateScene: () => void;
}> = ({ conflict, disabled, applying, onKeepLocal, onRegenerateScene }) => {
  const { t } = useI18n();
  const showActions = conflict.status === "pending" && (!disabled || applying);
  return (
    <article className="a4n-flat-surface overflow-hidden">
      <div className="flex items-start gap-3 px-3 py-3">
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
            <h3 className="font-semibold text-book-text-main">
              {labelFrom(TARGET_KIND_KEYS, conflict.target_kind, t, "novel.sourceSyncDialog.relatedContent")}
            </h3>
            <span className={severityClass(conflict.severity)}>
              {labelFrom(SEVERITY_KEYS, conflict.severity, t)}
            </span>
          </div>
          <p className="mt-1 text-small leading-relaxed text-book-text-muted">
            {labelFrom(SOURCE_CHANGE_KEYS, conflict.source_change_kind, t)} · {labelFrom(LOCAL_STATE_KEYS, conflict.local_state_kind, t)} · {labelFrom(CONFLICT_STATUS_KEYS, conflict.status, t)}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {conflict.regenerate_scene_id != null && conflict.status === "pending" && !disabled ? (
            <Dropdown
              items={[
                {
                  label: t("novel.sourceSyncDialog.regenerateScene"),
                  onClick: onRegenerateScene,
                  icon: <RefreshCw size={12} />,
                },
              ]}
            />
          ) : null}
          {showActions ? (
            <BookButton
              variant="secondary"
              size="sm"
              onClick={onKeepLocal}
              disabled={disabled}
              className="!text-xs"
            >
              {applying ? <LoadingSpinner size="sm" /> : <CheckCircle2 size={13} />}
              {t("novel.sourceSyncDialog.keepCurrent")}
            </BookButton>
          ) : null}
        </div>
      </div>
      <div className="grid border-t border-book-border/40 text-small leading-relaxed text-book-text-muted md:grid-cols-2 md:divide-x md:divide-book-border/40">
        <JsonSummary title={t("novel.sourceSyncDialog.sourceSummary")} value={conflict.source_summary_json} />
        <JsonSummary title={t("novel.sourceSyncDialog.localSummary")} value={conflict.local_summary_json} />
      </div>
      <details className="border-t border-book-border/40 px-3 py-2 text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer rounded px-1 py-0.5 transition-colors hover:bg-book-bg-tertiary hover:text-book-text-main">
          {t("novel.sourceSyncDialog.diagnostics")}
        </summary>
        <div className="mt-2 space-y-2">
          {conflict.status === "obsolete" ? <p>{t("novel.sourceSyncDialog.obsoleteReferences")}</p> : null}
          <dl className="grid grid-cols-[4.5rem_minmax(0,1fr)] gap-x-2 gap-y-1 font-mono">
            <dt>{t("novel.sourceSyncDialog.conflictScope")}</dt>
            <dd className="min-w-0 break-all">{conflict.conflict_scope}</dd>
            <dt>{t("novel.sourceSyncDialog.targetLocation")}</dt>
            <dd className="min-w-0 break-all">{conflict.target_kind} · {conflict.target_id}</dd>
            <dt>{t("novel.sourceSyncDialog.sourceStatus")}</dt>
            <dd className="min-w-0 break-all">{conflict.source_change_kind}</dd>
            <dt>{t("novel.sourceSyncDialog.localState")}</dt>
            <dd className="min-w-0 break-all">{conflict.local_state_kind}</dd>
          </dl>
          <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-book-bg-tertiary/45 p-2 font-mono">
            {JSON.stringify({
              source: conflict.source_summary_json,
              local: conflict.local_summary_json,
              affected_refs: conflict.affected_refs_json,
              decision: conflict.decision_json,
            }, null, 2)}
          </pre>
        </div>
      </details>
    </article>
  );
};

const JsonSummary: React.FC<{ title: string; value: Record<string, unknown> }> = ({ title, value }) => {
  const { locale, t } = useI18n();
  return (
    <div className="min-w-0 px-3 py-2.5">
      <div className="mb-1 text-caption font-medium text-book-text-sub">{title}</div>
      <div className="break-words text-book-text-sub">{formatSummary(value, t, locale)}</div>
    </div>
  );
};

function severityClass(severity: string): string {
  if (severity === "blocking") {
    return "text-small font-semibold text-error";
  }
  if (severity === "warning") {
    return "text-small font-semibold text-warning";
  }
  return "text-small font-semibold text-book-primary";
}

function formatSummary(
  value: Record<string, unknown>,
  t: MessageTranslator,
  locale: Locale,
): string {
  const entries = Object.entries(value).filter(
    ([key, item]) => key in SUMMARY_KEY_KEYS && item != null && item !== "",
  );
  if (entries.length === 0) return t("novel.sourceSyncDialog.summaryEmpty");
  return entries
    .slice(0, 6)
    .map(([key, item]) => t("novel.sourceSyncDialog.summaryItem", {
      label: summaryKeyLabel(key, t),
      value: formatValue(key, item, t, locale),
    }))
    .join(t("common.summarySeparator"));
}

function summaryKeyLabel(key: string, t: MessageTranslator): string {
  return t(SUMMARY_KEY_KEYS[key] ?? "novel.sourceSyncDialog.unknown");
}

function formatValue(
  key: string,
  value: unknown,
  t: MessageTranslator,
  locale: Locale,
): string {
  if (key === "chapter_number" && typeof value === "number") {
    return t("novel.sourceSyncDialog.chapterValue", { number: formatNumber(value, locale) });
  }
  if (key === "page_number" && typeof value === "number") {
    return t("novel.sourceSyncDialog.pageValue", { number: formatNumber(value, locale) });
  }
  if (key === "panel_number" && typeof value === "number") {
    return t("novel.sourceSyncDialog.panelValue", { number: formatNumber(value, locale) });
  }
  if (key === "scene_index" && typeof value === "number") {
    return t("novel.sourceSyncDialog.sceneValue", { number: formatNumber(value, locale) });
  }
  if (key === "line_index" && typeof value === "number") {
    return t("novel.sourceSyncDialog.lineValue", { number: formatNumber(value, locale) });
  }
  if (key === "segment_index" && typeof value === "number") {
    return t("novel.sourceSyncDialog.segmentValue", { number: formatNumber(value + 1, locale) });
  }
  if (typeof value === "string") return value;
  if (typeof value === "number") return formatNumber(value, locale);
  if (typeof value === "boolean") return value ? t("novel.sourceSyncDialog.booleanYes") : t("novel.sourceSyncDialog.booleanNo");
  if (Array.isArray(value)) {
    const readable = value.filter((item) => typeof item === "string" || typeof item === "number");
    return readable.length === value.length && readable.length <= 4
      ? readable.map((item) => typeof item === "number" ? formatNumber(item, locale) : item).join(t("common.listSeparator"))
      : t("novel.sourceSyncDialog.itemCount", { count: value.length });
  }
  if (value && typeof value === "object") {
    const count = Object.keys(value as Record<string, unknown>).length;
    return count > 0
      ? t("novel.sourceSyncDialog.infoCount", { count: formatNumber(count, locale) })
      : t("novel.sourceSyncDialog.none");
  }
  return t("novel.sourceSyncDialog.unknownValue");
}
