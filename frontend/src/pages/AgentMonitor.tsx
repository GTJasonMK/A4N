import React, { useCallback, useEffect, useState } from "react";
import { Activity, Bot, Clock3, Monitor, RefreshCw, Terminal, Wifi, WifiOff } from "lucide-react";
import { PageHeader } from "../components/shell/PageHeader";
import { BackButton, useSmartBack } from "../components/ui/BackButton";
import { BookIconButton } from "../components/ui/BookButton";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../api";
import { listAgents, subscribeAgentEvents, type AgentEvent, type AgentStatus, type AgentView } from "../api/agents";
import { shouldUseHttpBackend } from "../utils/runtimeMode";

const STATUS_LABELS: Record<AgentStatus, string> = {
  idle: "待命",
  running: "工作中",
  paused: "已暂停",
  completed: "已完成",
  failed: "失败",
  offline: "离线",
};

const STATUS_TONES: Record<AgentStatus, string> = {
  idle: "text-book-text-muted bg-book-bg-tertiary/70",
  running: "text-book-primary bg-book-primary/10",
  paused: "text-warning bg-warning-bg",
  completed: "text-success bg-success-bg",
  failed: "text-error bg-error-bg",
  offline: "text-book-text-muted bg-book-bg-tertiary/50",
};

export const AgentMonitor: React.FC = () => {
  const smartBack = useSmartBack();
  const [agents, setAgents] = useState<AgentView[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const httpBackend = shouldUseHttpBackend();

  const refresh = useCallback(async (manual = false) => {
    if (manual) setRefreshing(true);
    try {
      const next = await listAgents();
      setAgents(next);
      setError(null);
    } catch (err) {
      setError(extractErrorMessage(err, "加载 Agent 状态失败"));
    } finally {
      setLoading(false);
      if (manual) setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    if (!httpBackend) {
      setLoading(false);
      setError("当前原生模式没有连接 Server，Agent 监控仅支持 HTTP-only 模式。");
      return;
    }
    void refresh();
    const refreshTimer = window.setInterval(() => void refresh(), 10000);
    const controller = new AbortController();
    let stopped = false;
    const connect = async () => {
      while (!stopped) {
        try {
          setConnected(true);
          await subscribeAgentEvents((event) => {
            setAgents((current) => applyAgentEvent(current, event));
          }, controller.signal);
        } catch (err) {
          if (controller.signal.aborted) return;
          setConnected(false);
          setError(extractErrorMessage(err, "Agent 实时连接已断开"));
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
    };
  }, [httpBackend, refresh]);

  const runningCount = agents.filter((agent) => agent.status === "running").length;
  const onlineCount = agents.filter((agent) => agent.status !== "offline").length;

  return (
    <div className="flex h-full min-h-0 flex-col bg-book-bg">
      <PageHeader
        leading={<BackButton iconOnly title="返回上一页" onClick={smartBack} />}
        icon={<span className="inline-flex h-9 w-9 items-center justify-center rounded-xl bg-book-primary/10 text-book-primary"><Bot size={18} /></span>}
        eyebrow="协作中枢"
        title="Agent 工作台"
        subtitle="查看 CLI、Web 和 Tauri 接入的 Agent 当前正在做什么。"
        meta={<span className={`inline-flex items-center gap-1.5 text-small ${connected ? "text-success" : "text-book-text-muted"}`}><span className={`h-1.5 w-1.5 rounded-full ${connected ? "bg-success" : "bg-book-text-muted/50"}`} />{connected ? "实时连接" : "轮询模式"}</span>}
        actions={<BookIconButton label="刷新 Agent 状态" onClick={() => void refresh(true)} variant="secondary" size="sm" disabled={refreshing}><RefreshCw size={14} className={refreshing ? "animate-spin" : undefined} /></BookIconButton>}
      />

      <main className="custom-scrollbar min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-4 pb-8 pt-3 sm:px-6">
        {!httpBackend ? <Notice>{error}</Notice> : null}
        {error && httpBackend ? <Notice>{error}</Notice> : null}
        <section aria-label="Agent 概览" className="grid grid-cols-1 gap-2.5 sm:grid-cols-3">
          <Summary label="在线 Agent" value={onlineCount} icon={<Wifi size={14} />} />
          <Summary label="正在工作" value={runningCount} icon={<Activity size={14} />} accent />
          <Summary label="已接入总数" value={agents.length} icon={<Monitor size={14} />} />
        </section>

        <section aria-label="Agent 列表" className="mt-4">
          <div className="mb-2 flex flex-wrap items-baseline gap-x-2 gap-y-1 px-1">
            <h2 className="text-sm font-semibold text-book-text-main">当前 Agent</h2>
            <span className="text-small text-book-text-muted">状态由 Agent 心跳维持</span>
            {loading ? <LoadingSpinner size="sm" className="ml-auto text-book-text-muted" /> : null}
          </div>
          {loading ? <div className="a4n-flat-surface flex min-h-28 items-center justify-center gap-2 text-xs text-book-text-muted"><LoadingSpinner size="sm" />正在读取 Agent 状态…</div> : null}
          {!loading && agents.length === 0 ? <div className="a4n-empty-state a4n-empty-state--framed px-4 py-10 text-center"><Bot size={24} className="mx-auto mb-2 text-book-text-muted/60" /><p className="text-sm text-book-text-sub">当前没有接入的 Agent</p><p className="mt-1 text-small text-book-text-muted">CLI 执行 `a4n agent register` 后会出现在这里。</p></div> : null}
          <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
            {agents.map((agent) => <AgentCard key={agent.agent_id} agent={agent} />)}
          </div>
        </section>
      </main>
    </div>
  );
};

function Summary({ label, value, icon, accent = false }: { label: string; value: number; icon: React.ReactNode; accent?: boolean }) {
  return <div className="a4n-flat-surface flex items-center gap-3 px-3.5 py-3"><span className={accent ? "text-book-primary" : "text-book-text-muted"}>{icon}</span><div><div className="text-lg font-semibold leading-none text-book-text-main">{value}</div><div className="mt-1 text-small text-book-text-muted">{label}</div></div></div>;
}

function AgentCard({ agent }: { agent: AgentView }) {
  const progress = agent.progress == null ? null : Math.max(0, Math.min(100, agent.progress));
  return <article className={`a4n-flat-surface min-w-0 p-3.5 ${agent.status === "running" ? "border-book-primary/35" : ""}`}>
    <div className="flex min-w-0 flex-wrap items-start gap-3">
      <span className={`mt-0.5 inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-xl ${agent.status === "running" ? "bg-book-primary/10 text-book-primary" : "bg-book-bg-tertiary text-book-text-muted"}`}><Bot size={17} /></span>
      <div className="min-w-0 flex-1"><div className="flex flex-wrap items-center gap-2"><h3 className="min-w-0 truncate text-sm font-semibold text-book-text-main">{agent.name}</h3><span className={`rounded-full px-2 py-0.5 text-caption ${STATUS_TONES[agent.status]}`}>{STATUS_LABELS[agent.status]}</span></div><div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-small text-book-text-muted"><span>{agent.client}</span><span aria-hidden="true">·</span><span>{agent.kind}</span>{agent.current_task_id ? <><span aria-hidden="true">·</span><span className="max-w-48 truncate font-mono">{agent.current_task_id}</span></> : null}</div></div>
    </div>
    <div className="mt-3 min-w-0 rounded-lg bg-book-bg-tertiary/45 px-3 py-2.5"><div className="flex flex-wrap items-baseline justify-between gap-2"><span className="text-sm font-medium text-book-text-main">{agent.phase || STATUS_LABELS[agent.status]}</span>{progress != null ? <span className="text-small tabular-nums text-book-primary">{Math.round(progress)}%</span> : null}</div>{agent.summary ? <p className="mt-1 break-words text-small leading-relaxed text-book-text-sub">{agent.summary}</p> : null}{progress != null ? <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-book-border/45"><div className="h-full rounded-full bg-book-primary transition-[width] duration-300" style={{ width: `${progress}%` }} /></div> : null}</div>
    <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-caption text-book-text-muted"><span className="inline-flex items-center gap-1"><Clock3 size={11} />最近心跳 {formatRelativeTime(agent.last_heartbeat_at)}</span>{agent.status === "offline" ? <span className="inline-flex items-center gap-1 text-warning"><WifiOff size={11} />租约已过期</span> : <span className="inline-flex items-center gap-1"><Terminal size={11} />{agent.project_id ? `项目 ${agent.project_id}` : "未绑定项目"}</span>}</div>
  </article>;
}

function Notice({ children }: { children: React.ReactNode }) { return <div role="status" className="mb-3 rounded-lg border border-warning/30 bg-warning-bg px-3 py-2 text-small leading-relaxed text-warning">{children}</div>; }

function applyAgentEvent(current: AgentView[], event: AgentEvent): AgentView[] {
  if (event.kind === "removed") return current.filter((agent) => agent.agent_id !== event.agent_id);
  if (!event.agent) return current;
  const index = current.findIndex((agent) => agent.agent_id === event.agent_id);
  if (index < 0) return [event.agent, ...current];
  const next = [...current];
  next[index] = event.agent;
  return next.sort((a, b) => b.updated_at.localeCompare(a.updated_at));
}

function formatRelativeTime(value: string): string {
  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) return "未知";
  const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
  if (seconds < 60) return `${seconds} 秒前`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)} 分钟前`;
  return new Date(timestamp).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });
}
