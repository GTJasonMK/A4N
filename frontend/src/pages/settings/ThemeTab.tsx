import React from "react";
import {
  CreateThemePanel,
  SingleThemeImportPanel,
} from "./theme/ThemeConfigPanels";
import { ThemeListSection } from "./theme/ThemeListSection";
import { useThemeTabController } from "./theme/useThemeTabController";

/**
 * 主题与外观 Tab
 *
 * 保留页面区块编排，主题数据加载和命令流程由 useThemeTabController 统一管理。
 */
export const ThemeTab: React.FC = () => {
  const themeTab = useThemeTabController();

  return (
    <div className="h-full flex flex-col gap-4 overflow-y-auto custom-scrollbar pr-2">
      <ThemeListSection
        themes={themeTab.themes}
        defaultTheme={themeTab.defaultTheme}
        scope={themeTab.scope}
        loading={themeTab.loading}
        hasUser={themeTab.hasUser}
        activatingId={themeTab.activating}
        deletingId={themeTab.deleting}
        exportingId={themeTab.exportingId}
        renamingId={themeTab.renamingId}
        renameValue={themeTab.renameValue}
        onScopeChange={themeTab.setScope}
        onRefresh={themeTab.refresh}
        onRenameInput={themeTab.setRenameValue}
        onRenameStart={themeTab.handleStartRename}
        onRenameConfirm={themeTab.handleConfirmRename}
        onRenameCancel={() => themeTab.setRenamingId(null)}
        onActivate={themeTab.handleActivate}
        onDelete={themeTab.handleDelete}
        onExport={themeTab.handleExport}
        onViewCss={themeTab.handleViewCss}
      />

      <CreateThemePanel
        name={themeTab.newName}
        mode={themeTab.newMode}
        creating={themeTab.creating}
        onNameChange={themeTab.setNewName}
        onModeChange={themeTab.setNewMode}
        onCreate={themeTab.handleCreate}
      />

      <SingleThemeImportPanel
        importText={themeTab.importText}
        importing={themeTab.importing}
        onImportTextChange={themeTab.setImportText}
        onImport={themeTab.handleImport}
      />
    </div>
  );
};
