import { apiInvoke } from "./index";
import { requireCurrentUserId } from "./authStorage";
import type { UsageGuideDocument } from "./generated/config";

export async function getUsageGuide(): Promise<UsageGuideDocument> {
  return apiInvoke<UsageGuideDocument>("get_usage_guide");
}

export async function updateUsageGuide(
  document: UsageGuideDocument,
): Promise<UsageGuideDocument> {
  return apiInvoke<UsageGuideDocument>("update_usage_guide", {
    callerUserId: requireCurrentUserId(),
    input: document,
  });
}
