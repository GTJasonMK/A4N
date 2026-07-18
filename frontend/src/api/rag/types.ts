export interface RagSearchResult {
  id: string;
  content: string;
  score: number;
  vector_score: number | null;
  bm25_score: number | null;
  metadata: unknown;
}

export interface RagEnhancedQuerySummary {
  kind: string;
  query: string;
  weight: number;
}

export interface RagEnhancedSearchOutput {
  results: RagSearchResult[];
  queries: RagEnhancedQuerySummary[];
  candidate_count: number;
}

export interface RagPreparedEnhancedSearchOutput {
  queries: RagEnhancedQuerySummary[];
  query_texts: string[];
  embedding_identity: {
    backend: string;
    model: string;
    dimension: number;
  };
  top_k: number;
  pull_k: number;
}

export interface RagIngestResult {
  status: string;
  item_count?: number;
  chunk_count: number;
  added_count: number;
  updated_count: number;
  skipped_count: number;
  deleted_count: number;
  success: boolean;
  errors: string[];
  chapter_count?: number;
  foreshadowing_count?: number;
  outline_count?: number;
  part_outline_count?: number;
  blueprint_count?: number;
  blueprint_metadata_count?: number;
  world_setting_count?: number;
  character_count?: number;
  protagonist_count?: number;
  character_state_count?: number;
  character_attribute_event_count?: number;
  relationship_count?: number;
  scene_profile_count?: number;
  scene_occurrence_count?: number;
  scene_transition_count?: number;
}

export interface PreparedClientRagIngestRecord {
  chunk_id: string;
  text: string;
  content_hash: string;
  outcome: string;
}

export interface ClientRagIngestPrepareOutput {
  project_id: number;
  operation_trace_id: string;
  embedding_identity: {
    backend: string;
    model: string;
    dimension: number;
  };
  records: PreparedClientRagIngestRecord[];
  expected_count: number;
  skipped_count: number;
}
