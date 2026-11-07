import { ThemedSelect } from "../ui/ThemedSelect";
import React, {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
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
  acceptPendingAgentMessage,
  appendPendingAgentMessage,
  clearPendingAgentMessages,
  createPendingAgentMessage,
  reconcilePendingAgentMessages,
  removePendingAgentMessage,
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
  listAgentThreads,
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
import { useAuthStore } from "../../store/auth";
import {
  beginDomainAgentOperation,
  domainAgentComposerOwnerKey,
  finishDomainAgentOperation,
  getDomainAgentDrafts,
  getDomainAgentPendingMessages,
  getDomainAgentOperations,
  getDomainAgentOwnerVersion,
  hasDomainAgentOperation,
  pendingDomainAgentClear,
  subscribeDomainAgentDrafts,
  updateDomainAgentDraftsForOwner as updateDomainAgentDrafts,
  updateDomainAgentPendingMessages,
} from "./domainAgentDrafts";
import {
  reduceAgentEvent,
  snapshotCursor,
  loadAgentSnapshotWithEvents,
  streamedAgentMessageIds,
} from "../../utils/agentEventReducer";
import { startAgentSnapshotRefreshPolling } from "../../utils/agentSnapshotPolling";
import {
  ApprovalRow,
  ExecutionRow,
  Notice,
} from "./DomainAgentConsoleRows";
import { agentStatusLabel } from "../../utils/agentStatusPresentation";
import {
  domainAgentBinding,
  domainAgentBindingKey,
  findDomainAgentThread,
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
  composerDirtyKey?: string;
  executionMode?: AgentExecutionMode;
  onExecutionModeChange?: (mode: AgentExecutionMode) => void;
  onClose?: () => void;
  onResourceChanged?: () => boolean | void | Promise<boolean | void>;
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
  composerDirtyKey,
  executionMode: controlledExecutionMode,
  onExecutionModeChange,
  onClose,
  onResourceChanged,
  onPendingContentDiffChange,
  getEditorSource,
  onEditorRequest,
}) => {
  const { locale, t } = useI18n();
  const authInitialized = useAuthStore((state) => state.initialized);
  const { addToast } = useToast();
  const inputId = useId();
  const [thread, setThread] = useState<AgentThreadView | null>(null);
  const [snapshot, setSnapshot] = useState<AgentThreadSnapshot | null>(null);
  const [streamedMessageIds, setStreamedMessageIds] = useState<Set<string>>(
    () => new Set(),
  );
  const [loading, setLoading] = useState(true);
  const pendingEditorRequests = useRef(new Set<string>());
  const [deliveryMode, setDeliveryMode] =
    useState<AgentDeliveryMode>("followup");
  const [localExecutionMode, setLocalExecutionMode] = useState<AgentExecutionMode>(() =>
    capability === "content_optimization" ? "review_writes" : "autonomous",
  );
  const executionMode = controlledExecutionMode ?? localExecutionMode;
  const setExecutionMode = onExecutionModeChange ?? setLocalExecutionMode;
  const [budgetRun, setBudgetRun] = useState<AgentRunView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const [reloadNonce, setReloadNonce] = useState(0);
  const [activityOpen, setActivityOpen] = useState(false);
  const mountedRef = useRef(true);
  const snapshotRequestRef = useRef(0);
  const snapshotRef = useRef<AgentThreadSnapshot | null>(null);
  const eventCursorRef = useRef(0);
  const reloadInFlightRef = useRef(false);
  const pendingEventsRef = useRef<AgentEvent[]>([]);
  const streamActivityAtRef = useRef(Date.now());
  const snapshotReadErrorRef = useRef<string | null>(null);
  const snapshotReadUnavailableRef = useRef(false);
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
  const activeBindingKeyRef = useRef(bindingKey);
  activeBindingKeyRef.current = bindingKey;
  const isCurrentBinding = useCallback(
    (requestBindingKey: string) => mountedRef.current && activeBindingKeyRef.current === requestBindingKey,
    [],
  );
  const resolvedComposerDirtyKey = domainAgentComposerOwnerKey(
    composerDirtyKey ?? `domain-agent-composer:${inputId}`,
  );
  const subscribeToComposer = useCallback(
    (listener: () => void) => subscribeDomainAgentDrafts(resolvedComposerDirtyKey, listener),
    [resolvedComposerDirtyKey],
  );
  const getComposerRevision = useCallback(
    () => getDomainAgentOwnerVersion(resolvedComposerDirtyKey),
    [resolvedComposerDirtyKey],
  );
  const draftRevision = useSyncExternalStore(
    subscribeToComposer,
    getComposerRevision,
    getComposerRevision,
  );
  const drafts = useMemo(
    () => authInitialized
      ? getDomainAgentDrafts(resolvedComposerDirtyKey)
      : {},
    [authInitialized, draftRevision, resolvedComposerDirtyKey],
  );
  const operations = useMemo(
    () => authInitialized
      ? getDomainAgentOperations(resolvedComposerDirtyKey)
      : [],
    [authInitialized, draftRevision, resolvedComposerDirtyKey],
  );
  const operationsForBinding = operations.filter((operation) => operation.bindingKey === bindingKey);
  const updateDrafts = useCallback((update: React.SetStateAction<Record<string, string>>) => {
    updateDomainAgentDrafts(resolvedComposerDirtyKey, update);
  }, [resolvedComposerDirtyKey]);
  const draft = drafts[bindingKey] ?? "";
  const refreshing = operationsForBinding.some((operation) => operation.kind === "refresh");
  const clearing = operationsForBinding.some((operation) => operation.kind === "clear");
  const sending = operationsForBinding.some((operation) => operation.kind === "send");
  const controlBusy = (operationsForBinding.find((operation) => operation.kind === "control")?.value as AgentControlAction | undefined) ?? null;
  const approvalBusy = operationsForBinding.find((operation) => operation.kind === "approval")?.value ?? null;
  const setDraft = (value: string) => {
    updateDrafts((current) => current[bindingKey] === value
      ? current
      : { ...current, [bindingKey]: value });
  };
  const hasComposerDraft = Object.values(drafts).some((value) => value.trim().length > 0);
  const operationInFlight =
    operations.length > 0;
  const requestClose = async () => {
    if (!onClose) return;
    if (operationInFlight) {
      addToast(t("common.operationInProgress"), "info");
      return;
    }
    if (hasComposerDraft && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: t("agent.domain.consoleLabel"),
        action: t("common.close"),
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    if (hasComposerDraft) updateDrafts({});
    onClose();
  };
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
    ? null
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
      const requestBindingKey = bindingKey;
      if (!binding || !isCurrentBinding(requestBindingKey)) return null;
      const requestId = snapshotRequestRef.current + 1;
      snapshotRequestRef.current = requestId;
      reloadInFlightRef.current = true;
      if (!quiet) setLoading(true);
      const isCurrent = () =>
        isCurrentBinding(requestBindingKey) && snapshotRequestRef.current === requestId;
      try {
        const next = await loadAgentSnapshotWithEvents(() => getAgentThread(threadId), pendingEventsRef.current, isCurrent);
        if (!isCurrent()) return null;
        if (!matchesDomainAgentThread(next.thread, binding)) {
          throw new Error(t("agent.domain.invalidContext"));
        }
        commitSnapshot(next);
        setError(null);
        return next;
      } catch (err) {
        if (isCurrent()) {
          setError(extractErrorMessage(err, t("agent.domain.loadFailed")));
        }
        return null;
      } finally {
        if (isCurrent()) reloadInFlightRef.current = false;
        if (isCurrent() && !quiet) {
          setLoading(false);
        }
      }
    },
    [binding, bindingKey, commitSnapshot, isCurrentBinding, t],
  );

  const ensureThread = useCallback(async (allowWhileClearing = false) => {
    const requestBindingKey = bindingKey;
    if (!isCurrentBinding(requestBindingKey)) return null;
    const initialClear = allowWhileClearing
      ? null
      : pendingDomainAgentClear(resolvedComposerDirtyKey, requestBindingKey);
    if (initialClear) {
      setLoading(true);
      await initialClear;
      if (!isCurrentBinding(requestBindingKey)) return null;
    }
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
      let threads = await listAgentThreads({
        capability: binding.capability,
        include_archived: false,
        limit: 200,
      });
      if (!isCurrentBinding(requestBindingKey)) return null;
      const clearAfterRead = allowWhileClearing
        ? null
        : pendingDomainAgentClear(resolvedComposerDirtyKey, requestBindingKey);
      if (clearAfterRead) {
        await clearAfterRead;
        if (!isCurrentBinding(requestBindingKey)) return null;
        threads = await listAgentThreads({
          capability: binding.capability,
          include_archived: false,
          limit: 200,
        });
        if (!isCurrentBinding(requestBindingKey)) return null;
      }
      let match = findDomainAgentThread(threads, binding);
      if (!match) {
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
      }
      if (!isCurrentBinding(requestBindingKey) ||
        (!allowWhileClearing && hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear"))) return null;
      const loaded = await loadSnapshot(match.id);
      if (!isCurrentBinding(requestBindingKey)) return null;
      return loaded ? loaded.thread : null;
    } catch (err) {
      if (isCurrentBinding(requestBindingKey)) {
        setError(extractErrorMessage(err, t("agent.domain.loadFailed")));
        setLoading(false);
      }
      return null;
    }
  }, [binding, bindingKey, displayResourceLabel, isCurrentBinding, loadSnapshot, resolvedComposerDirtyKey, t, title]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      snapshotRequestRef.current += 1;
    };
  }, []);

  useLayoutEffect(() => {
    snapshotRequestRef.current += 1;
    snapshotRef.current = null;
    eventCursorRef.current = 0;
    reloadInFlightRef.current = false;
    pendingEventsRef.current = [];
    setThread(null);
    setSnapshot(null);
    setStreamedMessageIds(new Set());
    setBudgetRun(null);
    setActivityOpen(false);
    setLoading(binding != null);
    setError(null);
    setConnectionError(null);
    setConnected(false);
  }, [binding, bindingKey]);

  useEffect(() => {
    void ensureThread();
  }, [bindingKey, ensureThread]);

  const threadId = thread?.id;
  const snapshotReadUnavailable = Boolean(
    threadId && (
      snapshot == null ||
      snapshot.thread.id !== threadId ||
      error != null
    ),
  );
  snapshotReadErrorRef.current = error;
  snapshotReadUnavailableRef.current = snapshotReadUnavailable;
  const ensureSnapshotCurrent = useCallback(() => {
    if (!snapshotReadUnavailableRef.current) return true;
    addToast(snapshotReadErrorRef.current ?? t("agent.domain.loadFailed"), "error");
    return false;
  }, [addToast, t]);
  const pendingSubmissions = getDomainAgentPendingMessages(resolvedComposerDirtyKey, bindingKey);
  const {
    viewportRef: messageViewportRef,
    isFollowingLatest,
    hasNewContent,
    scrollToLatest,
  } = useFollowLatest(snapshot?.messages, {
    resetKey: threadId ?? bindingKey,
  });

  useEffect(() => {
    if (!threadId) return undefined;
    const requestBindingKey = bindingKey;
    return startAgentSnapshotRefreshPolling(threadId, () => connected, () => streamActivityAtRef.current, (pollThreadId) => {
      if (
        isCurrentBinding(requestBindingKey) &&
        !hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear")
      ) {
        void loadSnapshot(pollThreadId, true);
      }
    });
  }, [bindingKey, connected, isCurrentBinding, loadSnapshot, resolvedComposerDirtyKey, threadId]);

  useEffect(() => {
    if (!snapshot || pendingSubmissions.length === 0) return;
    updateDomainAgentPendingMessages(resolvedComposerDirtyKey, (current) =>
      reconcilePendingAgentMessages(current, bindingKey, snapshot.messages));
  }, [bindingKey, pendingSubmissions, snapshot]);

  useEffect(() => {
    if (!threadId) return undefined;
    const requestBindingKey = bindingKey;
    const controller = new AbortController();
    pendingEventsRef.current = [];
    let stopped = false;
    const initial = snapshotRef.current;

    const connect = async () => {
      if (!isCurrentBinding(requestBindingKey)) return;
      if (!initial) {
        const loaded = await loadSnapshot(threadId, true);
        if (!loaded || stopped || !isCurrentBinding(requestBindingKey)) return;
      }
      while (!stopped) {
        try {
          if (!isCurrentBinding(requestBindingKey)) return;
          setConnectionError(null);
          await subscribeAgentThreadEvents(
            {
              thread_id: threadId,
              after_sequence: eventCursorRef.current,
              limit: 200,
            },
            (event) => {
              if (
                stopped || event.thread_id !== threadId ||
                !isCurrentBinding(requestBindingKey)
              ) return;
              rememberStreamedMessages([event]);
              if (hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear")) return;
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
            () => {
              if (!stopped && isCurrentBinding(requestBindingKey)) {
                streamActivityAtRef.current = Date.now();
                setConnected(true);
              }
            },
          );
        } catch (err) {
          if (controller.signal.aborted) return;
          if (!isCurrentBinding(requestBindingKey)) return;
          setConnected(false);
          setConnectionError(
            extractErrorMessage(err, t("agent.domain.connectionLost")),
          );
        }
        if (!stopped && isCurrentBinding(requestBindingKey)) {
          setConnected(false);
          await new Promise((resolve) => window.setTimeout(resolve, 1500));
        }
      }
    };
    void connect();
    return () => {
      stopped = true;
      controller.abort();
      setConnected(false);
    };
  }, [bindingKey, isCurrentBinding, loadSnapshot, rememberStreamedMessages, resolvedComposerDirtyKey, t, threadId]);

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
    const requestBindingKey = bindingKey;
    const pendingPrefix = `${requestBindingKey}:`;
    if (
      !onEditorRequest ||
      !snapshot ||
      !isCurrentBinding(requestBindingKey) ||
      [...pendingEditorRequests.current].some((key) => key.startsWith(pendingPrefix))
    )
      return;
    for (const event of snapshot.events) {
      const request = event.editor_request;
      if (
        !request ||
        !event.run_id ||
        !matchesEditorSourceScope(request.source, projectId, chapterId) ||
        request.source.view_instance_id !== getEditorSource?.()?.view_instance_id
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
        pendingEditorRequests.current.has(`${requestBindingKey}:${request.execution_id}`)
      )
        continue;
      const requestKey = `${requestBindingKey}:${request.execution_id}`;
      pendingEditorRequests.current.add(requestKey);
      void onEditorRequest(request, event.run_id)
        .catch((error) => {
          if (isCurrentBinding(requestBindingKey)) {
            addToast(
              extractErrorMessage(error, t("agent.domain.resourceRefreshFailed")),
              "error",
            );
          }
        })
        .finally(() => {
          pendingEditorRequests.current.delete(requestKey);
          if (isCurrentBinding(requestBindingKey)) void loadSnapshot(snapshot.thread.id, true);
        });
    }
  }, [
    addToast,
    bindingKey,
    chapterId,
    getEditorSource,
    isCurrentBinding,
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
    !refreshing &&
    !sending &&
    controlBusy == null &&
    approvalBusy == null &&
    !snapshotReadUnavailable &&
    !clearing;

  useEffect(() => {
    const requestBindingKey = bindingKey;
    if (
      !snapshot ||
      !binding ||
      !isCurrentBinding(requestBindingKey) ||
      !matchesDomainAgentThread(snapshot.thread, binding)
    ) return;
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
    void Promise.resolve(onResourceChanged())
      .then((refreshed) => {
        if (!isCurrentBinding(requestBindingKey) || refreshed !== false) return;
        addToast(t("agent.domain.resourceRefreshFailed"), "error");
      })
      .catch((err) => {
        if (!isCurrentBinding(requestBindingKey)) return;
        addToast(
          extractErrorMessage(err, t("agent.domain.resourceRefreshFailed")),
          "error",
        );
      });
  }, [
    addToast,
    binding,
    bindingKey,
    isCurrentBinding,
    latestMutationExecutionId,
    onResourceChanged,
    snapshot,
    t,
  ]);

  const refresh = async () => {
    const requestBindingKey = bindingKey;
    if (
      hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "refresh") ||
      hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear")
    ) return;
    const operation = beginDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "refresh");
    if (!operation) return;
    try {
      if (isCurrentBinding(requestBindingKey)) await ensureThread();
    } finally {
      finishDomainAgentOperation(resolvedComposerDirtyKey, operation);
    }
  };

  const clearContext = async () => {
    const requestBindingKey = bindingKey;
    const requestedThread = thread;
    if (
      !requestedThread ||
      !ensureSnapshotCurrent() ||
      !isAgentThreadOperational(requestedThread) ||
      getDomainAgentOperations(resolvedComposerDirtyKey).some((operation) => operation.bindingKey === requestBindingKey)
    )
      return;
    const confirmed = await confirmDialog({
      title: t("agent.clearContextTitle"),
      message: t("agent.clearContextMessage"),
      confirmText: t("agent.clearContextConfirm"),
      cancelText: t("common.cancel"),
      dialogType: "warning",
    });
    if (!confirmed || !isCurrentBinding(requestBindingKey) || !ensureSnapshotCurrent()) return;
    if (getDomainAgentOperations(resolvedComposerDirtyKey).some((operation) => operation.bindingKey === requestBindingKey)) return;

    const threadId = requestedThread.id;
    const operation = beginDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear");
    if (!operation) return;
    snapshotRequestRef.current += 1;
    try {
      await clearAgentThreadContext(threadId, t("agent.clearContextTimeout"));
      updateDrafts((current) => {
        if (!current[requestBindingKey]) return current;
        const next = { ...current };
        delete next[requestBindingKey];
        return next;
      });
      if (!isCurrentBinding(requestBindingKey)) return;
      setThread(null);
      setSnapshot(null);
      snapshotRef.current = null;
      eventCursorRef.current = 0;
      setStreamedMessageIds(new Set());
      updateDomainAgentPendingMessages(resolvedComposerDirtyKey, (current) =>
        clearPendingAgentMessages(current, requestBindingKey));
      addToast(t("agent.clearContextSuccess"), "success");
      void ensureThread(true).then((threadLoaded) => {
        if (isCurrentBinding(requestBindingKey) && !threadLoaded) {
          addToast(t("agent.domain.loadFailed"), "error");
        }
      });
    } catch (err) {
      if (!isCurrentBinding(requestBindingKey)) return;
      const message = extractErrorMessage(err, t("agent.clearContextFailed"));
      setError(message);
      addToast(message, "error");
    } finally {
      finishDomainAgentOperation(resolvedComposerDirtyKey, operation);
    }
  };

  const send = async () => {
    const requestBindingKey = bindingKey;
    const requestThread = thread;
    const content = draft.trim();
    if (
      !requestThread ||
      !content ||
      !canSend ||
      !ensureSnapshotCurrent() ||
      hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "send") ||
      hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear")
    )
      return;
    const operation = beginDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "send");
    if (!operation) return;
    const clientMessageId = randomClientMessageId();
    const pendingMessage = createPendingAgentMessage(
      requestThread.id,
      content,
      clientMessageId,
    );
    updateDomainAgentPendingMessages(resolvedComposerDirtyKey, (current) =>
      appendPendingAgentMessage(current, requestBindingKey, { message: pendingMessage, acceptedMessageId: null }));
    updateDrafts((current) => ({ ...current, [requestBindingKey]: "" }));
    try {
      const accepted = await sendAgentMessage({
        thread_id: requestThread.id,
        client_message_id: clientMessageId,
        content,
        delivery_mode: deliveryMode,
        execution_mode: executionMode,
        editor_source: getEditorSource?.() ?? undefined,
      });
      updateDomainAgentPendingMessages(resolvedComposerDirtyKey, (current) =>
        acceptPendingAgentMessage(current, requestBindingKey, pendingMessage.id, accepted.message_id));
      if (!isCurrentBinding(requestBindingKey) || hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear")) return;
      void loadSnapshot(requestThread.id, true).then((next) => {
        if (next && isCurrentBinding(requestBindingKey) &&
          !hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear")) {
          rememberStreamedMessages(next.events, accepted.run_id);
        }
      });
    } catch (err) {
      updateDomainAgentPendingMessages(resolvedComposerDirtyKey, (current) =>
        removePendingAgentMessage(current, requestBindingKey, pendingMessage.id));
      updateDrafts((current) => ({ ...current, [requestBindingKey]: current[requestBindingKey] || content }));
      if (isCurrentBinding(requestBindingKey)) {
        const message = extractErrorMessage(err, t("agent.domain.sendFailed"));
        setError(message);
        addToast(message, "error");
      }
    } finally {
      finishDomainAgentOperation(resolvedComposerDirtyKey, operation);
    }
  };

  const control = async (action: AgentControlAction) => {
    const requestBindingKey = bindingKey;
    const requestThread = thread;
    const requestRun = currentRun;
    if (
      !requestThread ||
      !requestRun ||
      !ensureSnapshotCurrent() ||
      hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "control") ||
      hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear")
    ) return;
    if (action === "resume" && (requestRun.waiting_reason === "budget" || requestRun.budget_limiting_run_id)) {
      setBudgetRun(requestRun);
      return;
    }
    const operation = beginDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "control", action);
    if (!operation) return;
    try {
      const updated = await controlAgentRun({
        thread_id: requestThread.id,
        run_id: requestRun.id,
        action,
      });
      if (!isCurrentBinding(requestBindingKey)) return;
      const current = snapshotRef.current;
      if (current?.thread.id === requestThread.id && current.runs.some((run) => run.id === updated.id)) {
        const next = {
          ...current,
          runs: current.runs.map((run) => run.id === updated.id ? updated : run),
        };
        snapshotRef.current = next;
        setSnapshot(next);
      }
      void loadSnapshot(requestThread.id, true);
    } catch (err) {
      if (isCurrentBinding(requestBindingKey)) {
        const message = extractErrorMessage(err, t("agent.domain.controlFailed"));
        setError(message);
        addToast(message, "error");
      }
    } finally {
      finishDomainAgentOperation(resolvedComposerDirtyKey, operation);
    }
  };

  const resolveApproval = async (
    approval: AgentApprovalView,
    decision: "approve" | "reject",
  ) => {
    const requestBindingKey = bindingKey;
    const requestThread = thread;
    if (
      !requestThread ||
      !ensureSnapshotCurrent() ||
      hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "clear") ||
      hasDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "approval")
    ) return;
    const operation = beginDomainAgentOperation(resolvedComposerDirtyKey, requestBindingKey, "approval", approval.id);
    if (!operation) return;
    try {
      const updated = await resolveAgentApproval({
        approval_id: approval.id,
        decision,
        reason:
          decision === "approve"
            ? t("agent.domain.approvedByUser")
            : t("agent.domain.rejectedByUser"),
      });
      if (!isCurrentBinding(requestBindingKey)) return;
      const current = snapshotRef.current;
      if (current?.thread.id === requestThread.id && current.approvals.some((item) => item.id === updated.id)) {
        const next = {
          ...current,
          approvals: current.approvals.map((item) => item.id === updated.id ? updated : item),
        };
        snapshotRef.current = next;
        setSnapshot(next);
      }
      void loadSnapshot(requestThread.id, true);
    } catch (err) {
      if (isCurrentBinding(requestBindingKey)) {
        const message = extractErrorMessage(
          err,
          t("agent.domain.approvalFailed"),
        );
        setError(message);
        addToast(message, "error");
      }
    } finally {
      finishDomainAgentOperation(resolvedComposerDirtyKey, operation);
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
      {budgetRun ? <AgentBudgetDialog run={budgetRun} onClose={() => setBudgetRun(null)} onResumed={() => {
        void loadSnapshot(budgetRun.thread_id, true);
      }} /> : null}
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
                {agentStatusLabel(thread.status, locale)}
              </span>
            ) : null}
          </div>
          {subtitle ? (
            <p className="agent-console-subtitle mt-0.5 min-w-0 truncate text-caption text-book-text-muted">
              {subtitle}
            </p>
          ) : null}
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
            disabled={loading || refreshing || operationsForBinding.length > 0}
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
              operationsForBinding.length > 0 ||
              !isAgentThreadOperational(thread)
            }
          >
            {clearing ? <LoadingSpinner size="sm" /> : <Trash2 size={13} />}
          </BookIconButton>
          {onClose ? (
            <BookIconButton
              label={t("common.close")}
              size="sm"
              onClick={() => void requestClose()}
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

      {loading && !snapshot && pendingSubmissions.length === 0 ? (
        <div className="flex min-h-0 flex-1 items-center justify-center text-xs text-book-text-muted">
          <LoadingSpinner size="sm" className="mr-2" />
          {t("agent.domain.loading")}
        </div>
      ) : snapshot || pendingSubmissions.length > 0 ? (
        <>
          <div className="relative min-h-0 min-w-0 flex-1">
            <div
              ref={messageViewportRef}
              className="agent-console-message-scroll custom-scrollbar h-full min-h-0 min-w-0 overflow-y-auto overflow-x-hidden px-2.5 py-2.5"
              aria-live="polite"
              style={{ overflowAnchor: "none" }}
            >
              {(snapshot?.messages.length ?? 0) === 0 &&
              pendingSubmissions.length === 0 &&
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
              {((snapshot?.messages.length ?? 0) > 0 ||
                pendingSubmissions.length > 0 ||
                currentRun != null) ? (
                <AgentMessageList
                  messages={snapshot?.messages ?? []}
                  locale={locale}
                  streamedMessageIds={streamedMessageIds}
                  pendingMessages={
                    pendingSubmissions.map((item) => item.message)
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
                  busy={operationsForBinding.length > 0}
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
                    refreshing ||
                    sending ||
                    controlBusy != null ||
                    approvalBusy != null ||
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
                    refreshing ||
                    sending ||
                    controlBusy != null ||
                    approvalBusy != null ||
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
                  refreshing ||
                  controlBusy != null ||
                  approvalBusy != null ||
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
                      disabled={operationsForBinding.length > 0}
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
                      disabled={operationsForBinding.length > 0}
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
                    operationsForBinding.length > 0 ||
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
