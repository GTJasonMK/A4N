import { apiInvoke } from "./index";
import type {
  OrphanProjectDataDirsSummary,
  StoragePathsSummary,
} from "./generated/host";

export type {
  OrphanProjectDataDirsSummary,
  StoragePathsSummary,
} from "./generated/host";

export async function getStoragePaths(
  callerUserId: number,
): Promise<StoragePathsSummary> {
  return apiInvoke<StoragePathsSummary>("get_storage_paths", { callerUserId });
}

export async function listOrphanProjectDataDirs(
  callerUserId: number,
): Promise<OrphanProjectDataDirsSummary> {
  return apiInvoke<OrphanProjectDataDirsSummary>(
    "list_orphan_project_data_dirs",
    {
      callerUserId,
    },
  );
}

export async function cleanupOrphanProjectDataDirs(
  callerUserId: number,
): Promise<OrphanProjectDataDirsSummary> {
  return apiInvoke<OrphanProjectDataDirsSummary>(
    "cleanup_orphan_project_data_dirs",
    {
      input: { caller_user_id: callerUserId },
    },
  );
}
