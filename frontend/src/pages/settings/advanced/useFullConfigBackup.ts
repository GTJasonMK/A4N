import { useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { exportAllConfigsJson, importAllConfigs } from "../../../api/config";
import { exportLocalRuntimeSettingsJson, importLocalRuntimeSettingsJson } from "../../../api/localRuntimeSettings";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { useAuthStore } from "../../../store/auth";
import { loadAndApplyActiveTheme } from "../../../theme/activeTheme";
import { downloadTextFile } from "../../../utils/downloadFile";
import { useI18n } from "../../../i18n";

export type ConfigBackupScope = "local" | "server";

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
  const { t } = useI18n();
  const [overwrite, setOverwrite] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [lastExportPath, setLastExportPath] = useState("");
  const mountedRef = useRef(true);
  const activeUserIdRef = useRef<number | null>(user?.id ?? null);
  const operationInFlightRef = useRef(false);

  activeUserIdRef.current = user?.id ?? null;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      operationInFlightRef.current = false;
    };
  }, []);

  const isCurrentUser = (userId: number | null) =>
    mountedRef.current && activeUserIdRef.current === userId;

  const handleExport = async () => {
    if (operationInFlightRef.current) return;
    operationInFlightRef.current = true;
    const requestUserId = user?.id ?? null;
    setExporting(true);
    try {
      const fileName = createDefaultConfigFileName(scope);
      const settingsJson = scope === "local"
        ? exportLocalRuntimeSettingsJson()
        : await exportAllConfigsJson();
      if (!isCurrentUser(requestUserId)) return;
      downloadTextFile(
        fileName,
        settingsJson,
        "application/json;charset=utf-8",
      );
      setLastExportPath(fileName);
      addToast(t("settingsAdvanced.exportedFile", { name: fileName }), "success");
    } catch (err) {
      if (isCurrentUser(requestUserId)) {
        addToast(extractErrorMessage(err, t("settingsAdvanced.exportFailed")), "error");
      }
    } finally {
      operationInFlightRef.current = false;
      if (mountedRef.current) setExporting(false);
    }
  };

  const handleImportFile = async (file: File) => {
    if (operationInFlightRef.current) return;
    operationInFlightRef.current = true;
    setImporting(true);
    const requestOverwrite = overwrite;
    const requestUserId = user?.id ?? null;
    let errorFallback = t("settingsAdvanced.readFileFailed");
    try {
      const settingsJson = await file.text();
      if (!isCurrentUser(requestUserId)) return;

      const trimmedSettingsJson = settingsJson.trim();
      if (!trimmedSettingsJson) {
        addToast(t("settingsAdvanced.emptyFile"), "error");
        return;
      }

      const ok = await confirmDialog({
        title: t("settingsAdvanced.importTitle"),
        message: t("settingsAdvanced.importMessage", {
          name: file.name,
          scope: t(scope === "local" ? "settingsAdvanced.localScope" : "settingsAdvanced.serverScope"),
          mode: scope === "local"
            ? t("settingsAdvanced.replaceLocal")
            : requestOverwrite ? t("settingsAdvanced.overwrite") : t("settingsAdvanced.keepExisting"),
        }),
        confirmText: t("settingsAdvanced.import"),
        dialogType: "warning",
      });
      if (!ok) return;
      if (!isCurrentUser(requestUserId)) return;

      errorFallback = t("settingsAdvanced.importFailed");
      let details: string[];
      if (scope === "local") {
        await importLocalRuntimeSettingsJson(trimmedSettingsJson);
        details = [t("settingsAdvanced.localRestored")];
      } else {
        details = await importAllConfigs({
          settings_json: trimmedSettingsJson,
          overwrite: requestOverwrite,
        });
      }
      if (!isCurrentUser(requestUserId)) return;
      if (scope === "server") await loadAndApplyActiveTheme(requestUserId);
      addToast(t("settingsAdvanced.importedDetails", { details: details.join(t("common.summarySeparator")) }), "success");
    } catch (err) {
      if (isCurrentUser(requestUserId)) {
        addToast(extractErrorMessage(err, errorFallback), "error");
      }
    } finally {
      operationInFlightRef.current = false;
      if (mountedRef.current) setImporting(false);
    }
  };

  return {
    overwrite,
    exporting,
    importing,
    lastExportPath,
    setOverwrite,
    handleExport,
    handleImportFile,
  };
};
