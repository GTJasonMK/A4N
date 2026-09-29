import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../../api";
import {
  activateTheme,
  createTheme,
  deleteTheme,
  exportThemePackage,
  exportTheme,
  importTheme,
  importThemePackage,
  listThemes,
  updateTheme,
  type Theme,
  type ThemeAppearance,
  type ThemeDocument,
  type ThemeMode,
} from "../../../api/theme";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { useAuthStore } from "../../../store/auth";
import {
  applyActiveTheme,
  applyThemePreview,
  getThemeAppearance,
  mergeSavedTheme,
  notifyThemeChanged,
  persistThemeAppearance,
  restoreActiveTheme,
  THEME_APPLIED_EVENT,
} from "../../../theme/activeTheme";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { downloadBase64ExportFile } from "../../../utils/downloadFile";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import { readThemePreset } from "./presets";
import { isDeletableTheme, type ThemeScope } from "./types";
import { THEME_DOCUMENT_VERSION } from "../../../theme/themeTokens";
import { useI18n } from "../../../i18n";
import { DEFAULT_STREAM_DISPLAY_SETTINGS } from "../../../utils/streamDisplaySettings";

export type ThemeDeleteOutcome =
  | { id: number; ok: true }
  | { id: number; ok: false; error: unknown };

export function deleteThemeIdsInParallel(
  ids: readonly number[],
  remove: (id: number) => Promise<void>,
): Promise<ThemeDeleteOutcome[]> {
  return Promise.all(
    ids.map(async (id): Promise<ThemeDeleteOutcome> => {
      try {
        await remove(id);
        return { id, ok: true };
      } catch (error) {
        return { id, ok: false, error };
      }
    }),
  );
}

export const useThemeTabController = () => {
  const { addToast } = useToast();
  const { user } = useAuthStore();
  const { t } = useI18n();
  const [themes, setThemes] = useState<Theme[]>([]);
  const [scope, setScope] = useState<ThemeScope>("all");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<{ reason: unknown } | null>(null);
  const [activating, setActivating] = useState<number | null>(null);
  const [deleting, setDeleting] = useState<number | null>(null);
  const [bulkDeleting, setBulkDeleting] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newMode, setNewMode] = useState<ThemeMode>("light");
  const [renamingId, setRenamingId] = useState<number | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [renameSaving, setRenameSaving] = useState(false);
  const [exportingId, setExportingId] = useState<number | null>(null);
  const [importing, setImporting] = useState(false);
  const [importText, setImportText] = useState("");
  const [appearance, setAppearance] = useState(getThemeAppearance);
  const [appearanceSaving, setAppearanceSaving] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editorSaving, setEditorSaving] = useState(false);
  const [packageExportingId, setPackageExportingId] = useState<number | null>(null);
  const [packageImporting, setPackageImporting] = useState(false);
  const userKey = String(user?.id ?? "anonymous");
  const mountedRef = useRef(true);
  const activeUserKeyRef = useRef(userKey);
  const refreshSeqRef = useRef(0);
  const hasLoadedThemesRef = useRef(false);
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
      setLoadError(null);
      try {
        const data = user ? await listThemes() : [];
        if (isCurrentRefresh(requestUserKey, requestSeq)) {
          hasLoadedThemesRef.current = true;
          setThemes(data);
        }
      } catch (err) {
        if (isCurrentRefresh(requestUserKey, requestSeq)) {
          setLoadError({ reason: err });
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
    [isCurrentRefresh, user, userKey],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const syncAppearance = () => setAppearance(getThemeAppearance());
    window.addEventListener(THEME_APPLIED_EVENT, syncAppearance);
    syncAppearance();
    return () => window.removeEventListener(THEME_APPLIED_EVENT, syncAppearance);
  }, []);

  useEffect(() => {
    mutationInFlightRef.current = false;
    exportInFlightRef.current = false;
    hasLoadedThemesRef.current = false;
    if (refreshInFlightUserRef.current !== userKey)
      refreshInFlightUserRef.current = null;
    setThemes([]);
    setLoadError(null);
    setLoading(true);
    setActivating(null);
    setDeleting(null);
    setBulkDeleting(false);
    setCreating(false);
    setExportingId(null);
    setImporting(false);
    setAppearanceSaving(false);
    setEditingId(null);
    setEditorSaving(false);
    setPackageExportingId(null);
    setPackageImporting(false);
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

  const commitThemes = (update: (current: Theme[]) => Theme[]) => {
    refreshSeqRef.current += 1;
    refreshInFlightUserRef.current = null;
    setThemes(update);
    setLoading(false);
    if (hasLoadedThemesRef.current) setLoadError(null);
  };

  const commitTheme = (saved: Theme) => {
    commitThemes((current) => mergeSavedTheme(current, saved));
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
      commitTheme(activated);
      applyActiveTheme(activated);
      notifyThemeChanged();
      addToast(t("settingsTheme.activated"), "success");
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.activateFailed")), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setActivating(null);
    }
  };

  const handleDelete = async (id: number, name: string) => {
    if (!beginMutation()) return;
    const requestUserKey = userKey;
    const confirmed = await confirmDialog({
      title: t("settingsTheme.delete"),
      message: t("settingsTheme.deleteMessage", { name }),
      confirmText: t("settingsTheme.delete"),
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
      commitThemes((current) => current.filter((theme) => theme.id !== id));
      addToast(t("settingsTheme.deleted"), "success");
      if (editingId === id) {
        setEditingId(null);
        restoreActiveTheme();
      }
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.deleteFailed")), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setDeleting(null);
    }
  };

  const handleDeleteMany = async (targets: Array<Pick<Theme, "id" | "name">>) => {
    const deletableTargets = targets.filter((target) => {
      const theme = themes.find((item) => item.id === target.id);
      return theme != null && isDeletableTheme(theme);
    });
    if (deletableTargets.length === 0 || !beginMutation()) return;
    const requestUserKey = userKey;
    const confirmed = await confirmDialog({
      title: t("settingsTheme.deleteSelected", { count: deletableTargets.length }),
      message: t("settingsTheme.deleteSelectedMessage", { count: deletableTargets.length }),
      confirmText: t("settingsTheme.deleteSelected", { count: deletableTargets.length }),
      dialogType: "danger",
    });
    if (!confirmed || !isCurrentUser(requestUserKey)) {
      finishMutation(requestUserKey);
      return;
    }
    setBulkDeleting(true);
    const deletedIds = new Set<number>();
    try {
      const outcomes = await deleteThemeIdsInParallel(
        deletableTargets.map((target) => target.id),
        deleteTheme,
      );
      for (const outcome of outcomes) {
        if (outcome.ok) deletedIds.add(outcome.id);
      }
      const failedCount = outcomes.length - deletedIds.size;
      if (!isCurrentUser(requestUserKey)) return;
      if (failedCount > 0) {
        addToast(
          t("settingsTheme.deleteSelectedPartial", {
            deleted: deletedIds.size,
            failed: failedCount,
          }),
          "error",
        );
      } else {
        addToast(t("settingsTheme.deletedSelected", { count: deletedIds.size }), "success");
      }
      if (editingId != null && deletedIds.has(editingId)) {
        setEditingId(null);
        restoreActiveTheme();
      }
      commitThemes((current) => current.filter((theme) => !deletedIds.has(theme.id)));
    } finally {
      if (finishMutation(requestUserKey)) setBulkDeleting(false);
    }
  };

  const handleCreate = async () => {
    const name = newName.trim();
    if (!name) {
      addToast(t("settingsTheme.nameRequired"), "error");
      return;
    }
    if (!beginMutation()) return;
    const requestUserKey = userKey;
    const mode = newMode;
    setCreating(true);
    try {
      const created = await createTheme({
        name,
        mode,
        document: {
          version: THEME_DOCUMENT_VERSION,
          tokens: readThemePreset(mode),
          backgrounds: {},
          stream_display: { ...DEFAULT_STREAM_DISPLAY_SETTINGS },
        },
      });
      if (!isCurrentUser(requestUserKey)) return;
      commitTheme(created);
      addToast(t("settingsTheme.createdNamed", { name }), "success");
      setNewName("");
      setNewMode("light");
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.createFailed")), "error");
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
      addToast(t("settingsTheme.nameEmpty"), "error");
      return;
    }
    const themeId = renamingId;
    if (themes.some((theme) => theme.id === themeId && theme.name === name))
      return;
    if (!beginMutation()) return;
    const requestUserKey = userKey;
    setRenameSaving(true);
    try {
      const saved = await updateTheme(themeId, { name });
      if (!isCurrentUser(requestUserKey)) return;
      commitTheme(saved);
      addToast(t("settingsTheme.renameSuccess"), "success");
      notifyThemeChanged();
      setRenamingId(null);
      setRenameValue("");
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.renameFailed")), "error");
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
        t("settingsTheme.copyJsonFailed"),
      );
      if (copied && isCurrentUser(requestUserKey)) {
        addToast(t("settingsTheme.copyJsonSuccess"), "success");
      }
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.exportFailed")), "error");
      }
    } finally {
      if (isCurrentUser(requestUserKey)) {
        exportInFlightRef.current = false;
        setExportingId(null);
      }
    }
  };

  const handleImport = async () => {
    const themeJson = importText.trim();
    if (!themeJson) {
      addToast(t("settingsTheme.importJsonRequired"), "error");
      return;
    }
    if (!beginMutation()) return;
    const requestUserKey = userKey;
    setImporting(true);
    try {
      const imported = await importTheme(themeJson);
      if (!isCurrentUser(requestUserKey)) return;
      commitTheme(imported);
      addToast(t("settingsTheme.importSuccess"), "success");
      notifyThemeChanged();
      setImportText("");
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.importFailed")), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setImporting(false);
    }
  };

  const handleAppearanceChange = async (nextAppearance: ThemeAppearance) => {
    if (!user || appearanceSaving || !beginMutation()) return;
    const requestUserKey = userKey;
    setAppearanceSaving(true);
    try {
      await persistThemeAppearance(
        nextAppearance,
        () => isCurrentUser(requestUserKey),
      );
      if (!isCurrentUser(requestUserKey)) return;
      addToast(t("settingsTheme.preferenceSaved"), "success");
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.preferenceSaveFailed")), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setAppearanceSaving(false);
    }
  };

  const handleEdit = (theme: Theme) => {
    if (!user || theme.user_id !== user.id || mutationInFlightRef.current) return;
    setEditingId(theme.id);
  };

  const handleEditorCancel = () => {
    setEditingId(null);
    restoreActiveTheme();
  };

  const handleEditorSave = async (
    themeId: number,
    input: { name?: string; mode?: ThemeMode; document?: ThemeDocument },
    pendingFiles: Record<string, File>,
    deleteResourceIds: number[],
  ): Promise<Theme | null> => {
    if (!user || editorSaving || !beginMutation()) return null;
    const requestUserKey = userKey;
    setEditorSaving(true);
    try {
      const uploads = await Promise.all(Object.entries(pendingFiles).map(async ([resourceKey, file]) => ({
          theme_id: themeId,
          resource_key: resourceKey,
          file_name: file.name,
          mime_type: file.type,
          data_base64: await readFileAsBase64(file),
        })));
      if (!isCurrentUser(requestUserKey)) return null;
      const saved = await updateTheme(themeId, { ...input, resource_uploads: uploads, delete_resource_ids: deleteResourceIds });
      if (!isCurrentUser(requestUserKey)) return null;
      commitTheme(saved);
      if (saved.is_active) applyActiveTheme(saved);
      else applyThemePreview(saved);
      notifyThemeChanged();
      addToast(t("settingsTheme.editorSaved"), "success");
      return saved;
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.editorSaveFailed")), "error");
      }
      return null;
    } finally {
      if (finishMutation(requestUserKey)) setEditorSaving(false);
    }
  };

  const handleExportPackage = async (themeId: number) => {
    if (packageExportingId != null) return;
    const requestUserKey = userKey;
    setPackageExportingId(themeId);
    try {
      const file = await exportThemePackage(themeId);
      if (!isCurrentUser(requestUserKey)) return;
      downloadBase64ExportFile(file);
      addToast(t("settingsTheme.packageExported"), "success");
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.packageExportFailed")), "error");
      }
    } finally {
      if (isCurrentUser(requestUserKey)) setPackageExportingId(null);
    }
  };

  const handleImportPackage = async (file: File) => {
    if (packageImporting || !beginMutation()) return;
    const requestUserKey = userKey;
    setPackageImporting(true);
    try {
      const theme = await importThemePackage({
        file_name: file.name,
        data_base64: await readFileAsBase64(file),
      });
      if (!isCurrentUser(requestUserKey)) return;
      commitTheme(theme);
      addToast(t("settingsTheme.packageImported", { name: theme.name }), "success");
      notifyThemeChanged();
    } catch (err) {
      if (isCurrentUser(requestUserKey)) {
        addToast(extractErrorMessage(err, t("settingsTheme.packageImportFailed")), "error");
      }
    } finally {
      if (finishMutation(requestUserKey)) setPackageImporting(false);
    }
  };

  const visibleThemes =
    scope === "user" && user
      ? themes.filter((theme) => theme.user_id === user.id)
      : themes;
  const operationInFlight =
    activating != null ||
    deleting != null ||
    bulkDeleting ||
    creating ||
    exportingId != null ||
    importing ||
    renameSaving ||
    appearanceSaving ||
    editorSaving ||
    packageExportingId != null ||
    packageImporting;

  return {
    themes: visibleThemes,
    scope,
    loading,
    loadError: loadError
      ? extractErrorMessage(
          loadError.reason,
          t("settingsTheme.loadFailed"),
        )
      : null,
    activating,
    deleting,
    bulkDeleting,
    creating,
    newName,
    newMode,
    renamingId,
    renameValue,
    renameSaving,
    exportingId,
    importing,
    importText,
    appearance,
    appearanceSaving,
    editingId,
    editorSaving,
    packageExportingId,
    packageImporting,
    operationInFlight,
    mutationBusy:
      operationInFlight ||
      renamingId != null ||
      editingId != null,
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
    handleDeleteMany,
    handleCreate,
    handleStartRename,
    handleConfirmRename,
    handleExport,
    handleImport,
    handleAppearanceChange,
    handleEdit,
    handleEditorCancel,
    handleEditorSave,
    handleExportPackage,
    handleImportPackage,
  };
};
