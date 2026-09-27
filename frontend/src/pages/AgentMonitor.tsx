import { ThemedSelect } from "../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Activity,
  Bot,
  Check,
  ChevronRight,
  CircleDot,
  FolderTree,
  Inbox,
  MessageSquare,
  Pause,
  Play,
  Plus,
  RefreshCw,
  Send,
  ShieldCheck,
  Square,
  Trash2,
  Users,
  X,
} from "lucide-react";
import { PageHeader } from "../components/shell/PageHeader";
import { BackButton, useSmartBack } from "../components/ui/BackButton";
import { BookButton, BookIconButton } from "../components/ui/BookButton";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { FollowLatestButton } from "../components/ui/FollowLatestButton";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { useToast } from "../components/feedback/Toast";
import { extractErrorMessage } from "../api";
import {
  AgentMessageList,
  createPendingAgentMessage,
  type PendingAgentMessage,
} from "../components/agent/AgentMessageList";
import { AgentContextMeter } from "../components/agent/AgentContextMeter";
import { AgentBudgetDialog } from "../components/agent/AgentBudgetDialog";
import {
  ACTIVE_AGENT_RUN_STATUSES,
  activeAgentRun,
  clearAgentThreadContext,
  controlAgentRun,
  createAgentThread,
  ensureMainAgentThread,
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
  type AgentExecutionView,
  type AgentMessageView,
  type AgentRunView,
  type AgentThreadSnapshot,
  type AgentThreadView,
} from "../api/agent";
import { useI18n, type Locale } from "../i18n";
import { useNavigate } from "react-router-dom";
import { extractWritingTarget } from "./agentMonitorUtils";
import {
  reduceAgentEvent,
  snapshotCursor,
  loadAgentSnapshotWithEvents,
  streamedAgentMessageIds,
} from "../utils/agentEventReducer";
import { isImeCompositionActive } from "../utils/ime";
import { useFollowLatest } from "../hooks/useFollowLatest";

const RUN_STATUS_TONES: Record<string, string> = {
  queued: "text-info bg-info-bg", running: "text-book-primary bg-book-primary/10",
  waiting: "text-warning bg-warning-bg", pausing: "text-warning bg-warning-bg",
  paused: "text-warning bg-warning-bg", cancelling: "text-warning bg-warning-bg",
  cancelled: "text-book-text-muted bg-book-bg-tertiary/70", completed: "text-success bg-success-bg",
  failed: "text-error bg-error-bg", interrupted: "text-warning bg-warning-bg",
  recovery_required: "text-error bg-error-bg",
};

const EVENT_TONES: Record<string, string> = {
  run_queued: "text-info", run_controlled: "text-warning", message_queued: "text-book-primary",
  approval_resolved: "text-success", run_completed: "text-success", run_failed: "text-error",
  execution_failed: "text-error",
};

const STATUS_LABELS: Record<string, { zh: string; en: string }> = {
  queued: { zh: "排队中", en: "Queued" }, running: { zh: "运行中", en: "Running" },
  waiting: { zh: "等待中", en: "Waiting" }, pausing: { zh: "暂停中", en: "Pausing" },
  paused: { zh: "已暂停", en: "Paused" }, cancelling: { zh: "取消中", en: "Cancelling" },
  cancelled: { zh: "已取消", en: "Cancelled" }, completed: { zh: "已完成", en: "Completed" },
  failed: { zh: "失败", en: "Failed" }, interrupted: { zh: "已中断", en: "Interrupted" },
  recovery_required: { zh: "需要恢复", en: "Recovery required" }, active: { zh: "活跃", en: "Active" },
  archived: { zh: "已归档", en: "Archived" }, pending: { zh: "待处理", en: "Pending" }, claimed: { zh: "处理中", en: "Claimed" },
  approved: { zh: "已批准", en: "Approved" }, rejected: { zh: "已拒绝", en: "Rejected" },
  expired: { zh: "已过期", en: "Expired" }, consumed: { zh: "已处理", en: "Consumed" },
  awaiting_user: { zh: "待确认", en: "Awaiting confirmation" },
  succeeded: { zh: "已成功", en: "Succeeded" },
  outcome_unknown: { zh: "结果待核实", en: "Outcome unknown" },
};

const labelFor = (value: string | null | undefined, locale: Locale): string => {
  if (!value) return locale === "zh-CN" ? "未知" : "Unknown";
  const known = STATUS_LABELS[value];
  return known ? (locale === "zh-CN" ? known.zh : known.en) : value.replace(/_/g, " ");
};

const randomClientMessageId = (): string =>
  typeof globalThis.crypto?.randomUUID === "function"
    ? globalThis.crypto.randomUUID()
    : `web-${Date.now()}-${Math.random().toString(36).slice(2)}`;

export const AgentMonitor: React.FC = () => {
  const smartBack = useSmartBack();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const [threads, setThreads] = useState<AgentThreadView[]>([]);
  const [selectedThreadId, setSelectedThreadId] = useState<string | null>(null);
  const [loadedSnapshot, setSnapshot] = useState<AgentThreadSnapshot | null>(null);
  const [streamedMessageIds, setStreamedMessageIds] = useState<Set<string>>(() => new Set());
  const [loading, setLoading] = useState(true);
  const [snapshotLoading, setSnapshotLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [pendingSubmission, setPendingSubmission] = useState<PendingAgentMessage | null>(null);
  const [deliveryMode, setDeliveryMode] = useState<AgentDeliveryMode>("followup");
  const [executionMode, setExecutionMode] = useState<AgentExecutionMode>("autonomous");
  const [sendingThreads, setSendingThreads] = useState<Record<string, boolean>>({});
  const [controlBusy, setControlBusy] = useState<AgentControlAction | null>(null);
  const [budgetRun, setBudgetRun] = useState<AgentRunView | null>(null);
  const [approvalBusy, setApprovalBusy] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createTitle, setCreateTitle] = useState("");
  const [createCapability, setCreateCapability] = useState("general");
  const [activityOpen, setActivityOpen] = useState(false);
  const [clearing, setClearing] = useState(false);
  const snapshotRequestRef = useRef(0);
  const snapshotRef = useRef<AgentThreadSnapshot | null>(null);
  const eventCursorRef = useRef(0);
  const reloadInFlightRef = useRef(false);
  const pendingEventsRef = useRef<AgentEvent[]>([]);
  const selectedThreadIdRef = useRef(selectedThreadId);
  const composingRef = useRef(false);
  const lastCompositionEndRef = useRef(0);
  const locateBaselineThreadRef = useRef<string | null>(null);
  const handledLocateMessageIdsRef = useRef<Set<string>>(new Set());

  const rememberStreamedMessages = useCallback((events: AgentEvent[], runId?: string) => {
    const ids = streamedAgentMessageIds(events, runId);
    if (ids.length === 0) return;
    setStreamedMessageIds((current) => {
      const next = new Set(current);
      ids.forEach((id) => next.add(id));
      return next.size === current.size ? current : next;
    });
  }, []);

  selectedThreadIdRef.current = selectedThreadId;
  const snapshot = loadedSnapshot?.thread.id === selectedThreadId ? loadedSnapshot : null;
  const {
    viewportRef: messageViewportRef,
    isFollowingLatest,
    hasNewContent,
    scrollToLatest,
  } = useFollowLatest(snapshot?.messages, {
    resetKey: selectedThreadId,
  });
  const draft = selectedThreadId ? drafts[selectedThreadId] ?? "" : "";
  const sending = selectedThreadId ? sendingThreads[selectedThreadId] === true : false;
  const setDraft = (value: string) => {
    if (selectedThreadId) setDrafts((current) => ({ ...current, [selectedThreadId]: value }));
  };

  useEffect(() => {
    const acceptedMessageId = pendingSubmission?.acceptedMessageId;
    if (
      acceptedMessageId
      && snapshot?.messages.some((message) => message.id === acceptedMessageId)
    ) {
      setPendingSubmission(null);
    }
  }, [pendingSubmission?.acceptedMessageId, snapshot]);

  const refreshThreads = useCallback(async (manual = false): Promise<boolean> => {
    if (manual) setRefreshing(true);
    try {
      let next = await listAgentThreads({ include_archived: false, limit: 200 });
      // Domain consoles create their child threads before the user opens the
      // workbench. Always materialize the durable main thread and keep it the
      // default selection instead of treating the first child as global.
      const main = next.find((thread) => thread.kind === "main") ?? await ensureMainAgentThread({});
      if (!next.some((thread) => thread.id === main.id)) next = [main, ...next];
      setThreads(next);
      setSelectedThreadId((current) => current && next.some((thread) => thread.id === current)
        ? current : next.find((thread) => thread.kind === "main")?.id ?? next[0]?.id ?? null);
      setError(null);
      return true;
    } catch (err) {
      setError(extractErrorMessage(err, t("agent.loadFailed")));
      return false;
    } finally {
      setLoading(false);
      if (manual) setRefreshing(false);
    }
  }, [t]);

  const loadSnapshot = useCallback(async (threadId: string, quiet = false): Promise<AgentThreadSnapshot | null> => {
    if (selectedThreadIdRef.current !== threadId) return null;
    const requestId = snapshotRequestRef.current + 1;
    snapshotRequestRef.current = requestId;
    reloadInFlightRef.current = true;
    const isCurrent = () => selectedThreadIdRef.current === threadId && snapshotRequestRef.current === requestId;
    if (!quiet) setSnapshotLoading(true);
    try {
      const next = await loadAgentSnapshotWithEvents(() => getAgentThread(threadId), pendingEventsRef.current, isCurrent);
      if (!isCurrent()) return null;
      snapshotRef.current = next;
      eventCursorRef.current = snapshotCursor(next);
      setSnapshot(next);
      setError(null);
      return next;
    } catch (err) {
      if (isCurrent()) setError(extractErrorMessage(err, t("agent.loadFailed")));
      return null;
    } finally {
      if (isCurrent()) {
        reloadInFlightRef.current = false;
        setSnapshotLoading(false);
      }
    }
  }, [t]);

  useEffect(() => {
    void refreshThreads();
    const timer = window.setInterval(() => void refreshThreads(), 15000);
    return () => window.clearInterval(timer);
  }, [refreshThreads]);

  useEffect(() => {
    if (!selectedThreadId) {
      snapshotRef.current = null;
      eventCursorRef.current = 0;
      reloadInFlightRef.current = false;
      pendingEventsRef.current = [];
      setSnapshot(null);
      setStreamedMessageIds(new Set());
      setPendingSubmission(null);
      setConnected(false);
      return undefined;
    }
    const refreshTimer = window.setInterval(() => void loadSnapshot(selectedThreadId, true), 7000);
    const controller = new AbortController();
    let stopped = false;
    snapshotRef.current = null;
    eventCursorRef.current = 0;
    reloadInFlightRef.current = false;
    pendingEventsRef.current = [];
    const connect = async () => {
      const initial = await loadSnapshot(selectedThreadId);
      if (!initial || stopped) return;
      while (!stopped) {
        try {
          setConnected(true);
          await subscribeAgentThreadEvents(
            { thread_id: selectedThreadId, after_sequence: eventCursorRef.current, limit: 200 },
            (event) => {
              if (event.thread_id !== selectedThreadId) return;
              rememberStreamedMessages([event]);
              pendingEventsRef.current.push(event);
              if (reloadInFlightRef.current) return;
              const current = snapshotRef.current;
              if (!current || current.thread.id !== selectedThreadId) return;
              const reduction = reduceAgentEvent(current, event, eventCursorRef.current);
              if (reduction.needsReload) {
                void loadSnapshot(selectedThreadId, true);
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
          setError(extractErrorMessage(err, t("agent.connectionLost")));
        }
        if (!stopped) {
          setConnected(false);
          await new Promise((resolve) => window.setTimeout(resolve, 1500));
        }
      }
    };
    void connect();
    return () => { stopped = true; controller.abort(); window.clearInterval(refreshTimer); setConnected(false); };
  }, [loadSnapshot, rememberStreamedMessages, selectedThreadId, t]);

  useEffect(() => {
    if (!snapshot) return;
    if (locateBaselineThreadRef.current !== snapshot.thread.id) {
      locateBaselineThreadRef.current = snapshot.thread.id;
      handledLocateMessageIdsRef.current = new Set(
        snapshot.messages
          .filter((message) => extractWritingTarget(message) != null)
          .map((message) => message.id),
      );
      return;
    }
    const next = [...snapshot.messages]
      .reverse()
      .map((message) => ({ message, target: extractWritingTarget(message) }))
      .find(({ message, target }) => target != null && !handledLocateMessageIdsRef.current.has(message.id));
    if (!next?.target) return;
    handledLocateMessageIdsRef.current.add(next.message.id);
    navigate(
      `/write/${next.target.projectId}?chapter=${next.target.chapterNumber}&agent=1`,
    );
  }, [navigate, snapshot]);

  const currentRun = useMemo(() => activeAgentRun(snapshot), [snapshot]);
  const contextRun = currentRun ?? (
    snapshot && snapshot.runs.length > 0
      ? snapshot.runs[snapshot.runs.length - 1]
      : null
  );
  const pendingApprovals = useMemo(() => snapshot?.approvals.filter((approval) => approval.status === "pending") ?? [], [snapshot]);

  useEffect(() => {
    if (pendingApprovals.length > 0) setActivityOpen(true);
  }, [pendingApprovals.length]);

  const selectThread = (threadId: string) => {
    if (threadId === selectedThreadId) return;
    setSelectedThreadId(threadId);
    snapshotRef.current = null;
    eventCursorRef.current = 0;
    reloadInFlightRef.current = false;
    setSnapshot(null);
    setStreamedMessageIds(new Set());
    setPendingSubmission(null);
    setActivityOpen(false);
    composingRef.current = false;
    lastCompositionEndRef.current = 0;
  };

  const send = async () => {
    const content = draft.trim();
    const threadId = selectedThreadId;
    if (!content || !threadId || sending) return;
    const clientMessageId = randomClientMessageId();
    const pendingMessage = createPendingAgentMessage(threadId, content, clientMessageId);
    setPendingSubmission({ message: pendingMessage, acceptedMessageId: null });
    setDraft("");
    setSendingThreads((current) => ({ ...current, [threadId]: true }));
    try {
      const accepted = await sendAgentMessage({ thread_id: threadId, client_message_id: clientMessageId, content, delivery_mode: deliveryMode, execution_mode: executionMode });
      setPendingSubmission((current) => current?.message.id === pendingMessage.id
        ? { ...current, acceptedMessageId: accepted.message_id }
        : current);
      const [next] = await Promise.all([loadSnapshot(threadId, true), refreshThreads()]);
      if (next?.messages.some((message) => message.id === accepted.message_id)) {
        setPendingSubmission((current) => current?.message.id === pendingMessage.id ? null : current);
      }
      if (next) rememberStreamedMessages(next.events, accepted.run_id);
    } catch (err) {
      setPendingSubmission((current) => current?.message.id === pendingMessage.id ? null : current);
      setDrafts((current) => ({ ...current, [threadId]: content }));
      addToast(extractErrorMessage(err, locale === "zh-CN" ? "发送 Agent 消息失败" : "Failed to send Agent message"), "error");
    } finally {
      setSendingThreads((current) => ({ ...current, [threadId]: false }));
    }
  };

  const control = async (action: AgentControlAction) => {
    if (!selectedThreadId || !currentRun || controlBusy) return;
    if (action === "resume" && (currentRun.waiting_reason === "budget" || currentRun.budget_limiting_run_id)) {
      setBudgetRun(currentRun);
      return;
    }
    setControlBusy(action);
    try {
      await controlAgentRun({ thread_id: selectedThreadId, run_id: currentRun.id, action });
      await Promise.all([loadSnapshot(selectedThreadId, true), refreshThreads()]);
    } catch (err) {
      addToast(extractErrorMessage(err, locale === "zh-CN" ? "控制 Agent run 失败" : "Failed to control Agent run"), "error");
    } finally { setControlBusy(null); }
  };

  const resolveApproval = async (approval: AgentApprovalView, decision: "approve" | "reject") => {
    if (approvalBusy) return;
    setApprovalBusy(approval.id);
    try {
      await resolveAgentApproval({ approval_id: approval.id, decision });
      if (selectedThreadId) await loadSnapshot(selectedThreadId, true);
    } catch (err) {
      addToast(extractErrorMessage(err, locale === "zh-CN" ? "处理审批失败" : "Failed to resolve approval"), "error");
    } finally { setApprovalBusy(null); }
  };

  const createSubagent = async () => {
    const capability = createCapability.trim();
    if (!capability) return;
    try {
      const created = await createAgentThread({ kind: "subagent", capability, title: createTitle.trim() || null });
      setCreateOpen(false); setCreateTitle(""); setCreateCapability("general");
      if (!await refreshThreads()) return;
      setSelectedThreadId(created.id);
    } catch (err) {
      addToast(extractErrorMessage(err, locale === "zh-CN" ? "创建子 Agent 失败" : "Failed to create sub-agent"), "error");
    }
  };

  const clearContext = async () => {
    const threadId = selectedThreadId;
    const thread = snapshot?.thread ?? threads.find((item) => item.id === threadId) ?? null;
    if (!threadId || !thread || clearing || sending || !isAgentThreadOperational(thread)) return;
    const confirmed = await confirmDialog({
      title: t("agent.clearContextTitle"),
      message: t("agent.clearContextMessage"),
      confirmText: t("agent.clearContextConfirm"),
      cancelText: t("common.cancel"),
      dialogType: "warning",
    });
    if (!confirmed) return;
    setClearing(true);
    try {
      await clearAgentThreadContext(threadId, t("agent.clearContextTimeout"));
      snapshotRef.current = null;
      eventCursorRef.current = 0;
      pendingEventsRef.current = [];
      setSnapshot(null);
      setStreamedMessageIds(new Set());
      setPendingSubmission(null);
      if (!await refreshThreads(true)) return;
      addToast(t("agent.clearContextSuccess"), "success");
    } catch (err) {
      const message = extractErrorMessage(err, t("agent.clearContextFailed"));
      setError(message);
      addToast(message, "error");
    } finally {
      setClearing(false);
    }
  };

  return (
    <div className="agent-monitor-page flex h-full min-h-0 flex-col bg-book-bg">
      {budgetRun ? <AgentBudgetDialog run={budgetRun} onClose={() => setBudgetRun(null)} onResumed={() => Promise.all([loadSnapshot(budgetRun.thread_id, true), refreshThreads()])} /> : null}
      <PageHeader
        className="agent-monitor-page-header"
        leading={<BackButton iconOnly title={t("nav.backToPrevious")} onClick={smartBack} />}
        icon={<span className="inline-flex h-9 w-9 items-center justify-center rounded-xl bg-book-primary/10 text-book-primary"><Bot size={18} /></span>}
        eyebrow={t("agent.collaborationHub")}
        title={t("agent.workbench")}
        subtitle={locale === "zh-CN" ? "查看任务进度与待确认操作" : "Track progress and review pending actions"}
        meta={<span className={`inline-flex items-center gap-1.5 text-small ${connected ? "text-success" : "text-book-text-muted"}`}><span className={`h-1.5 w-1.5 rounded-full ${connected ? "bg-success" : "bg-book-text-muted/50"}`} />{connected ? (locale === "zh-CN" ? "事件流已连接" : "Event stream") : t("agent.polling")}</span>}
        actions={<BookIconButton label={t("agent.refresh")} onClick={() => void refreshThreads(true)} variant="secondary" size="sm" disabled={refreshing}><RefreshCw size={14} className={refreshing ? "animate-spin" : undefined} /></BookIconButton>}
      />
      <main className="agent-monitor-body custom-scrollbar">
        {error ? <Notice>{error}</Notice> : null}
        <section className="agent-monitor-layout">
          <aside className="agent-monitor-thread-rail a4n-flat-surface">
            <div className="agent-monitor-rail-head">
              <div className="min-w-0 flex-1">
                <div className="flex min-w-0 items-center gap-2">
                  <h2 className="text-sm font-semibold text-book-text-main">{locale === "zh-CN" ? "线程" : "Threads"}</h2>
                  <span className="agent-monitor-rail-count">{threads.length}</span>
                </div>
                <p className="truncate text-caption text-book-text-muted">{locale === "zh-CN" ? "主 Agent 与领域子 Agent" : "Main and domain sub-agents"}</p>
              </div>
              <BookIconButton label={locale === "zh-CN" ? "新建子 Agent" : "New sub-agent"} onClick={() => setCreateOpen((open) => !open)} size="sm"><Plus size={14} /></BookIconButton>
            </div>
            {createOpen ? (
              <div className="agent-monitor-create-form">
                <label className="flex min-w-0 flex-col gap-1 text-caption text-book-text-muted">
                  {locale === "zh-CN" ? "线程名称（可选）" : "Thread title (optional)"}
                  <input aria-label={locale === "zh-CN" ? "线程名称（可选）" : "Thread title (optional)"} value={createTitle} onChange={(event) => setCreateTitle(event.target.value)} placeholder={locale === "zh-CN" ? "线程名称（可选）" : "Thread title (optional)"} className="pyqt-input text-xs" />
                </label>
                <label className="flex min-w-0 flex-col gap-1 text-caption text-book-text-muted">
                  {locale === "zh-CN" ? "能力域" : "Capability"}
                  <input aria-label={locale === "zh-CN" ? "子 Agent 能力域" : "Sub-agent capability"} value={createCapability} onChange={(event) => setCreateCapability(event.target.value)} placeholder={locale === "zh-CN" ? "能力域，例如 content_optimization" : "Capability, e.g. content_optimization"} className="pyqt-input text-xs" />
                </label>
                <div className="flex gap-2">
                  <BookButton size="sm" className="flex-1" onClick={() => void createSubagent()}><Plus size={13} />{locale === "zh-CN" ? "创建" : "Create"}</BookButton>
                  <BookButton size="sm" variant="ghost" onClick={() => setCreateOpen(false)}>{locale === "zh-CN" ? "关闭" : "Close"}</BookButton>
                </div>
              </div>
            ) : null}
            {loading ? <div className="flex min-h-28 items-center justify-center gap-2 text-xs text-book-text-muted"><LoadingSpinner size="sm" />{t("common.loading")}</div> : null}
            {!loading && threads.length === 0 ? <div className="a4n-empty-state a4n-empty-state--compact px-2"><Bot size={22} /><p className="text-xs">{locale === "zh-CN" ? "还没有持久线程" : "No durable threads yet"}</p></div> : null}
            <div className="agent-monitor-thread-list custom-scrollbar">
              {threads.map((thread) => (
                <button key={thread.id} type="button" onClick={() => selectThread(thread.id)}
                  aria-current={selectedThreadId === thread.id ? "true" : undefined}
                  aria-busy={sendingThreads[thread.id] || undefined}
                  data-active={selectedThreadId === thread.id ? "1" : undefined}
                  className="agent-monitor-thread-item a4n-sidebar-item flex min-w-0 items-center gap-2 text-left">
                  <span className={`inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-lg ${thread.kind === "main" ? "bg-book-primary/12 text-book-primary" : "bg-book-bg-tertiary text-book-text-muted"}`}><Bot size={14} /></span>
                  <span className="min-w-0 flex-1">
                    <span className="flex min-w-0 items-center gap-1.5">
                      <span className="min-w-0 flex-1 truncate text-xs font-medium">{thread.title || (thread.kind === "main" ? (locale === "zh-CN" ? "主 Agent" : "Main Agent") : thread.capability || (locale === "zh-CN" ? "子 Agent" : "Sub-agent"))}</span>
                      <span className="shrink-0 text-[10px] text-book-text-muted">{thread.kind === "main" ? "M" : "S"}</span>
                    </span>
                    <span className="mt-0.5 block truncate text-caption text-book-text-muted">{thread.capability || thread.bound_resource?.kind || labelFor(thread.status, locale)}</span>
                  </span>
                  {sendingThreads[thread.id] ? <LoadingSpinner size="sm" /> : <ChevronRight size={13} className="shrink-0 text-book-text-muted/60" />}
                </button>
              ))}
            </div>
          </aside>
          <section className="agent-monitor-session a4n-flat-surface">
            {!selectedThreadId ? <div className="a4n-empty-state m-auto"><Bot size={28} /><p className="text-sm">{locale === "zh-CN" ? "选择一个 Agent 线程开始" : "Select an Agent thread"}</p></div> : <>
              <ThreadHeader
                key={selectedThreadId}
                thread={snapshot?.thread ?? threads.find((thread) => thread.id === selectedThreadId) ?? null}
                locale={locale}
                loading={snapshotLoading}
                activityOpen={activityOpen}
                activityCount={pendingApprovals.length + (currentRun ? 1 : 0)}
                onToggleActivity={() => setActivityOpen((open) => !open)}
                onClearContext={() => void clearContext()}
                clearDisabled={clearing || sending}
              />
              <div className={`agent-monitor-workspace ${activityOpen ? "agent-monitor-workspace--activity-open" : ""}`}>
                <div className="agent-monitor-conversation">
                  <div className="agent-monitor-message-stage">
                    <div ref={messageViewportRef} role="log" aria-label={locale === "zh-CN" ? "消息记录" : "Message history"} tabIndex={0}
                      className="agent-monitor-message-log custom-scrollbar focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-book-primary"
                      style={{ overflowAnchor: "none" }}>
                      <MessageSection
                        messages={snapshot?.messages ?? []}
                        locale={locale}
                        streamedMessageIds={streamedMessageIds}
                        pendingMessages={pendingSubmission ? [pendingSubmission.message] : []}
                        activeRun={contextRun}
                      />
                      {snapshot?.messages.length === 0 && !snapshotLoading && !pendingSubmission && !currentRun ? <div className="a4n-empty-state a4n-empty-state--compact"><MessageSquare size={22} /><p className="text-xs">{locale === "zh-CN" ? "等待第一条消息" : "Waiting for the first message"}</p></div> : null}
                    </div>
                    <FollowLatestButton
                      isFollowingLatest={isFollowingLatest}
                      hasNewContent={hasNewContent}
                      newContentLabel={locale === "zh-CN" ? "有新消息 · 回到最新" : "New messages · Jump to latest"}
                      latestLabel={locale === "zh-CN" ? "回到最新" : "Back to latest"}
                      onClick={scrollToLatest}
                      className="absolute bottom-3 right-3 z-10"
                    />
                  </div>
                  <div className="agent-monitor-composer-seat">
                    <div className="agent-monitor-composer">
                      <textarea
                        key={selectedThreadId}
                        aria-label={locale === "zh-CN" ? "Agent 消息" : "Agent message"}
                        aria-keyshortcuts="Control+Enter Meta+Enter"
                        value={draft}
                        onChange={(event) => setDraft(event.target.value)}
                        onCompositionStart={() => { composingRef.current = true; }}
                        onBlur={() => {
                          composingRef.current = false;
                          lastCompositionEndRef.current = 0;
                        }}
                        onCompositionEnd={(event) => {
                          composingRef.current = false;
                          lastCompositionEndRef.current = Date.now();
                          setDraft(event.currentTarget.value);
                        }}
                        onKeyDown={(event) => {
                          if (event.key === "Enter" && (event.ctrlKey || event.metaKey) && !isImeCompositionActive(
                            event.nativeEvent, composingRef.current, lastCompositionEndRef.current,
                          )) {
                            event.preventDefault();
                            void send();
                          }
                        }}
                        rows={2}
                        maxLength={8000}
                        placeholder={locale === "zh-CN" ? "告诉当前 Agent 下一步做什么…" : "Tell this Agent what to do next…"}
                        className="agent-monitor-composer-input pyqt-input min-w-0 resize-y leading-relaxed"
                        disabled={sending}
                      />
                      <div className="agent-monitor-composer-toolbar">
                        <div className="agent-monitor-composer-controls">
                          <div className="agent-monitor-composer-mode">
                            <ShieldCheck size={13} aria-hidden="true" />
                            <ThemedSelect aria-label={locale === "zh-CN" ? "执行模式" : "Execution mode"} value={executionMode} onChange={(selectedValue) => setExecutionMode(selectedValue as AgentExecutionMode)} className="agent-monitor-composer-select"><option value="autonomous">{locale === "zh-CN" ? "自主执行" : "Autonomous"}</option><option value="review_writes">{locale === "zh-CN" ? "写入需审核" : "Review writes"}</option><option value="advice_only">{locale === "zh-CN" ? "仅建议" : "Advice only"}</option></ThemedSelect>
                          </div>
                          <ThemedSelect aria-label={locale === "zh-CN" ? "消息交付模式" : "Delivery mode"} value={deliveryMode} onChange={(selectedValue) => setDeliveryMode(selectedValue as AgentDeliveryMode)} className="agent-monitor-composer-select"><option value="followup">{locale === "zh-CN" ? "追加" : "Follow-up"}</option><option value="steer">{locale === "zh-CN" ? "转向" : "Steer"}</option><option value="inject">{locale === "zh-CN" ? "注入" : "Inject"}</option></ThemedSelect>
                        </div>
                        <AgentContextMeter run={contextRun} locale={locale} />
                        {currentRun && (currentRun.waiting_reason === "budget" || currentRun.budget_limiting_run_id) ? (
                          <BookButton size="sm" variant="warning" onClick={() => setBudgetRun(currentRun)}>{t("agent.budget.title")}</BookButton>
                        ) : null}
                        <div className="agent-monitor-composer-actions">
                          {currentRun && ACTIVE_AGENT_RUN_STATUSES.has(currentRun.status) ? (
                            <BookIconButton
                              label={locale === "zh-CN" ? "取消运行" : "Cancel run"}
                              title={locale === "zh-CN" ? "取消当前运行" : "Cancel current run"}
                              variant="ghost"
                              size="md"
                              className="text-error hover:bg-error-bg hover:text-error"
                              onClick={() => void control("cancel")}
                              disabled={controlBusy != null || currentRun.status === "cancelling"}
                            >
                              {controlBusy === "cancel" ? <LoadingSpinner size="sm" /> : <Square size={15} />}
                            </BookIconButton>
                          ) : null}
                          <BookIconButton
                            label={locale === "zh-CN" ? "发送消息" : "Send message"}
                            title={locale === "zh-CN" ? "发送消息（⌘/Ctrl + Enter）" : "Send message (⌘/Ctrl + Enter)"}
                            variant="secondary"
                            size="md"
                            onClick={() => void send()}
                            disabled={!draft.trim() || sending}
                          >
                            {sending ? <LoadingSpinner size="sm" /> : <Send size={15} />}
                          </BookIconButton>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
                {activityOpen ? <div className="agent-monitor-activity-pane"><ActivityPanel snapshot={snapshot} currentRun={currentRun} pendingApprovals={pendingApprovals} locale={locale} controlBusy={controlBusy} approvalBusy={approvalBusy} onControl={control} onResolve={resolveApproval} onSelectThread={selectThread} /></div> : null}
              </div>
            </>}
          </section>
        </section>
      </main>
    </div>
  );
};

function ThreadHeader({
  thread,
  locale,
  loading,
  activityOpen,
  activityCount,
  onToggleActivity,
  onClearContext,
  clearDisabled,
}: {
  thread: AgentThreadView | null;
  locale: Locale;
  loading: boolean;
  activityOpen: boolean;
  activityCount: number;
  onToggleActivity: () => void;
  onClearContext: () => void;
  clearDisabled: boolean;
}) {
  if (!thread) return <div className="border-b border-book-border/50 px-3 py-3 text-xs text-book-text-muted">{loading ? "…" : "—"}</div>;
  return (
    <header className="agent-monitor-session-header">
      <span className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-book-primary/10 text-book-primary"><Bot size={17} /></span>
      <div className="agent-monitor-session-identity">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <h2 className="min-w-0 max-w-full truncate text-sm font-semibold text-book-text-main">{thread.title || (thread.kind === "main" ? (locale === "zh-CN" ? "主 Agent" : "Main Agent") : thread.capability || (locale === "zh-CN" ? "子 Agent" : "Sub-agent"))}</h2>
          <span className={`rounded-full px-2 py-0.5 text-caption ${thread.kind === "main" ? "text-book-primary bg-book-primary/10" : "text-book-text-muted bg-book-bg-tertiary"}`}>{thread.kind === "main" ? (locale === "zh-CN" ? "主 Agent" : "Main") : (locale === "zh-CN" ? "子 Agent" : "Sub-agent")}</span>
          <span className={`rounded-full px-2 py-0.5 text-caption ${thread.status === "active" ? "text-success bg-success-bg" : "text-book-text-muted bg-book-bg-tertiary"}`}>{labelFor(thread.status, locale)}</span>
        </div>
        <div className="mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 text-caption text-book-text-muted">
          <span>{thread.capability || (locale === "zh-CN" ? "全局编排" : "Orchestration")}</span>
          {thread.bound_resource?.label ? <><span aria-hidden="true">·</span><span className="max-w-56 truncate">{thread.bound_resource.label}</span></> : null}
        </div>
      </div>
      <div className="agent-monitor-session-tools">
        <span className="shrink-0 text-caption text-book-text-muted">{loading ? <LoadingSpinner size="sm" /> : `${locale === "zh-CN" ? "事件" : "Events"} ${thread.next_event_sequence - 1}`}</span>
        <BookIconButton
          label={locale === "zh-CN" ? "清空 Agent 上下文" : "Clear Agent context"}
          title={locale === "zh-CN" ? "清空 Agent 上下文" : "Clear Agent context"}
          variant="ghost"
          className="text-error hover:bg-error-bg hover:text-error"
          onClick={onClearContext}
          disabled={clearDisabled || thread.status === "archived"}
        >
          <Trash2 size={14} />
        </BookIconButton>
        <BookButton
          size="sm"
          variant={activityOpen ? "primary" : "ghost"}
          aria-pressed={activityOpen}
          onClick={onToggleActivity}
          className="agent-monitor-activity-toggle"
        >
          <Activity size={13} />
          {locale === "zh-CN" ? "轨迹" : "Trace"}
          {activityCount > 0 ? <span className="agent-monitor-activity-count">{activityCount}</span> : null}
        </BookButton>
      </div>
    </header>
  );
}

function MessageSection({
  messages,
  locale,
  streamedMessageIds,
  pendingMessages,
  activeRun,
}: {
  messages: AgentMessageView[];
  locale: Locale;
  streamedMessageIds: ReadonlySet<string>;
  pendingMessages: AgentMessageView[];
  activeRun: AgentRunView | null;
}) {
  return (
    <AgentMessageList
      messages={messages}
      locale={locale}
      streamedMessageIds={streamedMessageIds}
      pendingMessages={pendingMessages}
      activeRun={activeRun}
    />
  );
}

function ActivityPanel({ snapshot, currentRun, pendingApprovals, locale, controlBusy, approvalBusy, onControl, onResolve, onSelectThread }: { snapshot: AgentThreadSnapshot | null; currentRun: AgentRunView | null; pendingApprovals: AgentApprovalView[]; locale: Locale; controlBusy: AgentControlAction | null; approvalBusy: string | null; onControl: (action: AgentControlAction) => void; onResolve: (approval: AgentApprovalView, decision: "approve" | "reject") => void; onSelectThread: (threadId: string) => void }) {
  const events = snapshot?.events.slice(-24).reverse() ?? [];
  const executions = snapshot?.executions.slice(-12).reverse() ?? [];
  const delegations = snapshot?.delegations.slice(-8).reverse() ?? [];
  return <aside className="custom-scrollbar min-h-0 min-w-0 overflow-y-auto overscroll-contain bg-book-bg-paper/35 p-3 sm:p-3.5"><section aria-label={locale === "zh-CN" ? "运行控制" : "Run controls"} className="space-y-2"><div className="flex items-center gap-2"><h3 className="text-xs font-semibold text-book-text-main">{locale === "zh-CN" ? "运行控制" : "Run control"}</h3>{currentRun ? <span className={`ml-auto rounded-full px-2 py-0.5 text-caption ${RUN_STATUS_TONES[currentRun.status] ?? "text-book-text-muted bg-book-bg-tertiary"}`}>{labelFor(currentRun.status, locale)}</span> : null}</div>{currentRun ? <div className="flex flex-wrap gap-2">{["queued", "running", "waiting"].includes(currentRun.status) ? <BookButton size="sm" variant="warning" onClick={() => onControl("pause")} disabled={controlBusy != null}><Pause size={13} />{locale === "zh-CN" ? "暂停" : "Pause"}</BookButton> : null}{["paused", "pausing"].includes(currentRun.status) ? <BookButton size="sm" onClick={() => onControl("resume")} disabled={controlBusy != null}><Play size={13} />{locale === "zh-CN" ? "继续" : "Resume"}</BookButton> : null}{controlBusy ? <LoadingSpinner size="sm" className="self-center text-book-text-muted" /> : null}</div> : <p className="text-caption text-book-text-muted">{locale === "zh-CN" ? "当前没有可控制的运行" : "No controllable run"}</p>}</section>{pendingApprovals.length > 0 ? <section aria-label={locale === "zh-CN" ? "待审批操作" : "Pending approvals"} className="mt-4 space-y-2"><h3 className="flex items-center gap-2 text-xs font-semibold text-book-text-main"><ShieldCheck size={14} className="text-warning" />{locale === "zh-CN" ? "待审批操作" : "Pending approvals"}</h3>{pendingApprovals.map((approval) => <ApprovalCard key={approval.id} approval={approval} locale={locale} busy={approvalBusy === approval.id} onResolve={onResolve} />)}</section> : null}{delegations.length > 0 ? <section className="mt-4 space-y-2"><h3 className="flex items-center gap-2 text-xs font-semibold text-book-text-main"><Users size={14} className="text-book-primary" />{locale === "zh-CN" ? "委派" : "Delegations"}</h3>{delegations.map((delegation) => <div key={delegation.id} className="rounded-xl border border-book-border/45 bg-book-bg-paper px-2.5 py-2 text-caption"><div className="flex min-w-0 items-center gap-2"><span className={`h-1.5 w-1.5 shrink-0 rounded-full ${delegation.status === "accepted" ? "bg-success" : delegation.status === "failed" ? "bg-error" : "bg-book-primary"}`} /><span className="min-w-0 flex-1 truncate text-book-text-sub">{delegation.capability}</span><span className="shrink-0 text-book-text-muted">{labelFor(delegation.status, locale)}</span></div><p className="mt-1 break-words text-book-text-muted">{delegation.task}</p><button type="button" className="mt-1 max-w-full truncate text-left font-mono text-[10px] text-book-primary hover:underline" onClick={() => onSelectThread(delegation.child_thread_id)}>{locale === "zh-CN" ? "打开子线程" : "Open child thread"} · #{delegation.child_thread_id}</button></div>)}</section> : null}{executions.length > 0 ? <section className="mt-4 space-y-2"><h3 className="flex items-center gap-2 text-xs font-semibold text-book-text-main"><FolderTree size={14} className="text-book-text-muted" />{locale === "zh-CN" ? "执行" : "Executions"}</h3><div className="space-y-1.5">{executions.map((execution) => <ExecutionRow key={execution.id} execution={execution} locale={locale} />)}</div></section> : null}<section className="mt-4 space-y-2"><h3 className="flex items-center gap-2 text-xs font-semibold text-book-text-main"><Activity size={14} className="text-book-text-muted" />{locale === "zh-CN" ? "事件轨迹" : "Event trail"}</h3>{events.length === 0 ? <p className="text-caption text-book-text-muted">{locale === "zh-CN" ? "暂无持久事件" : "No durable events"}</p> : <div className="space-y-1.5">{events.map((event) => <EventRow key={`${event.thread_id}-${event.sequence}`} event={event} />)}</div>}</section>{snapshot?.inbox.some((item) => item.status === "pending") ? <section className="mt-4 rounded-xl border border-warning/35 bg-warning-bg/50 p-2.5 text-caption text-warning"><div className="flex items-center gap-2 font-semibold"><Inbox size={14} />{locale === "zh-CN" ? "Inbox 有待处理输入" : "Pending inbox input"}</div><p className="mt-1">{snapshot.inbox.filter((item) => item.status === "pending").length} {locale === "zh-CN" ? "条消息等待 Agent 处理" : "message(s) waiting"}</p></section> : null}{snapshot?.thread ? <ThreadDetails thread={snapshot.thread} locale={locale} /> : null}</aside>;
}

function ThreadDetails({ thread, locale }: { thread: AgentThreadView; locale: Locale }) {
  return <details className="mt-4 border-t border-book-border/25 pt-3 text-caption text-book-text-muted"><summary className="w-fit cursor-pointer hover:text-book-text-main">{locale === "zh-CN" ? "诊断信息" : "Diagnostics"}</summary><dl className="mt-2 space-y-1 [overflow-wrap:anywhere]"><div><dt>{locale === "zh-CN" ? "线程编号" : "Thread ID"}</dt><dd className="font-mono">{thread.id}</dd></div>{thread.bound_resource ? <div><dt>{locale === "zh-CN" ? "关联资源" : "Bound resource"}</dt><dd className="font-mono">{thread.bound_resource.kind}:{thread.bound_resource.id}</dd></div> : null}</dl></details>;
}

function ApprovalCard({ approval, locale, busy, onResolve }: { approval: AgentApprovalView; locale: Locale; busy: boolean; onResolve: (approval: AgentApprovalView, decision: "approve" | "reject") => void }) {
  return <div className="rounded-xl border border-warning/35 bg-warning-bg/40 p-2.5"><p className="break-words text-xs font-medium text-book-text-main">{approval.command}</p>{approval.reason ? <p className="mt-1 break-words text-caption text-book-text-muted">{approval.reason}</p> : null}<div className="mt-2 flex gap-2"><BookButton size="sm" className="flex-1" onClick={() => onResolve(approval, "approve")} disabled={busy}><Check size={13} />{locale === "zh-CN" ? "批准" : "Approve"}</BookButton><BookButton size="sm" variant="danger" className="flex-1" onClick={() => onResolve(approval, "reject")} disabled={busy}><X size={13} />{locale === "zh-CN" ? "拒绝" : "Reject"}</BookButton>{busy ? <LoadingSpinner size="sm" className="self-center" /> : null}</div></div>;
}

function ExecutionRow({ execution, locale }: { execution: AgentExecutionView; locale: Locale }) {
  return <div className="rounded-lg border border-book-border/35 bg-book-bg-paper px-2 py-1.5"><div className="flex min-w-0 items-center gap-2 text-caption"><CircleDot size={11} className={execution.status === "succeeded" ? "text-success" : execution.status === "failed" || execution.status === "outcome_unknown" ? "text-error" : "text-book-primary"} /><span className="min-w-0 flex-1 truncate text-book-text-sub">{execution.tool_name || execution.kind}</span><span className="shrink-0 text-book-text-muted">{labelFor(execution.status, locale)}</span></div>{execution.error ? <p className="mt-1 break-words text-[10px] text-error">{execution.error}</p> : null}</div>;
}

function EventRow({ event }: { event: AgentEvent }) {
  const payload = event.payload && typeof event.payload === "object" && !Array.isArray(event.payload) ? event.payload as Record<string, unknown> : null;
  const text = typeof payload?.text === "string" ? payload.text : null;
  return <div className="flex min-w-0 gap-2 rounded-lg px-1 py-1"><span className={`mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-current ${EVENT_TONES[event.kind] ?? "text-book-text-muted"}`} /><div className="min-w-0 flex-1"><div className="flex min-w-0 items-center gap-2 text-caption"><span className="min-w-0 flex-1 truncate font-medium text-book-text-sub">{event.kind.replace(/_/g, " ")}</span><span className="shrink-0 font-mono text-[10px] text-book-text-muted">#{event.sequence}</span></div>{text ? <p className="mt-0.5 break-words text-[10px] leading-relaxed text-book-text-muted">{text}</p> : null}</div></div>;
}

function Notice({ children }: { children: React.ReactNode }) {
  return <div role="status" className="mb-3 rounded-lg border border-warning/30 bg-warning-bg px-3 py-2 text-small leading-relaxed text-warning">{children}</div>;
}
