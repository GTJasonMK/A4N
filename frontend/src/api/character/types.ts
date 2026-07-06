interface AttributeCounts {
  explicit: number;
  implicit: number;
  social: number;
}

export interface CharacterProfile {
  id: number;
  project_id: number;
  name: string;
  gender: string | null;
  age: string | null;
  base_attributes: unknown;
  current_state: unknown;
  character_arc: unknown | null;
  voice_fingerprint: unknown | null;
  appearance: string | null;
  background: string | null;
  personality: string | null;
  last_synced_chapter: number;
  attribute_counts: AttributeCounts;
  created_at: string | null;
  updated_at: string | null;
}

export interface AttributeEvent {
  id: number;
  profile_id: number;
  chapter_number: number;
  timestamp: string | null;
  category: string;
  key: string;
  operation: string;
  change_description: string;
  old_value: unknown;
  new_value: unknown;
  cause: string;
  evidence: string;
}

export interface AttributeMutationResult {
  event: AttributeEvent;
}

export interface SnapshotReference {
  id: number;
  profile_id: number;
  chapter_number: number;
  snapshot_hash: string;
  reference_count: number;
  attribute_counts: AttributeCounts;
  state_summary: string | null;
  current_location: string | null;
  current_goal: string | null;
  emotional_state: string | null;
  changes_in_chapter: number;
  behaviors_in_chapter: number;
  created_at: string | null;
}

export interface ProfileChapterState {
  id: number;
  profile_id: number;
  chapter_number: number;
  snapshot_hash: string;
  attribute_counts: AttributeCounts;
  explicit_attributes: unknown;
  implicit_attributes: unknown;
  social_attributes: unknown;
  state_summary: string | null;
  current_location: string | null;
  current_goal: string | null;
  emotional_state: string | null;
  changes_in_chapter: number;
  behaviors_in_chapter: number;
  created_at: string | null;
}

export interface Snapshot {
  content_hash: string;
  entity_type: string;
  entity_id: number;
  sequence: number;
  data: unknown;
  created_at: string | null;
}

export interface SnapshotDelta {
  from_hash: string;
  to_hash: string;
  delta: unknown;
}

export interface ProfileSyncConflict {
  profile_id: number;
  has_conflict: boolean;
  last_synced_chapter: number;
  max_available_chapter: number;
  available_snapshot_chapters: number[];
}

export interface BehaviorRecord {
  id: number;
  profile_id: number;
  chapter_number: number;
  behavior_description: string;
  behavior_type: string;
  original_text: string;
  behavior_tags: unknown;
  classification_results: unknown;
  related_characters: unknown | null;
  impact_analysis: string | null;
  created_at: string | null;
}

export interface DeletionMark {
  id: number;
  profile_id: number;
  category: string;
  key: string;
  chapter_number: number;
  reason: string;
  evidence: string;
  consecutive_count: number;
  threshold: number;
  last_marked_chapter: number;
  is_executed: boolean;
  created_at: string;
  updated_at: string;
}

interface ImplicitEvidence {
  behavior_id: number;
  chapter_number: number;
  behavior_type: string;
  behavior_description: string;
  original_text: string;
  behavior_tags: unknown;
  classification: unknown;
  created_at: string | null;
}

export interface ImplicitAttributeReview {
  attribute_key: string;
  current_value: unknown;
  total: number;
  conform_count: number;
  non_conform_count: number;
  conform_rate: number;
  threshold_reached: boolean;
  evidence: ImplicitEvidence[];
}

export interface ImplicitReview {
  profile_id: number;
  threshold: number;
  window_chapters: number;
  evidence_limit: number;
  attributes: ImplicitAttributeReview[];
  needs_review: string[];
}

export interface ImplicitUpdateDecision {
  profile_id: number;
  attribute_key: string;
  current_value: unknown;
  decision: "keep" | "modify" | "delete";
  reasoning: string;
  suggested_new_value: unknown | null;
  evidence_summary: string;
  stats: ImplicitAttributeReview;
}

export interface ApplyImplicitUpdateDecisionResult {
  action: "keep" | "modify" | "mark_deletion";
  event: AttributeEvent | null;
  deletion_mark: DeletionMark | null;
  ready_for_deletion: boolean;
  threshold: number;
}

export interface DiscoverCharacterProfilesResult {
  status: string;
  chapters_analyzed: number;
  chapter_numbers: number[];
  discovered_count: number;
  created: CharacterProfile[];
  existing: Array<{
    name: string;
    aliases: unknown;
    reason: string;
  }>;
}

export interface SyncCharacterProfileResult {
  status: string;
  name: string;
  chapters_analyzed: number;
  sync_range: {
    from: number;
    to: number;
  };
  synced_chapter: number;
  changes_applied: number;
  behaviors_recorded: number;
  classification_suggestions_applied: number;
  deletions_marked: number;
  deletions_executed: number;
  deleted_deletion_marks: number;
  rewound_deletion_marks: number;
  implicit_reviews_checked: number;
  implicit_reviews_applied: number;
  snapshot_hash: string;
  state_indexes_upserted: number;
  deleted_state_rag: number;
  state_rag_indexed_count: number;
  state_rag_indexed: boolean;
  state_rag_error: string | null;
  deleted_attribute_event_rag: number;
  attribute_event_rag_indexed_count: number;
  attribute_event_rag_indexed: boolean;
  attribute_event_rag_error: string | null;
}

export interface SyncAllCharacterProfilesResult {
  status: string;
  target_chapter: number | null;
  total: number;
  synced: Array<{
    name: string;
    result: SyncCharacterProfileResult;
  }>;
  skipped: Array<{
    name: string;
    synced_chapter: number;
    reason: string;
  }>;
  failed: Array<{
    name: string;
    error: string;
  }>;
}

export interface DeleteProfileResult {
  deleted_state_rag: number;
  deleted_attribute_event_rag: number;
}

export interface RollbackToChapterResult {
  deleted_events: number;
  deleted_snapshots: number;
  deleted_behaviors: number;
  deleted_state_indexes: number;
  deleted_state_rag: number;
  deleted_attribute_event_rag: number;
  deleted_deletion_marks: number;
  rewound_deletion_marks: number;
  last_synced_chapter: number;
}
