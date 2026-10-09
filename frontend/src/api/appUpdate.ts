import { Channel, invoke } from "@tauri-apps/api/core";
import { isDesktopTauriRuntime, shouldUseHttpBackend } from "../utils/runtimeMode";

export interface AppUpdateInfo {
  version: string;
  notes: string | null;
  pubDate: string | null;
  target: string;
}

export interface AppUpdateProgress {
  stage: "preparing" | "downloading" | "installing";
  downloadedBytes: number;
  totalBytes: number | null;
}

export interface AppUpdatePreflight {
  backupPath: string | null;
  migrations: {
    name: string;
    affectedProjects: number;
    mediaFilesToCopy: number;
  }[];
}

export interface AppUpdateRecoveryState {
  state: string;
  backupPath: string | null;
  targetVersion: string;
  previousVersion: string;
  pendingMigrations: string[];
  failureReason: string | null;
}

interface UpdateProgressEvent {
  event: "preparing" | "started" | "progress" | "finished";
  data?: { migrationCount?: number; contentLength?: number | null; chunkLength?: number };
}

function canUpdate(): boolean {
  return isDesktopTauriRuntime() && !shouldUseHttpBackend();
}

let checkedForAppUpdate = false;
let cachedAppUpdate: AppUpdateInfo | null = null;
let appUpdateCheck: Promise<AppUpdateInfo | null> | null = null;

export async function checkForAppUpdate(force = false): Promise<AppUpdateInfo | null> {
  if (!canUpdate()) return null;
  if (appUpdateCheck) return appUpdateCheck;
  if (!force && checkedForAppUpdate) return cachedAppUpdate;

  const request = invoke<AppUpdateInfo | null>("check_app_update");
  appUpdateCheck = request;
  try {
    cachedAppUpdate = await request;
    checkedForAppUpdate = true;
    return cachedAppUpdate;
  } finally {
    if (appUpdateCheck === request) appUpdateCheck = null;
  }
}

export async function prepareAppUpdateInstall(version: string): Promise<AppUpdatePreflight> {
  if (!canUpdate()) throw new Error("桌面端更新只在本地 Tauri 模式可用");
  return invoke<AppUpdatePreflight>("prepare_app_update_install", { expectedVersion: version });
}

export async function downloadPreparedAppUpdate(
  version: string,
  onProgress: (progress: AppUpdateProgress) => void,
): Promise<void> {
  if (!canUpdate()) throw new Error("桌面端更新只在本地 Tauri 模式可用");

  let downloadedBytes = 0;
  let totalBytes: number | null = null;
  const progress = new Channel<UpdateProgressEvent>();
  progress.onmessage = (message) => {
    switch (message.event) {
      case "preparing":
        onProgress({ stage: "preparing", downloadedBytes, totalBytes });
        break;
      case "started":
        downloadedBytes = message.data?.chunkLength ?? 0;
        totalBytes = message.data?.contentLength ?? null;
        onProgress({ stage: "downloading", downloadedBytes, totalBytes });
        break;
      case "progress":
        downloadedBytes += message.data?.chunkLength ?? 0;
        onProgress({ stage: "downloading", downloadedBytes, totalBytes });
        break;
      case "finished":
        if (totalBytes !== null) downloadedBytes = totalBytes;
        onProgress({ stage: "downloading", downloadedBytes, totalBytes });
        break;
    }
  };
  await invoke("download_app_update", { expectedVersion: version, progress });
}

export async function installDownloadedAppUpdate(version: string): Promise<void> {
  if (!canUpdate()) throw new Error("桌面端更新只在本地 Tauri 模式可用");
  await invoke("install_app_update", { expectedVersion: version });
}

export async function getAppUpdateRecoveryState(): Promise<AppUpdateRecoveryState | null> {
  if (!canUpdate()) return null;
  return invoke<AppUpdateRecoveryState | null>("get_update_recovery_state");
}

export async function clearAppUpdateRecoveryState(): Promise<void> {
  if (!canUpdate()) throw new Error("桌面端更新只在本地 Tauri 模式可用");
  await invoke("clear_update_recovery_state");
}
