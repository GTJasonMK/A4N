import { apiInvoke } from "../index";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  ConfirmReverseEngineeringCandidateInput,
  GetReverseEngineeringCandidateInput,
  ReverseEngineerInput,
  ReverseEngineeringCandidate,
} from "../generated/host";

export async function reverseEngineer(
  input: ReverseEngineerInput,
  taskId?: string | null,
): Promise<ReverseEngineeringCandidate> {
  return apiInvoke<ReverseEngineeringCandidate>("reverse_engineer", {
    input,
    taskId: taskId ?? null,
  });
}

export async function previewReverseEngineerRequest(
  input: ReverseEngineerInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_reverse_engineer_request", { input });
}

export async function getReverseEngineeringCandidate(
  input: GetReverseEngineeringCandidateInput,
): Promise<ReverseEngineeringCandidate | null> {
  return apiInvoke<ReverseEngineeringCandidate | null>(
    "get_reverse_engineering_candidate",
    { input },
  );
}

export async function confirmReverseEngineeringCandidate(
  input: ConfirmReverseEngineeringCandidateInput,
): Promise<ReverseEngineeringCandidate> {
  return apiInvoke<ReverseEngineeringCandidate>("confirm_reverse_engineering_candidate", {
    input,
  });
}
