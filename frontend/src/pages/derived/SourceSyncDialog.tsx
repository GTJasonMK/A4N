import React, { useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, GitCompare, Loader2, RefreshCw, Unplug } from "lucide-react";
import { applySourceSyncDecisions, detachDerivedProject, previewSourceSync, type SourceSyncConflictSummary, type SourceSyncPlanSummary } from "../../api/sourceSync";
import { extractErrorMessage } from "../../api";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { Modal } from "../../components/ui/Modal";
import { BookButton } from "../../components/ui/BookButton";

interface SourceSyncDialogProps {
  open: boolean;
  projectId: number;
  projectTitle: string;
  sourceSyncStatus: string;
  canEditSourceSync: boolean;
  onClose: () => void;
  onChanged: () => void | Promise<void>;
}

const SEVERITY_LABELS: Record<string, string> = {
  info: "普通更新",
  warning: "需确认",
  blocking: "阻塞冲突",
};

const STATUS_LABELS: Record<string, string> = {
  current: "当前源文本",
  root_updated: "根文本已更新",
  conflict: "存在冲突",
  detached: "已分离",
};

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
  const [plan, setPlan] = useState<SourceSyncPlanSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [applyingConflictId, setApplyingConflictId] = useState<number | null>(null);
  const [detachConfirming, setDetachConfirming] = useState(false);
  const [detaching, setDetaching] = useState(false);
  const mountedRef = useRef(false);
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
      addToast("当前账号没有同步此派生项目的权限", "error");
      return;
    }
    if (previewBusyRef.current) {
      addToast("正在生成同步计划", "error");
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
      addToast("同步计划已生成", "success");
    } catch (err) {
      if (isCurrentAction(targetScope, previewSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "生成同步计划失败"), "error");
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
      addToast("当前账号没有同步此派生项目的权限", "error");
      return;
    }
    if (applyBusyRef.current) {
      addToast("正在应用无冲突项", "error");
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
          ? "当前同步计划已完成"
          : "当前同步计划没有可继续应用的无冲突项",
        "error",
      );
      return;
    }
    const targetProjectId = projectId;
    const targetPreviewScope = { projectId: targetProjectId, token: scopeRef.current.token };
    const requestSeq = applySeqRef.current + 1;
    applySeqRef.current = requestSeq;
    const targetSyncRunId = requestPlan.run.id;
    setApplying(true);
    try {
      const updatedRun = await applySourceSyncDecisions({
        sync_run_id: targetSyncRunId,
        decisions: [],
      });
      if (!isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) return;
      addToast("无冲突项已应用", "success");
      await onChanged();
      if (!isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) return;
      if (!isCurrentScope(targetPreviewScope)) return;
      const resolvedAt = new Date().toISOString();
      setPlan((current) => {
        if (!current || current.run.id !== requestPlan.run.id) return current;
        return {
          ...current,
          run: updatedRun,
          conflicts: current.conflicts.map((item) =>
            item.status === "pending" && item.severity === "info"
              ? {
                  ...item,
                  status: "resolved",
                  resolution_strategy: "keep_local",
                  decision_json: {
                    system_action: "apply_unconflicted_item",
                    source_change_kind: item.source_change_kind,
                    local_state_kind: item.local_state_kind,
                  },
                  resolved_at: resolvedAt,
                }
              : item,
          ),
        };
      });
    } catch (err) {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "应用无冲突项失败"), "error");
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
      addToast("当前账号没有同步此派生项目的权限", "error");
      return;
    }
    if (conflict.status !== "pending") return;
    if (applyBusyRef.current) {
      addToast("正在应用同步决策", "error");
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
      addToast("当前同步计划已完成", "error");
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
      const updatedRun = await applySourceSyncDecisions({
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
      setPlan((current) => {
        if (!current || current.run.id !== requestPlan.run.id) return current;
        return {
          run: updatedRun,
          conflicts: current.conflicts.map((item) =>
            item.id === conflict.id
              ? {
                  ...item,
                  status: "ignored",
                  resolution_strategy: "keep_local",
                  decision_json: decisionPayload,
                  resolved_at: new Date().toISOString(),
                }
              : item,
          ),
        };
      });
      addToast("已保留当前本地内容", "success");
      await onChanged();
    } catch (err) {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "应用同步决策失败"), "error");
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
      addToast("当前账号没有同步此派生项目的权限", "error");
      return;
    }
    if (conflict.status !== "pending") return;
    const sceneId = galgameSceneIdForConflict(conflict);
    if (!sceneId) {
      addToast("该冲突无法定位 Galgame 场", "error");
      return;
    }
    if (applyBusyRef.current) {
      addToast("正在应用同步决策", "error");
      return;
    }
    const ok = await confirmDialog({
      title: "重建 Galgame 场",
      message: "将用目标源快照锚点重建该场，旧行、选择支、该场背景/CG 生成意图和媒体绑定会失效。确定继续？",
      confirmText: "重建场",
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
      addToast("当前同步计划已完成", "error");
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
      const updatedRun = await applySourceSyncDecisions({
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
      const resolvedAt = new Date().toISOString();
      setPlan((current) => {
        if (!current || current.run.id !== requestPlan.run.id) return current;
        return {
          run: updatedRun,
          conflicts: current.conflicts.map((item) =>
            isSameGalgameSceneConflict(item, sceneId)
              ? {
                  ...item,
                  status: "resolved",
                  resolution_strategy: "regenerate_scene",
                  decision_json:
                    item.id === conflict.id
                      ? decisionPayload
                      : {
                          system_action: "resolved_by_regenerate_scene",
                          galgame_scene_id: sceneId,
                        },
                  resolved_at: resolvedAt,
                }
              : item,
          ),
        };
      });
      addToast("Galgame 场已重建", "success");
      await onChanged();
    } catch (err) {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "重建 Galgame 场失败"), "error");
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
      addToast("当前账号没有分离此派生项目的权限", "error");
      return;
    }
    if (detachBusyRef.current) {
      addToast("正在分离派生项目", "error");
      return;
    }
    detachBusyRef.current = true;
    const targetProjectId = projectId;
    const requestSeq = detachSeqRef.current + 1;
    detachSeqRef.current = requestSeq;
    setDetachConfirming(true);
    const ok = await confirmDialog({
      title: "分离派生项目",
      message: "分离后该项目不再跟随根文本更新，当前源快照会成为固定基线。确定继续？",
      confirmText: "分离项目",
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
      addToast("派生项目已分离", "success");
      setPlan(null);
      await onChanged();
    } catch (err) {
      if (isCurrentMutation(targetProjectId, detachSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "分离派生项目失败"), "error");
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
      title="源文本同步"
      subtitle={`项目：${projectTitle}`}
      icon={<GitCompare size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
      footer={
        <div className="flex flex-wrap justify-end gap-2 px-4 py-3">
          <BookButton variant="secondary" size="sm" onClick={handleClose} disabled={busy}>
            关闭
          </BookButton>
          {canEditSourceSync ? (
            <>
              <BookButton
                variant="warning"
                size="sm"
                onClick={handleDetach}
                disabled={busy || sourceSyncStatus === "detached"}
              >
                {detaching ? <Loader2 size={13} className="animate-spin" /> : <Unplug size={13} />}
                分离项目
              </BookButton>
              <BookButton variant="secondary" size="sm" onClick={handlePreview} disabled={busy}>
                {loading ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
                生成同步计划
              </BookButton>
              <BookButton
                variant="primary"
                size="sm"
                onClick={handleApplyUnconflicted}
                disabled={!canApplyUnconflicted || busy}
              >
                {applying ? <Loader2 size={13} className="animate-spin" /> : <CheckCircle2 size={13} />}
                应用无冲突项
              </BookButton>
            </>
          ) : null}
        </div>
      }
    >
      <div className="space-y-4">
        <section className="grid gap-2 text-xs sm:grid-cols-4">
          <InfoBlock label="当前状态" value={STATUS_LABELS[sourceSyncStatus] ?? sourceSyncStatus} />
          <InfoBlock label="同步计划" value={plan ? `#${plan.run.id}` : "未生成"} />
          <InfoBlock label="源快照" value={plan ? `${plan.run.from_snapshot_id} -> ${plan.run.to_snapshot_id}` : "-"} />
          <InfoBlock
            label="冲突统计"
            value={plan ? `${conflictCounts.info}/${conflictCounts.warning}/${conflictCounts.blocking}` : "-"}
          />
        </section>

        {!plan ? (
          <div className="a4n-page-surface px-4 py-5 text-sm text-book-text-muted">
            点击“生成同步计划”后，系统只创建 dry run 和冲突清单，不会改写提示词、媒体、片段或时间线。
          </div>
        ) : plan.conflicts.length === 0 ? (
          <div className="rounded border border-success/30 bg-success/10 px-4 py-4 text-sm text-book-text-main">
            {planCanResolveConflicts ? "当前同步计划没有冲突，可以直接应用无冲突项。" : "当前同步计划已完成。"}
          </div>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-xs font-semibold text-book-text-main">
              <AlertTriangle size={13} className="text-warning" />
              冲突清单
            </div>
            <div className="max-h-[48vh] space-y-2 overflow-y-auto custom-scrollbar pr-1">
              {plan.conflicts.map((conflict) => (
                <ConflictRow
                  key={conflict.id}
                  conflict={conflict}
                  disabled={busy || !canEditSourceSync || !planCanResolveConflicts}
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

const InfoBlock: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="a4n-stat-tile">
    <div className="mb-1 text-[10px] text-book-text-muted">{label}</div>
    <div className="break-words text-xs text-book-text-main">{value}</div>
  </div>
);

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
}> = ({ conflict, disabled, applying, onKeepLocal, onRegenerateScene }) => (
  <article className="a4n-interactive-row px-3 py-2">
    <div className="flex flex-wrap items-center gap-2 text-xs">
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
        <span className={severityClass(conflict.severity)}>
          {SEVERITY_LABELS[conflict.severity] ?? conflict.severity}
        </span>
        <span className="font-semibold text-book-text-main">
          {conflict.conflict_scope} / {conflict.target_kind} #{conflict.target_id}
        </span>
        <span className="text-book-text-muted">
          {conflict.source_change_kind} · {conflict.local_state_kind} · {conflict.status}
        </span>
      </div>
      <BookButton
        variant="secondary"
        size="sm"
        onClick={onKeepLocal}
        disabled={disabled || conflict.status !== "pending"}
      >
        {applying ? <Loader2 size={13} className="animate-spin" /> : <CheckCircle2 size={13} />}
        保留当前
      </BookButton>
      {canRegenerateGalgameScene(conflict) ? (
        <BookButton
          variant="warning"
          size="sm"
          onClick={onRegenerateScene}
          disabled={disabled || conflict.status !== "pending"}
        >
          {applying ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
          重建场
        </BookButton>
      ) : null}
    </div>
    <div className="mt-2 grid gap-2 text-[11px] leading-relaxed text-book-text-muted md:grid-cols-2">
      <JsonSummary title="源文本变化" value={conflict.source_summary_json} />
      <JsonSummary title="本地对象" value={conflict.local_summary_json} />
    </div>
  </article>
);

function canRegenerateGalgameScene(conflict: SourceSyncConflictSummary): boolean {
  return galgameSceneIdForConflict(conflict) != null;
}

function galgameSceneIdForConflict(conflict: SourceSyncConflictSummary): number | null {
  if (conflict.conflict_scope !== "text_anchor") return null;
  if (conflict.target_kind === "galgame_scene") return conflict.target_id;
  if (conflict.target_kind !== "galgame_line") return null;
  const raw = conflict.affected_refs_json.galgame_scene_id;
  return typeof raw === "number" && Number.isFinite(raw) ? raw : null;
}

function isSameGalgameSceneConflict(conflict: SourceSyncConflictSummary, sceneId: number): boolean {
  return conflict.status === "pending" && galgameSceneIdForConflict(conflict) === sceneId;
}

const JsonSummary: React.FC<{ title: string; value: Record<string, unknown> }> = ({ title, value }) => (
  <div className="a4n-stat-tile px-2 py-1.5">
    <div className="mb-1 text-[10px] font-medium text-book-text-sub">{title}</div>
    <div className="break-words">{formatSummary(value)}</div>
  </div>
);

function severityClass(severity: string): string {
  if (severity === "blocking") {
    return "rounded border border-error/35 bg-error/10 px-1.5 py-0.5 text-[10px] font-semibold text-error";
  }
  if (severity === "warning") {
    return "rounded border border-warning/35 bg-warning/10 px-1.5 py-0.5 text-[10px] font-semibold text-warning";
  }
  return "rounded border border-book-primary/30 bg-book-primary/10 px-1.5 py-0.5 text-[10px] font-semibold text-book-primary";
}

function formatSummary(value: Record<string, unknown>): string {
  const entries = Object.entries(value).filter(([, item]) => item != null && item !== "");
  if (entries.length === 0) return "-";
  return entries
    .slice(0, 6)
    .map(([key, item]) => `${key}: ${formatValue(item)}`)
    .join("；");
}

function formatValue(value: unknown): string {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}
