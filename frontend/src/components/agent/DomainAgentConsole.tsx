import { ThemedSelect } from "../ui/ThemedSelect";
import React, {
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Bot,
  Check,
  CircleAlert,
  CircleDot,
  Clock3,
  Film,
  Pause,
  Play,
  RefreshCw,
  Send,
  ShieldCheck,
  Square,
  Workflow,
  X,
} from "lucide-react";
import { extractErrorMessage } from "../../api";
import { compactAgentValue } from "./AgentMessageRow";
import { AgentMessageList } from "./AgentMessageList";
import {
  controlAgentRun,
  createAgentThread,
  getAgentThread,
  listAgentThreads,
  resolveAgentApproval,
  sendAgentMessage,
  subscribeAgentThreadEvents,
  type AgentApprovalView,
  type AgentControlAction,
  type AgentDeliveryMode,
  type AgentEvent,
  type AgentExecutionMode,
  type AgentExecutionView,
  type AgentRunView,
  type AgentScope,
  type AgentThreadSnapshot,
  type AgentThreadView,
} from "../../api/agent";
import { BookButton, BookIconButton } from "../ui/BookButton";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import { useToast } from "../feedback/Toast";
import { isImeCompositionActive } from "../../utils/ime";
import { useI18n, type Locale } from "../../i18n";
import { reduceAgentEvent, snapshotCursor } from "../../utils/agentEventReducer";

export type DomainAgentCapability = "content_optimization" | "canvas" | "director";

export interface DomainAgentConsoleProps {
  capability: DomainAgentCapability;
  projectId: number;
  chapterId?: number | null;
  resourceLabel?: string | null;
  compact?: boolean;
  fill?: boolean;
  className?: string;
  onClose?: () => void;
  onResourceChanged?: () => void | Promise<void>;
  getEditorSource?: () => import("../../api/agent").AgentEditorSource | null;
  onEditorRequest?: (request: import("../../api/agent").AgentEditorRequest, runId: string) => Promise<void>;
}

export interface DomainAgentBinding {
  capability: DomainAgentCapability;
  projectId: number;
  chapterId: number | null;
  resourceKind: "project" | "chapter";
  resourceId: string;
}

const ACTIVE_RUN_STATUSES = new Set<AgentRunView["status"]>([
  "queued",
  "running",
  "waiting",
  "pausing",
  "paused",
  "cancelling",
]);

const RUN_STATUS_TONES: Record<string, string> = {
  queued: "text-info bg-info-bg",
  running: "text-book-primary bg-book-primary/10",
  waiting: "text-warning bg-warning-bg",
  pausing: "text-warning bg-warning-bg",
  paused: "text-warning bg-warning-bg",
  cancelling: "text-warning bg-warning-bg",
  cancelled: "text-book-text-muted bg-book-bg-tertiary/70",
  completed: "text-success bg-success-bg",
  failed: "text-error bg-error-bg",
  interrupted: "text-warning bg-warning-bg",
  recovery_required: "text-error bg-error-bg",
};

const RUN_STATUS_LABELS: Record<
  string,
  { zh: string; en: string }
> = {
  queued: { zh: "排队中", en: "Queued" },
  running: { zh: "运行中", en: "Running" },
  waiting: { zh: "等待中", en: "Waiting" },
  pausing: { zh: "暂停中", en: "Pausing" },
  paused: { zh: "已暂停", en: "Paused" },
  cancelling: { zh: "取消中", en: "Cancelling" },
  cancelled: { zh: "已取消", en: "Cancelled" },
  completed: { zh: "已完成", en: "Completed" },
  failed: { zh: "失败", en: "Failed" },
  interrupted: { zh: "已中断", en: "Interrupted" },
  recovery_required: { zh: "需要恢复", en: "Recovery required" },
  active: { zh: "活跃", en: "Active" },
  archived: { zh: "已归档", en: "Archived" },
  pending: { zh: "待处理", en: "Pending" },
  claimed: { zh: "处理中", en: "Claimed" },
  approved: { zh: "已批准", en: "Approved" },
  rejected: { zh: "已拒绝", en: "Rejected" },
  expired: { zh: "已过期", en: "Expired" },
  consumed: { zh: "已处理", en: "Consumed" },
};

const labelFor = (value: string | null | undefined, locale: Locale): string => {
  if (!value) return locale === "zh-CN" ? "未知" : "Unknown";
  const known = RUN_STATUS_LABELS[value];
  return known
    ? locale === "zh-CN"
      ? known.zh
      : known.en
    : value.replace(/_/g, " ");
};

const randomClientMessageId = (): string =>
  typeof globalThis.crypto?.randomUUID === "function"
    ? globalThis.crypto.randomUUID()
    : `domain-agent-${Date.now()}-${Math.random().toString(36).slice(2)}`;

function positiveId(value: number | null | undefined): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}

export function domainAgentBinding(
  capability: DomainAgentCapability,
  projectId: number,
  chapterId?: number | null,
): DomainAgentBinding | null {
  if (!positiveId(projectId)) return null;
  if (capability === "content_optimization") {
    if (!positiveId(chapterId)) return null;
    return {
      capability,
      projectId,
      chapterId,
      resourceKind: "chapter",
      resourceId: String(chapterId),
    };
  }
  if (capability === "director") {
    if (!positiveId(chapterId)) return null;
    return {
      capability,
      projectId,
      chapterId,
      resourceKind: "chapter",
      resourceId: String(chapterId),
    };
  }
  if (chapterId != null) return null;
  return {
    capability,
    projectId,
    chapterId: null,
    resourceKind: "project",
    resourceId: String(projectId),
  };
}

export function matchesDomainAgentThread(
  thread: AgentThreadView,
  binding: DomainAgentBinding,
): boolean {
  if (thread.kind !== "subagent" || thread.capability !== binding.capability) {
    return false;
  }
  const scope = thread.scope;
  if (
    scope.project_id !== binding.projectId ||
    scope.chapter_id !== binding.chapterId
  ) {
    return false;
  }
  const resource = thread.bound_resource;
  if (
    resource?.kind === binding.resourceKind &&
    resource.id === binding.resourceId
  ) {
    return true;
  }
  return (
    scope.resource_kind === binding.resourceKind &&
    scope.resource_id === binding.resourceId
  );
}

export function findDomainAgentThread(
  threads: AgentThreadView[],
  binding: DomainAgentBinding,
): AgentThreadView | null {
  const matches = threads.filter((thread) =>
    matchesDomainAgentThread(thread, binding),
  );
  return (
    matches.find((thread) => thread.status === "active") ?? matches[0] ?? null
  );
}

function scopeForBinding(binding: DomainAgentBinding): AgentScope {
  return {
    resource_kind: binding.resourceKind,
    resource_id: binding.resourceId,
    project_id: binding.projectId,
    chapter_id: binding.chapterId,
    // The runtime derives the authoritative domain tool catalog from the
    // capability. Keeping this list empty avoids a second frontend tool source.
    allowed_capabilities: [binding.capability],
    allowed_tools: [],
  };
}

function activeRun(snapshot: AgentThreadSnapshot | null): AgentRunView | null {
  return snapshot
    ? [...snapshot.runs]
        .reverse()
        .find((run) => ACTIVE_RUN_STATUSES.has(run.status)) ?? null
    : null;
}

function executionTone(execution: AgentExecutionView): string {
  if (execution.status === "succeeded") return "text-success";
  if (
    execution.status === "failed" ||
    execution.status === "rejected" ||
    execution.status === "outcome_unknown"
  ) {
    return "text-error";
  }
  if (execution.status === "awaiting_user") return "text-warning";
  return "text-book-primary";
}

export const DomainAgentConsole: React.FC<DomainAgentConsoleProps> = ({
  capability,
  projectId,
  chapterId,
  resourceLabel,
  compact = false,
  fill = false,
  className = "",
  onClose,
  onResourceChanged,
  getEditorSource,
  onEditorRequest,
}) => {
  const { locale, t } = useI18n();
  const { addToast } = useToast();
  const inputId = useId();
  const [thread, setThread] = useState<AgentThreadView | null>(null);
  const [snapshot, setSnapshot] = useState<AgentThreadSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [sending, setSending] = useState(false);
  const pendingEditorRequests = useRef(new Set<string>());
  const [draft, setDraft] = useState("");
  const [deliveryMode, setDeliveryMode] =
    useState<AgentDeliveryMode>("followup");
  const [executionMode, setExecutionMode] =
    useState<AgentExecutionMode>("autonomous");
  const [controlBusy, setControlBusy] =
    useState<AgentControlAction | null>(null);
  const [approvalBusy, setApprovalBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const [reloadNonce, setReloadNonce] = useState(0);
  const [activityOpen, setActivityOpen] = useState(false);
  const snapshotRequestRef = useRef(0);
  const eventCursorRef = useRef(0);
  const composingRef = useRef(false);
  const lastCompositionEndRef = useRef(0);
  const mutationBaselineRef = useRef<{
    bindingKey: string;
    executionId: string | null;
  } | null>(null);

  const binding = useMemo(
    () => domainAgentBinding(capability, projectId, chapterId),
    [capability, chapterId, projectId],
  );
  const bindingKey = binding
    ? `${binding.capability}:${binding.resourceKind}:${binding.resourceId}`
    : "invalid";
  const displayResourceLabel =
    resourceLabel?.trim() ||
    (binding
      ? binding.resourceKind === "chapter"
        ? `${locale === "zh-CN" ? "章节" : "Chapter"} ${binding.resourceId}`
        : `${locale === "zh-CN" ? "项目" : "Project"} ${binding.resourceId}`
      : locale === "zh-CN"
        ? "当前上下文"
        : "Current context");
  const isOptimization = capability === "content_optimization";
  const isDirector = capability === "director";
  const title = isOptimization
    ? t("agent.domain.contentTitle")
    : isDirector
      ? t("agent.domain.directorTitle")
      : t("agent.domain.canvasTitle");
  const subtitle = isOptimization
    ? t("agent.domain.contentSubtitle", { resource: displayResourceLabel })
    : isDirector
      ? t("agent.domain.directorSubtitle", { resource: displayResourceLabel })
      : t("agent.domain.canvasSubtitle", { resource: displayResourceLabel });
  const icon = isOptimization ? <Bot size={15} /> : isDirector ? <Film size={15} /> : <Workflow size={15} />;

  const loadSnapshot = useCallback(
    async (threadId: string, quiet = false): Promise<AgentThreadSnapshot | null> => {
      const requestId = snapshotRequestRef.current + 1;
      snapshotRequestRef.current = requestId;
      if (!quiet) setLoading(true);
      try {
        const next = await getAgentThread(threadId);
        if (snapshotRequestRef.current !== requestId) return null;
        setSnapshot(next);
        setThread(next.thread);
        eventCursorRef.current = snapshotCursor(next);
        setError(null);
        return next;
      } catch (err) {
        if (snapshotRequestRef.current === requestId) {
          setError(extractErrorMessage(err, t("agent.domain.loadFailed")));
        }
        return null;
      } finally {
        if (snapshotRequestRef.current === requestId && !quiet) {
          setLoading(false);
        }
      }
    },
    [t],
  );

  const ensureThread = useCallback(async () => {
    if (!binding) {
      setThread(null);
      setSnapshot(null);
      setLoading(false);
      setError(t("agent.domain.invalidContext"));
      return;
    }
    setLoading(true);
    setError(null);
    setConnectionError(null);
    try {
      const threads = await listAgentThreads({
        capability: binding.capability,
        include_archived: false,
        limit: 200,
      });
      let match = findDomainAgentThread(threads, binding);
      if (!match) {
        try {
          match = await createAgentThread({
            kind: "subagent",
            capability: binding.capability,
            title: `${title} · ${displayResourceLabel}`,
            bound_resource: {
              kind: binding.resourceKind,
              id: binding.resourceId,
              label: displayResourceLabel,
              revision: null,
              hash: null,
            },
            scope: scopeForBinding(binding),
          });
        } catch (createError) {
          // Another tab or the main Agent may have won the canonical create.
          const retryThreads = await listAgentThreads({
            capability: binding.capability,
            include_archived: false,
            limit: 200,
          });
          match = findDomainAgentThread(retryThreads, binding);
          if (!match) throw createError;
        }
      }
      setThread(match);
      await loadSnapshot(match.id);
    } catch (err) {
      setError(extractErrorMessage(err, t("agent.domain.loadFailed")));
      setLoading(false);
    }
  }, [binding, displayResourceLabel, loadSnapshot, t, title]);

  useEffect(() => {
    setThread(null);
    setSnapshot(null);
    setDraft("");
    setConnectionError(null);
    setConnected(false);
    void ensureThread();
  }, [bindingKey, ensureThread]);

  const threadId = thread?.id;

  useEffect(() => {
    if (!threadId) return undefined;
    const controller = new AbortController();
    let stopped = false;
    let cursor = 0;
    const initial = snapshot;
    cursor = initial ? snapshotCursor(initial) : eventCursorRef.current;
    const refreshTimer = window.setInterval(
      () => void loadSnapshot(threadId, true).then((next) => {
        if (next) cursor = snapshotCursor(next);
      }),
      7000,
    );

    const connect = async () => {
      if (!initial) {
        const loaded = await loadSnapshot(threadId, true);
        if (!loaded || stopped) return;
        cursor = snapshotCursor(loaded);
      }
      while (!stopped) {
        try {
          setConnected(true);
          setConnectionError(null);
          await subscribeAgentThreadEvents(
            { thread_id: threadId, after_sequence: cursor, limit: 200 },
            (event) => {
              if (event.thread_id !== threadId) return;
              let needsReload = false;
              setSnapshot((current) => {
                if (!current || current.thread.id !== threadId) return current;
                const reduction = reduceAgentEvent(current, event, cursor);
                needsReload = reduction.needsReload;
                if (reduction.needsReload) return current;
                cursor = reduction.cursor;
                return reduction.snapshot;
              });
              if (needsReload) {
                void loadSnapshot(threadId, true).then((next) => {
                  if (next) cursor = snapshotCursor(next);
                });
              }
            },
            controller.signal,
          );
        } catch (err) {
          if (controller.signal.aborted) return;
          setConnected(false);
          setConnectionError(
            extractErrorMessage(err, t("agent.domain.connectionLost")),
          );
        }
        if (!stopped) {
          setConnected(false);
          await new Promise((resolve) => window.setTimeout(resolve, 1500));
        }
      }
    };
    void connect();
    return () => {
      stopped = true;
      controller.abort();
      window.clearInterval(refreshTimer);
      setConnected(false);
    };
  }, [loadSnapshot, t, threadId]);

  useEffect(() => {
    if (reloadNonce === 0) return;
    void ensureThread();
  }, [ensureThread, reloadNonce]);

  const currentRun = useMemo(() => activeRun(snapshot), [snapshot]);
  const pendingApprovals = useMemo(
    () =>
      snapshot?.approvals.filter((approval) => approval.status === "pending") ??
      [],
    [snapshot],
  );
  const executions = useMemo(
    () => snapshot?.executions.slice(-8).reverse() ?? [],
    [snapshot],
  );
  const latestMutationExecutionId = useMemo(() => {
    const resourceKind =
      capability === "content_optimization"
        ? "chapter"
        : capability === "director"
          ? "chapter_visual_plan"
          : "canvas_graph";
    return (
      snapshot?.executions
        .slice()
        .reverse()
        .find(
          (execution) =>
            execution.effect !== "read" &&
            execution.result_refs.some((ref) => ref.kind === resourceKind) &&
            execution.status === "succeeded",
        )?.id ?? null
    );
  }, [capability, snapshot]);

  useEffect(() => {
    const source = getEditorSource?.();
    if (!source || !onEditorRequest || !snapshot || pendingEditorRequests.current.size > 0) return;
    for (const event of snapshot.events) {
      const request = event.editor_request;
      if (!request || !event.run_id || request.source.view_instance_id !== source.view_instance_id) continue;
      const execution = snapshot.executions.find((item) => item.id === request.execution_id);
      const run = snapshot.runs.find((item) => item.id === event.run_id);
      if (!execution || !["prepared", "awaiting_user", "dispatching"].includes(execution.status)
        || run?.status !== "waiting" || run.waiting_reason !== "external"
        || pendingEditorRequests.current.has(request.execution_id)) continue;
      pendingEditorRequests.current.add(request.execution_id);
      void onEditorRequest(request, event.run_id).catch((error) => {
        addToast(extractErrorMessage(error, t("agent.domain.resourceRefreshFailed")), "error");
      }).finally(() => {
        pendingEditorRequests.current.delete(request.execution_id);
        void loadSnapshot(snapshot.thread.id, true);
      });
    }
  }, [addToast, getEditorSource, loadSnapshot, onEditorRequest, snapshot, t]);
  const latestEvent = useMemo<AgentEvent | null>(
    () => snapshot?.events[snapshot.events.length - 1] ?? null,
    [snapshot],
  );
  const canSend =
    Boolean(thread) &&
    thread?.status === "active" &&
    draft.trim().length > 0 &&
    !sending;

  useEffect(() => {
    if (!snapshot) return;
    const baseline = mutationBaselineRef.current;
    if (!baseline || baseline.bindingKey !== bindingKey) {
      mutationBaselineRef.current = {
        bindingKey,
        executionId: latestMutationExecutionId,
      };
      return;
    }
    if (
      !latestMutationExecutionId ||
      baseline.executionId === latestMutationExecutionId
    ) {
      return;
    }
    baseline.executionId = latestMutationExecutionId;
    if (!onResourceChanged) return;
    void Promise.resolve(onResourceChanged()).catch((err) => {
      addToast(
        extractErrorMessage(err, t("agent.domain.resourceRefreshFailed")),
        "error",
      );
    });
  }, [
    addToast,
    bindingKey,
    latestMutationExecutionId,
    onResourceChanged,
    snapshot,
    t,
  ]);

  const refresh = async () => {
    if (refreshing) return;
    setRefreshing(true);
    try {
      await ensureThread();
    } finally {
      setRefreshing(false);
    }
  };

  const send = async () => {
    const content = draft.trim();
    if (!thread || !content || !canSend) return;
    setSending(true);
    try {
      await sendAgentMessage({
        thread_id: thread.id,
        client_message_id: randomClientMessageId(),
        content,
        delivery_mode: deliveryMode,
        execution_mode: executionMode,
        editor_source: getEditorSource?.() ?? undefined,
      });
      setDraft("");
      await loadSnapshot(thread.id, true);
    } catch (err) {
      const message = extractErrorMessage(err, t("agent.domain.sendFailed"));
      setError(message);
      addToast(message, "error");
    } finally {
      setSending(false);
    }
  };

  const control = async (action: AgentControlAction) => {
    if (!thread || !currentRun || controlBusy) return;
    setControlBusy(action);
    try {
      await controlAgentRun({
        thread_id: thread.id,
        run_id: currentRun.id,
        action,
      });
      await loadSnapshot(thread.id, true);
    } catch (err) {
      const message = extractErrorMessage(err, t("agent.domain.controlFailed"));
      setError(message);
      addToast(message, "error");
    } finally {
      setControlBusy(null);
    }
  };

  const resolveApproval = async (
    approval: AgentApprovalView,
    decision: "approve" | "reject",
  ) => {
    if (approvalBusy) return;
    setApprovalBusy(approval.id);
    try {
      await resolveAgentApproval({
        approval_id: approval.id,
        decision,
        reason:
          decision === "approve"
            ? t("agent.domain.approvedByUser")
            : t("agent.domain.rejectedByUser"),
      });
      if (thread) await loadSnapshot(thread.id, true);
    } catch (err) {
      const message = extractErrorMessage(
        err,
        t("agent.domain.approvalFailed"),
      );
      setError(message);
      addToast(message, "error");
    } finally {
      setApprovalBusy(null);
    }
  };

  const shellClass = fill
    ? "h-full min-h-0"
    : compact
      ? "h-[min(23rem,36vh)] min-h-[16rem]"
      : "h-[min(31rem,44vh)] min-h-[18rem]";

  if (!binding) {
    return (
      <section
        aria-label={t("agent.domain.consoleLabel")}
        className={`a4n-flat-surface flex min-w-0 flex-col overflow-hidden ${shellClass} ${className}`}
      >
        <Notice>{t("agent.domain.invalidContext")}</Notice>
      </section>
    );
  }

  return (
    <section
      aria-label={t("agent.domain.consoleLabel")}
      className={`a4n-flat-surface flex min-w-0 flex-col overflow-hidden ${shellClass} ${className}`}
    >
      <header className="agent-console-header flex min-w-0 shrink-0 items-start gap-2.5 border-b border-book-border/50 bg-book-bg-paper/65 px-3 py-2.5">
        <span className="agent-console-icon inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-xl bg-book-primary/10 text-book-primary">
          {icon}
        </span>
        <div className="agent-console-heading min-w-0 flex-1">
          <div className="agent-console-title-row flex min-w-0 flex-wrap items-center gap-1.5">
            <h2 className="agent-console-title min-w-0 truncate text-xs font-semibold text-book-text-main">
              {title}
            </h2>
            <span className="agent-console-subagent rounded-full bg-book-primary/10 px-1.5 py-0.5 text-[10px] text-book-primary">
              {t("agent.domain.subagent")}
            </span>
            {thread ? (
              <span
                className={`agent-console-thread-status rounded-full px-1.5 py-0.5 text-[10px] ${
                  thread.status === "active"
                    ? "bg-success-bg text-success"
                    : "bg-book-bg-tertiary text-book-text-muted"
                }`}
              >
                {labelFor(thread.status, locale)}
              </span>
            ) : null}
          </div>
          <p className="agent-console-subtitle mt-0.5 min-w-0 truncate text-caption text-book-text-muted">
            {subtitle}
          </p>
          <p className="agent-console-shared-control mt-0.5 min-w-0 truncate text-[10px] text-book-text-muted">
            {t("agent.domain.sharedControl")}
          </p>
        </div>
        <div className="agent-console-actions flex shrink-0 items-center gap-1">
          <span
            className={`agent-console-connection hidden items-center gap-1 text-[10px] sm:inline-flex ${
              connected ? "text-success" : "text-book-text-muted"
            }`}
            role="status"
            aria-live="polite"
          >
            <span
              className={`h-1.5 w-1.5 rounded-full ${
                connected ? "bg-success" : "bg-book-border"
              }`}
              aria-hidden="true"
            />
            {connected ? t("agent.domain.live") : t("agent.domain.polling")}
          </span>
          <BookIconButton
            label={t("agent.domain.refresh")}
            variant="secondary"
            size="sm"
            onClick={() => void refresh()}
            disabled={loading || refreshing}
          >
            <RefreshCw
              size={13}
              className={loading || refreshing ? "animate-spin" : undefined}
            />
          </BookIconButton>
          {onClose ? (
            <BookIconButton
              label={t("common.close")}
              size="sm"
              onClick={onClose}
            >
              <X size={13} />
            </BookIconButton>
          ) : null}
        </div>
      </header>

      {error ? (
        <Notice onRetry={() => setReloadNonce((value) => value + 1)}>
          {error}
        </Notice>
      ) : null}
      {!error && connectionError ? (
        <div
          role="status"
          className="agent-console-connection-notice flex min-w-0 shrink-0 items-center gap-2 border-b border-warning/25 bg-warning-bg/50 px-3 py-1.5 text-[10px] text-warning"
        >
          <Clock3 size={12} className="shrink-0" />
          <span className="min-w-0 flex-1 break-words">{connectionError}</span>
        </div>
      ) : null}

      {loading && !snapshot ? (
        <div className="flex min-h-0 flex-1 items-center justify-center text-xs text-book-text-muted">
          <LoadingSpinner size="sm" className="mr-2" />
          {t("agent.domain.loading")}
        </div>
      ) : snapshot ? (
        <>
          <div
            className="agent-console-message-scroll custom-scrollbar min-h-0 min-w-0 flex-1 overflow-y-auto overflow-x-hidden px-2.5 py-2.5"
            aria-live="polite"
          >
            {snapshot.messages.length === 0 ? error ? (
              <div className="agent-console-error-state" role="status">
                <CircleAlert size={16} aria-hidden="true" />
                <span>{t("agent.domain.loadFailed")}</span>
              </div>
            ) : (
              <div className="a4n-empty-state a4n-empty-state--compact min-h-0 py-8 text-xs">
                <Bot size={20} />
                <p>{t("agent.domain.empty")}</p>
              </div>
            ) : (
              <AgentMessageList
                messages={snapshot.messages}
                locale={locale}
                agentLabel={t("agent.domain.domainAgent")}
              />
            )}
          </div>

          {executions.length > 0 || latestEvent ? (
            <details
              className="shrink-0 border-t border-book-border/45 bg-book-bg-paper/35"
              open={activityOpen}
              onToggle={(event) =>
                setActivityOpen(event.currentTarget.open)
              }
            >
              <summary className="flex min-h-9 cursor-pointer list-none items-center gap-2 px-3 py-2 text-[10px] text-book-text-muted [&::-webkit-details-marker]:hidden">
                <CircleDot size={11} className="text-book-primary" />
                <span>{t("agent.domain.activity")}</span>
                <span className="rounded-full bg-book-bg-tertiary px-1.5 py-0.5 tabular-nums">
                  {executions.length}
                </span>
                {latestEvent ? (
                  <span className="min-w-0 flex-1 truncate text-book-text-muted/80">
                    {latestEvent.kind.replace(/_/g, " ")}
                  </span>
                ) : null}
              </summary>
              <div className="max-h-32 space-y-1.5 overflow-y-auto border-t border-book-border/35 px-3 py-2 custom-scrollbar">
                {executions.map((execution) => (
                  <ExecutionRow
                    key={execution.id}
                    execution={execution}
                    locale={locale}
                  />
                ))}
              </div>
            </details>
          ) : null}

          {pendingApprovals.length > 0 ? (
            <div className="shrink-0 space-y-1.5 border-t border-warning/30 bg-warning-bg/30 px-2.5 py-2">
              {pendingApprovals.map((approval) => (
                <ApprovalRow
                  key={approval.id}
                  approval={approval}
                  locale={locale}
                  busy={approvalBusy === approval.id}
                  onResolve={resolveApproval}
                />
              ))}
            </div>
          ) : null}

          <div className="shrink-0 border-t border-book-border/50 bg-book-bg-paper/70 p-2.5">
            {currentRun ? (
              <div className="mb-2 flex min-w-0 flex-wrap items-center gap-1.5">
                <span className="text-[10px] text-book-text-muted">
                  {t("agent.domain.runStatus")}
                </span>
                <span
                  className={`rounded-full px-1.5 py-0.5 text-[10px] ${
                    RUN_STATUS_TONES[currentRun.status] ??
                    "bg-book-bg-tertiary text-book-text-muted"
                  }`}
                >
                  {labelFor(currentRun.status, locale)}
                </span>
                <div className="ml-auto flex gap-1.5">
                  {["queued", "running", "waiting"].includes(
                    currentRun.status,
                  ) ? (
                    <BookButton
                      size="sm"
                      variant="warning"
                      onClick={() => void control("pause")}
                      disabled={controlBusy != null}
                    >
                      <Pause size={12} />
                      {t("agent.domain.pause")}
                    </BookButton>
                  ) : null}
                  {["paused", "pausing"].includes(currentRun.status) ? (
                    <BookButton
                      size="sm"
                      onClick={() => void control("resume")}
                      disabled={controlBusy != null}
                    >
                      <Play size={12} />
                      {t("agent.domain.resume")}
                    </BookButton>
                  ) : null}
                  {ACTIVE_RUN_STATUSES.has(currentRun.status) ? (
                    <BookButton
                      size="sm"
                      variant="danger"
                      onClick={() => void control("cancel")}
                      disabled={controlBusy != null}
                    >
                      <Square size={11} />
                      {t("agent.domain.cancel")}
                    </BookButton>
                  ) : null}
                </div>
              </div>
            ) : null}
            <div className="mb-2 flex min-w-0 flex-wrap items-center gap-1.5">
              <label className="min-w-0 flex-1 sm:flex-none">
                <span className="sr-only">{t("agent.domain.deliveryMode")}</span>
                <ThemedSelect
                  value={deliveryMode}
                  onChange={(event) =>
                    setDeliveryMode(event.target.value as AgentDeliveryMode)
                  }
                  className="pyqt-input h-8 min-h-8 w-full !py-1 text-[10px] sm:w-auto"
                  aria-label={t("agent.domain.deliveryMode")}
                  disabled={thread?.status !== "active" || sending}
                >
                  <option value="followup">{t("agent.domain.followup")}</option>
                  <option value="steer">{t("agent.domain.steer")}</option>
                  <option value="inject">{t("agent.domain.inject")}</option>
                </ThemedSelect>
              </label>
              <label className="min-w-0 flex-1 sm:flex-none">
                <span className="sr-only">{t("agent.domain.executionMode")}</span>
                <ThemedSelect
                  value={executionMode}
                  onChange={(event) =>
                    setExecutionMode(event.target.value as AgentExecutionMode)
                  }
                  className="pyqt-input h-8 min-h-8 w-full !py-1 text-[10px] sm:w-auto"
                  aria-label={t("agent.domain.executionMode")}
                  disabled={thread?.status !== "active" || sending}
                >
                  <option value="autonomous">{t("agent.domain.autonomous")}</option>
                  <option value="review_writes">{t("agent.domain.reviewWrites")}</option>
                  <option value="advice_only">{t("agent.domain.adviceOnly")}</option>
                </ThemedSelect>
              </label>
            </div>
            <form
              className="flex min-w-0 items-end gap-2"
              onSubmit={(event) => {
                event.preventDefault();
                void send();
              }}
            >
              <label htmlFor={inputId} className="sr-only">
                {t("agent.domain.messageLabel")}
              </label>
              <textarea
                id={inputId}
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                onCompositionStart={() => {
                  composingRef.current = true;
                }}
                onCompositionEnd={(event) => {
                  lastCompositionEndRef.current = Date.now();
                  setDraft(event.currentTarget.value);
                  window.setTimeout(() => {
                    composingRef.current = false;
                  }, 0);
                }}
                onKeyDown={(event) => {
                  if (
                    event.key === "Enter" &&
                    !event.shiftKey &&
                    !isImeCompositionActive(
                      event.nativeEvent,
                      composingRef.current,
                      lastCompositionEndRef.current,
                    )
                  ) {
                    event.preventDefault();
                    void send();
                  }
                }}
                rows={1}
                maxLength={8000}
                placeholder={t("agent.domain.messagePlaceholder")}
                className="pyqt-input min-h-10 min-w-0 flex-1 resize-y text-xs leading-relaxed sm:min-h-8"
                disabled={thread?.status !== "active" || sending}
              />
              <BookIconButton
                label={t("agent.domain.send")}
                variant="secondary"
                size="md"
                onClick={() => void send()}
                disabled={!canSend}
              >
                {sending ? <LoadingSpinner size="sm" /> : <Send size={14} />}
              </BookIconButton>
            </form>
            {thread?.status === "archived" ? (
              <p className="mt-1.5 text-[10px] text-warning">
                {t("agent.domain.archivedHint")}
              </p>
            ) : null}
          </div>
        </>
      ) : error ? (
        <div className="agent-console-error-state" role="status">
          <CircleAlert size={16} aria-hidden="true" />
          <span>{t("agent.domain.loadFailed")}</span>
        </div>
      ) : (
        <div className="flex min-h-0 flex-1 items-center justify-center text-xs text-book-text-muted">
          {t("agent.domain.empty")}
        </div>
      )}
    </section>
  );
};

function ExecutionRow({
  execution,
  locale,
}: {
  execution: AgentExecutionView;
  locale: Locale;
}) {
  return (
    <div className="min-w-0 rounded-lg border border-book-border/35 bg-book-bg-paper px-2 py-1.5">
      <div className="flex min-w-0 items-center gap-1.5 text-[10px]">
        <CircleDot size={10} className={executionTone(execution)} />
        <span className="min-w-0 flex-1 truncate text-book-text-sub">
          {execution.tool_name || execution.kind}
        </span>
        <span className="shrink-0 text-book-text-muted">
          {labelFor(execution.status, locale)}
        </span>
      </div>
      {execution.result_summary ? (
        <p className="mt-0.5 break-words text-[10px] leading-relaxed text-book-text-muted">
          {compactAgentValue(execution.result_summary)}
        </p>
      ) : null}
      {execution.error ? (
        <p className="mt-0.5 break-words text-[10px] leading-relaxed text-error">
          {execution.error}
        </p>
      ) : null}
    </div>
  );
}

function ApprovalRow({
  approval,
  locale,
  busy,
  onResolve,
}: {
  approval: AgentApprovalView;
  locale: Locale;
  busy: boolean;
  onResolve: (
    approval: AgentApprovalView,
    decision: "approve" | "reject",
  ) => void;
}) {
  return (
    <div className="min-w-0 rounded-xl border border-warning/35 bg-warning-bg/55 p-2">
      <div className="flex min-w-0 items-start gap-1.5">
        <ShieldCheck size={13} className="mt-0.5 shrink-0 text-warning" />
        <div className="min-w-0 flex-1">
          <p className="break-words text-xs font-medium text-book-text-main">
            {approval.command}
          </p>
          {approval.reason ? (
            <p className="mt-0.5 break-words text-[10px] leading-relaxed text-book-text-muted">
              {approval.reason}
            </p>
          ) : null}
        </div>
      </div>
      <div className="mt-1.5 flex gap-1.5">
        <BookButton
          size="sm"
          className="min-w-0 flex-1"
          onClick={() => onResolve(approval, "approve")}
          disabled={busy}
        >
          <Check size={12} />
          {locale === "zh-CN" ? "批准" : "Approve"}
        </BookButton>
        <BookButton
          size="sm"
          variant="danger"
          className="min-w-0 flex-1"
          onClick={() => onResolve(approval, "reject")}
          disabled={busy}
        >
          <X size={12} />
          {locale === "zh-CN" ? "拒绝" : "Reject"}
        </BookButton>
        {busy ? <LoadingSpinner size="sm" className="self-center" /> : null}
      </div>
    </div>
  );
}

function Notice({
  children,
  onRetry,
}: {
  children: React.ReactNode;
  onRetry?: () => void;
}) {
  const { t } = useI18n();
  return (
    <div
      role="alert"
      className="flex min-w-0 shrink-0 items-start gap-2 border-b border-error/30 bg-error-bg px-3 py-2 text-[10px] leading-relaxed text-error"
    >
      <span className="min-w-0 flex-1 break-words">{children}</span>
      {onRetry ? (
        <BookButton size="sm" variant="secondary" onClick={onRetry}>
          {t("common.retry")}
        </BookButton>
      ) : null}
    </div>
  );
}
