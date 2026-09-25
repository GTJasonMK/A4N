import React, { useEffect, useRef } from "react";
import { Check, Download, Pencil, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { type Theme } from "../../../api/theme";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { parseCssRgb, rgb } from "../../../utils/cssColor";
import { useI18n } from "../../../i18n";
import { isDeletableTheme } from "./types";
import { isImeCompositionActive } from "../../../utils/ime";

interface ThemeCardProps {
  theme: Theme;
  activating: boolean;
  deleting: boolean;
  exporting: boolean;
  editing: boolean;
  renaming: boolean;
  renameValue: string;
  renameSaving: boolean;
  mutationBusy: boolean;
  selectable: boolean;
  selected: boolean;
  onRenameInput: (value: string) => void;
  onRenameStart: () => void;
  onRenameConfirm: () => void;
  onRenameCancel: () => void;
  onActivate: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onExport: () => void;
  onSelect: () => void;
}

export const ThemeCard: React.FC<ThemeCardProps> = ({
  theme,
  activating,
  deleting,
  exporting,
  editing,
  renaming,
  renameValue,
  renameSaving,
  mutationBusy,
  selectable,
  selected,
  onRenameInput,
  onRenameStart,
  onRenameConfirm,
  onRenameCancel,
  onActivate,
  onEdit,
  onDelete,
  onExport,
  onSelect,
}) => {
  const { t } = useI18n();
  const composingRef = useRef(false);
  const compositionEndedAtRef = useRef(0);
  useEffect(() => {
    composingRef.current = false;
    compositionEndedAtRef.current = 0;
  }, [renaming]);
  const renameReady =
    renameValue.trim().length > 0 && renameValue.trim() !== theme.name.trim();
  const previewColor = (key: string): string =>
    rgb(parseCssRgb(theme.document.tokens[key] ?? "", [204, 204, 204]));
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
      label: t("settingsTheme.export"),
      icon: <Download size={12} />,
      disabled: mutationBusy,
      onClick: onExport,
    },
  ];
  if (isDeletableTheme(theme)) {
    moreItems.push({ type: "divider" });
    moreItems.push({
      label: t("settingsTheme.delete"),
      icon: <Trash2 size={12} />,
      danger: true,
      disabled: mutationBusy,
      onClick: onDelete,
    });
  }

  return (
    <li className="a4n-flat-surface flex min-w-0 items-stretch overflow-hidden">
      <div className="flex w-8 shrink-0 overflow-hidden sm:w-12" style={{ background: previewColors.bg }}>
        <ColorBand color={previewColors.primary} label={t("settingsTheme.colorPrimary")} />
        <ColorBand color={previewColors.bg} label={t("settingsTheme.colorBackground")} />
        <ColorBand color={previewColors.bgPaper} label={t("settingsTheme.colorSurface")} />
        <ColorBand color={previewColors.text} label={t("settingsTheme.colorText")} />
        <ColorBand color={previewColors.border} label={t("settingsTheme.colorBorder")} />
      </div>

      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-2 p-3">
        <div className="flex min-w-0 basis-full flex-1 items-center gap-3 sm:basis-0">
          {selectable ? <span className="flex h-4 w-4 shrink-0 items-center justify-center">
            <input
              type="checkbox"
              checked={selectable && selected}
              disabled={!selectable || mutationBusy}
              onChange={selectable ? () => onSelect() : undefined}
              aria-label={
                selectable
                  ? t("settingsTheme.selectTheme", { name: theme.name })
                  : t("settingsTheme.selectionUnavailable", { name: theme.name })
              }
              title={
                selectable
                  ? t("settingsTheme.selectTheme", { name: theme.name })
                  : t("settingsTheme.selectionUnavailable", { name: theme.name })
              }
              className="h-4 w-4 shrink-0"
            />
          </span> : null}
          <div className="min-w-0 flex-1">
          {renaming ? (
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <input
                value={renameValue}
                aria-label={t("settingsTheme.rename")}
                onChange={(event) => onRenameInput(event.target.value)}
                disabled={renameSaving}
                className="pyqt-input min-w-32 flex-1 !px-2 !py-1 text-xs"
                autoFocus
                onCompositionStart={() => { composingRef.current = true; }}
                onCompositionEnd={() => {
                  composingRef.current = false;
                  compositionEndedAtRef.current = Date.now();
                }}
                onKeyDown={(event) => {
                  if (isImeCompositionActive(event.nativeEvent, composingRef.current, compositionEndedAtRef.current)) {
                    event.stopPropagation();
                    return;
                  }
                  if (event.key === "Enter" && renameReady && !renameSaving) {
                    event.preventDefault();
                    onRenameConfirm();
                  }
                  if (event.key === "Escape" && !renameSaving) {
                    event.preventDefault();
                    event.stopPropagation();
                    onRenameCancel();
                  }
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
                    <LoadingSpinner size="sm" /> {t("settingsTheme.saving")}
                  </span>
                ) : (
                  t("common.save")
                )}
              </BookButton>
              <BookButton
                variant="ghost"
                size="sm"
                onClick={onRenameCancel}
                disabled={renameSaving}
              >
                {t("settingsTheme.cancel")}
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
                    className="min-w-0 break-words text-left text-sm font-semibold text-book-text-main hover:text-book-primary disabled:cursor-not-allowed disabled:opacity-55"
                    title={t("settingsTheme.rename")}
                  >
                    {theme.name}
                  </button>
                ) : (
                  <span className="min-w-0 break-words text-sm font-semibold text-book-text-main">
                    {theme.name}
                  </span>
                )}
                {theme.is_active ? (
                  <span className="text-caption font-medium text-book-primary">{t("settingsTheme.active")}</span>
                ) : null}
              </div>
              <div className="mt-0.5 text-small text-book-text-muted">
                {isOwned
                  ? t("settingsTheme.userCustom")
                  : theme.is_system_default
                    ? t("settingsTheme.systemDefault")
                    : t("settingsTheme.systemTheme")} ·{" "}
                {theme.mode === "dark" ? t("settingsTheme.dark") : t("settingsTheme.light")}
              </div>
            </>
          )}
          </div>
        </div>
        {!renaming ? (
          <div className="flex basis-full items-center justify-end gap-2 sm:basis-auto">
            {isOwned ? (
              <BookIconButton
                label={t("settingsTheme.edit")}
                variant="secondary"
                size="sm"
                onClick={onEdit}
                disabled={mutationBusy && !editing}
                title={t("settingsTheme.edit")}
                aria-pressed={editing}
                className={editing ? "!bg-book-primary/8 !text-book-primary" : undefined}
              >
                <Pencil size={12} />
              </BookIconButton>
            ) : null}
            {isOwned && !theme.is_active ? (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={onActivate}
                disabled={mutationBusy}
              >
                {activating ? <LoadingSpinner size="sm" /> : <Check size={12} />}
                {t("settingsTheme.activate")}
              </BookButton>
            ) : null}
            {exporting || deleting ? (
              <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
                <LoadingSpinner size="sm" />
              </span>
            ) : mutationBusy ? null : (
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
