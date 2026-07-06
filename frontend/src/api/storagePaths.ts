import { apiInvoke } from "./index";

export interface StoragePathsSummary {
  application_home: string;
  data_dir: string;
  config_file: string;
  database_url: string;
  projects_dir: string;
  exports_dir: string;
  backups_dir: string;
  cache_dir: string;
  embedding_models_dir: string;
  temp_dir: string;
  log_dir: string;
  webview_dir: string;
  frontend_state_file: string;
  prompts_dir: string;
  prompts_git_dir: string;
  lance_dir: string;
  tantivy_dir: string;
}

export interface OrphanProjectDataDirSummary {
  project_id: number;
  path: string;
  size_bytes: number;
}

export interface OrphanProjectDataDirsSummary {
  items: OrphanProjectDataDirSummary[];
  total_count: number;
  total_size_bytes: number;
}

export async function getStoragePaths(): Promise<StoragePathsSummary> {
  return apiInvoke<StoragePathsSummary>("get_storage_paths");
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
