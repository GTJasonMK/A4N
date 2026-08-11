import type {
  AcceptImportReviewInput as GeneratedAcceptImportReviewInput,
  CheckImportDedupInput as GeneratedCheckImportDedupInput,
  DedupResult as GeneratedDedupResult,
  ImportProjectTextUploadInput as GeneratedImportProjectTextUploadInput,
  ImportReviewStatus,
  ImportReviewSummary,
  ConfirmReverseEngineeringCandidateInput as GeneratedConfirmReverseEngineeringCandidateInput,
  ReverseEngineerInput as GeneratedReverseEngineerInput,
} from "../generated/host";

export type ImportReview = ImportReviewSummary;
export type { ImportReviewStatus };
export { MAX_PROJECT_TEXT_UPLOAD_BYTES } from "../generated/host";

interface ImportAnalysisStage {
  name: string;
  completed: number;
  total: number;
  status: string;
}

export interface ProjectImportAnalysisProgress {
  status: string;
  input_fingerprint?: unknown;
  current_stage?: string;
  stages?: Record<string, ImportAnalysisStage>;
  message?: string;
  error?: string | null;
  overall_progress?: number;
  started_at?: string;
  completed_at?: string;
  cancelled_at?: string;
}

export interface ImportProjectAnalysisSummary {
  progress: ProjectImportAnalysisProgress;
  analyzed_chapters: number;
}

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
