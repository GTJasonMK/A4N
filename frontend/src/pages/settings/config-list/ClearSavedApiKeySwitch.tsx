import React from "react";
import { useI18n } from "../../../i18n";

interface ClearSavedApiKeySwitchProps {
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

export const ClearSavedApiKeySwitch: React.FC<
  ClearSavedApiKeySwitchProps
> = ({ checked, disabled = false, onChange }) => {
  const { t } = useI18n();
  return (
  <button
    type="button"
    role="switch"
    aria-checked={checked}
    disabled={disabled}
    onClick={() => onChange(!checked)}
    className="-mt-1 flex w-full items-center gap-3 border-t border-book-border/35 pt-2.5 text-left disabled:cursor-not-allowed disabled:opacity-60"
  >
    <span className="min-w-0 flex-1">
      <span className="block text-xs font-medium text-book-text-sub">
        {t("settings.clearApiKeyOnSave")}
      </span>
      <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
        {checked ? t("settings.removeSavedApiKey") : t("settings.keepSavedApiKey")}
      </span>
    </span>
    <span
      className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
        checked ? "bg-error/80" : "bg-book-text-muted/25"
      }`}
      aria-hidden="true"
    >
      <span
        className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
          checked ? "translate-x-4" : "translate-x-[2px]"
        }`}
      />
    </span>
  </button>
  );
};
