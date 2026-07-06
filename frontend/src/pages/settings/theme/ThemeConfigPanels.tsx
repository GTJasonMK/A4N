import React from "react";
import { Loader2, Moon, Plus, Sun, Upload } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { type ThemeMode } from "./types";

interface CreateThemePanelProps {
  name: string;
  mode: ThemeMode;
  creating: boolean;
  onNameChange: (value: string) => void;
  onModeChange: (mode: ThemeMode) => void;
  onCreate: () => void;
}

export const CreateThemePanel: React.FC<CreateThemePanelProps> = ({
  name,
  mode,
  creating,
  onNameChange,
  onModeChange,
  onCreate,
}) => (
  <div className="pyqt-card p-4">
    <h4 className="text-sm font-serif text-book-text-main mb-3 inline-flex items-center gap-2">
      <Plus size={14} /> 创建主题
    </h4>
    <div className="grid grid-cols-[1fr_auto] gap-2 items-end">
      <BookInput
        label="主题名称"
        value={name}
        onChange={(event) => onNameChange(event.target.value)}
        placeholder="例如：默认亮色 / 自定义暖色"
        disabled={creating}
      />
      <div className="flex items-center gap-1">
        <button
          type="button"
          onClick={() => onModeChange("light")}
          data-active={mode === "light" ? "1" : undefined}
          className="pyqt-tab"
          title="基于亮色模板"
        >
          <Sun size={14} />
        </button>
        <button
          type="button"
          onClick={() => onModeChange("dark")}
          data-active={mode === "dark" ? "1" : undefined}
          className="pyqt-tab"
          title="基于暗色模板"
        >
          <Moon size={14} />
        </button>
        <BookButton variant="primary" onClick={onCreate} disabled={creating}>
          {creating ? <Loader2 size={14} className="animate-spin" /> : "创建"}
        </BookButton>
      </div>
    </div>
    <p className="text-[11px] text-book-text-muted mt-2">
      基于当前模板生成；详细颜色 token 编辑功能后续接入。
    </p>
  </div>
);

interface SingleThemeImportPanelProps {
  importText: string;
  importing: boolean;
  onImportTextChange: (value: string) => void;
  onImport: () => void;
}

export const SingleThemeImportPanel: React.FC<SingleThemeImportPanelProps> = ({
  importText,
  importing,
  onImportTextChange,
  onImport,
}) => (
  <div className="pyqt-card p-4">
    <h4 className="text-sm font-serif text-book-text-main mb-3 inline-flex items-center gap-2">
      <Upload size={14} /> 单主题导入
    </h4>
    <textarea
      value={importText}
      onChange={(event) => onImportTextChange(event.target.value)}
      placeholder="粘贴 export_theme 生成的 JSON…"
      className="pyqt-input w-full text-xs font-mono h-24 resize-y"
    />
    <div className="flex justify-end mt-2">
      <BookButton variant="primary" onClick={onImport} disabled={importing}>
        {importing ? <Loader2 size={12} className="animate-spin" /> : "导入"}
      </BookButton>
    </div>
  </div>
);
