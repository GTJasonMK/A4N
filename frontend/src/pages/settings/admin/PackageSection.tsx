import React, { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, Power, RefreshCw } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  getPackageOverview,
  getPackageOperation,
  getPackagePreferences,
  readPackageRejection,
  startPackageLifecycle,
  updatePackagePreferences,
} from "../../../api/package";
import type {
  PackageId,
  PackageLifecycleState,
  PackageOverview,
  PackageLifecycleSummary,
  PackagePreferencesSummary,
} from "../../../api/generated/page";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { useI18n } from "../../../i18n";

const OPERATION_POLL_INTERVAL_MS = 1_000;
const MAX_OPERATION_POLLS = 60;

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
  const [preferences, setPreferences] = useState<PackagePreferencesSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [busyPackage, setBusyPackage] = useState<string | null>(null);
  const [preferenceBusy, setPreferenceBusy] = useState(false);
  const [snapshotStale, setSnapshotStale] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const pollingOperationsRef = useRef(new Set<string>());

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      pollingOperationsRef.current.clear();
    };
  }, []);

  const load = useCallback(async (initial = false): Promise<PackageOverview[] | null> => {
    if (initial) setLoading(true);
    else setRefreshing(true);
    setError(null);
    try {
      const [nextOverview, nextPreferences] = await Promise.all([
        getPackageOverview(),
        getPackagePreferences(),
      ]);
      if (!mountedRef.current) return nextOverview;
      setOverview(nextOverview);
      setPreferences(nextPreferences);
      setSnapshotStale(false);
      return nextOverview;
    } catch (nextError) {
      if (!mountedRef.current) return null;
      setSnapshotStale(true);
      if (initial) {
        setOverview([]);
        setPreferences(null);
      }
      setError(formatPackageError(nextError, t("settingsAdmin.packageLoadFailed"), t));
      return null;
    } finally {
      if (mountedRef.current) {
        setLoading(false);
        setRefreshing(false);
      }
    }
  }, [t]);

  useEffect(() => {
    void load(true);
  }, [load]);

  const pollOperation = useCallback(async (initial: PackageLifecycleSummary, packageId: PackageId) => {
    let operation = initial;
    let hadRefreshFailure = false;
    try {
      for (let attempt = 0; attempt < MAX_OPERATION_POLLS; attempt += 1) {
        if (!mountedRef.current || isTerminalLifecycleState(operation.state)) break;
        await waitForNextOperationPoll();
        if (!mountedRef.current) return;
        try {
          operation = await getPackageOperation({ operation_id: operation.operation_id });
          const refreshed = await load();
          hadRefreshFailure = refreshed === null;
        } catch (nextError) {
          hadRefreshFailure = true;
          if (mountedRef.current) {
            setSnapshotStale(true);
            setError(formatPackageError(nextError, t("settingsAdmin.packageActionFailed"), t));
          }
        }
      }
      if (!mountedRef.current) return;
      if (operation.state === "failed") {
        setError(
          operation.error
            ? `${t("settingsAdmin.packageOperationError", { error: operation.error })}${operation.reason_code ? ` · ${t("settingsAdmin.packageOperationReason", { reason: operation.reason_code })}` : ""}`
            : t("settingsAdmin.packageActionFailed"),
        );
      } else if (!isTerminalLifecycleState(operation.state)) {
        setError(t("settingsAdmin.packageOperationStillRunning"));
      } else if (!hadRefreshFailure) {
        setError(null);
      }
    } finally {
      pollingOperationsRef.current.delete(initial.operation_id);
      if (mountedRef.current) {
        setBusyPackage((current) => current === packageId ? null : current);
      }
    }
  }, [load, t]);

  const ensureOperationPolling = useCallback((item: PackageOverview, operation: PackageLifecycleSummary) => {
    if (isTerminalLifecycleState(operation.state) || pollingOperationsRef.current.has(operation.operation_id)) {
      return;
    }
    pollingOperationsRef.current.add(operation.operation_id);
    setBusyPackage(item.package_id);
    void pollOperation(operation, item.package_id);
  }, [pollOperation]);

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
      snapshotStale ||
      item.required ||
      !["enabled", "disabled"].includes(item.host_state) ||
      (item.lifecycle != null && isActiveLifecycleState(item.lifecycle.state))
    ) {
      return;
    }
    setBusyPackage(item.package_id);
    setError(null);
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
      await load();
      if (operation.state === "failed") {
        setError(
          operation.error
            ? `${t("settingsAdmin.packageOperationError", { error: operation.error })}${operation.reason_code ? ` · ${t("settingsAdmin.packageOperationReason", { reason: operation.reason_code })}` : ""}`
            : t("settingsAdmin.packageActionFailed"),
        );
      }
    } catch (nextError) {
      setError(formatPackageError(nextError, t("settingsAdmin.packageActionFailed"), t));
    } finally {
      if (!operationTracked) setBusyPackage(null);
    }
  };

  const toggleHidden = async (packageId: PackageOverview["package_id"], hidden: boolean) => {
    if (!preferences || snapshotStale) return;
    const nextHidden = new Set(preferences.hidden_package_ids);
    if (hidden) nextHidden.add(packageId);
    else nextHidden.delete(packageId);
    setPreferenceBusy(true);
    setError(null);
    try {
      setPreferences(
        await updatePackagePreferences({
          hidden_package_ids: [...nextHidden],
        }),
      );
    } catch (nextError) {
      setError(formatPackageError(nextError, t("settingsAdmin.packagePreferenceFailed"), t));
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
          <p className="mt-0.5 text-small text-book-text-muted">
            {canManageLifecycle
              ? t("settingsAdmin.packageHint")
              : t("settingsAbout.packagePreferencesHint")}
          </p>
        </div>
        <BookIconButton
          label={t("settingsAdmin.refresh")}
          onClick={() => void load()}
          disabled={loading || refreshing}
        >
          <RefreshCw size={14} className={loading || refreshing ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>

      {error ? (
        <div className="flex min-w-0 items-start gap-2 border-l-2 border-error px-3 py-2 text-xs text-error" role="alert">
          <AlertTriangle size={14} className="mt-0.5 shrink-0" />
          <span className="min-w-0 break-words">{error}</span>
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
              canManageLifecycle={canManageLifecycle}
              snapshotStale={snapshotStale}
              busy={busyPackage === item.package_id}
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
  canManageLifecycle: boolean;
  snapshotStale: boolean;
  busy: boolean;
  preferenceBusy: boolean;
  onToggle: () => void;
  onHiddenChange: (hidden: boolean) => void;
}> = ({ item, hidden, canManageLifecycle, snapshotStale, busy, preferenceBusy, onToggle, onHiddenChange }) => {
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
            disabled={snapshotStale || item.required || transitional || busy}
          >
            {snapshotStale
              ? t("common.unknown")
              : busy || transitional
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
            disabled={snapshotStale || preferenceBusy}
            onChange={(event) => onHiddenChange(event.target.checked)}
            className="h-4 w-4 accent-book-primary"
          />
          <span>{t("settingsAdmin.hidePackage")}</span>
        </label>
      ) : null}
    </li>
  );
};
