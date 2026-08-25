import React, { useEffect, useRef } from "react";
import { type ThemeAppearance } from "../../api/theme";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import {
  CreateThemePanel,
  SingleThemeImportPanel,
  ThemePackagePanel,
} from "./theme/ThemeConfigPanels";
import { ThemeEditorPanel } from "./theme/ThemeEditorPanel";
import { ThemeListSection } from "./theme/ThemeListSection";
import { useThemeTabController } from "./theme/useThemeTabController";
import { useI18n } from "../../i18n";

/**
 * 主题与外观 Tab
 *
 * 保留页面区块编排，主题数据加载和命令流程由 useThemeTabController 统一管理。
 */
export const ThemeTab: React.FC = () => {
  const themeTab = useThemeTabController();
  const { t } = useI18n();
  const editingTheme = themeTab.themes.find((theme) => theme.id === themeTab.editingId) ?? null;
  const editorRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const editor = editorRef.current;
    if (!editingTheme || !editor || typeof editor.scrollIntoView !== "function") return;
    editor.scrollIntoView({ block: "start" });
  }, [editingTheme?.id]);
  const appearanceOptions: Array<{
    id: ThemeAppearance;
    label: string;
    disabled: boolean;
  }> = [
    { id: "system", label: t("settingsTheme.system"), disabled: !themeTab.hasUser || themeTab.mutationBusy },
    { id: "light", label: t("settingsTheme.light"), disabled: !themeTab.hasUser || themeTab.mutationBusy },
    { id: "dark", label: t("settingsTheme.dark"), disabled: !themeTab.hasUser || themeTab.mutationBusy },
  ];

  return (
    <div className="custom-scrollbar flex h-auto flex-col gap-4 overflow-visible pb-12 md:h-full md:overflow-y-auto md:pr-2">
      <section className="a4n-flat-surface grid min-w-0 grid-cols-1 gap-2 px-3 py-3 sm:flex sm:flex-wrap sm:items-center sm:gap-3 sm:px-4">
        <div className="min-w-0 flex-1">
          <h2 className="text-sm font-semibold text-book-text-main">{t("settingsTheme.appearance")}</h2>
          <p className="mt-0.5 text-small text-book-text-muted">{t("settingsTheme.appearanceHint")}</p>
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
        hasUser={themeTab.hasUser}
        activatingId={themeTab.activating}
        deletingId={themeTab.deleting}
        bulkDeleting={themeTab.bulkDeleting}
        exportingId={themeTab.exportingId}
        editingId={themeTab.editingId}
        renamingId={themeTab.renamingId}
        renameValue={themeTab.renameValue}
        renameSaving={themeTab.renameSaving}
        mutationBusy={themeTab.mutationBusy}
        onScopeChange={themeTab.setScope}
        onRefresh={themeTab.refresh}
        onRenameInput={themeTab.setRenameValue}
        onRenameStart={themeTab.handleStartRename}
        onRenameConfirm={themeTab.handleConfirmRename}
        onRenameCancel={() => themeTab.setRenamingId(null)}
        onActivate={themeTab.handleActivate}
        onEdit={themeTab.handleEdit}
        onDelete={themeTab.handleDelete}
        onDeleteMany={themeTab.handleDeleteMany}
        onExport={themeTab.handleExport}
      />

      {editingTheme ? (
        <div ref={editorRef} className="min-w-0 scroll-mt-3">
          <ThemeEditorPanel
            theme={editingTheme}
            saving={themeTab.editorSaving}
            onCancel={themeTab.handleEditorCancel}
            onSave={(input, pendingFiles, deleteResourceIds) =>
              themeTab.handleEditorSave(
                editingTheme.id,
                input,
                pendingFiles,
                deleteResourceIds,
              )
            }
          />
        </div>
      ) : null}

      <CreateThemePanel
        name={themeTab.newName}
        mode={themeTab.newMode}
        creating={themeTab.creating}
        disabled={themeTab.mutationBusy}
        onNameChange={themeTab.setNewName}
        onModeChange={themeTab.setNewMode}
        onCreate={themeTab.handleCreate}
      />

      <SingleThemeImportPanel
        importText={themeTab.importText}
        importing={themeTab.importing}
        disabled={themeTab.mutationBusy}
        onImportTextChange={themeTab.setImportText}
        onImport={themeTab.handleImport}
      />

      <ThemePackagePanel
        themes={themeTab.themes}
        exportingId={themeTab.packageExportingId}
        importing={themeTab.packageImporting}
        disabled={themeTab.mutationBusy}
        onExport={themeTab.handleExportPackage}
        onImport={themeTab.handleImportPackage}
      />
    </div>
  );
};
