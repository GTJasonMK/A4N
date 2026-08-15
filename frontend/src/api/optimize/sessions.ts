import { apiInvoke } from "../index";
import type { OptimizePatchSet, OptimizeSession, OptimizeTurn } from "./types";
import { type ApiId as Id, idNum } from "../shared";

export async function listOptimizeSessions(
  projectId: Id,
  chapterId?: Id | null,
): Promise<OptimizeSession[]> {
  return apiInvoke<OptimizeSession[]>("list_optimize_sessions", {
    input: {
      project_id: idNum(projectId),
      chapter_id: chapterId == null ? null : idNum(chapterId),
    },
  });
}

export async function getOptimizeSession(sessionId: Id): Promise<OptimizeSession> {
  return apiInvoke<OptimizeSession>("get_optimize_session", {
    input: { session_id: idNum(sessionId) },
  });
}

export async function listOptimizeTurns(sessionId: Id): Promise<OptimizeTurn[]> {
  return apiInvoke<OptimizeTurn[]>("list_optimize_turns", {
    input: { session_id: idNum(sessionId) },
  });
}

export async function listOptimizePatchSets(sessionId: Id): Promise<OptimizePatchSet[]> {
  return apiInvoke<OptimizePatchSet[]>("list_optimize_patch_sets", {
    input: { session_id: idNum(sessionId) },
  });
}

export async function deleteOptimizeSession(sessionId: Id): Promise<void> {
  return apiInvoke<void>("delete_optimize_session", {
    input: { session_id: idNum(sessionId) },
  });
}

export async function recordOptimizeAgentEvent(
  sessionId: Id,
  content: string,
): Promise<void> {
  return apiInvoke<void>("record_optimize_agent_event", {
    input: { session_id: idNum(sessionId), content },
  });
}

export async function discardPendingOptimizePatchSets(sessionId: Id): Promise<void> {
  return apiInvoke<void>("discard_pending_optimize_patch_sets", {
    input: { session_id: idNum(sessionId) },
  });
}
