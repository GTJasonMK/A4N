import type {
  AnalyzeChapterInput as ApiAnalyzeChapterInput,
  ApplyOptimizationPatchesInput as ApiApplyOptimizationPatchesInput,
  ApplyOptimizationPatchesResponse as ApiApplyOptimizationPatchesResponse,
  CreateOptimizationChatSessionInput as ApiCreateOptimizationChatSessionInput,
  OptimizationAgentPermissions as ApiOptimizationAgentPermissions,
  OptimizationPatch as ApiOptimizationPatch,
  OptimizationPatchSetSummary,
  OptimizationSessionSummary,
  OptimizationTurnEventSummary,
  OptimizationTurnSummary,
  PrepareOptimizationAnalysisSessionInput as ApiPrepareOptimizationAnalysisSessionInput,
  ProposeOptimizationPatchesInput as ApiProposeOptimizationPatchesInput,
  SendOptimizationMessageInput as ApiSendOptimizationMessageInput,
} from "../generated/host";

export type OptimizeSession = OptimizationSessionSummary;
export type OptimizeTurnEvent = OptimizationTurnEventSummary;
export type OptimizeTurn = OptimizationTurnSummary;
export type OptimizationAgentPermissions = ApiOptimizationAgentPermissions;
export type AnalyzeChapterInput = ApiAnalyzeChapterInput;
export type CreateOptimizationChatSessionInput = ApiCreateOptimizationChatSessionInput;
export type PrepareOptimizationAnalysisSessionInput =
  ApiPrepareOptimizationAnalysisSessionInput;
export type OptimizationPatch = ApiOptimizationPatch;
export type OptimizePatchSet = OptimizationPatchSetSummary;
export type ApplyOptimizationPatchesResponse = ApiApplyOptimizationPatchesResponse;
export type ProposeOptimizationPatchesInput = ApiProposeOptimizationPatchesInput;
export type ApplyOptimizationPatchesInput = ApiApplyOptimizationPatchesInput;
export type SendOptimizationMessageInput = ApiSendOptimizationMessageInput;
