import type {
  ApplyImplicitUpdateDecisionResultSummary,
  AttributeEventSummary,
  BehaviorRecordSummary,
  CharacterProfileSummary,
  DeletionMarkSummary,
  ImplicitAttributeReviewSummary,
  ImplicitReviewSummary,
  ImplicitUpdateDecisionSummary,
  ProfileChapterStateSummary,
  ProfileSyncConflictSummary,
  RollbackResultSummary,
  SnapshotDeltaSummary,
  SnapshotReferenceSummary,
  SnapshotSummary,
} from "../generated/host";

export type CharacterProfile = CharacterProfileSummary;
export type AttributeEvent = AttributeEventSummary;
export type SnapshotReference = SnapshotReferenceSummary;
export type ProfileChapterState = ProfileChapterStateSummary;
export type Snapshot = SnapshotSummary;
export type SnapshotDelta = SnapshotDeltaSummary;
export type ProfileSyncConflict = ProfileSyncConflictSummary;
export type BehaviorRecord = BehaviorRecordSummary;
export type DeletionMark = DeletionMarkSummary;
export type ImplicitAttributeReview = ImplicitAttributeReviewSummary;
export type ImplicitReview = ImplicitReviewSummary;
export type ImplicitUpdateDecision = ImplicitUpdateDecisionSummary;
export type ApplyImplicitUpdateDecisionResult =
  ApplyImplicitUpdateDecisionResultSummary;
export type RollbackToChapterResult = RollbackResultSummary;

export interface DiscoverCharacterProfilesResult {
  status: string;
  chapters_analyzed: number;
  chapter_numbers: number[];
  discovered_count: number;
  created: Array<{ id: number }>;
  existing: Array<{
    name: string;
    aliases: unknown;
    reason: string;
  }>;
}
