import type {
  AcceptImportReviewInput as GeneratedAcceptImportReviewInput,
  CheckImportDedupInput as GeneratedCheckImportDedupInput,
  DedupResult as GeneratedDedupResult,
  ImportProjectAnalysisSummary,
  ImportProjectTextUploadInput as GeneratedImportProjectTextUploadInput,
  ProjectImportAnalysisProgress,
  ImportReviewStatus,
  ImportReviewSummary,
  ConfirmReverseEngineeringCandidateInput as GeneratedConfirmReverseEngineeringCandidateInput,
  GetReverseEngineeringCandidateInput as GeneratedGetReverseEngineeringCandidateInput,
  ReverseEngineerInput as GeneratedReverseEngineerInput,
  ReverseEngineeringCandidate,
  ReverseEngineeringCandidateState,
  ReverseEngineeringTarget,
} from "../generated/host";

export type ImportReview = ImportReviewSummary;
export type {
  ImportProjectAnalysisSummary,
  ImportReviewStatus,
  ProjectImportAnalysisProgress,
  ReverseEngineeringCandidate,
  ReverseEngineeringCandidateState,
  ReverseEngineeringTarget,
};
export {
  MAX_PROJECT_TEXT_UPLOAD_BYTES,
  REVERSE_ENGINEERING_PROGRESS_TOTAL,
} from "../generated/host";

export type DedupResult = GeneratedDedupResult;

export type ImportProjectTextUploadInput = GeneratedImportProjectTextUploadInput;
export type AcceptImportReviewInput = GeneratedAcceptImportReviewInput;

export type CheckImportDedupInput = GeneratedCheckImportDedupInput;

export type ReverseEngineerInput = GeneratedReverseEngineerInput;

export type GetReverseEngineeringCandidateInput =
  GeneratedGetReverseEngineeringCandidateInput;

export type ConfirmReverseEngineeringCandidateInput =
  GeneratedConfirmReverseEngineeringCandidateInput;
