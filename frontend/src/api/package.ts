import { apiInvoke } from ".";
import type {
  GetPackageOperationInput,
  ListPackageCommandAvailabilityInput,
  PackageCommandAvailability,
  PackageId,
  PackageOverview,
  PackagePreferencesSummary,
  PackageReasonCode,
  PackageLifecycleSummary,
  StartPackageLifecycleInput,
  UpdatePackagePreferencesInput,
} from "./generated/page";

export interface PackageRejection {
  code: string;
  message: string;
  retryable: boolean;
  trace_id: string | null;
  details: {
    category?: string;
    command?: string;
    package_id?: PackageId;
    reason_code?: PackageReasonCode;
    status_version?: number;
    [key: string]: unknown;
  };
}

const PACKAGE_REASON_CODES = new Set<PackageReasonCode>([
  "allowed",
  "not_installed",
  "incompatible",
  "disabled",
  "disabling",
  "capability_unavailable",
  "maintenance_unavailable",
  "permission_denied",
  "host_admin_required",
  "operation_in_progress",
  "version_conflict",
  "unknown_command",
]);

const PACKAGE_IDS = new Set<PackageId>([
  "platform",
  "novel-management",
  "export",
  "ai-workflow",
  "vector-rag",
  "lineage-view",
  "agent",
  "manga",
  "video",
  "galgame",
  "canvas",
  "external-import",
]);

const isRecord = (value: unknown): value is Record<string, unknown> =>
  Boolean(value && typeof value === "object" && !Array.isArray(value));

const readString = (value: unknown): string | undefined =>
  typeof value === "string" && value.trim() ? value : undefined;

/** Reads the canonical rejection when a native bridge preserves its envelope. */
export const readPackageRejection = (error: unknown): PackageRejection | null => {
  const candidate = isRecord(error)
    ? error as Record<string, unknown> & { cause?: unknown }
    : null;
  const value = candidate && isRecord(candidate.details)
    ? candidate
    : candidate && isRecord(candidate.cause)
      ? candidate.cause
      : null;
  if (!value) return null;
  const details = isRecord(value.details) ? value.details : null;
  const code = readString(value.code);
  const message = readString(value.message);
  if (!code || !message || typeof value.retryable !== "boolean" || !details) return null;

  const packageId = readString(details.package_id);
  const reasonCode = readString(details.reason_code);
  return {
    code,
    message,
    retryable: value.retryable,
    trace_id: typeof value.trace_id === "string" || value.trace_id === null
      ? value.trace_id
      : null,
    details: {
      ...details,
      ...(packageId && PACKAGE_IDS.has(packageId as PackageId)
        ? { package_id: packageId as PackageId }
        : {}),
      ...(reasonCode && PACKAGE_REASON_CODES.has(reasonCode as PackageReasonCode)
        ? { reason_code: reasonCode as PackageReasonCode }
        : {}),
    },
  };
};

export const getPackageOverview = (): Promise<PackageOverview[]> =>
  apiInvoke<PackageOverview[]>("get_package_overview");

export const listPackageCommandAvailability = (
  input: ListPackageCommandAvailabilityInput = {},
): Promise<PackageCommandAvailability[]> =>
  apiInvoke<PackageCommandAvailability[]>("list_package_command_availability", { input });

export const getPackageOperation = (
  input: GetPackageOperationInput,
): Promise<PackageLifecycleSummary> =>
  apiInvoke<PackageLifecycleSummary>("get_package_operation", { input });

export const startPackageLifecycle = (
  input: StartPackageLifecycleInput,
): Promise<PackageLifecycleSummary> =>
  apiInvoke<PackageLifecycleSummary>("start_package_lifecycle", { input });

export const getPackagePreferences = (): Promise<PackagePreferencesSummary> =>
  apiInvoke<PackagePreferencesSummary>("get_package_preferences");

export const updatePackagePreferences = (
  input: UpdatePackagePreferencesInput,
): Promise<PackagePreferencesSummary> =>
  apiInvoke<PackagePreferencesSummary>("update_package_preferences", { input });
