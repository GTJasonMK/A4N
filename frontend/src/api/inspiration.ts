import { apiInvoke } from "./index";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import type {
  GenerationTaskHandle,
  GenerationTaskSummary,
  StartInspirationGenerationInput,
} from "./generated/generationContracts";

/**
 * Inspiration API wrapper —— 对齐 crates/a4n-tauri/src/commands/inspiration.rs
 *
 * 关键约定：
 *  - get_inspiration_history(project_id, limit) —— 按 project_id 不是 conversation_id
 *  - generation target 固定为 inspiration_conversation / project_id
 */

export interface InspirationMessage {
  id: number;
  role: string;
  content: string;
  created_at: string;
}

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
// Generation 对话
// ============================================================================

export function inspirationGenerationProject(
  summary: GenerationTaskSummary,
  projectId: number,
): number | null {
  const target = summary.started.target;
  return summary.started.project_id === projectId
    && target?.kind === "inspiration_conversation"
    && target.id === String(projectId)
    ? projectId
    : null;
}

export async function startInspirationGeneration(
  input: StartInspirationGenerationInput,
): Promise<GenerationTaskHandle> {
  return apiInvoke<GenerationTaskHandle>("start_inspiration_generation", {
    userId: currentUserId(),
    input: {
      ...input,
      project_id: idNum(input.project_id),
    },
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
