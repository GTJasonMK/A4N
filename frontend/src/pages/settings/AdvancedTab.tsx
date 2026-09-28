import React from "react";
import { FullConfigBackupPanel } from "./advanced/FullConfigBackupPanel";
import { useFullConfigBackup, type ConfigBackupScope } from "./advanced/useFullConfigBackup";
import { useAuthStore } from "../../store/auth";

/**
 * 高级设置 Tab
 */
export const AdvancedTab: React.FC = () => {
  const isAdmin = useAuthStore((state) => state.user?.role === "admin");

  return (
    <div className="custom-scrollbar flex h-auto flex-col gap-4 overflow-visible md:h-full md:overflow-y-auto md:pr-2">
      <ConfigBackupSection scope="local" />
      {isAdmin && <ConfigBackupSection scope="server" />}
    </div>
  );
};

const ConfigBackupSection: React.FC<{ scope: ConfigBackupScope }> = ({ scope }) => {
  const fullConfigBackup = useFullConfigBackup(scope);
  return (
      <FullConfigBackupPanel
        scope={scope}
        overwrite={fullConfigBackup.overwrite}
        exporting={fullConfigBackup.exporting}
        preparingImport={fullConfigBackup.preparingImport}
        importing={fullConfigBackup.importing}
        lastExportFileName={fullConfigBackup.lastExportFileName}
        onOverwriteChange={fullConfigBackup.setOverwrite}
        onExport={fullConfigBackup.handleExport}
        onImportFile={fullConfigBackup.handleImportFile}
      />
  );
};
