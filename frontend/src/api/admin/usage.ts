import { apiInvoke } from "../index";
import type { DailyUsagePoint, UsageLogSummary } from "../generated/host";
import { type ApiId as Id, idNum } from "../shared";

export async function getUserUsage(
  userId: Id,
  limit: number,
): Promise<UsageLogSummary[]> {
  return apiInvoke<UsageLogSummary[]>("get_user_usage", {
    userId: idNum(userId),
    limit,
  });
}

export async function getTokenSummary(userId: Id): Promise<unknown> {
  return apiInvoke<unknown>("get_token_summary", {
    userId: idNum(userId),
  });
}

export async function getUsageTrends(days: number): Promise<DailyUsagePoint[]> {
  return apiInvoke<DailyUsagePoint[]>("get_usage_trends", {
    input: { days },
  });
}
