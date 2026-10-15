import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { extractErrorMessage } from "../../../api";
import { exportAllConfigsJson, importAllConfigs } from "../../../api/config";
import { exportLocalRuntimeSettingsJson, importLocalRuntimeSettingsJson } from "../../../api/localRuntimeSettings";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { useAuthStore } from "../../../store/auth";
import { loadAndApplyActiveTheme } from "../../../theme/activeTheme";
import { downloadTextFile } from "../../../utils/downloadFile";
import { getActiveLocale, translateMessage, type MessageKey } from "../../../i18n";

const translateCurrent = (key: MessageKey, params?: Record<string, string | number>) =>
  translateMessage(getActiveLocale(), key, params);

type BackupOperation =
  | { status: "exporting" }
  | { status: "preparingImport"; overwrite: boolean }
  | { status: "importing"; overwrite: boolean };

const activeBackupOperations = new Map<string, BackupOperation>();
const backupOperationListeners = new Set<() => void>();

const subscribeBackupOperations = (listener: () => void) => {
  backupOperationListeners.add(listener);
  return () => backupOperationListeners.delete(listener);
};

const updateBackupOperation = (key: string, operation: BackupOperation | null) => {
  if (operation) activeBackupOperations.set(key, operation);
  else activeBackupOperations.delete(key);
  backupOperationListeners.forEach((listener) => listener());
};

export type ConfigBackupScope = "local" | "server";

const operationKey = (scope: ConfigBackupScope) => scope;

const createDefaultConfigFileName = (scope: ConfigBackupScope) => {
  const timestamp = new Date()
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\.\d{3}Z$/, "");
  return `a4n-${scope}-config-${timestamp}.json`;
};

export const useFullConfigBackup = (scope: ConfigBackupScope) => {
  const { addToast } = useToast();
  const { user } = useAuthStore();
  const [overwrite, setOverwrite] = useState(false);
  const [lastExportFileName, setLastExportFileName] = useState("");
  const mountedRef = useRef(true);
  const requestKey = operationKey(scope);
  const activeOperation = useSyncExternalStore(
    subscribeBackupOperations,
    () => activeBackupOperations.get(requestKey) ?? null,
    () => null,
  );
  const exporting = activeOperation?.status === "exporting";
  const preparingImport = activeOperation?.status === "preparingImport";
  const importing = activeOperation?.status === "importing";
  const visibleOverwrite = activeOperation && activeOperation.status !== "exporting"
    ? activeOperation.overwrite
    : overwrite;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const isSameUser = (userId: number | null) => useAuthStore.getState().user?.id === userId;

  const handleExport = async () => {
    const requestUserId = user?.id ?? null;
    if (activeBackupOperations.has(requestKey)) {
      addToast(translateCurrent("settingsAdvanced.operationInProgress"), "info");
      return;
    }
    updateBackupOperation(requestKey, { status: "exporting" });
    try {
      const fileName = createDefaultConfigFileName(scope);
      const settingsJson = scope === "local"
        ? exportLocalRuntimeSettingsJson()
        : await exportAllConfigsJson();
      if (!isSameUser(requestUserId)) return;
      downloadTextFile(
        fileName,
        settingsJson,
        "application/json;charset=utf-8",
      );
      if (mountedRef.current) setLastExportFileName(fileName);
      addToast(translateCurrent("settingsAdvanced.exportedFile", { name: fileName }), "success");
    } catch (err) {
      if (isSameUser(requestUserId)) {
        addToast(extractErrorMessage(err, translateCurrent("settingsAdvanced.exportFailed")), "error");
      }
    } finally {
      updateBackupOperation(requestKey, null);
    }
  };

  const handleImportFile = async (file: File) => {
    const requestUserId = user?.id ?? null;
    if (activeBackupOperations.has(requestKey)) {
      addToast(translateCurrent("settingsAdvanced.operationInProgress"), "info");
      return;
    }
    const requestOverwrite = overwrite;
    updateBackupOperation(requestKey, { status: "preparingImport", overwrite: requestOverwrite });
    let errorFallbackKey:
      | "settingsAdvanced.readFileFailed"
      | "settingsAdvanced.importFailed" = "settingsAdvanced.readFileFailed";
    try {
      const settingsJson = await file.text();
      if (!isSameUser(requestUserId)) return;

      const trimmedSettingsJson = settingsJson.trim();
      if (!trimmedSettingsJson) {
        addToast(translateCurrent("settingsAdvanced.emptyFile"), "error");
        return;
      }

      const ok = await confirmDialog({
        title: translateCurrent("settingsAdvanced.importTitle"),
        message: translateCurrent("settingsAdvanced.importMessage", {
          name: file.name,
          scope: translateCurrent(scope === "local" ? "settingsAdvanced.localScope" : "settingsAdvanced.serverScope"),
          mode: scope === "local"
            ? translateCurrent("settingsAdvanced.replaceLocal")
            : requestOverwrite
              ? translateCurrent("settingsAdvanced.overwrite")
              : translateCurrent("settingsAdvanced.keepExisting"),
        }),
        confirmText: translateCurrent("settingsAdvanced.import"),
        dialogType: "warning",
      });
      if (!ok) return;
      if (!isSameUser(requestUserId)) return;

      updateBackupOperation(requestKey, { status: "importing", overwrite: requestOverwrite });
      errorFallbackKey = "settingsAdvanced.importFailed";
      let details: string[];
      if (scope === "local") {
        await importLocalRuntimeSettingsJson(trimmedSettingsJson);
        details = [translateCurrent("settingsAdvanced.localRestored")];
      } else {
        details = await importAllConfigs({
          settings_json: trimmedSettingsJson,
          overwrite: requestOverwrite,
        });
      }
      if (!isSameUser(requestUserId)) return;
      if (scope === "server" && !await loadAndApplyActiveTheme(requestUserId)) {
        if (!isSameUser(requestUserId)) return;
        addToast(translateCurrent("settingsAdvanced.importedDetails", {
          details: details.join(translateCurrent("common.summarySeparator")),
        }), "info");
        addToast(translateCurrent("settingsAdvanced.themeRefreshFailed"), "error");
        return;
      }
      if (isSameUser(requestUserId)) {
        addToast(translateCurrent("settingsAdvanced.importedDetails", {
          details: details.join(translateCurrent("common.summarySeparator")),
        }), "success");
      }
    } catch (err) {
      if (isSameUser(requestUserId)) {
        addToast(extractErrorMessage(err, translateCurrent(errorFallbackKey)), "error");
      }
    } finally {
      updateBackupOperation(requestKey, null);
    }
  };

  return {
    overwrite: visibleOverwrite,
    exporting,
    preparingImport,
    importing,
    lastExportFileName,
    setOverwrite,
    handleExport,
    handleImportFile,
  };
};
