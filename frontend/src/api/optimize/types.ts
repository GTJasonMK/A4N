export interface OptimizeSession {
  id: number;
  project_id: number;
  chapter_id: number;
  name: string | null;
  status: string;
  issues: unknown | null;
  agent_permissions: unknown | null;
  agent_instruction: string | null;
  fixes_applied: number;
  summary: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface OptimizeTurnEvent {
  id: number;
  turn_id: number;
  seq: number;
  kind: string;
  status: string;
  public_text: string;
  payload: unknown | null;
  created_at: string | null;
}

export interface OptimizeTurn {
  id: number;
  session_id: number;
  user_text: string;
  status: string;
  permissions_snapshot: unknown | null;
  final_reply: string | null;
  error: string | null;
  events: OptimizeTurnEvent[];
  created_at: string | null;
  updated_at: string | null;
}

export interface OptimizePatchSet {
  id: number;
  turn_id: number;
  session_id: number;
  chapter_id: number;
  base_content_hash: string;
  status: string;
  summary: string;
  patches: OptimizationPatch[];
  created_at: string | null;
  updated_at: string | null;
}

export interface OptimizationAgentPermissions {
  read_project_detail: boolean;
  read_blueprint: boolean;
  read_outlines: boolean;
  read_character_profiles: boolean;
  read_foreshadowing: boolean;
  automatic_writeback: boolean;
}

export interface AnalyzeChapterInput {
  project_id: number;
  chapter_number: number;
  name: string;
  mode: string;
  scope?: string | null;
  permissions?: OptimizationAgentPermissions | null;
  agent_instruction?: string | null;
}

export interface CreateOptimizationChatSessionInput {
  project_id: number;
  chapter_number: number;
  name: string;
  permissions?: OptimizationAgentPermissions | null;
}

export interface PrepareOptimizationAnalysisSessionInput {
  session_id: number;
  mode: string;
  scope?: string | null;
  permissions?: OptimizationAgentPermissions | null;
  agent_instruction?: string | null;
}

export interface OptimizationPatch {
  patch_id: string;
  issue_id: string | null;
  operation: "replace" | "delete" | "insert_before" | "insert_after";
  old_text: string;
  new_text: string;
  reason: string;
  risk: string | null;
}

export interface ProposeOptimizationPatchesResponse {
  session_id: number;
  project_id: number;
  chapter_id: number;
  base_content_hash: string;
  summary: string;
  patches: OptimizationPatch[];
}

export interface ApplyOptimizationPatchesResponse {
  session_id: number;
  chapter_id: number;
  applied_count: number;
  word_count: number;
  content_hash: string;
}

export interface SendOptimizationMessageResponse {
  session_id: number;
  turn: OptimizeTurn;
}

export interface ProposeOptimizationPatchesInput {
  session_id: number;
  issue_ids?: string[] | null;
  agent_instruction?: string | null;
}

export interface ApplyOptimizationPatchesInput {
  session_id: number;
  base_content_hash: string;
  patches: OptimizationPatch[];
  confirmed_apply: boolean;
}

export interface SendOptimizationMessageInput {
  session_id: number;
  message: string;
  permissions?: OptimizationAgentPermissions | null;
}
