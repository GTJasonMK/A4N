import { apiInvoke } from "../index";
import { type ApiId as Id, idNum } from "../shared";

export async function getAdminOverview(callerUserId: Id): Promise<unknown> {
  return apiInvoke<unknown>("get_admin_overview", {
    callerUserId: idNum(callerUserId),
  });
}

export async function healthCheck(callerUserId: Id): Promise<unknown> {
  return apiInvoke<unknown>("health_check", {
    callerUserId: idNum(callerUserId),
  });
}
