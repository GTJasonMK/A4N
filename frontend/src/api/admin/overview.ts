import { apiInvoke } from "../index";
import { type ApiId as Id, idNum } from "../shared";
import type { AdminHealthCheck, AdminOverviewSummary } from "./types";

export async function getAdminOverview(callerUserId: Id): Promise<AdminOverviewSummary> {
  return apiInvoke<AdminOverviewSummary>("get_admin_overview", {
    callerUserId: idNum(callerUserId),
  });
}

export async function healthCheck(callerUserId: Id): Promise<AdminHealthCheck> {
  return apiInvoke<AdminHealthCheck>("health_check", {
    callerUserId: idNum(callerUserId),
  });
}
