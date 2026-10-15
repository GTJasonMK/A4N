import React, { useCallback, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, CirclePause, RefreshCw } from "lucide-react";
import type { MessageKey } from "../../../i18n";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  getAdminOverview,
  healthCheck,
  type AdminHealthCheck,
  type AdminOverviewSummary,
  type AdminServiceHealth,
} from "../../../api/admin";
import { BookIconButton } from "../../../components/ui/BookButton";
import { useAdminSectionLoader } from "./useAdminSectionLoader";
import { formatNumber, useI18n } from "../../../i18n";

export const OverviewSection: React.FC = () => {
  const { t } = useI18n();
  const [overview, setOverview] = useState<AdminOverviewSummary | null>(null);
  const [health, setHealth] = useState<AdminHealthCheck | null>(null);
  const [healthLoading, setHealthLoading] = useState(true);
  const [healthError, setHealthError] = useState<{ reason: unknown } | null>(null);
  const mountedRef = useRef(false);
  const healthSeqRef = useRef(0);
  const loadOverview = useCallback(() => getAdminOverview(), []);
  const applyOverview = useCallback((nextOverview: AdminOverviewSummary) => {
    setOverview(nextOverview);
  }, []);
  const { loading, error, load } = useAdminSectionLoader(
    "admin-overview",
    loadOverview,
    applyOverview,
    "settingsAdmin.loadFailed",
  );
  React.useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      healthSeqRef.current += 1;
    };
  }, []);
  const refreshHealth = useCallback(async () => {
    const requestSeq = healthSeqRef.current + 1;
    healthSeqRef.current = requestSeq;
    setHealthLoading(true);
    setHealth(null);
    setHealthError(null);
    try {
      const nextHealth = await healthCheck();
      if (!mountedRef.current || healthSeqRef.current !== requestSeq) return;
      setHealth(nextHealth);
    } catch (error) {
      if (!mountedRef.current || healthSeqRef.current !== requestSeq) return;
      setHealthError({ reason: error });
    } finally {
      if (mountedRef.current && healthSeqRef.current === requestSeq) {
        setHealthLoading(false);
      }
    }
  }, []);
  const refresh = useCallback(async () => {
    setOverview(null);
    await Promise.all([load(), refreshHealth()]);
  }, [load, refreshHealth]);

  React.useEffect(() => {
    void refreshHealth();
  }, [refreshHealth]);

  return (
    <div className="flex flex-col gap-3">
      <OverviewSummary overview={overview} loading={loading} onRefresh={refresh} />
      {error ? <p role="alert" className="text-xs text-error">{error}</p> : null}
      {healthError ? (
        <p role="alert" className="text-xs text-error">
          {extractErrorMessage(healthError.reason, t("settingsAdmin.healthCheckFailed"))}
        </p>
      ) : null}
      <ServiceHealthPanel
        health={health}
        loading={healthLoading}
        failed={health == null && healthError != null}
      />
    </div>
  );
};

const OverviewSummary: React.FC<{
  overview: AdminOverviewSummary | null;
  loading: boolean;
  onRefresh: () => void | Promise<void>;
}> = ({ overview, loading, onRefresh }) => {
  const { t, locale } = useI18n();
  const items: Array<[string, string]> = [
    [
      t("settingsAdmin.usersMetric"),
      overview
        ? t("settingsAdmin.userMetricDetail", {
            total: formatNumber(overview.total_users, locale),
            active: formatNumber(overview.daily_active_users, locale),
          })
        : "",
    ],
    [
      t("settingsAdmin.contentMetric"),
      overview
        ? t("settingsAdmin.contentMetricDetail", {
            projects: formatNumber(overview.total_projects, locale),
            chapters: formatNumber(overview.total_chapters, locale),
          })
        : "",
    ],
    [
      t("settingsAdmin.tasksMetric"),
      overview
        ? t("settingsAdmin.taskMetricDetail", {
            total: formatNumber(overview.task_queue_count, locale),
            pending: formatNumber(overview.pending_tasks, locale),
            running: formatNumber(overview.running_tasks, locale),
          })
        : "",
    ],
    [
      t("settingsAdmin.aiCallsMetric"),
      overview
        ? t("settingsAdmin.aiCallMetricDetail", {
            calls: formatNumber(overview.total_llm_calls, locale),
            configs: formatNumber(
              overview.llm_config_count +
              overview.embedding_config_count +
              overview.image_gen_config_count,
              locale,
            ),
          })
        : "",
    ],
  ];

  return (
    <section className="a4n-flat-surface overflow-hidden">
      <div className="flex min-h-10 items-center gap-2 border-b border-book-border/30 px-3 py-1.5">
        <h3 className="text-xs font-medium text-book-text-main">{t("settingsAdmin.overviewTitle")}</h3>
        <span className="flex-1" />
        <BookIconButton
          label={t("settingsAdmin.refresh")}
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
              {value || (loading ? t("settingsAdmin.loadingData") : t("settingsAdmin.noData"))}
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
};

const ServiceHealthPanel: React.FC<{
  health: AdminHealthCheck | null;
  loading: boolean;
  failed: boolean;
}> = ({
  health,
  loading,
  failed,
}) => {
  const { t } = useI18n();
  const services = health?.services ?? [];
  const healthyCount = services.filter((service) => service.healthy).length;

  return (
    <section>
      <div className="mb-2 flex min-h-8 flex-wrap items-center gap-x-3 gap-y-1 px-1">
        <h3 className="text-xs font-medium text-book-text-main">{t("settingsAdmin.systemHealth")}</h3>
        <span className="min-w-0 flex-1 text-right text-small text-book-text-muted">
          {health ? `${serviceModeLabel(health.runtime_profile, t)} · ` : ""}
          {services.length > 0
            ? t("settingsAdmin.healthyCount", { healthy: healthyCount, total: services.length })
            : loading || failed
              ? ""
              : t("settingsAdmin.noData")}
        </span>
      </div>

      {loading && services.length === 0 ? (
        <div className="a4n-flat-surface inline-flex w-full items-center justify-center gap-1.5 py-4 text-small text-book-text-muted">
          <LoadingSpinner size="sm" />
          {t("settingsAdmin.loadingData")}
        </div>
      ) : failed ? null : services.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--framed py-4 text-center text-xs text-book-text-muted">
          {t("settingsAdmin.noServices")}
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
  const { t } = useI18n();
  const url = service.url ?? "";

  return (
    <li
      className="a4n-flat-surface min-w-0 px-3 py-2.5 text-xs"
      data-service-health={service.name}
    >
      <div className="flex min-w-0 items-center gap-3">
        <div className="min-w-0 flex-1 font-medium text-book-text-main">{service.name}</div>
        <span className={`inline-flex shrink-0 items-center gap-1 text-small ${service.mode === "disabled" ? "text-book-text-muted" : service.healthy ? "text-success" : "text-error"}`}>
          {service.mode === "disabled" ? <CirclePause size={11} /> : service.healthy ? <CheckCircle2 size={11} /> : <AlertTriangle size={11} />}
          {service.mode === "disabled" ? t("settingsAdmin.disabled") : service.healthy ? t("settingsAdmin.healthy") : t("settingsAdmin.unhealthy")}
        </span>
      </div>
      {service.error_message ? <p className="mt-2 break-words text-error">{service.error_message}</p> : null}
      <details className="group mt-1.5 text-small text-book-text-muted">
        <summary className="w-fit cursor-pointer select-none rounded px-1 py-0.5 transition-colors hover:bg-book-bg-tertiary hover:text-book-text-main">
          {t("settingsAdmin.diagnostics")}
        </summary>
        <div className="mt-1.5 grid gap-3 border-l border-book-border/45 pl-3 sm:grid-cols-2">
          <div className="min-w-0">
            <div className="mb-0.5 text-book-text-muted">{t("settingsAdmin.connection")}</div>
            <div className="text-book-text-sub">{serviceModeLabel(service.mode, t)}</div>
            <div className="mt-0.5 truncate font-mono text-book-text-muted" title={url}>
              {url || t("settingsAdmin.notConfiguredService")}
            </div>
          </div>
          <div>
            <div className="mb-0.5 text-book-text-muted">{t("settingsAdmin.version")}</div>
            <VersionLines service={service} />
          </div>
          <div>
            <div className="mb-0.5 text-book-text-muted">{t("settingsAdmin.process")}</div>
            <div>{service.pid ?? "—"}</div>
          </div>
          <div className="min-w-0">
            <div className="mb-0.5 text-book-text-muted">{t("settingsAdmin.errorsAndPrograms")}</div>
            {!service.error_message ? (
              <div className="text-book-text-sub">{t("settingsAdmin.noErrors")}</div>
            ) : null}
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

type ServiceModeValue = AdminServiceHealth["mode"] | AdminHealthCheck["runtime_profile"];

const SERVICE_MODE_LABELS: Record<ServiceModeValue, MessageKey> = {
  full: "settingsAdmin.builtinService",
  minimal: "settingsAdmin.minimalBuiltin",
  embedded: "settingsAdmin.embedded",
  sidecar: "settingsAdmin.sidecar",
  remote: "settingsAdmin.remote",
  disabled: "settingsAdmin.disabled",
};

const serviceModeLabel = (mode: ServiceModeValue, t: ReturnType<typeof useI18n>["t"]): string => t(SERVICE_MODE_LABELS[mode]);

const VersionLines: React.FC<{ service: AdminServiceHealth }> = ({ service }) => {
  const { t } = useI18n();
  return (
  <div className="space-y-0.5 font-mono text-small leading-snug text-book-text-sub">
    <div className="truncate" title={service.service_version ?? ""}>
      {t("settingsAdmin.serviceVersion", { value: service.service_version ?? "—" })}
    </div>
    <div className="truncate" title={service.protocol_version?.toString() ?? ""}>
      {t("settingsAdmin.protocolVersion", { value: service.protocol_version ?? "—" })}
    </div>
    <div className="truncate" title={service.capability_version ?? ""}>
      {t("settingsAdmin.capabilityVersion", { value: service.capability_version ?? "—" })}
    </div>
  </div>
  );
};
