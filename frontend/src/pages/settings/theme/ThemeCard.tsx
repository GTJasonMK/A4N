import React, { useMemo } from "react";
import { Check, Loader2, Trash2 } from "lucide-react";
import { type Theme } from "../../../api/theme";
import { getThemeCssVariable } from "../../../theme/themeCssVariables";

interface ThemeCardProps {
  theme: Theme;
  activating: boolean;
  activateDisabled: boolean;
  deleting: boolean;
  exporting: boolean;
  renaming: boolean;
  renameValue: string;
  onRenameInput: (value: string) => void;
  onRenameStart: () => void;
  onRenameConfirm: () => void;
  onRenameCancel: () => void;
  onActivate: () => void;
  onDelete: () => void;
  onExport: () => void;
  onViewCss: () => void;
}

export const ThemeCard: React.FC<ThemeCardProps> = ({
  theme,
  activating,
  activateDisabled,
  deleting,
  exporting,
  renaming,
  renameValue,
  onRenameInput,
  onRenameStart,
  onRenameConfirm,
  onRenameCancel,
  onActivate,
  onDelete,
  onExport,
  onViewCss,
}) => {
  const previewColors = useMemo(() => {
    const safeRgb = (raw: string | undefined): string =>
      raw ? `rgb(${raw})` : "#ccc";
    return {
      primary: safeRgb(getThemeCssVariable(theme.css_variables, "color-primary")),
      bg: safeRgb(getThemeCssVariable(theme.css_variables, "color-bg-primary")),
      bgPaper: safeRgb(getThemeCssVariable(theme.css_variables, "color-bg-secondary")),
      text: safeRgb(getThemeCssVariable(theme.css_variables, "color-text-primary")),
      border: safeRgb(getThemeCssVariable(theme.css_variables, "color-border")),
    };
  }, [theme]);

  return (
    <li className="pyqt-card overflow-hidden flex flex-col">
      <div
        className="h-20 flex items-center px-3 gap-1"
        style={{
          background: previewColors.bg,
          borderBottom: `1px solid ${previewColors.border}`,
        }}
      >
        <ColorChip color={previewColors.primary} label="P" />
        <ColorChip color={previewColors.bg} label="B" />
        <ColorChip color={previewColors.bgPaper} label="C" />
        <ColorChip color={previewColors.text} label="T" />
        <ColorChip color={previewColors.border} label="L" />
      </div>

      <div className="p-3 flex flex-col gap-2 flex-1">
        <div className="flex items-center justify-between gap-2">
          {renaming ? (
            <div className="flex items-center gap-1 flex-1 min-w-0">
              <input
                value={renameValue}
                onChange={(event) => onRenameInput(event.target.value)}
                className="pyqt-input !py-1 !px-2 text-xs flex-1 min-w-0"
                autoFocus
                onKeyDown={(event) => {
                  if (event.key === "Enter") onRenameConfirm();
                  if (event.key === "Escape") onRenameCancel();
                }}
              />
              <button
                type="button"
                onClick={onRenameConfirm}
                className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[24px]"
              >
                ✓
              </button>
              <button
                type="button"
                onClick={onRenameCancel}
                className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[24px]"
              >
                ✕
              </button>
            </div>
          ) : (
            <button
              type="button"
              onClick={onRenameStart}
              className="text-sm font-semibold text-book-text-main truncate text-left hover:underline"
              title="点击重命名"
            >
              {theme.name}
            </button>
          )}
          {theme.is_active && (
            <span className="pyqt-pill border-book-primary/40 bg-book-primary/10 text-book-primary !text-[10px]">
              激活
            </span>
          )}
        </div>
        <div className="text-[11px] text-book-text-muted">
          {theme.is_system_default ? "系统默认" : "用户自定义"} · {theme.mode}
        </div>
        <div className="mt-auto pt-2 flex items-center gap-1.5 flex-wrap">
          {!theme.is_active && (
            <button
              type="button"
              onClick={onActivate}
              disabled={activateDisabled}
              className="pyqt-btn-secondary !min-h-9 !px-2.5 text-xs sm:!min-h-[28px]"
            >
              {activating ? (
                <Loader2 size={12} className="animate-spin" />
              ) : (
                <>
                  <Check size={12} className="mr-1" />
                  激活
                </>
              )}
            </button>
          )}
          <button
            type="button"
            onClick={onExport}
            disabled={exporting}
            className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[28px]"
            title="导出 JSON 到剪贴板"
          >
            {exporting ? <Loader2 size={11} className="animate-spin" /> : "导出"}
          </button>
          <button
            type="button"
            onClick={onViewCss}
            className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[28px]"
            title="复制 CSS 变量"
          >
            CSS
          </button>
          {!theme.is_system_default && (
            <button
              type="button"
              onClick={onDelete}
              disabled={deleting || theme.is_active}
              title={theme.is_active ? "激活中的主题不可删除" : ""}
              className="pyqt-btn-secondary !min-h-9 !px-2.5 text-xs !border-error/50 !text-error hover:!bg-error-bg sm:!min-h-[28px]"
            >
              {deleting ? <Loader2 size={12} className="animate-spin" /> : <Trash2 size={12} />}
            </button>
          )}
        </div>
      </div>
    </li>
  );
};

const ColorChip: React.FC<{ color: string; label: string }> = ({ color, label }) => (
  <div
    className="w-9 h-9 rounded border border-book-border/50 flex items-center justify-center text-[10px] font-mono font-bold text-white shadow"
    style={{ background: color, textShadow: "0 1px 2px rgba(0,0,0,0.4)" }}
    title={`${label} ${color}`}
  >
    {label}
  </div>
);
