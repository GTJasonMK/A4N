import { apiInvoke } from "./index";
import type {
  CleanupOrphanProjectDataDirsInput,
  OrphanProjectDataDirsSummary,
  StoragePathsSummary,
} from "./generated/host";

export type {
  CleanupOrphanProjectDataDirsInput,
  OrphanProjectDataDirsSummary,
  StoragePathsSummary,
} from "./generated/host";

export async function getStoragePaths(): Promise<StoragePathsSummary> {
  return apiInvoke<StoragePathsSummary>("get_storage_paths");
}

export async function listOrphanProjectDataDirs(): Promise<OrphanProjectDataDirsSummary> {
  return apiInvoke<OrphanProjectDataDirsSummary>("list_orphan_project_data_dirs");
}

export async function cleanupOrphanProjectDataDirs(
  input: CleanupOrphanProjectDataDirsInput,
): Promise<OrphanProjectDataDirsSummary> {
  return apiInvoke<OrphanProjectDataDirsSummary>("cleanup_orphan_project_data_dirs", {
    input,
  });
}
