import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import {
  activateTheme,
  createTheme,
  deleteTheme,
  exportTheme,
  getSystemDefaultTheme,
  getThemeCssVariables,
  importTheme,
  listThemes,
  listUserThemes,
  updateTheme,
  type Theme,
} from "../../../api/theme";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { useAuthStore } from "../../../store/auth";
import { applyActiveTheme } from "../../../theme/activeTheme";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { PRESET_VARS_DARK, PRESET_VARS_LIGHT } from "./presets";
import { type ThemeMode, type ThemeScope } from "./types";

export const useThemeTabController = () => {
  const { addToast } = useToast();
  const { user } = useAuthStore();
  const [themes, setThemes] = useState<Theme[]>([]);
  const [scope, setScope] = useState<ThemeScope>("all");
  const [loading, setLoading] = useState(false);
  const [activating, setActivating] = useState<number | null>(null);
  const [deleting, setDeleting] = useState<number | null>(null);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newMode, setNewMode] = useState<ThemeMode>("light");
  const [renamingId, setRenamingId] = useState<number | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [exportingId, setExportingId] = useState<number | null>(null);
  const [importing, setImporting] = useState(false);
  const [importText, setImportText] = useState("");
  const [defaultTheme, setDefaultTheme] = useState<Theme | null>(null);
  const scopeKey = `${scope}:${user?.id ?? "anonymous"}`;
  const mountedRef = useRef(true);
  const activeScopeKeyRef = useRef(scopeKey);
  const refreshSeqRef = useRef(0);
  const refreshInFlightScopeRef = useRef<string | null>(null);
  const activateSeqRef = useRef(0);
  const activatingRef = useRef<number | null>(null);
  const createSeqRef = useRef(0);
  const creatingRef = useRef(false);
  const deleteSeqRef = useRef(0);
  const deletingRef = useRef<number | null>(null);
  const exportSeqRef = useRef(0);
  const exportingRef = useRef<number | null>(null);
  const cssSeqRef = useRef(0);
  const cssRef = useRef<number | null>(null);
  const importSeqRef = useRef(0);
  const importingRef = useRef(false);
  const renameSeqRef = useRef(0);
  const renamingRef = useRef<number | null>(null);
  const renamingIdRef = useRef<number | null>(renamingId);
  const renameValueRef = useRef(renameValue);

  activeScopeKeyRef.current = scopeKey;
  renamingIdRef.current = renamingId;
  renameValueRef.current = renameValue;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshInFlightScopeRef.current = null;
      activateSeqRef.current += 1;
      activatingRef.current = null;
      createSeqRef.current += 1;
      creatingRef.current = false;
      deleteSeqRef.current += 1;
      deletingRef.current = null;
      exportSeqRef.current += 1;
      exportingRef.current = null;
      cssSeqRef.current += 1;
      cssRef.current = null;
      importSeqRef.current += 1;
      importingRef.current = false;
      renameSeqRef.current += 1;
      renamingRef.current = null;
    };
  }, []);

  const isCurrentScope = useCallback((requestScopeKey: string) => {
    return mountedRef.current && activeScopeKeyRef.current === requestScopeKey;
  }, []);

  const isCurrentRefresh = useCallback((requestScopeKey: string, seq: number) => {
    return isCurrentScope(requestScopeKey) && refreshSeqRef.current === seq;
  }, [isCurrentScope]);

  const refresh = useCallback(async (options?: { force?: boolean }) => {
    const requestScopeKey = scopeKey;
    if (!options?.force && refreshInFlightScopeRef.current === requestScopeKey) return;
    refreshInFlightScopeRef.current = requestScopeKey;
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const listPromise = user
        ? scope === "user"
          ? listUserThemes(user.id)
          : listThemes(user.id)
        : Promise.resolve([]);
      const [data, systemDefaultTheme] = await Promise.all([
        listPromise,
        getSystemDefaultTheme(),
      ]);
      if (!isCurrentRefresh(requestScopeKey, requestSeq)) return;
      setThemes(data);
      setDefaultTheme(systemDefaultTheme);
    } catch (err) {
      if (!isCurrentRefresh(requestScopeKey, requestSeq)) return;
      addToast(extractErrorMessage(err, "加载主题列表失败"), "error");
    } finally {
      if (
        refreshInFlightScopeRef.current === requestScopeKey &&
        refreshSeqRef.current === requestSeq
      ) {
        refreshInFlightScopeRef.current = null;
      }
      if (isCurrentRefresh(requestScopeKey, requestSeq)) setLoading(false);
    }
  }, [addToast, isCurrentRefresh, scope, scopeKey, user]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    exportSeqRef.current += 1;
    exportingRef.current = null;
    cssSeqRef.current += 1;
    cssRef.current = null;
    if (refreshInFlightScopeRef.current !== scopeKey) {
      refreshInFlightScopeRef.current = null;
    }
    deleteSeqRef.current += 1;
    deletingRef.current = null;
    renameSeqRef.current += 1;
    renamingRef.current = null;
    setThemes([]);
    setDefaultTheme(null);
    setActivating(null);
    setDeleting(null);
    setExportingId(null);
    setRenamingId(null);
    setRenameValue("");
  }, [scopeKey]);

  useEffect(() => {
    if (renamingId != null && !themes.some((theme) => theme.id === renamingId)) {
      setRenamingId(null);
      setRenameValue("");
    }
  }, [renamingId, themes]);

  const copyToClipboard = useCallback(
    async (text: string, failureMessage: string): Promise<boolean> => {
      try {
        await copyTextToClipboard(text);
        return true;
      } catch (err) {
        addToast(extractErrorMessage(err, failureMessage), "error");
        return false;
      }
    },
    [addToast],
  );

  const handleActivate = async (id: number) => {
    if (activatingRef.current != null) {
      addToast("已有主题激活操作进行中，请等待完成", "error");
      return;
    }
    const requestSeq = activateSeqRef.current + 1;
    activateSeqRef.current = requestSeq;
    activatingRef.current = id;
    setActivating(id);
    try {
      const activated = await activateTheme(id);
      if (!mountedRef.current || activateSeqRef.current !== requestSeq) return;
      applyActiveTheme(activated);
      addToast("已激活主题", "success");
      await refresh({ force: true });
    } catch (err) {
      if (!mountedRef.current || activateSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "激活失败"), "error");
    } finally {
      if (mountedRef.current && activateSeqRef.current === requestSeq) {
        activatingRef.current = null;
        setActivating(null);
      }
    }
  };

  const handleDelete = async (id: number, name: string) => {
    if (deletingRef.current != null) return;
    deletingRef.current = id;
    const ok = await confirmDialog({
      title: "删除主题",
      message: `删除主题"${name}"？此操作不可撤销。`,
      confirmText: "删除",
      dialogType: "danger",
    });
    if (!ok) {
      if (deletingRef.current === id) deletingRef.current = null;
      return;
    }
    const requestScopeKey = scopeKey;
    if (!isCurrentScope(requestScopeKey)) {
      if (deletingRef.current === id) deletingRef.current = null;
      return;
    }
    const requestSeq = deleteSeqRef.current + 1;
    deleteSeqRef.current = requestSeq;
    setDeleting(id);
    try {
      await deleteTheme(id);
      if (!isCurrentScope(requestScopeKey) || deleteSeqRef.current !== requestSeq) return;
      addToast("已删除主题", "success");
      await refresh({ force: true });
    } catch (err) {
      if (!isCurrentScope(requestScopeKey) || deleteSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "删除失败"), "error");
    } finally {
      if (deleteSeqRef.current === requestSeq) deletingRef.current = null;
      if (isCurrentScope(requestScopeKey) && deleteSeqRef.current === requestSeq) setDeleting(null);
    }
  };

  const handleCreate = async () => {
    if (creatingRef.current) return;
    if (!newName.trim()) {
      addToast("请输入主题名称", "error");
      return;
    }
    const trimmedName = newName.trim();
    const requestSeq = createSeqRef.current + 1;
    createSeqRef.current = requestSeq;
    creatingRef.current = true;
    const requestMode = newMode;
    setCreating(true);
    try {
      await createTheme({
        name: trimmedName,
        mode: requestMode,
        css_variables: requestMode === "dark" ? PRESET_VARS_DARK : PRESET_VARS_LIGHT,
      });
      if (!mountedRef.current || createSeqRef.current !== requestSeq) return;
      addToast(`已创建主题：${trimmedName}`, "success");
      setNewName((current) => (current.trim() === trimmedName ? "" : current));
      await refresh({ force: true });
    } catch (err) {
      if (!mountedRef.current || createSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "创建主题失败"), "error");
    } finally {
      if (createSeqRef.current === requestSeq) creatingRef.current = false;
      if (mountedRef.current && createSeqRef.current === requestSeq) {
        setCreating(false);
      }
    }
  };

  const handleStartRename = (theme: Theme) => {
    if (renamingRef.current != null) return;
    setRenamingId(theme.id);
    setRenameValue(theme.name);
  };

  const handleConfirmRename = async () => {
    if (renamingId == null) return;
    if (renamingRef.current != null) return;
    if (!renameValue.trim()) {
      addToast("名称不能为空", "error");
      return;
    }
    const requestScopeKey = scopeKey;
    const requestRenamingId = renamingId;
    const requestName = renameValue.trim();
    const requestSeq = renameSeqRef.current + 1;
    renameSeqRef.current = requestSeq;
    renamingRef.current = requestRenamingId;
    try {
      await updateTheme(requestRenamingId, { name: requestName });
      if (
        !isCurrentScope(requestScopeKey) ||
        renameSeqRef.current !== requestSeq
      ) {
        return;
      }
      if (
        renamingIdRef.current === requestRenamingId &&
        renameValueRef.current.trim() === requestName
      ) {
        addToast("已重命名", "success");
        setRenamingId(null);
      } else if (renamingIdRef.current === requestRenamingId) {
        addToast("已重命名，后续修改仍保留为未保存名称", "success");
      } else {
        addToast("已重命名", "success");
      }
      await refresh({ force: true });
    } catch (err) {
      if (
        !isCurrentScope(requestScopeKey) ||
        renameSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, "重命名失败"), "error");
    } finally {
      if (renameSeqRef.current === requestSeq) renamingRef.current = null;
    }
  };

  const handleExport = async (id: number) => {
    if (exportingRef.current != null) return;
    const requestSeq = exportSeqRef.current + 1;
    exportSeqRef.current = requestSeq;
    exportingRef.current = id;
    setExportingId(id);
    try {
      const json = await exportTheme(id);
      if (!mountedRef.current || exportSeqRef.current !== requestSeq) return;
      const copied = await copyToClipboard(json, "主题 JSON 已生成，但复制到剪贴板失败");
      if (!mountedRef.current || exportSeqRef.current !== requestSeq) return;
      if (copied) addToast("主题 JSON 已复制到剪贴板", "success");
    } catch (err) {
      if (!mountedRef.current || exportSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "导出失败"), "error");
    } finally {
      if (exportSeqRef.current === requestSeq) exportingRef.current = null;
      if (mountedRef.current && exportSeqRef.current === requestSeq) setExportingId(null);
    }
  };

  const handleViewCss = async (id: number) => {
    if (cssRef.current != null) return;
    const requestSeq = cssSeqRef.current + 1;
    cssSeqRef.current = requestSeq;
    cssRef.current = id;
    try {
      const css = await getThemeCssVariables(id);
      if (!mountedRef.current || cssSeqRef.current !== requestSeq) return;
      const copied = await copyToClipboard(css, "CSS 变量已生成，但复制到剪贴板失败");
      if (!mountedRef.current || cssSeqRef.current !== requestSeq) return;
      if (copied) addToast("CSS 变量已复制", "success");
    } catch (err) {
      if (!mountedRef.current || cssSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "读取 CSS 失败"), "error");
    } finally {
      if (cssSeqRef.current === requestSeq) cssRef.current = null;
    }
  };

  const handleImport = async () => {
    if (importingRef.current) return;
    if (!importText.trim()) {
      addToast("请粘贴主题 JSON", "error");
      return;
    }
    const requestSeq = importSeqRef.current + 1;
    importSeqRef.current = requestSeq;
    importingRef.current = true;
    const requestText = importText.trim();
    setImporting(true);
    try {
      await importTheme(requestText);
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      addToast("已导入主题", "success");
      setImportText((current) => (current.trim() === requestText ? "" : current));
      await refresh({ force: true });
    } catch (err) {
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "导入失败"), "error");
    } finally {
      if (importSeqRef.current === requestSeq) importingRef.current = false;
      if (mountedRef.current && importSeqRef.current === requestSeq) {
        setImporting(false);
      }
    }
  };

  return {
    themes,
    defaultTheme,
    scope,
    loading,
    activating,
    deleting,
    creating,
    newName,
    newMode,
    renamingId,
    renameValue,
    exportingId,
    importing,
    importText,
    hasUser: Boolean(user),
    refresh,
    setScope,
    setNewName,
    setNewMode,
    setRenameValue,
    setImportText,
    setRenamingId,
    handleActivate,
    handleDelete,
    handleCreate,
    handleStartRename,
    handleConfirmRename,
    handleExport,
    handleViewCss,
    handleImport,
  };
};
