import React from "react";
import { Wrench } from "lucide-react";
import { FullConfigBackupPanel } from "./advanced/FullConfigBackupPanel";
import { useFullConfigBackup } from "./advanced/useFullConfigBackup";

/**
 * 高级设置 Tab
 */
export const AdvancedTab: React.FC = () => {
  const fullConfigBackup = useFullConfigBackup();

  return (
    <div className="h-full flex flex-col gap-4 overflow-y-auto custom-scrollbar pr-2">
      <FullConfigBackupPanel
        overwrite={fullConfigBackup.overwrite}
        exporting={fullConfigBackup.exporting}
        importing={fullConfigBackup.importing}
        lastExportPath={fullConfigBackup.lastExportPath}
        onOverwriteChange={fullConfigBackup.setOverwrite}
        onExport={fullConfigBackup.handleExport}
        onImportFile={fullConfigBackup.handleImportFile}
      />

      {/* 高级工具区 */}
      <div className="pyqt-card p-4">
        <h4 className="text-sm font-serif text-book-text-main mb-2 inline-flex items-center gap-2">
          <Wrench size={14} /> 运维工具
        </h4>
        <p className="text-xs text-book-text-muted leading-relaxed">
          数据库备份 / 恢复 / 清缓存 / 重置等运维操作需要管理员权限，
          请通过 SETTINGS → "管理员" 标签访问（仅 role=admin 可见）。
        </p>
      </div>
    </div>
  );
};
