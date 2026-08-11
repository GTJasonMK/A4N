import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import {
  activateTheme,
  createTheme,
  deleteTheme,
  exportTheme,
  importTheme,
  listThemes,
  updateTheme,
  type Theme,
  type ThemeMode,
} from "../../../api/theme";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { useAuthStore } from "../../../store/auth";
import { applyActiveTheme } from "../../../theme/activeTheme";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { readThemePreset } from "./presets";
import { type ThemeScope } from "./types";

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
  const [renameSaving, setRenameSaving] = useState(false);
  const [exportingId, setExportingId] = useState<number | null>(null);
  const [importing, setImporting] = useState(false);
  const [importText, setImportText] = useState("");
  const userKey = String(user?.id ?? "anonymous");
  const mountedRef = useRef(true);
  const activeUserKeyRef = useRef(userKey);
  const refreshSeqRef = useRef(0);
  const refreshInFlightUserRef = useRef<string | null>(null);
  const mutationInFlightRef = useRef(false);
  const exportInFlightRef = useRef(false);

  activeUserKeyRef.current = userKey;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshInFlightUserRef.current = null;
      mutationInFlightRef.current = false;
      exportInFlightRef.current = false;
    };
  }, []);

  const isCurrentUser = useCallback(
    (requestUserKey: string) =>
      mountedRef.current && activeUserKeyRef.current === requestUserKey,
    [],
  );

  const isCurrentRefresh = useCallback(
    (requestUserKey: string, seq: number) =>
      isCurrentUser(requestUserKey) && refreshSeqRef.current === seq,
    [isCurrentUser],
  );

  const refresh = useCallback(
    async (options?: { force?: boolean }) => {
      const requestUserKey = userKey;
      if (!options?.force && refreshInFlightUserRef.current === requestUserKey)
        return;
      refreshInFlightUserRef.current = requestUserKey;
      const requestSeq = refreshSeqRef.current + 1;
      refreshSeqRef.current = requestSeq;
      setLoading(true);
      try {
        const data = user ? await listThemes(user.id) : [];
        if (isCurrentRefresh(requestUserKey, requestSeq)) setThemes(data);
      } catch (err) {
        if (isCurrentRefresh(requestUserKey, requestSeq)) {
          addToast(extractErrorMessage(err, "加载主题列表失败"), "error");
        }
      } finally {
        if (
          refreshInFlightUserRef.current === requestUserKey &&
          refreshSeqRef.current === requestSeq
        ) {
          refreshInFlightUserRef.current = null;
        }
        if (isCurrentRefresh(requestUserKey, requestSeq)) setLoading(false);
      }
    },
    [addToast, isCurrentRefresh, user, userKey],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    mutationInFlightRef.current = false;
    exportInFlightRef.current = false;
    if (refreshInFlightUserRef.current !== userKey)
      refreshInFlightUserRef.current = null;
    setThemes([]);
    setActivating(null);
    setDeleting(null);
    setCreating(false);
    setExportingId(null);
    setImporting(false);
    setRenamingId(null);
    setRenameValue("");
    setRenameSaving(false);
  }, [userKey]);

  useEffect(() => {
    if (
      renamingId != null &&
      !themes.some((theme) => theme.id === renamingId)
    ) {
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

  const beginMutation = () => {
    if (mutationInFlightRef.current) return false;
    mutationInFlightRef.current = true;
    return true;
  };

  const finishMutation = (requestUserKey: string) => {
    if (!isCurrentUser(requestUserKey)) return false;
    mutationInFlightRef.current = false;
    return true;
  };

  const handleActivate = async (id: number) => {
    if (!beginMutation()) return;
    const requestUserKey = userKey;
    setActivating(id);
    try {
      const activated = await activateTheme(id);
      if (!isCurrentUser(requestUserKey)) return;
      applyActiveTheme(activated);
      addToast("已激活主题", "success");
      await refresh({ force: true });
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, "激活失败"), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setActivating(null);
    }
  };

  const handleDelete = async (id: number, name: string) => {
    if (!beginMutation()) return;
    const requestUserKey = userKey;
    const confirmed = await confirmDialog({
      title: "删除主题",
      message: `删除主题"${name}"？此操作不可撤销。`,
      confirmText: "删除",
      dialogType: "danger",
    });
    if (!confirmed || !isCurrentUser(requestUserKey)) {
      finishMutation(requestUserKey);
      return;
    }
    setDeleting(id);
    try {
      await deleteTheme(id);
      if (!isCurrentUser(requestUserKey)) return;
      addToast("已删除主题", "success");
      await refresh({ force: true });
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, "删除失败"), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setDeleting(null);
    }
  };

  const handleCreate = async () => {
    const name = newName.trim();
    if (!name) {
      addToast("请输入主题名称", "error");
      return;
    }
    if (!beginMutation()) return;
    const requestUserKey = userKey;
    const mode = newMode;
    setCreating(true);
    try {
      await createTheme({ name, mode, css_variables: readThemePreset(mode) });
      if (!isCurrentUser(requestUserKey)) return;
      addToast(`已创建主题：${name}`, "success");
      setNewName("");
      await refresh({ force: true });
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, "创建主题失败"), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setCreating(false);
    }
  };

  const handleStartRename = (theme: Theme) => {
    if (mutationInFlightRef.current) return;
    setRenamingId(theme.id);
    setRenameValue(theme.name);
  };

  const handleConfirmRename = async () => {
    if (renamingId == null) return;
    const name = renameValue.trim();
    if (!name) {
      addToast("名称不能为空", "error");
      return;
    }
    const themeId = renamingId;
    if (themes.some((theme) => theme.id === themeId && theme.name === name))
      return;
    if (!beginMutation()) return;
    const requestUserKey = userKey;
    setRenameSaving(true);
    try {
      await updateTheme(themeId, { name });
      if (!isCurrentUser(requestUserKey)) return;
      addToast("已重命名", "success");
      setRenamingId(null);
      setRenameValue("");
      await refresh({ force: true });
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, "重命名失败"), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setRenameSaving(false);
    }
  };

  const handleExport = async (id: number) => {
    if (exportInFlightRef.current) return;
    exportInFlightRef.current = true;
    const requestUserKey = userKey;
    setExportingId(id);
    try {
      const json = await exportTheme(id);
      if (!isCurrentUser(requestUserKey)) return;
      const copied = await copyToClipboard(
        json,
        "主题 JSON 已生成，但复制到剪贴板失败",
      );
      if (copied && isCurrentUser(requestUserKey)) {
        addToast("主题 JSON 已复制到剪贴板", "success");
      }
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, "导出失败"), "error");
      }
    } finally {
      if (isCurrentUser(requestUserKey)) {
        exportInFlightRef.current = false;
        setExportingId(null);
      }
    }
  };

  const handleViewCss = async (theme: Theme) => {
    const css = [
      ":root {",
      ...Object.entries(theme.css_variables).map(
        ([key, value]) =>
          `  ${key.startsWith("--") ? key : `--${key}`}: ${value};`,
      ),
      "}",
    ].join("\n");
    if (await copyToClipboard(css, "CSS 变量复制失败")) {
      addToast("CSS 变量已复制", "success");
    }
  };

  const handleImport = async () => {
    const themeJson = importText.trim();
    if (!themeJson) {
      addToast("请粘贴主题 JSON", "error");
      return;
    }
    if (!beginMutation()) return;
    const requestUserKey = userKey;
    setImporting(true);
    try {
      await importTheme(themeJson);
      if (!isCurrentUser(requestUserKey)) return;
      addToast("已导入主题", "success");
      setImportText("");
      await refresh({ force: true });
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, "导入失败"), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setImporting(false);
    }
  };

  const visibleThemes =
    scope === "user" && user
      ? themes.filter((theme) => theme.user_id === user.id)
      : themes;

  return {
    themes: visibleThemes,
    scope,
    loading,
    activating,
    deleting,
    creating,
    newName,
    newMode,
    renamingId,
    renameValue,
    renameSaving,
    exportingId,
    importing,
    importText,
    mutationBusy:
      activating != null ||
      deleting != null ||
      creating ||
      importing ||
      renamingId != null ||
      renameSaving,
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
