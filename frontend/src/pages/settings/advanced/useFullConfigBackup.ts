import { useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import { exportAllConfigsJson, importAllConfigs } from "../../../api/theme";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { useAuthStore } from "../../../store/auth";
import { loadAndApplyActiveTheme } from "../../../theme/activeTheme";
import { downloadTextFile } from "../../../utils/downloadFile";

const FULL_CONFIG_SCOPE =
  "LLM / Embedding / Image / Video / Prompts / Themes / SystemSettings";

const createDefaultConfigFileName = () => {
  const timestamp = new Date().toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "");
  return `a4n-config-${timestamp}.json`;
};

export const useFullConfigBackup = (onImported?: () => Promise<void> | void) => {
  const { addToast } = useToast();
  const { user } = useAuthStore();
  const [overwrite, setOverwrite] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [lastExportPath, setLastExportPath] = useState("");
  const mountedRef = useRef(true);
  const exportSeqRef = useRef(0);
  const importSeqRef = useRef(0);
  const exportingRef = useRef(false);
  const importingRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      exportSeqRef.current += 1;
      importSeqRef.current += 1;
      exportingRef.current = false;
      importingRef.current = false;
    };
  }, []);

  const handleExport = async () => {
    if (exportingRef.current || importingRef.current) return;
    const requestSeq = exportSeqRef.current + 1;
    exportSeqRef.current = requestSeq;
    exportingRef.current = true;
    setExporting(true);
    try {
      const fileName = createDefaultConfigFileName();
      const settingsJson = await exportAllConfigsJson();
      if (!mountedRef.current || exportSeqRef.current !== requestSeq) return;
      downloadTextFile(fileName, settingsJson, "application/json;charset=utf-8");
      setLastExportPath(fileName);
      addToast(`已导出全量配置文件：${fileName}`, "success");
    } catch (err) {
      if (!mountedRef.current || exportSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "导出全量配置失败"), "error");
    } finally {
      if (exportSeqRef.current === requestSeq) exportingRef.current = false;
      if (mountedRef.current && exportSeqRef.current === requestSeq) setExporting(false);
    }
  };

  const handleImportFile = async (file: File) => {
    if (exportingRef.current || importingRef.current) return;
    const requestSeq = importSeqRef.current + 1;
    importSeqRef.current = requestSeq;
    importingRef.current = true;
    setImporting(true);
    const requestOverwrite = overwrite;
    const requestUserId = user?.id ?? null;
    let errorFallback = "读取配置文件失败";
    try {
      const settingsJson = await file.text();
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;

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
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;

      errorFallback = "导入全量配置失败";
      const result = await importAllConfigs(trimmedSettingsJson, requestOverwrite);
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      addToast(
        `${result.message}：${result.details.join("；") || "无明细"}`,
        result.success ? "success" : "error",
      );
      await onImported?.();
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      await loadAndApplyActiveTheme(requestUserId);
    } catch (err) {
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, errorFallback), "error");
    } finally {
      if (importSeqRef.current === requestSeq) importingRef.current = false;
      if (mountedRef.current && importSeqRef.current === requestSeq) setImporting(false);
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
