import { useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { exportAllConfigsJson, importAllConfigs } from "../../../api/config";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { useAuthStore } from "../../../store/auth";
import { loadAndApplyActiveTheme } from "../../../theme/activeTheme";
import { downloadTextFile } from "../../../utils/downloadFile";

const FULL_CONFIG_SCOPE =
  "LLM / Embedding / Image / Video / Prompts / Themes / SystemSettings";

const createDefaultConfigFileName = () => {
  const timestamp = new Date()
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\.\d{3}Z$/, "");
  return `a4n-config-${timestamp}.json`;
};

export const useFullConfigBackup = () => {
  const { addToast } = useToast();
  const { user } = useAuthStore();
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
      const fileName = createDefaultConfigFileName();
      const settingsJson = await exportAllConfigsJson();
      if (!isCurrentUser(requestUserId)) return;
      downloadTextFile(
        fileName,
        settingsJson,
        "application/json;charset=utf-8",
      );
      setLastExportPath(fileName);
      addToast(`已导出全量配置文件：${fileName}`, "success");
    } catch (err) {
      if (isCurrentUser(requestUserId)) {
        addToast(extractErrorMessage(err, "导出全量配置失败"), "error");
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
    let errorFallback = "读取配置文件失败";
    try {
      const settingsJson = await file.text();
      if (!isCurrentUser(requestUserId)) return;

      const trimmedSettingsJson = settingsJson.trim();
      if (!trimmedSettingsJson) {
        addToast("配置文件内容为空", "error");
        return;
      }

      const ok = await confirmDialog({
        title: "导入全量配置",
        message: `将从文件「${file.name}」导入全项目配置（${FULL_CONFIG_SCOPE}），${
          requestOverwrite ? "覆盖同名项" : "保留同名项（仅插入新项）"
        }，确认继续？`,
        confirmText: "导入",
        dialogType: "warning",
      });
      if (!ok) return;
      if (!isCurrentUser(requestUserId)) return;

      errorFallback = "导入全量配置失败";
      const details = await importAllConfigs({
        settings_json: trimmedSettingsJson,
        overwrite: requestOverwrite,
      });
      if (!isCurrentUser(requestUserId)) return;
      addToast(`全量导入完成：${details.join("；")}`, "success");
      await loadAndApplyActiveTheme(requestUserId);
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
