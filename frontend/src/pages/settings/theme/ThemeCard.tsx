import React, { useMemo } from "react";
import { Check, Code2, Download, Loader2, Trash2 } from "lucide-react";
import { type Theme } from "../../../api/theme";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { getThemeCssVariable } from "../../../theme/themeCssVariables";

interface ThemeCardProps {
  theme: Theme;
  activating: boolean;
  activateDisabled: boolean;
  deleting: boolean;
  exporting: boolean;
  renaming: boolean;
  renameValue: string;
  renameSaving: boolean;
  mutationBusy: boolean;
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
  renameSaving,
  mutationBusy,
  onRenameInput,
  onRenameStart,
  onRenameConfirm,
  onRenameCancel,
  onActivate,
  onDelete,
  onExport,
  onViewCss,
}) => {
  const renameReady =
    renameValue.trim().length > 0 && renameValue.trim() !== theme.name.trim();
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
  const moreItems: DropdownItem[] = [
    {
      label: "导出主题",
      icon: <Download size={12} />,
      onClick: onExport,
    },
    {
      label: "查看颜色变量",
      icon: <Code2 size={12} />,
      onClick: onViewCss,
    },
  ];
  if (!theme.is_system_default && !theme.is_active) {
    moreItems.push({ type: "divider" });
    moreItems.push({
      label: "删除主题",
      icon: <Trash2 size={12} />,
      danger: true,
      disabled: mutationBusy,
      onClick: onDelete,
    });
  }

  return (
    <li className="a4n-flat-surface flex min-w-0 items-stretch overflow-hidden">
      <div className="flex w-16 shrink-0 overflow-hidden sm:w-24" style={{ background: previewColors.bg }}>
        <ColorBand color={previewColors.primary} label="主色" />
        <ColorBand color={previewColors.bg} label="背景" />
        <ColorBand color={previewColors.bgPaper} label="表面" />
        <ColorBand color={previewColors.text} label="文字" />
        <ColorBand color={previewColors.border} label="边线" />
      </div>

      <div className="flex min-w-0 flex-1 items-center gap-3 p-3">
        <div className="min-w-0 flex-1">
          {renaming ? (
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <input
                value={renameValue}
                onChange={(event) => onRenameInput(event.target.value)}
                disabled={renameSaving}
                className="pyqt-input min-w-32 flex-1 !px-2 !py-1 text-xs"
                autoFocus
                onKeyDown={(event) => {
                  if (event.key === "Enter" && renameReady && !renameSaving) onRenameConfirm();
                  if (event.key === "Escape" && !renameSaving) onRenameCancel();
                }}
              />
              <BookButton
                variant="primary"
                size="sm"
                onClick={onRenameConfirm}
                disabled={!renameReady || renameSaving}
              >
                {renameSaving ? (
                  <span className="inline-flex items-center gap-1.5">
                    <Loader2 size={12} className="animate-spin" /> 保存中…
                  </span>
                ) : (
                  "保存"
                )}
              </BookButton>
              <BookButton
                variant="ghost"
                size="sm"
                onClick={onRenameCancel}
                disabled={renameSaving}
              >
                取消
              </BookButton>
            </div>
          ) : (
            <>
              <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
                <button
                  type="button"
                  onClick={onRenameStart}
                  disabled={mutationBusy}
                  className="min-w-0 truncate text-left text-sm font-semibold text-book-text-main hover:text-book-primary disabled:cursor-not-allowed disabled:opacity-55"
                  title="重命名主题"
                >
                  {theme.name}
                </button>
                {theme.is_active ? (
                  <span className="text-[10px] font-medium text-book-primary">使用中</span>
                ) : null}
              </div>
              <div className="mt-0.5 text-[11px] text-book-text-muted">
                {theme.is_system_default ? "系统默认" : "用户自定义"} ·{" "}
                {theme.mode === "dark" ? "暗色" : "亮色"}
              </div>
            </>
          )}
        </div>
        {!renaming ? (
          <div className="flex shrink-0 items-center gap-2">
            {!theme.is_active ? (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={onActivate}
                disabled={activateDisabled}
              >
                {activating ? <Loader2 size={12} className="animate-spin" /> : <Check size={12} />}
                使用
              </BookButton>
            ) : null}
            {exporting || deleting ? (
              <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
                <Loader2 size={12} className="animate-spin" />
              </span>
            ) : (
              <Dropdown items={moreItems} />
            )}
          </div>
        ) : null}
      </div>
    </li>
  );
};

const ColorBand: React.FC<{ color: string; label: string }> = ({ color, label }) => (
  <div
    className="h-full min-w-0 flex-1"
    style={{ background: color }}
    title={label}
  />
);
