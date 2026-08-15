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
  ReverseEngineerInput as GeneratedReverseEngineerInput,
} from "../generated/host";

export type ImportReview = ImportReviewSummary;
export type {
  ImportProjectAnalysisSummary,
  ImportReviewStatus,
  ProjectImportAnalysisProgress,
};
export { MAX_PROJECT_TEXT_UPLOAD_BYTES } from "../generated/host";

export type DedupResult = GeneratedDedupResult;

export type ImportProjectTextUploadInput = GeneratedImportProjectTextUploadInput;
export type AcceptImportReviewInput = GeneratedAcceptImportReviewInput;

export type CheckImportDedupInput = GeneratedCheckImportDedupInput;

export type ReverseEngineerInput = Omit<
  GeneratedReverseEngineerInput,
  "content_sample" | "variables"
> & {
  content_sample: string;
  variables?: Record<string, unknown> | null;
};

export type ConfirmReverseEngineeringCandidateInput =
  GeneratedConfirmReverseEngineeringCandidateInput;
