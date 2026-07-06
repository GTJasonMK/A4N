export interface ModelRequestPreviewMessage {
  role?: string;
  content?: string;
  [key: string]: unknown;
}

export interface ModelRequestPreviewStep {
  key: string;
  title: string;
  kind: string;
  prompt_name?: string | null;
  state: "ready" | "preview_placeholder" | "blocked" | "reused" | string;
  note?: string | null;
  system_prompt?: string | null;
  user_prompt?: string | null;
  messages?: ModelRequestPreviewMessage[] | unknown;
  request?: unknown;
  context_manifest?: unknown;
  context_plan?: unknown;
  context_pack_refs?: unknown;
  total_chars: number;
  char_counts?: Record<string, unknown> | null;
}

export interface ModelRequestPreview {
  workflow: string;
  title: string;
  steps: ModelRequestPreviewStep[];
  ready_step_count: number;
  total_chars: number;
}
