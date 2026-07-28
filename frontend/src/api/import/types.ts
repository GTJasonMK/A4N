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
}

export interface ProjectImportAnalysisProgress {
  status: string;
  current_stage?: string;
  stages?: Record<string, ImportAnalysisStage>;
  message?: string;
  error?: string | null;
  needs_part_outlines?: boolean;
  overall_progress?: number;
  started_at?: string;
  completed_at?: string;
  cancelled_at?: string;
}

export interface ImportProjectAnalysisSummary {
  progress: ProjectImportAnalysisProgress;
  analyzed_chapters: number;
  outlined_chapters: number;
  generated_parts: number;
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
