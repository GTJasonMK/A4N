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
  Users,
  X,
} from "lucide-react";
import { PageHeader } from "../components/shell/PageHeader";
import { BackButton, useSmartBack } from "../components/ui/BackButton";
import { BookButton, BookIconButton } from "../components/ui/BookButton";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { useToast } from "../components/feedback/Toast";
import { extractErrorMessage } from "../api";
import { AgentMessageList } from "../components/agent/AgentMessageList";
import {
  controlAgentRun,
  createAgentThread,
  ensureMainAgentThread,
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
  type AgentMessageView,
  type AgentRunView,
  type AgentThreadSnapshot,
  type AgentThreadView,
} from "../api/agent";
import { useI18n, type Locale } from "../i18n";
import { useNavigate } from "react-router-dom";
import { extractWritingTarget } from "./agentMonitorUtils";
import { reduceAgentEvent, snapshotCursor } from "../utils/agentEventReducer";
import { isImeCompositionActive } from "../utils/ime";

const ACTIVE_RUN_STATUSES = new Set<AgentRunView["status"]>([
  "queued", "running", "waiting", "pausing", "paused", "cancelling",
]);

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
};

const labelFor = (value: string | null | undefined, locale: Locale): string => {
  if (!value) return locale === "zh-CN" ? "未知" : "Unknown";
  const known = STATUS_LABELS[value];
  return known ? (locale === "zh-CN" ? known.zh : known.en) : value.replace(/_/g, " ");
};

const activeRun = (snapshot: AgentThreadSnapshot | null): AgentRunView | null =>
  snapshot ? [...snapshot.runs].reverse().find((run) => ACTIVE_RUN_STATUSES.has(run.status)) ?? null : null;

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
  const [loading, setLoading] = useState(true);
  const [snapshotLoading, setSnapshotLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [deliveryMode, setDeliveryMode] = useState<AgentDeliveryMode>("followup");
  const [executionMode, setExecutionMode] = useState<AgentExecutionMode>("autonomous");
  const [sendingThreads, setSendingThreads] = useState<Record<string, boolean>>({});
  const [controlBusy, setControlBusy] = useState<AgentControlAction | null>(null);
  const [approvalBusy, setApprovalBusy] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createTitle, setCreateTitle] = useState("");
  const [createCapability, setCreateCapability] = useState("general");
  const snapshotRequestRef = useRef(0);
  const selectedThreadIdRef = useRef(selectedThreadId);
  const composingRef = useRef(false);
  const lastCompositionEndRef = useRef(0);
  const locateBaselineThreadRef = useRef<string | null>(null);
  const handledLocateMessageIdsRef = useRef<Set<string>>(new Set());

  selectedThreadIdRef.current = selectedThreadId;
  const snapshot = loadedSnapshot?.thread.id === selectedThreadId ? loadedSnapshot : null;
  const draft = selectedThreadId ? drafts[selectedThreadId] ?? "" : "";
  const sending = selectedThreadId ? sendingThreads[selectedThreadId] === true : false;
  const setDraft = (value: string) => {
    if (selectedThreadId) setDrafts((current) => ({ ...current, [selectedThreadId]: value }));
  };

  const refreshThreads = useCallback(async (manual = false) => {
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
    } catch (err) {
      setError(extractErrorMessage(err, t("agent.loadFailed")));
    } finally {
      setLoading(false);
      if (manual) setRefreshing(false);
    }
  }, [t]);

  const loadSnapshot = useCallback(async (threadId: string, quiet = false): Promise<AgentThreadSnapshot | null> => {
    if (selectedThreadIdRef.current !== threadId) return null;
    const requestId = snapshotRequestRef.current + 1;
    snapshotRequestRef.current = requestId;
    const isCurrent = () => selectedThreadIdRef.current === threadId && snapshotRequestRef.current === requestId;
    if (!quiet) setSnapshotLoading(true);
    try {
      const next = await getAgentThread(threadId);
      if (!isCurrent()) return null;
      setSnapshot(next);
      setError(null);
      return next;
    } catch (err) {
      if (isCurrent()) setError(extractErrorMessage(err, t("agent.loadFailed")));
      return null;
    } finally {
      if (isCurrent()) setSnapshotLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshThreads();
    const timer = window.setInterval(() => void refreshThreads(), 15000);
    return () => window.clearInterval(timer);
  }, [refreshThreads]);

  useEffect(() => {
    if (!selectedThreadId) {
      setSnapshot(null);
      setConnected(false);
      return undefined;
    }
    const refreshTimer = window.setInterval(() => void loadSnapshot(selectedThreadId, true), 7000);
    const controller = new AbortController();
    let stopped = false;
    let cursor = 0;
    const connect = async () => {
      const initial = await loadSnapshot(selectedThreadId);
      if (!initial || stopped) return;
      cursor = snapshotCursor(initial);
      while (!stopped) {
        try {
          setConnected(true);
          await subscribeAgentThreadEvents(
            { thread_id: selectedThreadId, after_sequence: cursor, limit: 200 },
            (event) => {
              let needsReload = false;
              setSnapshot((current) => {
                if (!current || current.thread.id !== selectedThreadId) return current;
                const reduction = reduceAgentEvent(current, event, cursor);
                needsReload = reduction.needsReload;
                if (reduction.needsReload) return current;
                cursor = reduction.cursor;
                return reduction.snapshot;
              });
              if (needsReload) {
                void loadSnapshot(selectedThreadId, true).then((next) => {
                  if (next) cursor = snapshotCursor(next);
                });
              }
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
  }, [loadSnapshot, selectedThreadId, t]);

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

  const currentRun = useMemo(() => activeRun(snapshot), [snapshot]);
  const pendingApprovals = useMemo(() => snapshot?.approvals.filter((approval) => approval.status === "pending") ?? [], [snapshot]);
  const availableThreadCount = useMemo(
    () => threads.filter((thread) => thread.status === "active").length,
    [threads],
  );

  const selectThread = (threadId: string) => {
    if (threadId === selectedThreadId) return;
    setSelectedThreadId(threadId);
    setSnapshot(null);
    composingRef.current = false;
    lastCompositionEndRef.current = 0;
  };

  const send = async () => {
    const content = draft.trim();
    if (!content || !selectedThreadId || sending) return;
    setSendingThreads((current) => ({ ...current, [selectedThreadId]: true }));
    try {
      await sendAgentMessage({ thread_id: selectedThreadId, client_message_id: randomClientMessageId(), content, delivery_mode: deliveryMode, execution_mode: executionMode });
      setDraft("");
      await Promise.all([loadSnapshot(selectedThreadId, true), refreshThreads()]);
    } catch (err) {
      addToast(extractErrorMessage(err, locale === "zh-CN" ? "发送 Agent 消息失败" : "Failed to send Agent message"), "error");
    } finally {
      setSendingThreads((current) => ({ ...current, [selectedThreadId]: false }));
    }
  };

  const control = async (action: AgentControlAction) => {
    if (!selectedThreadId || !currentRun || controlBusy) return;
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
      await refreshThreads(); setSelectedThreadId(created.id);
    } catch (err) {
      addToast(extractErrorMessage(err, locale === "zh-CN" ? "创建子 Agent 失败" : "Failed to create sub-agent"), "error");
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col bg-book-bg">
      <PageHeader
        leading={<BackButton iconOnly title={t("nav.backToPrevious")} onClick={smartBack} />}
        icon={<span className="inline-flex h-9 w-9 items-center justify-center rounded-xl bg-book-primary/10 text-book-primary"><Bot size={18} /></span>}
        eyebrow={t("agent.collaborationHub")}
        title={t("agent.workbench")}
        subtitle={locale === "zh-CN" ? "主 Agent、子 Agent 与用户共享同一条持久运行轨迹。" : "One durable trail for the main Agent, sub-agents, and you."}
        meta={<span className={`inline-flex items-center gap-1.5 text-small ${connected ? "text-success" : "text-book-text-muted"}`}><span className={`h-1.5 w-1.5 rounded-full ${connected ? "bg-success" : "bg-book-text-muted/50"}`} />{connected ? (locale === "zh-CN" ? "事件流已连接" : "Event stream") : t("agent.polling")}</span>}
        actions={<BookIconButton label={t("agent.refresh")} onClick={() => void refreshThreads(true)} variant="secondary" size="sm" disabled={refreshing}><RefreshCw size={14} className={refreshing ? "animate-spin" : undefined} /></BookIconButton>}
      />
      <main className="custom-scrollbar flex min-h-0 flex-1 flex-col overflow-y-auto overflow-x-hidden px-3 pb-8 pt-3 sm:px-5">
        {error ? <Notice>{error}</Notice> : null}
        <section aria-label={locale === "zh-CN" ? "Agent 概览" : "Agent overview"} className="grid grid-cols-3 gap-3 border-b border-book-border/50 pb-3">
          <Summary label={locale === "zh-CN" ? "持久线程" : "Durable threads"} value={threads.length} icon={<MessageSquare size={14} />} />
          <Summary label={locale === "zh-CN" ? "可用线程" : "Available threads"} value={availableThreadCount} icon={<Activity size={14} />} accent />
          <Summary label={locale === "zh-CN" ? "待审批" : "Pending approvals"} value={pendingApprovals.length} icon={<ShieldCheck size={14} />} />
        </section>
        <section className="mt-3 grid min-w-0 flex-none grid-cols-1 gap-3 lg:grid-cols-[minmax(13rem,18rem)_minmax(0,1fr)] xl:min-h-0 xl:flex-1">
          <aside className="a4n-flat-surface flex min-w-0 flex-col overflow-hidden p-2.5 lg:min-h-0">
            <div className="flex min-w-0 items-center gap-2 px-1 pb-2"><div className="min-w-0 flex-1"><h2 className="text-sm font-semibold text-book-text-main">{locale === "zh-CN" ? "线程" : "Threads"}</h2><p className="truncate text-caption text-book-text-muted">{locale === "zh-CN" ? "主 Agent 与领域子 Agent" : "Main and domain sub-agents"}</p></div><BookIconButton label={locale === "zh-CN" ? "新建子 Agent" : "New sub-agent"} onClick={() => setCreateOpen((open) => !open)} size="sm"><Plus size={14} /></BookIconButton></div>
            {createOpen ? <div className="mb-2 space-y-2 rounded-xl border border-book-border/50 bg-book-bg-tertiary/35 p-2"><input aria-label={locale === "zh-CN" ? "线程名称（可选）" : "Thread title (optional)"} value={createTitle} onChange={(event) => setCreateTitle(event.target.value)} placeholder={locale === "zh-CN" ? "线程名称（可选）" : "Thread title (optional)"} className="pyqt-input h-8 min-h-8 text-xs" /><input aria-label={locale === "zh-CN" ? "子 Agent 能力域" : "Sub-agent capability"} value={createCapability} onChange={(event) => setCreateCapability(event.target.value)} placeholder={locale === "zh-CN" ? "能力域，例如 content_optimization" : "Capability, e.g. content_optimization"} className="pyqt-input h-8 min-h-8 text-xs" /><div className="flex gap-2"><BookButton size="sm" className="flex-1" onClick={() => void createSubagent()}><Plus size={13} />{locale === "zh-CN" ? "创建" : "Create"}</BookButton><BookButton size="sm" variant="ghost" onClick={() => setCreateOpen(false)}>{locale === "zh-CN" ? "关闭" : "Close"}</BookButton></div></div> : null}
            {loading ? <div className="flex min-h-28 items-center justify-center gap-2 text-xs text-book-text-muted"><LoadingSpinner size="sm" />{t("common.loading")}</div> : null}
            {!loading && threads.length === 0 ? <div className="a4n-empty-state a4n-empty-state--compact px-2"><Bot size={22} /><p className="text-xs">{locale === "zh-CN" ? "还没有持久线程" : "No durable threads yet"}</p></div> : null}
            <div className="custom-scrollbar min-w-0 space-y-1 overflow-y-auto overscroll-contain lg:min-h-0 lg:flex-1">
              {threads.map((thread) => (
                <button key={thread.id} type="button" onClick={() => selectThread(thread.id)}
                  aria-current={selectedThreadId === thread.id ? "true" : undefined}
                  aria-busy={sendingThreads[thread.id] || undefined}
                  data-active={selectedThreadId === thread.id ? "1" : undefined}
                  className="a4n-sidebar-item flex min-w-0 items-center gap-2 text-left">
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
          <section className="a4n-flat-surface flex min-w-0 min-h-0 flex-col overflow-hidden">
            {!selectedThreadId ? <div className="a4n-empty-state m-auto"><Bot size={28} /><p className="text-sm">{locale === "zh-CN" ? "选择一个 Agent 线程开始" : "Select an Agent thread"}</p></div> : <>
              <ThreadHeader key={selectedThreadId} thread={snapshot?.thread ?? threads.find((thread) => thread.id === selectedThreadId) ?? null} locale={locale} loading={snapshotLoading} />
              <div className="grid min-h-0 flex-1 grid-cols-1 xl:grid-cols-[minmax(0,1fr)_minmax(15rem,21rem)]">
                <div className="flex min-h-0 min-w-0 flex-col border-b border-book-border/50 xl:border-b-0 xl:border-r">
                  <div role="log" aria-label={locale === "zh-CN" ? "消息记录" : "Message history"} tabIndex={0}
                    className="custom-scrollbar max-h-[60dvh] min-h-48 flex-1 space-y-3 overflow-y-auto overscroll-contain px-3 py-3 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-book-primary sm:px-4 xl:max-h-none xl:min-h-0">
                    <MessageSection messages={snapshot?.messages ?? []} locale={locale} />
                    {snapshot?.messages.length === 0 && !snapshotLoading ? <div className="a4n-empty-state a4n-empty-state--compact"><MessageSquare size={22} /><p className="text-xs">{locale === "zh-CN" ? "等待第一条消息" : "Waiting for the first message"}</p></div> : null}
                  </div>
                  <div className="shrink-0 border-t border-book-border/50 bg-book-bg-paper/70 p-2.5 sm:p-3">
                    <div className="mb-2 flex min-w-0 flex-wrap items-center gap-2"><ThemedSelect aria-label={locale === "zh-CN" ? "消息交付模式" : "Delivery mode"} value={deliveryMode} onChange={(event) => setDeliveryMode(event.target.value as AgentDeliveryMode)} className="pyqt-input h-8 min-h-8 max-w-full flex-1 text-xs sm:flex-none"><option value="followup">{locale === "zh-CN" ? "Follow-up · 追加" : "Follow-up"}</option><option value="steer">{locale === "zh-CN" ? "Steer · 转向" : "Steer"}</option><option value="inject">{locale === "zh-CN" ? "Inject · 注入" : "Inject"}</option></ThemedSelect><ThemedSelect aria-label={locale === "zh-CN" ? "执行模式" : "Execution mode"} value={executionMode} onChange={(event) => setExecutionMode(event.target.value as AgentExecutionMode)} className="pyqt-input h-8 min-h-8 max-w-full flex-1 text-xs sm:flex-none"><option value="autonomous">{locale === "zh-CN" ? "自主执行" : "Autonomous"}</option><option value="review_writes">{locale === "zh-CN" ? "写入需审核" : "Review writes"}</option><option value="advice_only">{locale === "zh-CN" ? "仅建议" : "Advice only"}</option></ThemedSelect>{currentRun ? <span className={`ml-auto rounded-full px-2 py-1 text-caption ${RUN_STATUS_TONES[currentRun.status] ?? "text-book-text-muted bg-book-bg-tertiary"}`}>{labelFor(currentRun.status, locale)}</span> : null}</div>
                    <div className="flex min-w-0 items-end gap-2">
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
                        className="pyqt-input min-h-16 min-w-0 flex-1 resize-y text-xs leading-relaxed"
                        disabled={sending}
                      />
                      <BookIconButton
                        label={locale === "zh-CN" ? "发送消息" : "Send message"}
                        title={locale === "zh-CN" ? "发送消息（⌘/Ctrl + Enter）" : "Send message (⌘/Ctrl + Enter)"}
                        variant="secondary" size="md" onClick={() => void send()} disabled={!draft.trim() || sending}
                      >
                        {sending ? <LoadingSpinner size="sm" /> : <Send size={15} />}
                      </BookIconButton>
                    </div>
                  </div>
                </div>
                <ActivityPanel snapshot={snapshot} currentRun={currentRun} pendingApprovals={pendingApprovals} locale={locale} controlBusy={controlBusy} approvalBusy={approvalBusy} onControl={control} onResolve={resolveApproval} onSelectThread={selectThread} />
              </div>
            </>}
          </section>
        </section>
      </main>
    </div>
  );
};

function Summary({ label, value, icon, accent = false }: { label: string; value: number; icon: React.ReactNode; accent?: boolean }) {
  return <div className="flex min-w-0 items-center gap-2.5 py-1"><span className={accent ? "text-book-primary" : "text-book-text-muted"}>{icon}</span><div className="min-w-0"><div className="text-base font-semibold leading-none text-book-text-main">{value.toLocaleString()}</div><div className="mt-1 truncate text-caption text-book-text-muted">{label}</div></div></div>;
}

function ThreadHeader({ thread, locale, loading }: { thread: AgentThreadView | null; locale: Locale; loading: boolean }) {
  const { t } = useI18n();
  if (!thread) return <div className="border-b border-book-border/50 px-3 py-3 text-xs text-book-text-muted">{loading ? "…" : "—"}</div>;
  return (
    <header className="flex min-w-0 flex-wrap items-start gap-3 border-b border-book-border/50 bg-book-bg-paper/55 px-3 py-3 sm:px-4">
      <span className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-book-primary/10 text-book-primary"><Bot size={17} /></span>
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <h2 className="min-w-0 max-w-full truncate text-sm font-semibold text-book-text-main">{thread.title || (thread.kind === "main" ? (locale === "zh-CN" ? "主 Agent" : "Main Agent") : thread.capability || (locale === "zh-CN" ? "子 Agent" : "Sub-agent"))}</h2>
          <span className={`rounded-full px-2 py-0.5 text-caption ${thread.kind === "main" ? "text-book-primary bg-book-primary/10" : "text-book-text-muted bg-book-bg-tertiary"}`}>{thread.kind === "main" ? (locale === "zh-CN" ? "主 Agent" : "Main") : (locale === "zh-CN" ? "子 Agent" : "Sub-agent")}</span>
          <span className={`rounded-full px-2 py-0.5 text-caption ${thread.status === "active" ? "text-success bg-success-bg" : "text-book-text-muted bg-book-bg-tertiary"}`}>{labelFor(thread.status, locale)}</span>
        </div>
        <div className="mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 text-caption text-book-text-muted">
          <span>{thread.capability || (locale === "zh-CN" ? "全局编排" : "Orchestration")}</span>
          {thread.bound_resource?.label ? <><span aria-hidden="true">·</span><span className="max-w-56 truncate">{thread.bound_resource.label}</span></> : null}
        </div>
        <details className="mt-1 text-caption text-book-text-muted">
          <summary className="w-fit cursor-pointer py-1 hover:text-book-text-main">{t("feedback.diagnostics")}</summary>
          <dl className="mt-1 space-y-1 [overflow-wrap:anywhere]">
            <div><dt>{locale === "zh-CN" ? "线程编号" : "Thread ID"}</dt><dd className="font-mono">{thread.id}</dd></div>
            {thread.bound_resource ? <div><dt>{locale === "zh-CN" ? "关联资源" : "Bound resource"}</dt><dd className="font-mono">{thread.bound_resource.kind}:{thread.bound_resource.id}</dd></div> : null}
          </dl>
        </details>
      </div>
      <span className="shrink-0 text-caption text-book-text-muted">{loading ? <LoadingSpinner size="sm" /> : `${locale === "zh-CN" ? "事件" : "Events"} ${thread.next_event_sequence - 1}`}</span>
    </header>
  );
}

function MessageSection({ messages, locale }: { messages: AgentMessageView[]; locale: Locale }) {
  return <AgentMessageList messages={messages} locale={locale} />;
}

function ActivityPanel({ snapshot, currentRun, pendingApprovals, locale, controlBusy, approvalBusy, onControl, onResolve, onSelectThread }: { snapshot: AgentThreadSnapshot | null; currentRun: AgentRunView | null; pendingApprovals: AgentApprovalView[]; locale: Locale; controlBusy: AgentControlAction | null; approvalBusy: string | null; onControl: (action: AgentControlAction) => void; onResolve: (approval: AgentApprovalView, decision: "approve" | "reject") => void; onSelectThread: (threadId: string) => void }) {
  const events = snapshot?.events.slice(-24).reverse() ?? [];
  const executions = snapshot?.executions.slice(-12).reverse() ?? [];
  const delegations = snapshot?.delegations.slice(-8).reverse() ?? [];
  return <aside className="custom-scrollbar min-h-0 min-w-0 overflow-y-auto overscroll-contain bg-book-bg-paper/35 p-3 sm:p-3.5"><section aria-label={locale === "zh-CN" ? "运行控制" : "Run controls"} className="space-y-2"><div className="flex items-center gap-2"><h3 className="text-xs font-semibold text-book-text-main">{locale === "zh-CN" ? "运行控制" : "Run control"}</h3>{currentRun ? <span className={`ml-auto rounded-full px-2 py-0.5 text-caption ${RUN_STATUS_TONES[currentRun.status] ?? "text-book-text-muted bg-book-bg-tertiary"}`}>{labelFor(currentRun.status, locale)}</span> : null}</div>{currentRun ? <div className="flex flex-wrap gap-2">{["queued", "running", "waiting"].includes(currentRun.status) ? <BookButton size="sm" variant="warning" onClick={() => onControl("pause")} disabled={controlBusy != null}><Pause size={13} />{locale === "zh-CN" ? "暂停" : "Pause"}</BookButton> : null}{["paused", "pausing"].includes(currentRun.status) ? <BookButton size="sm" onClick={() => onControl("resume")} disabled={controlBusy != null}><Play size={13} />{locale === "zh-CN" ? "继续" : "Resume"}</BookButton> : null}{ACTIVE_RUN_STATUSES.has(currentRun.status) ? <BookButton size="sm" variant="danger" onClick={() => onControl("cancel")} disabled={controlBusy != null}><Square size={12} />{locale === "zh-CN" ? "取消" : "Cancel"}</BookButton> : null}{controlBusy ? <LoadingSpinner size="sm" className="self-center text-book-text-muted" /> : null}</div> : <p className="text-caption text-book-text-muted">{locale === "zh-CN" ? "当前没有可控制的运行" : "No controllable run"}</p>}</section>{pendingApprovals.length > 0 ? <section aria-label={locale === "zh-CN" ? "待审批操作" : "Pending approvals"} className="mt-4 space-y-2"><h3 className="flex items-center gap-2 text-xs font-semibold text-book-text-main"><ShieldCheck size={14} className="text-warning" />{locale === "zh-CN" ? "待审批操作" : "Pending approvals"}</h3>{pendingApprovals.map((approval) => <ApprovalCard key={approval.id} approval={approval} locale={locale} busy={approvalBusy === approval.id} onResolve={onResolve} />)}</section> : null}{delegations.length > 0 ? <section className="mt-4 space-y-2"><h3 className="flex items-center gap-2 text-xs font-semibold text-book-text-main"><Users size={14} className="text-book-primary" />{locale === "zh-CN" ? "委派" : "Delegations"}</h3>{delegations.map((delegation) => <div key={delegation.id} className="rounded-xl border border-book-border/45 bg-book-bg-paper px-2.5 py-2 text-caption"><div className="flex min-w-0 items-center gap-2"><span className={`h-1.5 w-1.5 shrink-0 rounded-full ${delegation.status === "accepted" ? "bg-success" : delegation.status === "failed" ? "bg-error" : "bg-book-primary"}`} /><span className="min-w-0 flex-1 truncate text-book-text-sub">{delegation.capability}</span><span className="shrink-0 text-book-text-muted">{labelFor(delegation.status, locale)}</span></div><p className="mt-1 break-words text-book-text-muted">{delegation.task}</p><button type="button" className="mt-1 max-w-full truncate text-left font-mono text-[10px] text-book-primary hover:underline" onClick={() => onSelectThread(delegation.child_thread_id)}>{locale === "zh-CN" ? "打开子线程" : "Open child thread"} · #{delegation.child_thread_id}</button></div>)}</section> : null}{executions.length > 0 ? <section className="mt-4 space-y-2"><h3 className="flex items-center gap-2 text-xs font-semibold text-book-text-main"><FolderTree size={14} className="text-book-text-muted" />{locale === "zh-CN" ? "执行" : "Executions"}</h3><div className="space-y-1.5">{executions.map((execution) => <ExecutionRow key={execution.id} execution={execution} locale={locale} />)}</div></section> : null}<section className="mt-4 space-y-2"><h3 className="flex items-center gap-2 text-xs font-semibold text-book-text-main"><Activity size={14} className="text-book-text-muted" />{locale === "zh-CN" ? "事件轨迹" : "Event trail"}</h3>{events.length === 0 ? <p className="text-caption text-book-text-muted">{locale === "zh-CN" ? "暂无持久事件" : "No durable events"}</p> : <div className="space-y-1.5">{events.map((event) => <EventRow key={`${event.thread_id}-${event.sequence}`} event={event} />)}</div>}</section>{snapshot?.inbox.some((item) => item.status === "pending") ? <section className="mt-4 rounded-xl border border-warning/35 bg-warning-bg/50 p-2.5 text-caption text-warning"><div className="flex items-center gap-2 font-semibold"><Inbox size={14} />{locale === "zh-CN" ? "Inbox 有待处理输入" : "Pending inbox input"}</div><p className="mt-1">{snapshot.inbox.filter((item) => item.status === "pending").length} {locale === "zh-CN" ? "条消息等待 Agent 处理" : "message(s) waiting"}</p></section> : null}</aside>;
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
