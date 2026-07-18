import React, { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, Loader2, RefreshCw } from "lucide-react";
import {
  getAdminOverview,
  healthCheck,
  type AdminHealthCheck,
  type AdminOverviewSummary,
  type AdminServiceHealth,
} from "../../../api/admin";
import { BookIconButton } from "../../../components/ui/BookButton";
import { useAdminSectionLoader } from "./useAdminSectionLoader";

export const OverviewSection: React.FC<{ callerUserId: number }> = ({ callerUserId }) => {
  const [overview, setOverview] = useState<AdminOverviewSummary | null>(null);
  const [health, setHealth] = useState<AdminHealthCheck | null>(null);
  const activeCallerUserIdRef = useRef(callerUserId);
  const mountedRef = useRef(true);
  const loadSeqRef = useRef(0);

  activeCallerUserIdRef.current = callerUserId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const isCurrentLoad = useCallback((requestCallerUserId: number, seq: number) => {
    return (
      mountedRef.current &&
      activeCallerUserIdRef.current === requestCallerUserId &&
      loadSeqRef.current === seq
    );
  }, []);

  const loadOverview = useCallback(async () => {
    const requestCallerUserId = callerUserId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    try {
      const [o, h] = await Promise.all([
        getAdminOverview(requestCallerUserId),
        healthCheck(requestCallerUserId),
      ]);
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      setOverview(o);
      setHealth(h);
    } catch (err) {
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      throw err;
    }
  }, [callerUserId, isCurrentLoad]);
  const { loading, load } = useAdminSectionLoader(loadOverview, "加载失败");

  useEffect(() => {
    setOverview(null);
    setHealth(null);
  }, [callerUserId]);

  return (
    <div className="flex flex-col gap-3">
      <OverviewSummary overview={overview} loading={loading} onRefresh={load} />
      <ServiceHealthPanel health={health} loading={loading} />
    </div>
  );
};

const OverviewSummary: React.FC<{
  overview: AdminOverviewSummary | null;
  loading: boolean;
  onRefresh: () => void | Promise<void>;
}> = ({ overview, loading, onRefresh }) => {
  const items: Array<[string, string]> = [
    [
      "用户",
      overview
        ? `${overview.total_users.toLocaleString()} 位 · 今日活跃 ${overview.daily_active_users.toLocaleString()} 位`
        : "",
    ],
    [
      "内容",
      overview
        ? `${overview.total_projects.toLocaleString()} 个项目 · ${overview.total_chapters.toLocaleString()} 个章节`
        : "",
    ],
    [
      "任务",
      overview
        ? `${overview.task_queue_count.toLocaleString()} 个 · 等待 ${overview.pending_tasks.toLocaleString()} · 执行中 ${overview.running_tasks.toLocaleString()}`
        : "",
    ],
    [
      "AI 调用",
      overview
        ? `${overview.total_llm_calls.toLocaleString()} 次 · ${(
            overview.llm_config_count +
            overview.embedding_config_count +
            overview.image_gen_config_count
          ).toLocaleString()} 项配置`
        : "",
    ],
  ];

  return (
    <section className="a4n-flat-surface overflow-hidden">
      <div className="flex min-h-10 items-center gap-2 border-b border-book-border/30 px-3 py-1.5">
        <h3 className="text-xs font-medium text-book-text-main">运行概览</h3>
        <span className="flex-1" />
        <BookIconButton
          label="刷新后台概览"
          onClick={onRefresh}
          size="sm"
          disabled={loading}
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      <dl className="divide-y divide-book-border/30">
        {items.map(([label, value]) => (
          <div key={label} className="grid min-w-0 grid-cols-[4rem_minmax(0,1fr)] gap-3 px-3 py-2.5 text-xs">
            <dt className="text-book-text-muted">{label}</dt>
            <dd className="min-w-0 truncate text-book-text-main" title={value}>
              {value || (loading ? "正在加载…" : "暂无数据")}
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
};

const ServiceHealthPanel: React.FC<{ health: AdminHealthCheck | null; loading: boolean }> = ({
  health,
  loading,
}) => {
  const services = health?.services ?? [];
  const healthyCount = services.filter((service) => service.healthy).length;

  return (
    <section>
      <div className="mb-2 flex min-h-8 flex-wrap items-center gap-x-3 gap-y-1 px-1">
        <h3 className="text-xs font-medium text-book-text-main">系统健康</h3>
        <span className="min-w-0 flex-1 text-right text-[11px] text-book-text-muted">
          {health?.runtime_profile ? `${serviceModeLabel(health.runtime_profile)} · ` : ""}
          {services.length > 0 ? `${healthyCount}/${services.length} 项正常` : "暂无数据"}
        </span>
      </div>

      {loading && services.length === 0 ? (
        <div className="a4n-flat-surface inline-flex w-full items-center justify-center gap-1.5 py-4 text-[11px] text-book-text-muted">
          <Loader2 size={12} className="animate-spin" />
          正在加载数据…
        </div>
      ) : services.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--framed py-4 text-center text-xs text-book-text-muted">
          无服务状态
        </div>
      ) : (
        <ul className="grid grid-cols-1 items-start gap-2 lg:grid-cols-2">
          {services.map((service) => (
            <ServiceHealthRow key={service.name} service={service} />
          ))}
        </ul>
      )}
    </section>
  );
};

const ServiceHealthRow: React.FC<{ service: AdminServiceHealth }> = ({ service }) => {
  const mode = service.service_mode ?? service.mode;
  const url = service.url ? `${service.url}${service.health_path ?? ""}` : "";

  return (
    <li
      className="a4n-flat-surface min-w-0 px-3 py-2.5 text-xs"
      data-service-health={service.name}
    >
      <div className="flex min-w-0 items-center gap-3">
        <div className="min-w-0 flex-1 font-medium text-book-text-main">{service.name}</div>
        <span className={`inline-flex shrink-0 items-center gap-1 text-[11px] ${service.healthy ? "text-success" : "text-error"}`}>
          {service.healthy ? <CheckCircle2 size={11} /> : <AlertTriangle size={11} />}
          {service.healthy ? "正常" : "异常"}
        </span>
      </div>
      <details className="group mt-1.5 text-[11px] text-book-text-muted">
        <summary className="w-fit cursor-pointer select-none rounded px-1 py-0.5 transition-colors hover:bg-book-bg-tertiary hover:text-book-text-main">
          诊断详情
        </summary>
        <div className="mt-1.5 grid gap-3 border-l border-book-border/45 pl-3 sm:grid-cols-2">
          <div className="min-w-0">
            <div className="mb-0.5 text-book-text-muted">连接</div>
            <div className="text-book-text-sub">{serviceModeLabel(mode)}</div>
            <div className="mt-0.5 truncate font-mono text-book-text-muted" title={url}>
              {url || "未配置服务地址"}
            </div>
          </div>
          <div>
            <div className="mb-0.5 text-book-text-muted">版本</div>
            <VersionLines service={service} />
          </div>
          <div>
            <div className="mb-0.5 text-book-text-muted">进程</div>
            <PidCell service={service} />
          </div>
          <div className="min-w-0">
            <div className="mb-0.5 text-book-text-muted">错误与程序</div>
            {service.error_message || service.error_code ? (
              <div className="break-words text-error">
                {service.error_message ?? service.error_code}
              </div>
            ) : (
              <div className="text-book-text-sub">无错误</div>
            )}
            {service.binary ? (
              <div className="mt-1 truncate font-mono text-book-text-muted" title={service.binary}>
                {service.binary}
              </div>
            ) : null}
          </div>
        </div>
      </details>
    </li>
  );
};

const SERVICE_MODE_LABELS: Record<string, string> = {
  full: "内置服务",
  embedded: "内置",
  sidecar: "随附进程",
  remote: "远程",
  disabled: "已停用",
};

const serviceModeLabel = (mode: string | null | undefined): string => {
  const normalized = String(mode ?? "").trim().toLowerCase();
  return SERVICE_MODE_LABELS[normalized] ?? (normalized ? "其他运行方式" : "未配置");
};

const PID_STATUS_LABELS: Record<string, string> = {
  observed: "运行中",
  missing: "未记录",
  stale: "已过期",
  invalid: "无效",
  unreadable: "不可读",
  unverifiable: "未确认",
};

const PidCell: React.FC<{ service: AdminServiceHealth }> = ({ service }) => {
  const status = service.pid_status ?? null;
  const label = status ? PID_STATUS_LABELS[status] ?? "未知状态" : null;

  return (
    <div className="space-y-1">
      <div>{service.pid ?? "—"}</div>
      {label && (
        <span className={status === "observed" ? "text-success" : "text-warning"}>
          {label}
        </span>
      )}
    </div>
  );
};

const VersionLines: React.FC<{ service: AdminServiceHealth }> = ({ service }) => (
  <div className="space-y-0.5 font-mono text-[11px] leading-snug text-book-text-sub">
    <div className="truncate" title={service.service_version ?? ""}>
      服务 {service.service_version ?? "—"}
    </div>
    <div className="truncate" title={service.protocol_version ?? ""}>
      协议 {service.protocol_version ?? "—"}
    </div>
    <div className="truncate" title={service.capability_version ?? ""}>
      能力 {service.capability_version ?? "—"}
    </div>
  </div>
);
