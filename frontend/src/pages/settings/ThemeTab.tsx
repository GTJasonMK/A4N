import React, { useCallback, useRef, useState } from "react";
import { type ThemeAppearance } from "../../api/theme";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { Modal } from "../../components/ui/Modal";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import {
  CreateThemePanel,
  SingleThemeImportPanel,
  ThemePackagePanel,
} from "./theme/ThemeConfigPanels";
import { THEME_EDITOR_FORM_ID, ThemeEditorPanel } from "./theme/ThemeEditorPanel";
import { BookButton } from "../../components/ui/BookButton";
import { ThemeListSection } from "./theme/ThemeListSection";
import { useThemeTabController } from "./theme/useThemeTabController";
import { useI18n } from "../../i18n";
import { useDirty } from "../../components/navigation/dirtyRegistry";

type ThemeAction = "create" | "import" | "package" | null;

/**
 * 主题与外观 Tab
 *
 * 保留页面区块编排，主题数据加载和命令流程由 useThemeTabController 统一管理。
 */
export const ThemeTab: React.FC = () => {
  const themeTab = useThemeTabController();
  const { t } = useI18n();
  const editingTheme = themeTab.themes.find((theme) => theme.id === themeTab.editingId) ?? null;
  const renamingTheme = themeTab.themes.find((theme) => theme.id === themeTab.renamingId) ?? null;
  const [themeAction, setThemeAction] = useState<ThemeAction>(null);
  const editorDirtyRef = useRef(false);
  const hasRenameDraft =
    renamingTheme != null && themeTab.renameValue.trim() !== renamingTheme.name.trim();
  const hasThemeDraft =
    (themeAction === "create" &&
      (themeTab.newName.trim().length > 0 || themeTab.newMode !== "light")) ||
    (themeAction === "import" && themeTab.importText.trim().length > 0) ||
    hasRenameDraft;
  useDirty(
    "settings-theme-draft",
    hasThemeDraft || editorDirtyRef.current,
    undefined,
    () => !themeTab.operationInFlight,
  );
  useDirty(
    "settings-theme-operation",
    themeTab.operationInFlight,
    undefined,
    () => !themeTab.operationInFlight && !themeTab.editorSaving,
  );

  const confirmDiscardThemeDraft = async (action: string): Promise<boolean> => {
    if (!hasThemeDraft) return true;
    return confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: t("settings.theme"),
        action,
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    });
  };

  const handleThemeActionClose = async () => {
    if (themeTab.mutationBusy) return;
    if (!(await confirmDiscardThemeDraft(t("common.close")))) return;
    setThemeAction(null);
    themeTab.setNewName("");
    themeTab.setNewMode("light");
    themeTab.setImportText("");
  };

  const handleRenameCancel = async () => {
    if (!(await confirmDiscardThemeDraft(t("settingsTheme.cancel")))) return;
    themeTab.setRenamingId(null);
    themeTab.setRenameValue("");
  };

  const handleThemeActionStart = (action: Exclude<ThemeAction, null>) => {
    if (themeTab.loading || themeTab.mutationBusy) return;
    setThemeAction(action);
  };

  const reportEditorDirty = useCallback((dirty: boolean) => {
    editorDirtyRef.current = dirty;
  }, []);

  const closeEditor = () => {
    editorDirtyRef.current = false;
    themeTab.handleEditorCancel();
  };

  const handleEditorClose = async () => {
    if (themeTab.editorSaving) return;
    if (editorDirtyRef.current) {
      const confirmed = await confirmDialog({
        title: t("common.unsavedChangesTitle"),
        message: t("common.unsavedChangesMessage", {
          name: editingTheme?.name ?? t("settingsTheme.editorTitle"),
          action: t("settingsTheme.cancel"),
        }),
        confirmText: t("common.discardDraft"),
        dialogType: "warning",
      });
      if (!confirmed) return;
    }
    closeEditor();
  };
  const appearanceOptions: Array<{
    id: ThemeAppearance;
    label: string;
    disabled: boolean;
  }> = [
    { id: "system", label: t("settingsTheme.system"), disabled: !themeTab.hasUser || themeTab.mutationBusy },
    { id: "light", label: t("settingsTheme.light"), disabled: !themeTab.hasUser || themeTab.mutationBusy },
    { id: "dark", label: t("settingsTheme.dark"), disabled: !themeTab.hasUser || themeTab.mutationBusy },
  ];
  const editorPanel = editingTheme ? (
    <ThemeEditorPanel
      theme={editingTheme}
      saving={themeTab.editorSaving}
      onDirtyChange={reportEditorDirty}
      onSave={(input, pendingFiles, deleteResourceIds) =>
        themeTab.handleEditorSave(
          editingTheme.id,
          input,
          pendingFiles,
          deleteResourceIds,
        )
      }
    />
  ) : null;

  return (
    <div
      data-settings-dirty={hasThemeDraft ? "true" : undefined}
      className="custom-scrollbar flex h-auto flex-col gap-4 overflow-visible pb-12 md:h-full md:overflow-y-auto md:pr-2"
    >
      <section className="a4n-flat-surface grid min-w-0 grid-cols-1 gap-2 px-3 py-3 sm:flex sm:flex-wrap sm:items-center sm:gap-3 sm:px-4">
        <div className="min-w-0 flex-1">
          <h2 className="text-sm font-semibold text-book-text-main">{t("settingsTheme.appearance")}</h2>
        </div>
        <SegmentedControl
          ariaLabel={t("settingsTheme.appearance")}
          value={themeTab.appearance}
          onChange={themeTab.handleAppearanceChange}
          className="justify-self-start"
          options={appearanceOptions}
        />
        {themeTab.appearanceSaving ? (
          <span className="text-small text-book-text-muted" role="status">
            {t("settingsTheme.saving")}
          </span>
        ) : null}
      </section>

      <ThemeListSection
        themes={themeTab.themes}
        scope={themeTab.scope}
        loading={themeTab.loading}
        loadError={themeTab.loadError}
        hasUser={themeTab.hasUser}
        activatingId={themeTab.activating}
        deletingId={themeTab.deleting}
        bulkDeleting={themeTab.bulkDeleting}
        exportingId={themeTab.exportingId}
        editingId={themeTab.editingId}
        renamingId={themeTab.renamingId}
        renameValue={themeTab.renameValue}
        renameSaving={themeTab.renameSaving}
        renameFeedback={themeTab.renameFeedback}
        mutationBusy={themeTab.mutationBusy || themeTab.loading}
        onScopeChange={themeTab.setScope}
        onRefresh={themeTab.refresh}
        onRenameInput={themeTab.setRenameValue}
        onRenameStart={themeTab.handleStartRename}
        onRenameConfirm={themeTab.handleConfirmRename}
        onRenameCancel={() => void handleRenameCancel()}
        onActivate={themeTab.handleActivate}
        onEdit={themeTab.handleEdit}
        onDelete={themeTab.handleDelete}
        onDeleteMany={themeTab.handleDeleteMany}
        onExport={themeTab.handleExport}
        onCreate={() => handleThemeActionStart("create")}
        onImport={() => handleThemeActionStart("import")}
        onPackage={() => handleThemeActionStart("package")}
      />

      <Modal
        isOpen={Boolean(editingTheme)}
        onClose={() => void handleEditorClose()}
        title={editingTheme?.name ?? t("settingsTheme.editorTitle")}
        maxWidthClassName="max-w-3xl"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar p-3"
        closeOnBackdrop={!themeTab.editorSaving}
        closeOnEscape={!themeTab.editorSaving}
        showCloseButton={!themeTab.editorSaving}
        footer={(
          <>
            <BookButton variant="secondary" onClick={() => void handleEditorClose()} disabled={themeTab.editorSaving}>{t("settingsTheme.cancel")}</BookButton>
            <BookButton type="submit" form={THEME_EDITOR_FORM_ID} disabled={themeTab.editorSaving}>
              {themeTab.editorSaving ? t("settingsTheme.saving") : t("common.save")}
            </BookButton>
          </>
        )}
      >
        {editorPanel}
      </Modal>

      <Modal
        isOpen={themeAction != null}
        onClose={() => void handleThemeActionClose()}
        title={
          themeAction === "create"
            ? t("settingsTheme.create")
            : themeAction === "import"
              ? t("settingsTheme.import")
              : t("settingsTheme.package")
        }
        maxWidthClassName="max-w-3xl"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar p-4"
        closeOnBackdrop={!themeTab.mutationBusy}
        closeOnEscape={!themeTab.mutationBusy}
        showCloseButton={!themeTab.mutationBusy}
      >
        {themeAction === "create" ? (
          <CreateThemePanel
            name={themeTab.newName}
            mode={themeTab.newMode}
            creating={themeTab.creating}
            disabled={themeTab.mutationBusy}
            onNameChange={themeTab.setNewName}
            onModeChange={themeTab.setNewMode}
            onCreate={themeTab.handleCreate}
          />
        ) : themeAction === "import" ? (
          <SingleThemeImportPanel
            importText={themeTab.importText}
            importing={themeTab.importing}
            disabled={themeTab.mutationBusy}
            onImportTextChange={themeTab.setImportText}
            onImport={themeTab.handleImport}
          />
        ) : themeAction === "package" ? (
          <ThemePackagePanel
            themes={themeTab.themes}
            exportingId={themeTab.packageExportingId}
            importing={themeTab.packageImporting}
            disabled={themeTab.mutationBusy}
            onExport={themeTab.handleExportPackage}
            onImport={themeTab.handleImportPackage}
          />
        ) : null}
      </Modal>
    </div>
  );
};
