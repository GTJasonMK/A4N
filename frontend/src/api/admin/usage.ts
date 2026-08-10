import { apiInvoke } from "../index";
import type { DailyUsagePoint, UsageLogSummary } from "../generated/host";
import { type ApiId as Id, idNum } from "../shared";

export async function getUserUsage(
  callerUserId: Id,
  userId: Id,
  limit: number,
): Promise<UsageLogSummary[]> {
  return apiInvoke<UsageLogSummary[]>("get_user_usage", {
    callerUserId: idNum(callerUserId),
    userId: idNum(userId),
    limit,
  });
}

export async function getTokenSummary(callerUserId: Id, userId: Id): Promise<unknown> {
  return apiInvoke<unknown>("get_token_summary", {
    callerUserId: idNum(callerUserId),
    userId: idNum(userId),
  });
}

export async function getUsageTrends(callerUserId: Id, days: number): Promise<DailyUsagePoint[]> {
  return apiInvoke<DailyUsagePoint[]>("get_usage_trends", {
    callerUserId: idNum(callerUserId),
    input: { days },
  });
}
