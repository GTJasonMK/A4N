import React from "react";
import { FullConfigBackupPanel } from "./advanced/FullConfigBackupPanel";
import { StreamDisplaySettingsPanel } from "./advanced/StreamDisplaySettingsPanel";
import { useFullConfigBackup } from "./advanced/useFullConfigBackup";

/**
 * 高级设置 Tab
 */
export const AdvancedTab: React.FC = () => {
  const fullConfigBackup = useFullConfigBackup();

  return (
    <div className="custom-scrollbar flex h-auto flex-col gap-4 overflow-visible md:h-full md:overflow-y-auto md:pr-2">
      <StreamDisplaySettingsPanel />
      <FullConfigBackupPanel
        overwrite={fullConfigBackup.overwrite}
        exporting={fullConfigBackup.exporting}
        importing={fullConfigBackup.importing}
        lastExportPath={fullConfigBackup.lastExportPath}
        onOverwriteChange={fullConfigBackup.setOverwrite}
        onExport={fullConfigBackup.handleExport}
        onImportFile={fullConfigBackup.handleImportFile}
      />
    </div>
  );
};
