import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  ApplyImplicitUpdateDecisionResult,
  BehaviorRecord,
  ImplicitReview,
  ImplicitUpdateDecision,
} from "./types";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问角色行为记录");
  return id;
};

export async function listBehaviors(input: {
  profile_id: number;
  chapter_from?: number | null;
  chapter_to?: number | null;
  limit?: number | null;
}): Promise<BehaviorRecord[]> {
  return apiInvoke<BehaviorRecord[]>("list_behaviors", {
    userId: currentUserId(),
    input,
  });
}

export async function getImplicitReview(input: {
  profile_id: number;
  threshold: number;
  window_chapters: number;
  evidence_limit: number;
}): Promise<ImplicitReview> {
  return apiInvoke<ImplicitReview>("get_implicit_review", {
    userId: currentUserId(),
    input,
  });
}

export async function checkImplicitUpdate(input: {
  profile_id: number;
  attribute_key: string;
  threshold: number;
  window_chapters: number;
  evidence_limit: number;
}): Promise<ImplicitUpdateDecision> {
  return apiInvoke<ImplicitUpdateDecision>("check_implicit_update", {
    userId: currentUserId(),
    input,
  });
}

export async function previewCheckImplicitUpdateRequest(input: {
  profile_id: number;
  attribute_key: string;
  threshold: number;
  window_chapters: number;
  evidence_limit: number;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_check_implicit_update_request", {
    userId: currentUserId(),
    input,
  });
}

export async function applyImplicitUpdateDecision(input: {
  decision: ImplicitUpdateDecision;
  chapter_number: number;
}): Promise<ApplyImplicitUpdateDecisionResult> {
  return apiInvoke<ApplyImplicitUpdateDecisionResult>(
    "apply_implicit_update_decision",
    { userId: currentUserId(), input },
  );
}
