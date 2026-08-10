import React from "react";
import { Check, Code2, Download, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { type Theme } from "../../../api/theme";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { parseCssRgb, rgb } from "../../../utils/cssColor";

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
  const previewColor = (key: string): string =>
    rgb(parseCssRgb(theme.css_variables[key] ?? "", [204, 204, 204]));
  const previewColors = {
    primary: previewColor("--color-primary"),
    bg: previewColor("--color-bg-primary"),
    bgPaper: previewColor("--color-bg-secondary"),
    text: previewColor("--color-text-primary"),
    border: previewColor("--color-border"),
  };
  const isOwned = theme.user_id != null;
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
  if (isOwned && !theme.is_active) {
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
                    <LoadingSpinner size="sm" /> 保存中…
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
                {isOwned ? (
                  <button
                    type="button"
                    onClick={onRenameStart}
                    disabled={mutationBusy}
                    className="min-w-0 truncate text-left text-sm font-semibold text-book-text-main hover:text-book-primary disabled:cursor-not-allowed disabled:opacity-55"
                    title="重命名主题"
                  >
                    {theme.name}
                  </button>
                ) : (
                  <span className="min-w-0 truncate text-sm font-semibold text-book-text-main">
                    {theme.name}
                  </span>
                )}
                {theme.is_active ? (
                  <span className="text-caption font-medium text-book-primary">使用中</span>
                ) : null}
              </div>
              <div className="mt-0.5 text-small text-book-text-muted">
                {isOwned
                  ? "用户自定义"
                  : theme.is_system_default
                    ? "系统默认"
                    : "系统主题"} ·{" "}
                {theme.mode === "dark" ? "暗色" : "亮色"}
              </div>
            </>
          )}
        </div>
        {!renaming ? (
          <div className="flex shrink-0 items-center gap-2">
            {isOwned && !theme.is_active ? (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={onActivate}
                disabled={activateDisabled}
              >
                {activating ? <LoadingSpinner size="sm" /> : <Check size={12} />}
                使用
              </BookButton>
            ) : null}
            {exporting || deleting ? (
              <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
                <LoadingSpinner size="sm" />
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
