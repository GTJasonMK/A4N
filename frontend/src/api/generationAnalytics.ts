import { apiInvoke } from "./index";
import type {
  GenerationCallAnalytics,
  GenerationCallAnalyticsInput,
} from "./generated/host";

export type {
  CallKindOverview,
  DailyCallMetric,
  GenerationCallAnalytics,
  GenerationCallAnalyticsInput,
  LlmWorkflowMetric,
  MediaWorkflowMetric,
  RecentLlmCall,
  RecentMediaCall,
} from "./generated/host";

export async function getGenerationCallAnalytics(
  callerUserId: number,
  input: GenerationCallAnalyticsInput,
): Promise<GenerationCallAnalytics> {
  return apiInvoke<GenerationCallAnalytics>("get_generation_call_analytics", {
    callerUserId,
    input,
  });
}
