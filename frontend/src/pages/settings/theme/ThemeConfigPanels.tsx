import React from "react";
import { Plus, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { type ThemeMode } from "./types";

interface CreateThemePanelProps {
  name: string;
  mode: ThemeMode;
  creating: boolean;
  disabled: boolean;
  onNameChange: (value: string) => void;
  onModeChange: (mode: ThemeMode) => void;
  onCreate: () => void;
}

export const CreateThemePanel: React.FC<CreateThemePanelProps> = ({
  name,
  mode,
  creating,
  disabled,
  onNameChange,
  onModeChange,
  onCreate,
}) => (
  <details className="a4n-flat-surface overflow-hidden">
    <summary className="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-xs text-book-text-sub transition-colors hover:bg-book-bg-tertiary/45 [&::-webkit-details-marker]:hidden">
      <Plus size={13} />
      <span>创建新主题</span>
      <span className="ml-auto text-small text-book-text-muted">从亮色或暗色模板开始</span>
    </summary>
    <div className="border-t border-book-border/35 p-4">
      <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
        <BookInput
          label="主题名称"
          value={name}
          onChange={(event) => onNameChange(event.target.value)}
          placeholder="例如：柔和暖色"
          disabled={disabled}
        />
        <div className="flex flex-wrap items-center gap-2">
          <SegmentedControl
            ariaLabel="主题明暗模式"
            value={mode}
            onChange={onModeChange}
            options={[
              { id: "light", label: "亮色", disabled },
              { id: "dark", label: "暗色", disabled },
            ]}
          />
          <BookButton
            variant="primary"
            onClick={onCreate}
            disabled={disabled || name.trim().length === 0}
          >
            {creating ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" /> 创建中…
              </span>
            ) : (
              "创建"
            )}
          </BookButton>
        </div>
      </div>
      <p className="mt-2 text-small text-book-text-muted">创建后可立即切换使用。</p>
    </div>
  </details>
);

interface SingleThemeImportPanelProps {
  importText: string;
  importing: boolean;
  disabled: boolean;
  onImportTextChange: (value: string) => void;
  onImport: () => void;
}

export const SingleThemeImportPanel: React.FC<SingleThemeImportPanelProps> = ({
  importText,
  importing,
  disabled,
  onImportTextChange,
  onImport,
}) => (
  <details className="a4n-flat-surface group overflow-hidden">
    <summary className="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-xs text-book-text-sub transition-colors hover:bg-book-bg-tertiary/45 [&::-webkit-details-marker]:hidden">
      <Upload size={13} />
      <span>导入主题</span>
      <span className="ml-auto text-small text-book-text-muted">低频工具</span>
    </summary>
    <div className="border-t border-book-border/35 p-4">
      <textarea
        value={importText}
        onChange={(event) => onImportTextChange(event.target.value)}
        disabled={disabled}
        placeholder="粘贴导出的主题配置…"
        className="pyqt-input h-24 w-full resize-y text-xs font-mono"
      />
      <div className="flex justify-end mt-2">
        <BookButton
          variant="primary"
          onClick={onImport}
          disabled={disabled || importText.trim().length === 0}
        >
          {importing ? (
            <span className="inline-flex items-center gap-1.5">
              <LoadingSpinner size="sm" /> 导入中…
            </span>
          ) : (
            "导入"
          )}
        </BookButton>
      </div>
    </div>
  </details>
);
