export type ImportReviewStatus =
  | "pending"
  | "analyzing"
  | "reviewing"
  | "accepted"
  | "rejected"
  | "modified";

export interface ImportReview {
  id: number;
  project_id: number;
  filename: string;
  file_type: string;
  detected_encoding: string | null;
  parsed_chapters: number;
  dedup_paragraphs: number;
  dedup_rate: number | null;
  analysis_result: unknown | null;
  review_status: ImportReviewStatus;
  created_at: string | null;
  updated_at: string | null;
}

import type {
  ImportFileResult as GeneratedImportFileResult,
  ImportUploadedFileInput as GeneratedImportUploadedFileInput,
  CheckImportDedupInput as GeneratedCheckImportDedupInput,
  DedupResult as GeneratedDedupResult,
  ParsedChapterSummary,
  ConfirmReverseEngineeringCandidateInput as GeneratedConfirmReverseEngineeringCandidateInput,
  ReverseEngineerInput as GeneratedReverseEngineerInput,
} from "../generated/host";

export type ParsedChapter = ParsedChapterSummary;
export type ImportFileResult = GeneratedImportFileResult;

export interface ImportAnalysisStatus {
  status: ImportReviewStatus;
  count: number;
}

interface ImportAnalysisStage {
  name: string;
  completed: number;
  total: number;
  status: string;
  proposal?: unknown;
}

export interface ProjectImportAnalysisProgress {
  status: string;
  input_fingerprint?: unknown;
  current_stage?: string;
  stages?: Record<string, ImportAnalysisStage>;
  message?: string;
  error?: string | null;
  needs_part_outlines?: boolean;
  planning_total_chapters?: number;
  planning_part_count?: number;
  overall_progress?: number;
  started_at?: string;
  completed_at?: string;
  cancelled_at?: string;
}

export type ImportCandidateState = "proposal" | "confirmed";

export interface ImportCandidateIdentity {
  state: ImportCandidateState;
  origin: "import_analysis";
  project_id: number;
}

export interface ImportCandidateSourceScope {
  chapter_start: number;
  chapter_end: number;
  chapter_count: number;
}

interface ImportBlueprintCandidateFields extends ImportCandidateIdentity {
  title: string;
  genre: string;
  one_sentence_summary: string;
  full_synopsis: string;
  world_setting: Record<string, unknown>;
  total_chapters: number;
  needs_part_outlines: boolean;
  chapters_per_part: number | null;
  source_scope: ImportCandidateSourceScope;
  input_fingerprint?: unknown;
}

export interface ImportBlueprintCandidateProposal extends ImportBlueprintCandidateFields {
  state: "proposal";
  apply_result?: Record<string, unknown> | null;
}

export interface ImportBlueprintCandidateConfirmed extends ImportBlueprintCandidateFields {
  state: "confirmed";
  confirmed_at: string;
  apply_result: ImportBlueprintCandidateApplyResult;
}

export type ImportBlueprintCandidate =
  | ImportBlueprintCandidateProposal
  | ImportBlueprintCandidateConfirmed;

interface ImportPartOutlineCandidateFields extends ImportCandidateIdentity {
  input_fingerprint?: unknown;
  parts: ImportPartOutlineCandidateItem[];
}

export interface ImportPartOutlineCandidateProposal extends ImportPartOutlineCandidateFields {
  state: "proposal";
  apply_result?: Record<string, unknown> | null;
}

export interface ImportPartOutlineCandidateConfirmed extends ImportPartOutlineCandidateFields {
  state: "confirmed";
  confirmed_at: string;
  apply_result: ImportPartOutlineCandidateApplyResult;
}

export type ImportPartOutlineCandidate =
  | ImportPartOutlineCandidateProposal
  | ImportPartOutlineCandidateConfirmed;

export interface ImportPartOutlineCandidateItem {
  part_number: number;
  title: string;
  start_chapter: number;
  end_chapter: number;
  summary: string;
  theme: string;
  key_events: unknown[];
  character_focus: unknown[];
  source_scope: ImportCandidateSourceScope;
}

/*
 * `apply_result` is intentionally optional only for proposal snapshots. A
 * confirmed snapshot is written by the backend with a structured result.
 */
export interface ImportBlueprintCandidateApplyResult {
  version: string;
  updated_fields?: string[];
  [key: string]: unknown;
}

export interface ImportPartOutlineCandidateApplyResult {
  part_count: number;
  inserted_count: number;
  updated_count?: number;
  reused_count: number;
  [key: string]: unknown;
}

export interface ImportProjectAnalysisSummary {
  progress: ProjectImportAnalysisProgress;
  analyzed_chapters: number;
  outlined_chapters: number;
  proposed_parts: number;
}

export type DedupResult = GeneratedDedupResult;

export type ImportUploadedFileInput = GeneratedImportUploadedFileInput;

export interface StartImportAnalysisInput {
  filename: string;
  file_type: string;
  detected_encoding?: string | null;
  chapters: ParsedChapter[];
  dedup_paragraphs: number;
  dedup_rate?: number | null;
}

export interface ModifyImportReviewInput {
  content?: string | null;
  analysis_result?: unknown | null;
  ai_summary?: string | null;
  ai_characters?: unknown | null;
  ai_world_setting?: unknown | null;
}

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
