import { apiInvoke } from "../index";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  ApplyImplicitUpdateDecisionInput,
  CheckImplicitUpdateInput,
  GetImplicitReviewInput,
  ListBehaviorsInput,
} from "../generated/host";
import type {
  ApplyImplicitUpdateDecisionResult,
  BehaviorRecord,
  ImplicitReview,
  ImplicitUpdateDecision,
} from "./types";

export async function listBehaviors(
  input: ListBehaviorsInput,
): Promise<BehaviorRecord[]> {
  return apiInvoke<BehaviorRecord[]>("list_behaviors", {
    input,
  });
}

export async function getImplicitReview(
  input: GetImplicitReviewInput,
): Promise<ImplicitReview> {
  return apiInvoke<ImplicitReview>("get_implicit_review", {
    input,
  });
}

export async function checkImplicitUpdate(
  input: CheckImplicitUpdateInput,
): Promise<ImplicitUpdateDecision> {
  return apiInvoke<ImplicitUpdateDecision>("check_implicit_update", {
    input,
  });
}

export async function previewCheckImplicitUpdateRequest(
  input: CheckImplicitUpdateInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_check_implicit_update_request", {
    input,
  });
}

export async function applyImplicitUpdateDecision(
  input: ApplyImplicitUpdateDecisionInput,
): Promise<ApplyImplicitUpdateDecisionResult> {
  return apiInvoke<ApplyImplicitUpdateDecisionResult>(
    "apply_implicit_update_decision",
    { input },
  );
}
