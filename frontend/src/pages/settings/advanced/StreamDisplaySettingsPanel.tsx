import React from "react";
import type { ThemeStreamDisplaySettings } from "../../../api/theme";
import { useI18n } from "../../../i18n";
import {
  MAX_STREAM_DISPLAY_RATE,
  MIN_STREAM_DISPLAY_RATE,
} from "../../../utils/streamDisplaySettings";

interface StreamDisplaySettingsPanelProps {
  value: ThemeStreamDisplaySettings;
  disabled?: boolean;
  onChange: (patch: Partial<ThemeStreamDisplaySettings>) => void;
}

export const StreamDisplaySettingsPanel: React.FC<StreamDisplaySettingsPanelProps> = ({
  value,
  disabled = false,
  onChange,
}) => {
  const { t } = useI18n();
  const enabled = value.enabled;
  const controlsDisabled = disabled || !enabled;

  return (
    <section aria-label={t("settingsTheme.streamDisplay")} className="w-full">
      <div className="flex min-w-0 flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-semibold text-book-text-main">{t("settingsTheme.streamDisplay")}</h3>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={enabled}
          aria-label={t("settingsTheme.streamDisplay")}
          onClick={() => onChange({ enabled: !enabled })}
          disabled={disabled}
          className="flex min-h-9 shrink-0 items-center gap-2 rounded-[var(--pyqt-radius-md)] border border-book-border/60 bg-book-bg-tertiary/45 px-2.5 text-left text-xs text-book-text-sub transition-colors hover:border-book-primary/45 hover:text-book-text-main focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary disabled:cursor-not-allowed disabled:opacity-55"
        >
          <span>{enabled ? t("settingsTheme.streamDisplayEnabled") : t("settingsTheme.streamDisplayDisabled")}</span>
          <span
            className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
              enabled ? "bg-book-primary" : "bg-book-text-muted/25"
            }`}
            aria-hidden="true"
          >
            <span
              className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
                enabled ? "translate-x-4" : "translate-x-[2px]"
              }`}
            />
          </span>
        </button>
      </div>
      <div className="a4n-flat-surface mt-3 min-w-0 p-3">
        <fieldset disabled={controlsDisabled} className={`min-w-0 ${controlsDisabled ? "opacity-55" : ""}`}>
          <legend className="text-xs font-medium text-book-text-sub">
            {t("settingsTheme.streamDisplayMode")}
          </legend>
          <div
            className="mt-2 grid min-w-0 grid-cols-1 gap-1 sm:grid-cols-2"
            role="radiogroup"
            aria-label={t("settingsTheme.streamDisplayMode")}
          >
            {(["fixed", "adaptive"] as const).map((option) => (
              <label
                key={option}
                className={!enabled
                  ? "min-w-0 cursor-not-allowed rounded-[var(--pyqt-radius-md)] border border-book-border/35 bg-book-bg-tertiary/15 px-3 py-2 text-book-text-muted transition-colors"
                  : value.mode === option
                    ? "min-w-0 cursor-pointer rounded-[var(--pyqt-radius-md)] border border-book-primary/55 bg-book-primary/10 px-3 py-2 text-book-text-main transition-colors"
                    : "min-w-0 cursor-pointer rounded-[var(--pyqt-radius-md)] border border-book-border/60 px-3 py-2 text-book-text-sub transition-colors hover:border-book-primary/35"}
              >
                <input
                  type="radio"
                  name="stream-display-mode"
                  value={option}
                  checked={value.mode === option}
                  onChange={() => onChange({ mode: option })}
                  className="sr-only"
                />
                <span className="block text-xs font-medium">
                  {t(
                    option === "fixed"
                      ? "settingsTheme.streamDisplayFixed"
                      : "settingsTheme.streamDisplayAdaptive",
                  )}
                </span>
                <span className="mt-0.5 block break-words text-small text-book-text-muted">
                  {t(
                    option === "fixed"
                      ? "settingsTheme.streamDisplayFixedHint"
                      : "settingsTheme.streamDisplayAdaptiveHint",
                  )}
                </span>
              </label>
            ))}
          </div>
          <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2 text-xs">
            <label htmlFor="stream-display-rate" className="font-medium text-book-text-sub">
              {t(
                value.mode === "adaptive"
                  ? "settingsTheme.streamDisplayInitialRate"
                  : "settingsTheme.streamDisplayRate",
              )}
            </label>
            <div className="flex min-w-0 w-full items-center gap-3 sm:w-auto sm:min-w-[18rem] sm:flex-1">
              <input
                id="stream-display-rate"
                type="range"
                min={MIN_STREAM_DISPLAY_RATE}
                max={MAX_STREAM_DISPLAY_RATE}
                step={1}
                value={value.rate}
                onChange={(event) => onChange({ rate: Number(event.currentTarget.value) })}
                className="a4n-range min-w-0 flex-1"
                aria-valuetext={`${value.rate} ${t("settingsTheme.streamDisplayRateUnit")}`}
              />
              <output htmlFor="stream-display-rate" className="shrink-0 font-mono tabular-nums text-book-text-main">
                {value.rate}
              </output>
              <span className="shrink-0 text-book-text-muted">{t("settingsTheme.streamDisplayRateUnit")}</span>
            </div>
          </div>
          <p className="mt-2 text-small leading-relaxed text-book-text-muted">
            {t("settingsTheme.streamDisplayRateHelp", {
              min: MIN_STREAM_DISPLAY_RATE,
              max: MAX_STREAM_DISPLAY_RATE,
            })}
          </p>
        </fieldset>
        {!enabled ? (
          <p className="mt-2 text-small leading-relaxed text-book-text-muted">
            {t("settingsTheme.streamDisplayDisabledHint")}
          </p>
        ) : null}
      </div>
    </section>
  );
};
