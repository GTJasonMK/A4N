import React, { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, Power, RefreshCw } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  getPackageOverview,
  getPackageOperation,
  readPackageRejection,
  startPackageLifecycle,
} from "../../../api/package";
import type {
  PackageId,
  PackageLifecycleState,
  PackageOverview,
  PackageLifecycleSummary,
} from "../../../api/generated/page";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { useI18n } from "../../../i18n";
import { usePackagePreferences, savePackagePreferences } from "../../../store/packagePreferences";

const OPERATION_POLL_INTERVAL_MS = 1_000;
const MAX_OPERATION_POLLS = 60;

type PackageSectionFailure = {
  source: "overview" | "preferences" | "operation" | "poll";
  operationId?: string;
  message: string;
};

export const isTerminalLifecycleState = (state: PackageLifecycleState): boolean =>
  state === "completed" || state === "failed";

export const isActiveLifecycleState = (state: PackageLifecycleState): boolean =>
  !isTerminalLifecycleState(state);

const formatPackageError = (
  error: unknown,
  fallback: string,
  t: ReturnType<typeof useI18n>["t"],
): string => {
  const rejection = readPackageRejection(error);
  if (!rejection) return extractErrorMessage(error, fallback);
  return [
    rejection.message,
    `code=${rejection.code}`,
    rejection.details.reason_code
      ? t("settingsAdmin.packageOperationReason", { reason: rejection.details.reason_code })
      : null,
    typeof rejection.details.status_version === "number"
      ? `status_version=${rejection.details.status_version}`
      : null,
  ].filter(Boolean).join(" · ");
};

const waitForNextOperationPoll = (): Promise<void> =>
  new Promise((resolve) => globalThis.setTimeout(resolve, OPERATION_POLL_INTERVAL_MS));

export const PackageSection: React.FC<{ canManageLifecycle?: boolean }> = ({
  canManageLifecycle = true,
}) => {
  const { t } = useI18n();
  const [overview, setOverview] = useState<PackageOverview[]>([]);
  const { preferences, error: preferenceError, refresh: refreshPreferences } = usePackagePreferences();
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [busyPackages, setBusyPackages] = useState<ReadonlySet<PackageId>>(new Set());
  const [preferenceBusy, setPreferenceBusy] = useState(false);
  const [snapshotStale, setSnapshotStale] = useState(false);
  const [error, setError] = useState<PackageSectionFailure | null>(null);
  const mountedRef = useRef(true);
  const pollingOperationsRef = useRef(new Set<string>());

  const setPackageBusy = useCallback((packageId: PackageId, busy: boolean) => {
    setBusyPackages((current) => {
      const next = new Set(current);
      if (busy) next.add(packageId);
      else next.delete(packageId);
      return next;
    });
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      pollingOperationsRef.current.clear();
    };
  }, []);

  const load = useCallback(async (initial = false, includePreferences = true): Promise<PackageOverview[] | null> => {
    if (initial) setLoading(true);
    else if (includePreferences) setRefreshing(true);
    try {
      const [overviewResult, preferencesResult] = await Promise.allSettled([
        getPackageOverview(),
        includePreferences ? refreshPreferences() : Promise.resolve(),
      ]);
      if (!mountedRef.current) {
        return overviewResult.status === "fulfilled" ? overviewResult.value : null;
      }
      if (overviewResult.status === "rejected") {
        setSnapshotStale(true);
        if (initial) setOverview([]);
        setError({
          source: "overview",
          message: formatPackageError(overviewResult.reason, t("settingsAdmin.packageLoadFailed"), t),
        });
        return null;
      }
      setOverview(overviewResult.value);
      setSnapshotStale(false);
      setError((current) => current?.source === "overview" ? null : current);
      if (includePreferences) {
        if (preferencesResult.status === "rejected") {
          setError({
            source: "preferences",
            message: formatPackageError(preferencesResult.reason, t("settingsAdmin.packagePreferenceLoadFailed"), t),
          });
        } else {
          setError((current) => current?.source === "preferences" ? null : current);
        }
      }
      return overviewResult.value;
    } finally {
      if (mountedRef.current) {
        setLoading(false);
        if (includePreferences) setRefreshing(false);
      }
    }
  }, [refreshPreferences, t]);

  useEffect(() => {
    void load(true);
  }, [load]);

  const pollOperation = useCallback(async (initial: PackageLifecycleSummary, packageId: PackageId) => {
    let operation = initial;
    let pollingFailed = false;
    try {
      for (let attempt = 0; attempt < MAX_OPERATION_POLLS; attempt += 1) {
        if (!mountedRef.current || isTerminalLifecycleState(operation.state)) break;
        await waitForNextOperationPoll();
        if (!mountedRef.current) return;
        try {
          operation = await getPackageOperation({ operation_id: operation.operation_id });
          setError((current) => current?.source === "poll" && current.operationId === operation.operation_id ? null : current);
          await load(false, false);
        } catch (nextError) {
          pollingFailed = true;
          if (mountedRef.current) {
            setSnapshotStale(true);
            setError({
              source: "poll",
              operationId: operation.operation_id,
              message: formatPackageError(nextError, t("settingsAdmin.packageActionFailed"), t),
            });
          }
          break;
        }
      }
      if (!mountedRef.current) return;
      if (operation.state === "failed") {
        setError(
          {
            source: "operation",
            message: operation.error
              ? `${t("settingsAdmin.packageOperationError", { error: operation.error })}${operation.reason_code ? ` · ${t("settingsAdmin.packageOperationReason", { reason: operation.reason_code })}` : ""}`
              : t("settingsAdmin.packageActionFailed"),
          },
        );
      } else if (!isTerminalLifecycleState(operation.state) && !pollingFailed) {
        setError({ source: "operation", message: t("settingsAdmin.packageOperationStillRunning") });
      }
    } finally {
      pollingOperationsRef.current.delete(initial.operation_id);
      if (mountedRef.current) {
        setPackageBusy(packageId, false);
      }
    }
  }, [load, setPackageBusy, t]);

  const ensureOperationPolling = useCallback((item: PackageOverview, operation: PackageLifecycleSummary) => {
    if (isTerminalLifecycleState(operation.state) || pollingOperationsRef.current.has(operation.operation_id)) {
      return;
    }
    pollingOperationsRef.current.add(operation.operation_id);
    setPackageBusy(item.package_id, true);
    void pollOperation(operation, item.package_id);
  }, [pollOperation, setPackageBusy]);

  useEffect(() => {
    for (const item of overview) {
      if (item.lifecycle && isActiveLifecycleState(item.lifecycle.state)) {
        ensureOperationPolling(item, item.lifecycle);
      }
    }
  }, [ensureOperationPolling, overview]);

  const toggleLifecycle = async (item: PackageOverview) => {
    if (
      !canManageLifecycle ||
      busyPackages.has(item.package_id) ||
      snapshotStale ||
      item.required ||
      !["enabled", "disabled"].includes(item.host_state) ||
      (item.lifecycle != null && isActiveLifecycleState(item.lifecycle.state))
    ) {
      return;
    }
    setPackageBusy(item.package_id, true);
    setError((current) => current?.source === "operation" || current?.source === "poll" ? null : current);
    let operationTracked = false;
    try {
      const operation = await startPackageLifecycle({
        package_id: item.package_id,
        action: item.host_state === "enabled" ? "disable" : "enable",
        operation_id: globalThis.crypto.randomUUID(),
        expected_status_version: item.status_version,
      });
      if (isActiveLifecycleState(operation.state)) {
        ensureOperationPolling(item, operation);
        operationTracked = true;
      }
      await load(false, false);
      if (operation.state === "failed") {
        setError(
          {
            source: "operation",
            message: operation.error
              ? `${t("settingsAdmin.packageOperationError", { error: operation.error })}${operation.reason_code ? ` · ${t("settingsAdmin.packageOperationReason", { reason: operation.reason_code })}` : ""}`
              : t("settingsAdmin.packageActionFailed"),
          },
        );
      }
    } catch (nextError) {
      setError({
        source: "operation",
        message: formatPackageError(nextError, t("settingsAdmin.packageActionFailed"), t),
      });
    } finally {
      if (!operationTracked) setPackageBusy(item.package_id, false);
    }
  };

  const toggleHidden = async (packageId: PackageOverview["package_id"], hidden: boolean) => {
    if (!preferences || preferenceError) return;
    const nextHidden = new Set(preferences.hidden_package_ids);
    if (hidden) nextHidden.add(packageId);
    else nextHidden.delete(packageId);
    setPreferenceBusy(true);
    setError((current) => current?.source === "preferences" ? null : current);
    try {
      await savePackagePreferences([...nextHidden]);
    } catch (nextError) {
      setError({
        source: "preferences",
        message: formatPackageError(nextError, t("settingsAdmin.packagePreferenceFailed"), t),
      });
    } finally {
      setPreferenceBusy(false);
    }
  };

  return (
    <section
      className="flex flex-col gap-3"
      aria-labelledby="package-section-title"
      aria-busy={loading || refreshing}
      data-package-section
      data-snapshot-stale={snapshotStale ? "1" : "0"}
    >
      <div className="flex min-h-10 flex-wrap items-center gap-2 border-b border-book-border/30 pb-1">
        <div className="min-w-0 flex-1">
          <h3 id="package-section-title" className="text-xs font-medium text-book-text-main">
            {canManageLifecycle
              ? t("settingsAdmin.packages")
              : t("settingsAbout.packagePreferences")}
          </h3>
        </div>
        <BookIconButton
          label={t("settingsAdmin.refresh")}
          onClick={() => { setError(null); void load(); }}
          disabled={loading || refreshing}
        >
          <RefreshCw size={14} className={loading || refreshing ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>

      {error ? (
        <div className="flex min-w-0 items-start gap-2 border-l-2 border-error px-3 py-2 text-xs text-error" role="alert">
          <AlertTriangle size={14} className="mt-0.5 shrink-0" />
          <span className="min-w-0 break-words">{error.message}</span>
        </div>
      ) : null}

      {loading && overview.length === 0 ? (
        <div className="a4n-flat-surface flex min-h-24 items-center justify-center gap-2 text-small text-book-text-muted">
          <LoadingSpinner size="sm" />
          {t("settingsAdmin.loadingData")}
        </div>
      ) : (
        <ul className="grid min-w-0 grid-cols-1 gap-2 lg:grid-cols-2">
          {overview.map((item) => (
            <PackageCard
              key={item.package_id}
              item={item}
              hidden={preferences?.hidden_package_ids.includes(item.package_id) ?? false}
              preferencesUnavailable={preferences == null || preferenceError != null}
              canManageLifecycle={canManageLifecycle}
              snapshotStale={snapshotStale}
              busy={busyPackages.has(item.package_id)}
              preferenceBusy={preferenceBusy}
              onToggle={() => void toggleLifecycle(item)}
              onHiddenChange={(hidden) => void toggleHidden(item.package_id, hidden)}
            />
          ))}
        </ul>
      )}
    </section>
  );
};

export const PackageCard: React.FC<{
  item: PackageOverview;
  hidden: boolean;
  preferencesUnavailable: boolean;
  canManageLifecycle: boolean;
  snapshotStale: boolean;
  busy: boolean;
  preferenceBusy: boolean;
  onToggle: () => void;
  onHiddenChange: (hidden: boolean) => void;
}> = ({ item, hidden, preferencesUnavailable, canManageLifecycle, snapshotStale, busy, preferenceBusy, onToggle, onHiddenChange }) => {
  const { t } = useI18n();
  const enabled = !snapshotStale && item.host_state === "enabled";
  const transitional = !snapshotStale && ["enabling", "disabling"].includes(item.host_state);
  const statusText = snapshotStale
    ? `${t("common.unknown")} · ${t("common.unknown")} · ${t("common.unknown")}`
    : `${item.installation} · ${item.host_state} · ${item.health}`;
  const lifecycleText = item.lifecycle && (
    item.lifecycle.state !== "completed" || item.lifecycle.reason_code || item.lifecycle.error
  )
    ? [
        t("settingsAdmin.packageOperationState", { state: item.lifecycle.state }),
        item.lifecycle.reason_code
          ? t("settingsAdmin.packageOperationReason", { reason: item.lifecycle.reason_code })
          : null,
        item.lifecycle.error
          ? t("settingsAdmin.packageOperationError", { error: item.lifecycle.error })
          : null,
      ].filter(Boolean).join(" · ")
    : null;
  const lifecycleActive = item.lifecycle != null && isActiveLifecycleState(item.lifecycle.state);

  return (
    <li
      className="a4n-flat-surface min-w-0 p-3 text-xs"
      data-package-id={item.package_id}
      data-installation={snapshotStale ? "unknown" : item.installation}
      data-host-state={snapshotStale ? "unknown" : item.host_state}
      data-status-version={item.status_version}
      data-operation-id={item.lifecycle?.operation_id}
      data-snapshot-stale={snapshotStale ? "1" : "0"}
    >
      <div className="flex min-w-0 flex-wrap items-start gap-2">
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-2 font-medium text-book-text-main">
            {enabled ? (
              <CheckCircle2 size={14} className="shrink-0 text-success" aria-hidden="true" />
            ) : (
              <Power size={14} className="shrink-0 text-book-text-muted" aria-hidden="true" />
            )}
            <span className="min-w-0 break-all">{item.package_id}</span>
          </div>
          <div className="mt-1 break-words font-mono text-small tabular-nums text-book-text-muted">
            {statusText} · v{item.status_version}
          </div>
          {lifecycleText ? (
            <div className="mt-1 break-words text-small text-warning" aria-live="polite">
              {lifecycleText}
            </div>
          ) : null}
        </div>
        {canManageLifecycle ? (
          <BookButton
            variant={enabled ? "secondary" : "primary"}
            size="sm"
            onClick={onToggle}
            aria-label={`${item.required ? t("settingsAdmin.requiredPackage") : enabled ? t("settingsAdmin.disablePackage") : t("settingsAdmin.enablePackage")} ${item.package_id}`}
            data-package-lifecycle
            disabled={snapshotStale || item.required || transitional || lifecycleActive || busy}
          >
            {snapshotStale
              ? t("common.unknown")
              : busy || transitional || lifecycleActive
              ? t("settingsAdmin.packageWorking")
              : item.required
                ? t("settingsAdmin.requiredPackage")
                : enabled
                  ? t("settingsAdmin.disablePackage")
                  : t("settingsAdmin.enablePackage")}
          </BookButton>
        ) : null}
      </div>
      {!item.required ? (
        <label className="mt-3 inline-flex min-h-8 max-w-full items-center gap-2 text-small text-book-text-muted">
          <input
            type="checkbox"
            checked={hidden}
            aria-label={`${t("settingsAdmin.hidePackage")} ${item.package_id}`}
            data-package-preference
            disabled={preferencesUnavailable || preferenceBusy}
            onChange={(event) => onHiddenChange(event.target.checked)}
            className="h-4 w-4 accent-book-primary"
          />
          <span>{t("settingsAdmin.hidePackage")}</span>
        </label>
      ) : null}
    </li>
  );
};
