import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type {
  CheckProfileSyncConflictInput,
  DiffProfileChaptersInput,
  GetProfileStateAtChapterInput,
  GetSnapshotInput,
  ListSnapshotsInput,
  RollbackToChapterInput,
} from "../generated/host";
import type {
  ProfileChapterState,
  ProfileSyncConflict,
  RollbackToChapterResult,
  Snapshot,
  SnapshotDelta,
  SnapshotReference,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";

export async function listSnapshots(
  profileId: Id,
  filters: {
    start_chapter?: number | null;
    end_chapter?: number | null;
  } = {},
): Promise<SnapshotReference[]> {
  return apiInvoke<SnapshotReference[]>("list_snapshots", {
    userId: currentUserId(),
    input: {
      profile_id: idNum(profileId),
      start_chapter: filters.start_chapter ?? null,
      end_chapter: filters.end_chapter ?? null,
    } satisfies ListSnapshotsInput,
  });
}

export async function getProfileStateAtChapter(
  profileId: Id,
  chapterNumber: number,
): Promise<ProfileChapterState> {
  return apiInvoke<ProfileChapterState>("get_profile_state_at_chapter", {
    userId: currentUserId(),
    input: {
      profile_id: idNum(profileId),
      chapter_number: chapterNumber,
    } satisfies GetProfileStateAtChapterInput,
  });
}

export async function getSnapshot(contentHash: string): Promise<Snapshot> {
  return apiInvoke<Snapshot>("get_snapshot", {
    userId: currentUserId(),
    input: { content_hash: contentHash } satisfies GetSnapshotInput,
  });
}

export async function diffProfileChapters(
  profileId: Id,
  fromChapter: number,
  toChapter: number,
): Promise<SnapshotDelta> {
  return apiInvoke<SnapshotDelta>("diff_profile_chapters", {
    userId: currentUserId(),
    input: {
      profile_id: idNum(profileId),
      from_chapter: fromChapter,
      to_chapter: toChapter,
    } satisfies DiffProfileChaptersInput,
  });
}

export async function checkProfileSyncConflict(
  profileId: Id,
): Promise<ProfileSyncConflict> {
  return apiInvoke<ProfileSyncConflict>("check_profile_sync_conflict", {
    userId: currentUserId(),
    input: {
      profile_id: idNum(profileId),
    } satisfies CheckProfileSyncConflictInput,
  });
}

export async function rollbackToChapter(
  profileId: Id,
  chapterNumber: number,
): Promise<RollbackToChapterResult> {
  return apiInvoke<RollbackToChapterResult>("rollback_to_chapter", {
    userId: currentUserId(),
    input: {
      profile_id: idNum(profileId),
      chapter_number: chapterNumber,
    } satisfies RollbackToChapterInput,
  });
}
