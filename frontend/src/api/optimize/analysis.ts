import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  GenerationTaskHandle,
  StartOptimizeAnalysisGenerationInput,
} from "../generated/generationContracts";
import type {
  AnalyzeChapterInput,
  CreateOptimizationChatSessionInput,
  OptimizeSession,
  PrepareOptimizationAnalysisSessionInput,
} from "./types";

export async function createOptimizationChatSession(
  input: CreateOptimizationChatSessionInput,
): Promise<OptimizeSession> {
  return apiInvoke<OptimizeSession>("create_optimization_chat_session", {
    userId: currentUserId(),
    input,
  });
}

export async function createOptimizationAnalysisSession(
  input: AnalyzeChapterInput,
): Promise<OptimizeSession> {
  return apiInvoke<OptimizeSession>("create_optimization_analysis_session", {
    userId: currentUserId(),
    input,
  });
}

export async function prepareOptimizationAnalysisSession(
  input: PrepareOptimizationAnalysisSessionInput,
): Promise<OptimizeSession> {
  return apiInvoke<OptimizeSession>("prepare_optimization_analysis_session", {
    userId: currentUserId(),
    input,
  });
}

export async function startOptimizeAnalysisGeneration(
  input: StartOptimizeAnalysisGenerationInput,
): Promise<GenerationTaskHandle> {
  return apiInvoke<GenerationTaskHandle>("start_optimize_analysis_generation", {
    input,
  });
}

export async function previewCreateOptimizationAnalysisSessionRequest(
  input: AnalyzeChapterInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>(
    "preview_create_optimization_analysis_session_request",
    { userId: currentUserId(), input },
  );
}

export async function previewPrepareOptimizationAnalysisSessionRequest(
  input: PrepareOptimizationAnalysisSessionInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>(
    "preview_prepare_optimization_analysis_session_request",
    { userId: currentUserId(), input },
  );
}
