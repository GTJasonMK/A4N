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

export interface ParsedChapter {
  number: number;
  title: string;
  content: string;
  content_hash: string | null;
}

export interface ImportFileResult {
  chapters: ParsedChapter[];
  detected_encoding: string;
  total_words: number;
  warnings: string[];
}

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

export interface DedupResult {
  content_hash: string;
  is_duplicate: boolean;
  closest_distance: number;
  matched_chapter_id: number | null;
}

export interface ImportUploadedFileInput {
  file_name: string;
  file_type: string;
  data_base64: string;
}

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

export interface CheckImportDedupInput {
  content: string;
  project_id: number;
}

export interface ReverseEngineerInput {
  project_id: number;
  content_sample: string;
  auto_context?: boolean;
  context_limit?: number | null;
  variables?: Record<string, unknown> | null;
}
