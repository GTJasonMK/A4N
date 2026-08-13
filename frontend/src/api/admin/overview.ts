import { apiInvoke } from "../index";
import type { AdminOverviewSummary } from "../generated/host";
import type { AdminHealthCheck } from "./types";

export async function getAdminOverview(): Promise<AdminOverviewSummary> {
  return apiInvoke<AdminOverviewSummary>("get_admin_overview");
}

export async function healthCheck(): Promise<AdminHealthCheck> {
  return apiInvoke<AdminHealthCheck>("health_check");
}
