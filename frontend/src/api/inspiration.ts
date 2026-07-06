import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";

/**
 * Inspiration API wrapper —— 对齐 crates/a4n-tauri/src/commands/inspiration.rs
 *
 * 关键约定：
 *  - get_inspiration_history(project_id, limit) —— 按 project_id 不是 conversation_id
 *  - 流式对话 input.message（不是 user_input）
 */

export interface InspirationMessage {
  id: number;
  conversation_id: number;
  role: string;
  content: string;
  tool_calls: unknown | null;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  chapter_number: number | null;
  is_important: boolean;
  created_at: string | null;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问灵感对话");
  return id;
};

// ============================================================================
// 对话历史
// ============================================================================

/** 后端 get_inspiration_history(project_id, limit) */
export async function getInspirationHistory(
  projectId: Id,
  limit: number,
): Promise<InspirationMessage[]> {
  return apiInvoke<InspirationMessage[]>("get_inspiration_history", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    limit,
  });
}

// ============================================================================
// LLM 对话
// ============================================================================

/** 后端 converse_inspiration(input: { project_id, user_input, variables? }) —— 返回 AI 回复字符串 */
export async function converseInspiration(input: {
  project_id: number;
  user_input: string;
  variables?: Record<string, unknown> | null;
}): Promise<string> {
  return apiInvoke<string>("converse_inspiration", { userId: currentUserId(), input });
}

/** 后端 converse_inspiration_stream(input: { project_id, message, variables? }) —— message 不是 user_input */
export async function converseInspirationStream(input: {
  project_id: number;
  message: string;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<string> {
  return apiInvoke<string>("converse_inspiration_stream", {
    userId: currentUserId(),
    input,
  });
}

export async function previewInspirationRequest(input: {
  project_id: number;
  user_input: string;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_inspiration_request", {
    userId: currentUserId(),
    input,
  });
}
