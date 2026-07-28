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
  ProposeOptimizationPatchesResponse as ApiProposeOptimizationPatchesResponse,
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
export type OptimizationPatch = Omit<ApiOptimizationPatch, "operation"> & {
  operation: "replace" | "delete" | "insert_before" | "insert_after";
};
export type OptimizePatchSet = Omit<OptimizationPatchSetSummary, "patches"> & {
  patches: OptimizationPatch[];
};
export type ProposeOptimizationPatchesResponse = Omit<
  ApiProposeOptimizationPatchesResponse,
  "patches"
> & {
  patches: OptimizationPatch[];
};
export type ApplyOptimizationPatchesResponse = ApiApplyOptimizationPatchesResponse;
export type ProposeOptimizationPatchesInput = ApiProposeOptimizationPatchesInput;
export type ApplyOptimizationPatchesInput = Omit<
  ApiApplyOptimizationPatchesInput,
  "patches"
> & {
  patches: OptimizationPatch[];
};
export type SendOptimizationMessageInput = ApiSendOptimizationMessageInput;
