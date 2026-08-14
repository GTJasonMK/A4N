import { apiInvoke } from "./index";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import type {
  InspirationMessageSummary,
  PreviewInspirationRequestInput,
} from "./generated/host";
import type {
  GenerationTaskHandle,
  GenerationTaskSummary,
  StartInspirationGenerationInput,
} from "./generated/generationContracts";

export async function getInspirationHistory(
  projectId: Id,
): Promise<InspirationMessageSummary[]> {
  return apiInvoke<InspirationMessageSummary[]>("get_inspiration_history", {
    input: { project_id: idNum(projectId) },
  });
}

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
    input: {
      ...input,
      project_id: idNum(input.project_id),
    },
  });
}

export async function previewInspirationRequest(
  input: PreviewInspirationRequestInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_inspiration_request", {
    input,
  });
}
