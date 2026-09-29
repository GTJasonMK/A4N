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
  CircleAlert,
  CircleDot,
  Clock3,
  Film,
  Pause,
  Play,
  RefreshCw,
  Send,
  Square,
  Trash2,
  Workflow,
  X,
} from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  AgentMessageList,
  createPendingAgentMessage,
  type PendingAgentMessage,
} from "./AgentMessageList";
import { AgentContextMeter } from "./AgentContextMeter";
import { AgentBudgetDialog } from "./AgentBudgetDialog";
import {
  pendingContentDiffForSnapshot,
  type PendingContentDiff,
} from "./contentOptimizationDiff";
import {
  ACTIVE_AGENT_RUN_STATUSES,
  activeAgentRun,
  clearAgentThreadContext,
  controlAgentRun,
  createAgentThread,
  getAgentThread,
  isAgentThreadOperational,
  resolveAgentApproval,
  sendAgentMessage,
  subscribeAgentThreadEvents,
  type AgentApprovalView,
  type AgentControlAction,
  type AgentDeliveryMode,
  type AgentEvent,
  type AgentExecutionMode,
  type AgentThreadSnapshot,
  type AgentThreadView,
  type AgentRunView,
} from "../../api/agent";
import { BookButton, BookIconButton } from "../ui/BookButton";
import { FollowLatestButton } from "../ui/FollowLatestButton";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import { confirmDialog } from "../feedback/ConfirmDialog";
import { useToast } from "../feedback/Toast";
import { useFollowLatest } from "../../hooks/useFollowLatest";
import { isImeCompositionActive } from "../../utils/ime";
import { useI18n } from "../../i18n";
import {
  reduceAgentEvent,
  snapshotCursor,
  loadAgentSnapshotWithEvents,
  streamedAgentMessageIds,
} from "../../utils/agentEventReducer";
import {
  ApprovalRow,
  ExecutionRow,
  Notice,
  labelFor,
} from "./DomainAgentConsoleRows";
import {
  domainAgentBinding,
  domainAgentBindingKey,
  matchesDomainAgentThread,
  matchesEditorSourceScope,
  positiveId,
  scopeForBinding,
  type DomainAgentCapability,
} from "./DomainAgentBinding";
export {
  domainAgentBinding,
  domainAgentBindingKey,
  findDomainAgentThread,
  matchesDomainAgentThread,
  matchesEditorSourceScope,
  type DomainAgentBinding,
  type DomainAgentCapability,
} from "./DomainAgentBinding";

export interface DomainAgentConsoleProps {
  capability: DomainAgentCapability;
  projectId: number;
  chapterId?: number | null;
  resourceLabel?: string | null;
  compact?: boolean;
  fill?: boolean;
  className?: string;
  onClose?: () => void;
  executionMode?: AgentExecutionMode;
  onExecutionModeChange?: (mode: AgentExecutionMode) => void;
  onResourceChanged?: () => void | Promise<void>;
  onPendingContentDiffChange?: (diff: PendingContentDiff | null) => void;
  getEditorSource?: () => import("../../api/agent").AgentEditorSource | null;
  onEditorRequest?: (
    request: import("../../api/agent").AgentEditorRequest,
    runId: string,
  ) => Promise<void>;
}

const randomClientMessageId = (): string =>
  typeof globalThis.crypto?.randomUUID === "function"
    ? globalThis.crypto.randomUUID()
    : `domain-agent-${Date.now()}-${Math.random().toString(36).slice(2)}`;

export const DomainAgentConsole: React.FC<DomainAgentConsoleProps> = ({
  capability,
  projectId,
  chapterId,
  resourceLabel,
  compact = false,
  fill = false,
  className = "",
  onClose,
  executionMode: controlledExecutionMode,
  onExecutionModeChange,
  onResourceChanged,
  onPendingContentDiffChange,
  getEditorSource,
  onEditorRequest,
}) => {
  const { locale, t } = useI18n();
  const { addToast } = useToast();
  const inputId = useId();
  const [thread, setThread] = useState<AgentThreadView | null>(null);
  const [snapshot, setSnapshot] = useState<AgentThreadSnapshot | null>(null);
  const [streamedMessageIds, setStreamedMessageIds] = useState<Set<string>>(
    () => new Set(),
  );
  const [pendingSubmission, setPendingSubmission] =
    useState<PendingAgentMessage | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [sending, setSending] = useState(false);
  const sendingRef = useRef(false);
  const pendingEditorRequests = useRef(new Set<string>());
  const [draft, setDraft] = useState("");
  const [deliveryMode, setDeliveryMode] =
    useState<AgentDeliveryMode>("followup");
  const [localExecutionMode, setLocalExecutionMode] = useState<AgentExecutionMode>(() =>
    capability === "content_optimization" ? "review_writes" : "autonomous",
  );
  const executionMode = controlledExecutionMode ?? localExecutionMode;
  const setExecutionMode = onExecutionModeChange ?? setLocalExecutionMode;
  const [controlBusy, setControlBusy] = useState<AgentControlAction | null>(
    null,
  );
  const [approvalBusy, setApprovalBusy] = useState<string | null>(null);
  const [budgetRun, setBudgetRun] = useState<AgentRunView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const [reloadNonce, setReloadNonce] = useState(0);
  const [activityOpen, setActivityOpen] = useState(false);
  const snapshotRequestRef = useRef(0);
  const snapshotRef = useRef<AgentThreadSnapshot | null>(null);
  const eventCursorRef = useRef(0);
  const reloadInFlightRef = useRef(false);
  const pendingEventsRef = useRef<AgentEvent[]>([]);
  const clearingRef = useRef(false);
  const composingRef = useRef(false);
  const lastCompositionEndRef = useRef(0);
  const mutationBaselineRef = useRef<{
    bindingKey: string;
    executionId: string | null;
  } | null>(null);

  const rememberStreamedMessages = useCallback(
    (events: AgentEvent[], runId?: string) => {
      const ids = streamedAgentMessageIds(events, runId);
      if (ids.length === 0) return;
      setStreamedMessageIds((current) => {
        const next = new Set(current);
        ids.forEach((id) => next.add(id));
        return next.size === current.size ? current : next;
      });
    },
    [],
  );

  const binding = useMemo(
    () => domainAgentBinding(capability, projectId, chapterId),
    [capability, chapterId, projectId],
  );
  const bindingKey = binding
    ? domainAgentBindingKey(binding)
    : "invalid";
  const bindingRef = useRef(binding);
  const bindingActive = useRef(true);
  const threadBindings = useRef(new Map<string, typeof binding>());
  bindingRef.current = binding;
  const isCurrentBinding = useCallback((expected: typeof binding) => expected != null && bindingActive.current && bindingRef.current === expected, []);
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
  const icon = isOptimization ? (
    <Bot size={15} />
  ) : isDirector ? (
    <Film size={15} />
  ) : (
    <Workflow size={15} />
  );

  const commitSnapshot = useCallback((next: AgentThreadSnapshot) => {
    snapshotRef.current = next;
    setSnapshot(next);
    setThread(next.thread);
    eventCursorRef.current = snapshotCursor(next);
  }, []);

  const loadSnapshot = useCallback(
    async (
      threadId: string,
      quiet = false,
    ): Promise<AgentThreadSnapshot | null> => {
      if (!bindingActive.current || threadBindings.current.get(threadId) !== bindingRef.current) return null;
      const requestId = snapshotRequestRef.current + 1;
      snapshotRequestRef.current = requestId;
      reloadInFlightRef.current = true;
      if (!quiet) setLoading(true);
      try {
        const next = await loadAgentSnapshotWithEvents(() => getAgentThread(threadId), pendingEventsRef.current, () => snapshotRequestRef.current === requestId);
        if (snapshotRequestRef.current !== requestId || !bindingActive.current
          || !bindingRef.current || !matchesDomainAgentThread(next.thread, bindingRef.current)) return null;
        commitSnapshot(next);
        setError(null);
        return next;
      } catch (err) {
        if (snapshotRequestRef.current === requestId) {
          setError(extractErrorMessage(err, t("agent.domain.loadFailed")));
        }
        return null;
      } finally {
        if (snapshotRequestRef.current === requestId) reloadInFlightRef.current = false;
        if (snapshotRequestRef.current === requestId && !quiet) {
          setLoading(false);
        }
      }
    },
    [commitSnapshot, t],
  );

  const ensureThread = useCallback(async () => {
    if (!binding) {
      snapshotRef.current = null;
      eventCursorRef.current = 0;
      reloadInFlightRef.current = false;
      setThread(null);
      setSnapshot(null);
      setLoading(false);
      setError(t("agent.domain.invalidContext"));
      return null;
    }
    setLoading(true);
    setError(null);
    setConnectionError(null);
    try {
      const match = await createAgentThread({
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
      if (!bindingActive.current || bindingRef.current !== binding) return null;
      threadBindings.current.set(match.id, binding);
      const loaded = await loadSnapshot(match.id);
      return loaded ? loaded.thread : null;
    } catch (err) {
      if (!bindingActive.current || bindingRef.current !== binding) return null;
      setError(extractErrorMessage(err, t("agent.domain.loadFailed")));
      setLoading(false);
      return null;
    }
  }, [binding, displayResourceLabel, loadSnapshot, t, title]);

  useEffect(() => {
    bindingActive.current = true;
    snapshotRequestRef.current += 1;
    snapshotRef.current = null;
    pendingEventsRef.current = [];
    pendingEditorRequests.current.clear();
    eventCursorRef.current = 0;
    reloadInFlightRef.current = false;
    setThread(null);
    setSnapshot(null);
    setStreamedMessageIds(new Set());
    setPendingSubmission(null);
    setDraft("");
    setConnectionError(null);
    setConnected(false);
    sendingRef.current = false;
    setSending(false);
    clearingRef.current = false;
    setClearing(false);
    setRefreshing(false);
    setControlBusy(null);
    setApprovalBusy(null);
    void ensureThread();
    return () => { bindingActive.current = false; snapshotRequestRef.current += 1; };
  }, [bindingKey, ensureThread]);

  const threadId = thread?.id;
  const {
    viewportRef: messageViewportRef,
    isFollowingLatest,
    hasNewContent,
    scrollToLatest,
  } = useFollowLatest(snapshot?.messages, {
    resetKey: threadId ?? bindingKey,
  });

  useEffect(() => {
    const acceptedMessageId = pendingSubmission?.acceptedMessageId;
    if (
      acceptedMessageId &&
      snapshot?.messages.some((message) => message.id === acceptedMessageId)
    ) {
      setPendingSubmission(null);
    }
  }, [pendingSubmission?.acceptedMessageId, snapshot]);

  useEffect(() => {
    if (!threadId) return undefined;
    const controller = new AbortController();
    pendingEventsRef.current = [];
    let stopped = false;
    const initial = snapshotRef.current;
    const refreshTimer = window.setInterval(() => {
      if (!clearingRef.current) void loadSnapshot(threadId, true);
    }, 7000);

    const connect = async () => {
      if (!initial) {
        const loaded = await loadSnapshot(threadId, true);
        if (!loaded || stopped) return;
      }
      while (!stopped) {
        try {
          setConnected(true);
          setConnectionError(null);
          await subscribeAgentThreadEvents(
            {
              thread_id: threadId,
              after_sequence: eventCursorRef.current,
              limit: 200,
            },
            (event) => {
              if (stopped || event.thread_id !== threadId || threadBindings.current.get(threadId) !== bindingRef.current) return;
              rememberStreamedMessages([event]);
              if (clearingRef.current) return;
              pendingEventsRef.current.push(event);
              if (reloadInFlightRef.current) return;
              const current = snapshotRef.current;
              if (!current || current.thread.id !== threadId) return;
              const reduction = reduceAgentEvent(
                current,
                event,
                eventCursorRef.current,
              );
              if (reduction.needsReload) {
                void loadSnapshot(threadId, true);
                return;
              }
              pendingEventsRef.current.splice(0);
              snapshotRef.current = reduction.snapshot;
              eventCursorRef.current = reduction.cursor;
              setSnapshot(reduction.snapshot);
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
  }, [loadSnapshot, rememberStreamedMessages, t, threadId]);

  useEffect(() => {
    if (reloadNonce === 0) return;
    void ensureThread();
  }, [ensureThread, reloadNonce]);

  const currentRun = useMemo(() => activeAgentRun(snapshot), [snapshot]);
  const contextRun =
    currentRun ??
    (snapshot && snapshot.runs.length > 0
      ? snapshot.runs[snapshot.runs.length - 1]
      : null);
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
  const pendingContentDiff = useMemo(
    () =>
      isOptimization && snapshot && positiveId(chapterId)
        ? pendingContentDiffForSnapshot(snapshot, chapterId)
        : null,
    [chapterId, isOptimization, snapshot],
  );

  useEffect(() => {
    if (!onPendingContentDiffChange) return undefined;
    onPendingContentDiffChange(pendingContentDiff);
    return () => onPendingContentDiffChange(null);
  }, [onPendingContentDiffChange, pendingContentDiff]);

  useEffect(() => {
    if (!onEditorRequest || !snapshot || pendingEditorRequests.current.size > 0)
      return;
    for (const event of snapshot.events) {
      const request = event.editor_request;
      if (
        !request ||
        !event.run_id ||
        !matchesEditorSourceScope(request.source, projectId, chapterId) ||
        getEditorSource?.()?.view_instance_id !== request.source.view_instance_id
      )
        continue;
      const execution = snapshot.executions.find(
        (item) => item.id === request.execution_id,
      );
      const run = snapshot.runs.find((item) => item.id === event.run_id);
      if (
        !execution ||
        !["prepared", "awaiting_user", "dispatching"].includes(
          execution.status,
        ) ||
        run?.status !== "waiting" ||
        run.waiting_reason !== "external" ||
        pendingEditorRequests.current.has(request.execution_id)
      )
        continue;
      pendingEditorRequests.current.add(request.execution_id);
      void onEditorRequest(request, event.run_id)
        .catch((error) => {
          addToast(
            extractErrorMessage(error, t("agent.domain.resourceRefreshFailed")),
            "error",
          );
        })
        .finally(() => {
          pendingEditorRequests.current.delete(request.execution_id);
          void loadSnapshot(snapshot.thread.id, true);
        });
    }
  }, [
    addToast,
    chapterId,
    getEditorSource,
    loadSnapshot,
    onEditorRequest,
    projectId,
    snapshot,
    t,
  ]);
  const latestEvent = useMemo<AgentEvent | null>(
    () => snapshot?.events[snapshot.events.length - 1] ?? null,
    [snapshot],
  );
  const canSend =
    Boolean(thread) &&
    Boolean(binding && thread && matchesDomainAgentThread(thread, binding)) &&
    (thread ? isAgentThreadOperational(thread) : false) &&
    draft.trim().length > 0 &&
    !sending &&
    !clearing;

  useEffect(() => {
    if (!snapshot || !binding || !matchesDomainAgentThread(snapshot.thread, binding)) return;
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
    binding,
    bindingKey,
    latestMutationExecutionId,
    onResourceChanged,
    snapshot,
    t,
  ]);

  const refresh = async () => {
    if (refreshing || clearingRef.current) return;
    setRefreshing(true);
    try {
      await ensureThread();
    } finally {
      if (isCurrentBinding(binding)) setRefreshing(false);
    }
  };

  const clearContext = async () => {
    if (
      !thread ||
      !isAgentThreadOperational(thread) ||
      clearingRef.current ||
      sendingRef.current
    )
      return;
    const confirmed = await confirmDialog({
      title: t("agent.clearContextTitle"),
      message: t("agent.clearContextMessage"),
      confirmText: t("agent.clearContextConfirm"),
      cancelText: t("common.cancel"),
      dialogType: "warning",
    });
    if (!confirmed || !isCurrentBinding(binding)) return;
    if (clearingRef.current) return;

    const threadId = thread.id;
    setClearing(true);
    clearingRef.current = true;
    snapshotRequestRef.current += 1;
    try {
      await clearAgentThreadContext(threadId, t("agent.clearContextTimeout"));
      if (!isCurrentBinding(binding)) return;
      setThread(null);
      setSnapshot(null);
      snapshotRef.current = null;
      eventCursorRef.current = 0;
      setStreamedMessageIds(new Set());
      setPendingSubmission(null);
      const loaded = await ensureThread();
      if (!isCurrentBinding(binding)) return;
      if (!loaded) {
        throw new Error(t("agent.clearContextFailed"));
      }
      setDraft("");
      addToast(t("agent.clearContextSuccess"), "success");
    } catch (err) {
      const message = extractErrorMessage(err, t("agent.clearContextFailed"));
      if (isCurrentBinding(binding)) setError(message);
      addToast(message, "error");
    } finally {
      if (isCurrentBinding(binding)) { clearingRef.current = false; setClearing(false); }
    }
  };

  const send = async () => {
    const content = draft.trim();
    if (
      !thread ||
      !content ||
      !canSend ||
      sendingRef.current ||
      clearingRef.current
    )
      return;
    const currentBinding = bindingRef.current;
    if (!currentBinding || !matchesDomainAgentThread(thread, currentBinding)) return;
    const isCurrent = () => bindingActive.current && bindingRef.current === currentBinding;
    const clientMessageId = randomClientMessageId();
    const pendingMessage = createPendingAgentMessage(
      thread.id,
      content,
      clientMessageId,
    );
    setPendingSubmission({ message: pendingMessage, acceptedMessageId: null });
    setDraft("");
    sendingRef.current = true;
    setSending(true);
    try {
      const accepted = await sendAgentMessage({
        thread_id: thread.id,
        client_message_id: clientMessageId,
        content,
        delivery_mode: deliveryMode,
        execution_mode: executionMode,
        editor_source: getEditorSource?.() ?? undefined,
      });
      if (clearingRef.current || !isCurrent()) return;
      setPendingSubmission((current) =>
        current?.message.id === pendingMessage.id
          ? { ...current, acceptedMessageId: accepted.message_id }
          : current,
      );
      const next = await loadSnapshot(thread.id, true);
      if (
        next?.messages.some((message) => message.id === accepted.message_id)
      ) {
        setPendingSubmission((current) =>
          current?.message.id === pendingMessage.id ? null : current,
        );
      }
      if (next && !clearingRef.current)
        rememberStreamedMessages(next.events, accepted.run_id);
    } catch (err) {
      const message = extractErrorMessage(err, t("agent.domain.sendFailed"));
      if (isCurrent()) {
        setPendingSubmission((current) => current?.message.id === pendingMessage.id ? null : current);
        setDraft((current) => current || content);
        setError(message);
      }
      addToast(message, "error");
    } finally {
      if (isCurrent()) { sendingRef.current = false; setSending(false); }
    }
  };

  const control = async (action: AgentControlAction) => {
    if (!thread || !currentRun || controlBusy || clearing || !isCurrentBinding(binding)) return;
    if (action === "resume" && (currentRun.waiting_reason === "budget" || currentRun.budget_limiting_run_id)) {
      setBudgetRun(currentRun);
      return;
    }
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
      if (isCurrentBinding(binding)) setError(message);
      addToast(message, "error");
    } finally {
      if (isCurrentBinding(binding)) setControlBusy(null);
    }
  };

  const resolveApproval = async (
    approval: AgentApprovalView,
    decision: "approve" | "reject",
  ) => {
    if (approvalBusy || !isCurrentBinding(binding)) return;
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
      if (isCurrentBinding(binding)) setError(message);
      addToast(message, "error");
    } finally {
      if (isCurrentBinding(binding)) setApprovalBusy(null);
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
      {budgetRun ? <AgentBudgetDialog run={budgetRun} onClose={() => setBudgetRun(null)} onResumed={() => loadSnapshot(budgetRun.thread_id, true)} /> : null}
      <header className="agent-console-header flex min-w-0 shrink-0 flex-wrap items-start gap-2.5 border-b border-book-border/50 bg-book-bg-paper/65 px-3 py-2.5">
        <span className="agent-console-icon inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-xl bg-book-primary/10 text-book-primary">
          {icon}
        </span>
        <div className="agent-console-heading min-w-0 flex-1 basis-40">
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
        </div>
        <div className="agent-console-actions ml-auto flex shrink-0 items-center gap-1">
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
          {executions.length > 0 || latestEvent ? (
            <BookButton
              size="sm"
              variant={activityOpen ? "secondary" : "ghost"}
              aria-pressed={activityOpen}
              aria-label={t("agent.domain.activity")}
              title={t("agent.domain.activity")}
              onClick={() => setActivityOpen((open) => !open)}
            >
              <CircleDot size={12} />
              <span>{executions.length}</span>
            </BookButton>
          ) : null}
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
          <BookIconButton
            label={t("agent.clearContext")}
            variant="ghost"
            size="sm"
            className="text-error hover:bg-error-bg hover:text-error"
            onClick={() => void clearContext()}
            disabled={
              !thread ||
              clearing ||
              sending ||
              !isAgentThreadOperational(thread)
            }
          >
            {clearing ? <LoadingSpinner size="sm" /> : <Trash2 size={13} />}
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

      {activityOpen && (executions.length > 0 || latestEvent) ? (
        <div className="agent-console-activity-panel">
          <div className="agent-console-activity-panel__heading">
            <span>{t("agent.domain.activity")}</span>
            {latestEvent ? (
              <span className="truncate">
                {latestEvent.kind.replace(/_/g, " ")}
              </span>
            ) : null}
          </div>
          <div className="agent-console-activity-panel__list custom-scrollbar">
            {executions.map((execution) => (
              <ExecutionRow
                key={execution.id}
                execution={execution}
                locale={locale}
              />
            ))}
          </div>
        </div>
      ) : null}

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
          <div className="relative min-h-0 min-w-0 flex-1">
            <div
              ref={messageViewportRef}
              className="agent-console-message-scroll custom-scrollbar h-full min-h-0 min-w-0 overflow-y-auto overflow-x-hidden px-2.5 py-2.5"
              aria-live="polite"
              style={{ overflowAnchor: "none" }}
            >
              {snapshot.messages.length === 0 &&
              !pendingSubmission &&
              !currentRun ? (
                error ? (
                  <div className="agent-console-error-state" role="status">
                    <CircleAlert size={16} aria-hidden="true" />
                    <span>{t("agent.domain.loadFailed")}</span>
                  </div>
                ) : (
                  <div className="a4n-empty-state a4n-empty-state--compact min-h-0 py-8 text-xs">
                    <Bot size={20} />
                    <p>{t("agent.domain.empty")}</p>
                  </div>
                )
              ) : null}
              {snapshot.messages.length > 0 ||
              pendingSubmission ||
              currentRun ? (
                <AgentMessageList
                  messages={snapshot.messages}
                  locale={locale}
                  streamedMessageIds={streamedMessageIds}
                  pendingMessages={
                    pendingSubmission ? [pendingSubmission.message] : []
                  }
                  activeRun={contextRun}
                  compactAssistantMessages
                />
              ) : null}
            </div>
            <FollowLatestButton
              isFollowingLatest={isFollowingLatest}
              hasNewContent={hasNewContent}
              newContentLabel={t("agent.domain.newMessages")}
              latestLabel={t("agent.domain.backToLatest")}
              onClick={scrollToLatest}
              className="absolute bottom-3 right-3 z-10"
            />
          </div>

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
            <div className="agent-console-control-row mb-2 flex min-w-0 flex-wrap items-center gap-1.5">
              <label className="min-w-0 flex-1 sm:flex-none">
                <span className="sr-only">
                  {t("agent.domain.deliveryMode")}
                </span>
                <ThemedSelect
                  value={deliveryMode}
                  onChange={(selectedValue) =>
                    setDeliveryMode(selectedValue as AgentDeliveryMode)
                  }
                  className="pyqt-input h-8 min-h-8 w-full !py-1 text-[10px] sm:w-auto"
                  aria-label={t("agent.domain.deliveryMode")}
                  disabled={
                    !thread ||
                    !isAgentThreadOperational(thread) ||
                    sending ||
                    clearing
                  }
                >
                  <option value="followup">{t("agent.domain.followup")}</option>
                  <option value="steer">{t("agent.domain.steer")}</option>
                  <option value="inject">{t("agent.domain.inject")}</option>
                </ThemedSelect>
              </label>
              <label className="min-w-0 flex-1 sm:flex-none">
                <span className="sr-only">
                  {t("agent.domain.executionMode")}
                </span>
                <ThemedSelect
                  value={executionMode}
                  onChange={(selectedValue) =>
                    setExecutionMode(selectedValue as AgentExecutionMode)
                  }
                  className="pyqt-input h-8 min-h-8 w-full !py-1 text-[10px] sm:w-auto"
                  aria-label={t("agent.domain.executionMode")}
                  disabled={
                    !thread ||
                    !isAgentThreadOperational(thread) ||
                    sending ||
                    clearing
                  }
                >
                  <option value="autonomous">
                    {t("agent.domain.autonomous")}
                  </option>
                  <option value="review_writes">
                    {t("agent.domain.reviewWrites")}
                  </option>
                  <option value="advice_only">
                    {t("agent.domain.adviceOnly")}
                  </option>
                </ThemedSelect>
              </label>
              <AgentContextMeter run={contextRun} locale={locale} />
              {currentRun && (currentRun.waiting_reason === "budget" || currentRun.budget_limiting_run_id) ? (
                <BookButton size="sm" variant="warning" onClick={() => setBudgetRun(currentRun)}>{t("agent.budget.title")}</BookButton>
              ) : null}
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
                disabled={
                  !thread ||
                  !isAgentThreadOperational(thread) ||
                  sending ||
                  clearing
                }
              />
              {currentRun ? (
                <div className="agent-console-run-actions">
                  {["queued", "running", "waiting"].includes(
                    currentRun.status,
                  ) ? (
                    <BookButton
                      size="sm"
                      variant="warning"
                      className="agent-console-run-action"
                      aria-label={t("agent.domain.pause")}
                      title={t("agent.domain.pause")}
                      onClick={() => void control("pause")}
                      disabled={controlBusy != null || clearing}
                    >
                      <Pause size={13} />
                    </BookButton>
                  ) : null}
                  {["paused", "pausing"].includes(currentRun.status) ? (
                    <BookButton
                      size="sm"
                      variant="secondary"
                      className="agent-console-run-action"
                      aria-label={t("agent.domain.resume")}
                      title={t("agent.domain.resume")}
                      onClick={() => void control("resume")}
                      disabled={controlBusy != null || clearing}
                    >
                      <Play size={13} />
                    </BookButton>
                  ) : null}
                  {controlBusy ? (
                    <LoadingSpinner
                      size="sm"
                      className="text-book-text-muted"
                    />
                  ) : null}
                </div>
              ) : null}
              {currentRun &&
              ACTIVE_AGENT_RUN_STATUSES.has(currentRun.status) ? (
                <BookButton
                  size="md"
                  variant="danger"
                  className="agent-console-stop-action"
                  aria-label={t("agent.domain.cancel")}
                  title={t("agent.domain.cancel")}
                  onClick={() => void control("cancel")}
                  disabled={
                    controlBusy != null ||
                    clearing ||
                    currentRun.status === "cancelling"
                  }
                >
                  {controlBusy === "cancel" ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Square size={13} />
                  )}
                </BookButton>
              ) : (
                <BookIconButton
                  label={t("agent.domain.send")}
                  variant="secondary"
                  size="md"
                  onClick={() => void send()}
                  disabled={!canSend}
                >
                  {sending ? <LoadingSpinner size="sm" /> : <Send size={14} />}
                </BookIconButton>
              )}
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
