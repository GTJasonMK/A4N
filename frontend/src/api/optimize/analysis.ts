import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
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

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法执行内容优化");
  return id;
};

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
    userId: currentUserId(),
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
