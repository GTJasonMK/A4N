import { apiInvoke } from "./index";

export type GenerationCallKind = "llm" | "image" | "video" | string;

export interface GenerationCallAnalyticsInput {
  days: number;
  recent_limit: number;
}

export interface CallKindOverview {
  kind: GenerationCallKind;
  calls: number;
  success: number;
  failed: number;
  pending: number;
  total_tokens: number;
  prompt_tokens: number;
  completion_tokens: number;
  prompt_cache_hit_tokens: number;
  prompt_cache_miss_tokens: number;
  prompt_cache_hit_ratio: number | null;
  avg_duration_ms: number | null;
}

export interface DailyCallMetric {
  date: string;
  llm_calls: number;
  image_calls: number;
  video_calls: number;
  llm_total_tokens: number;
  llm_prompt_cache_hit_tokens: number;
  llm_prompt_cache_miss_tokens: number;
}

interface LlmWorkflowMetric {
  workflow: string;
  step_key: string;
  calls: number;
  total_tokens: number;
  prompt_tokens: number;
  completion_tokens: number;
  prompt_cache_hit_tokens: number;
  prompt_cache_miss_tokens: number;
  prompt_cache_hit_ratio: number | null;
  avg_duration_ms: number | null;
}

interface MediaWorkflowMetric {
  kind: GenerationCallKind;
  workflow: string;
  step_key: string;
  calls: number;
  success: number;
  failed: number;
  pending: number;
  avg_duration_ms: number | null;
}

export interface RecentLlmCall {
  id: number;
  created_at: string;
  workflow: string;
  step_key: string;
  provider: string | null;
  model: string | null;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  total_tokens: number | null;
  prompt_cache_hit_tokens: number | null;
  prompt_cache_miss_tokens: number | null;
  prompt_cache_hit_ratio: number | null;
  duration_ms: number | null;
  streamed: boolean;
}

export interface RecentMediaCall {
  id: number;
  created_at: string;
  completed_at: string | null;
  kind: GenerationCallKind;
  workflow: string;
  step_key: string;
  title: string;
  provider: string | null;
  model: string | null;
  status: string;
  duration_ms: number | null;
}

export interface GenerationCallAnalytics {
  generated_at: string;
  since_at: string;
  days: number;
  overview: CallKindOverview[];
  daily: DailyCallMetric[];
  llm_by_workflow: LlmWorkflowMetric[];
  media_by_workflow: MediaWorkflowMetric[];
  recent_llm_calls: RecentLlmCall[];
  recent_media_calls: RecentMediaCall[];
}

export async function getGenerationCallAnalytics(
  callerUserId: number,
  input: GenerationCallAnalyticsInput,
): Promise<GenerationCallAnalytics> {
  return apiInvoke<GenerationCallAnalytics>("get_generation_call_analytics", {
    callerUserId,
    input,
  });
}
