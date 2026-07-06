import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { OptimizePatchSet, OptimizeSession, OptimizeTurn } from "./types";
import { type ApiId as Id, idNum } from "../shared";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问优化会话");
  return id;
};

export async function listOptimizeSessions(
  projectId: Id,
  chapterId?: Id | null,
): Promise<OptimizeSession[]> {
  return apiInvoke<OptimizeSession[]>("list_optimize_sessions", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterId: chapterId == null ? null : idNum(chapterId),
  });
}

export async function getOptimizeSession(sessionId: Id): Promise<OptimizeSession> {
  return apiInvoke<OptimizeSession>("get_optimize_session", {
    userId: currentUserId(),
    sessionId: idNum(sessionId),
  });
}

export async function listOptimizeTurns(sessionId: Id): Promise<OptimizeTurn[]> {
  return apiInvoke<OptimizeTurn[]>("list_optimize_turns", {
    userId: currentUserId(),
    sessionId: idNum(sessionId),
  });
}

export async function listOptimizePatchSets(sessionId: Id): Promise<OptimizePatchSet[]> {
  return apiInvoke<OptimizePatchSet[]>("list_optimize_patch_sets", {
    userId: currentUserId(),
    sessionId: idNum(sessionId),
  });
}

export async function deleteOptimizeSession(sessionId: Id): Promise<void> {
  return apiInvoke<void>("delete_optimize_session", {
    userId: currentUserId(),
    sessionId: idNum(sessionId),
  });
}

export async function recordOptimizeAgentEvent(
  sessionId: Id,
  content: string,
): Promise<void> {
  return apiInvoke<void>("record_optimize_agent_event", {
    userId: currentUserId(),
    sessionId: idNum(sessionId),
    content,
  });
}

export async function rejectPendingOptimizePatchSets(sessionId: Id): Promise<void> {
  return apiInvoke<void>("reject_pending_optimize_patch_sets", {
    userId: currentUserId(),
    sessionId: idNum(sessionId),
  });
}
