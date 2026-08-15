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

const TARGET_KIND_LABELS: Record<string, string> = {
  chapter: "章节",
  segment: "视频片段",
  galgame_scene: "GALGame 场次",
  galgame_line: "GALGame 台词",
  character: "人物资料",
  object: "物品资料",
  scene: "场景资料",
};

const CONFLICT_STATUS_LABELS: Record<string, string> = {
  pending: "待处理",
  resolved: "已按来源处理",
  ignored: "已保留当前内容",
};

const SOURCE_CHANGE_LABELS: Record<string, string> = {
  added: "来源新增",
  updated: "来源已更新",
  reordered: "来源顺序变化",
  deleted: "来源已删除",
  moved: "来源位置变化",
  split: "来源内容已拆分",
};

const LOCAL_STATE_LABELS: Record<string, string> = {
  user_edited: "已由用户编辑",
  generated: "已有生成内容",
  imported: "已有导入内容",
  assembled: "已进入成片装配",
  clean: "当前内容未修改",
  draft: "当前内容为草稿",
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
      addToast("正在检查来源更新", "error");
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
        addToast(extractErrorMessage(err, "检查来源更新失败"), "error");
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
      addToast("无冲突项已应用", "success");
      await onChanged();
      if (!isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) return;
      setPlan(updatedPlan);
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
      addToast("该冲突无法定位 GALGame 场", "error");
      return;
    }
    if (applyBusyRef.current) {
      addToast("正在应用同步决策", "error");
      return;
    }
    const ok = await confirmDialog({
      title: "重建 GALGame 场",
      message: "将根据最新来源内容重建该场，旧行、选择支、背景、CG 生成设置和媒体绑定会失效。确定继续？",
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
      addToast("GALGame 场已重建", "success");
      await onChanged();
    } catch (err) {
      if (isCurrentMutation(targetProjectId, applySeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "重建 GALGame 场失败"), "error");
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
      message: "分离后该项目不再跟随原项目更新，当前来源内容会保留为固定版本。确定继续？",
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
      title="来源同步"
      subtitle={projectTitle}
      icon={<GitCompare size={15} />}
      maxWidthClassName="max-w-5xl"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
      footer={canEditSourceSync ? (
        <div className="flex w-full items-center gap-2">
          {!busy && sourceSyncStatus !== "detached" ? (
            <Dropdown
              items={[
                {
                  label: "分离来源项目",
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
            {plan ? "重新检查" : "检查更新"}
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
              应用安全更新
            </BookButton>
          ) : null}
        </div>
      ) : undefined}
    >
      <div className="space-y-4">
        <section className="flex flex-wrap gap-x-5 gap-y-1 text-xs leading-relaxed">
          <span>
            <span className="text-book-text-muted">来源状态</span>{" "}
            <span className="font-medium text-book-text-main">
              {STATUS_LABELS[sourceSyncStatus] ?? "状态未知"}
            </span>
          </span>
          <span>
            <span className="text-book-text-muted">更新检查</span>{" "}
            <span className="font-medium text-book-text-main">{plan ? "已完成" : "尚未检查"}</span>
          </span>
          {plan ? (
            <span>
              <span className="text-book-text-muted">需要处理</span>{" "}
              <span className="font-medium text-book-text-main">
                {[
                  conflictCounts.info > 0 ? `普通 ${conflictCounts.info}` : null,
                  conflictCounts.warning > 0 ? `确认 ${conflictCounts.warning}` : null,
                  conflictCounts.blocking > 0 ? `阻塞 ${conflictCounts.blocking}` : null,
                ].filter(Boolean).join(" · ") || "无需处理"}
              </span>
            </span>
          ) : null}
        </section>

        {!plan ? (
          <p className="text-sm leading-relaxed text-book-text-muted">
            点击“检查更新”只会比较来源与当前项目，不会修改任何内容。
          </p>
        ) : plan.conflicts.length === 0 ? (
          <div className="rounded-[var(--pyqt-radius-lg)] border border-success/30 bg-success/10 px-4 py-4 text-sm text-book-text-main">
            {planCanResolveConflicts ? "没有需要人工处理的冲突，可以应用安全更新。" : "本次来源同步已完成。"}
          </div>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-xs font-semibold text-book-text-main">
              <AlertTriangle size={13} className="text-warning" />
              冲突清单
            </div>
            <div className="space-y-2">
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
  const showActions = conflict.status === "pending" && (!disabled || applying);
  return (
    <article className="a4n-flat-surface overflow-hidden">
      <div className="flex items-start gap-3 px-3 py-3">
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
            <h3 className="font-semibold text-book-text-main">
              {TARGET_KIND_LABELS[conflict.target_kind] ?? "关联内容"}
            </h3>
            <span className={severityClass(conflict.severity)}>
              {SEVERITY_LABELS[conflict.severity] ?? "普通更新"}
            </span>
          </div>
          <p className="mt-1 text-small leading-relaxed text-book-text-muted">
            {SOURCE_CHANGE_LABELS[conflict.source_change_kind] ?? "来源已变化"} · {LOCAL_STATE_LABELS[conflict.local_state_kind] ?? "当前项目有改动"} · {CONFLICT_STATUS_LABELS[conflict.status] ?? "状态未知"}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {canRegenerateGalgameScene(conflict) && conflict.status === "pending" && !disabled ? (
            <Dropdown
              items={[
                {
                  label: "根据来源重建场",
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
              保留当前
            </BookButton>
          ) : null}
        </div>
      </div>
      <div className="grid border-t border-book-border/40 text-small leading-relaxed text-book-text-muted md:grid-cols-2 md:divide-x md:divide-book-border/40">
        <JsonSummary title="源文本变化" value={conflict.source_summary_json} />
        <JsonSummary title="当前项目" value={conflict.local_summary_json} />
      </div>
      <details className="border-t border-book-border/40 px-3 py-2 text-caption text-book-text-muted">
        <summary className="w-fit cursor-pointer rounded px-1 py-0.5 transition-colors hover:bg-book-bg-tertiary hover:text-book-text-main">
          诊断信息
        </summary>
        <div className="mt-2 space-y-2">
          <dl className="grid grid-cols-[4.5rem_minmax(0,1fr)] gap-x-2 gap-y-1 font-mono">
            <dt>冲突范围</dt>
            <dd className="min-w-0 break-all">{conflict.conflict_scope}</dd>
            <dt>目标定位</dt>
            <dd className="min-w-0 break-all">{conflict.target_kind} · {conflict.target_id}</dd>
            <dt>来源状态</dt>
            <dd className="min-w-0 break-all">{conflict.source_change_kind}</dd>
            <dt>本地状态</dt>
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

const JsonSummary: React.FC<{ title: string; value: Record<string, unknown> }> = ({ title, value }) => (
  <div className="min-w-0 px-3 py-2.5">
    <div className="mb-1 text-caption font-medium text-book-text-sub">{title}</div>
    <div className="break-words text-book-text-sub">{formatSummary(value)}</div>
  </div>
);

function severityClass(severity: string): string {
  if (severity === "blocking") {
    return "text-small font-semibold text-error";
  }
  if (severity === "warning") {
    return "text-small font-semibold text-warning";
  }
  return "text-small font-semibold text-book-primary";
}

function formatSummary(value: Record<string, unknown>): string {
  const entries = Object.entries(value).filter(
    ([key, item]) => key in SUMMARY_KEY_LABELS && item != null && item !== "",
  );
  if (entries.length === 0) return "有相关内容变化，可展开诊断信息查看定位信息。";
  return entries
    .slice(0, 6)
    .map(([key, item]) => `${summaryKeyLabel(key)}：${formatValue(key, item)}`)
    .join("；");
}

const SUMMARY_KEY_LABELS: Record<string, string> = {
  title: "标题",
  text: "内容",
  content: "内容",
  summary: "摘要",
  chapter_number: "章节",
  scene_index: "场次",
  line_index: "行号",
  status: "状态",
  source_text: "来源内容",
  local_text: "当前内容",
  name: "名称",
  chapter_title: "章节标题",
  scene_title: "场次标题",
  old_text: "原内容",
  new_text: "新内容",
  reason: "变化原因",
  page_number: "页码",
  panel_number: "分镜",
  segment_index: "片段",
};

function summaryKeyLabel(key: string): string {
  return SUMMARY_KEY_LABELS[key] ?? "其他信息";
}

function formatValue(key: string, value: unknown): string {
  if (key === "chapter_number" && typeof value === "number") return `第 ${value} 章`;
  if (key === "page_number" && typeof value === "number") return `第 ${value} 页`;
  if (key === "panel_number" && typeof value === "number") return `分镜 ${value}`;
  if (key === "scene_index" && typeof value === "number") return `场次 ${value + 1}`;
  if (key === "line_index" && typeof value === "number") return `第 ${value + 1} 行`;
  if (key === "segment_index" && typeof value === "number") return `片段 ${value + 1}`;
  if (typeof value === "string") return value;
  if (typeof value === "number") return String(value);
  if (typeof value === "boolean") return value ? "是" : "否";
  if (Array.isArray(value)) {
    const readable = value.filter((item) => typeof item === "string" || typeof item === "number");
    return readable.length === value.length && readable.length <= 4
      ? readable.join("、")
      : `${value.length} 项`;
  }
  if (value && typeof value === "object") {
    const count = Object.keys(value as Record<string, unknown>).length;
    return count > 0 ? `${count} 项信息` : "无";
  }
  return "—";
}
