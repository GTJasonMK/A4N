import React from "react";
import {
  useStreamDisplayMode,
  useStreamDisplayRate,
} from "../../../hooks/usePacedStreamText";
import { useI18n } from "../../../i18n";
import {
  MAX_STREAM_DISPLAY_RATE,
  MIN_STREAM_DISPLAY_RATE,
} from "../../../utils/streamDisplaySettings";

export const StreamDisplaySettingsPanel: React.FC = () => {
  const { t } = useI18n();
  const [rate, setRate] = useStreamDisplayRate();
  const [mode, setMode] = useStreamDisplayMode();

  return (
    <section aria-label={t("settingsAdvanced.streamDisplay")} className="w-full max-w-3xl">
      <h3 className="text-sm font-semibold text-book-text-main">{t("settingsAdvanced.streamDisplay")}</h3>
      <p className="mt-1 text-small leading-relaxed text-book-text-muted">
        {t("settingsAdvanced.streamDisplayHint")}
      </p>
      <div className="a4n-flat-surface mt-3 min-w-0 p-3">
        <fieldset className="min-w-0">
          <legend className="text-xs font-medium text-book-text-sub">
            {t("settingsAdvanced.streamDisplayMode")}
          </legend>
          <div
            className="mt-2 grid min-w-0 grid-cols-1 gap-1 sm:grid-cols-2"
            role="radiogroup"
            aria-label={t("settingsAdvanced.streamDisplayMode")}
          >
            {(["fixed", "adaptive"] as const).map((option) => (
              <label
                key={option}
                className={mode === option
                  ? "min-w-0 cursor-pointer rounded-[var(--pyqt-radius-md)] border border-book-primary/55 bg-book-primary/10 px-3 py-2 text-book-text-main transition-colors"
                  : "min-w-0 cursor-pointer rounded-[var(--pyqt-radius-md)] border border-book-border/60 px-3 py-2 text-book-text-sub transition-colors hover:border-book-primary/35"}
              >
                <input
                  type="radio"
                  name="stream-display-mode"
                  value={option}
                  checked={mode === option}
                  onChange={() => setMode(option)}
                  className="sr-only"
                />
                <span className="block text-xs font-medium">
                  {t(
                    option === "fixed"
                      ? "settingsAdvanced.streamDisplayFixed"
                      : "settingsAdvanced.streamDisplayAdaptive",
                  )}
                </span>
                <span className="mt-0.5 block break-words text-small text-book-text-muted">
                  {t(
                    option === "fixed"
                      ? "settingsAdvanced.streamDisplayFixedHint"
                      : "settingsAdvanced.streamDisplayAdaptiveHint",
                  )}
                </span>
              </label>
            ))}
          </div>
        </fieldset>
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2 text-xs">
          <label htmlFor="stream-display-rate" className="font-medium text-book-text-sub">
            {t(
              mode === "adaptive"
                ? "settingsAdvanced.streamDisplayInitialRate"
                : "settingsAdvanced.streamDisplayRate",
            )}
          </label>
          <div className="flex min-w-0 w-full items-center gap-3 sm:w-auto sm:min-w-[18rem] sm:flex-1">
            <input
              id="stream-display-rate"
              type="range"
              min={MIN_STREAM_DISPLAY_RATE}
              max={MAX_STREAM_DISPLAY_RATE}
              step={1}
              value={rate}
              onChange={(event) => setRate(Number(event.currentTarget.value))}
              className="a4n-range min-w-0 flex-1"
              aria-valuetext={`${rate} ${t("settingsAdvanced.streamDisplayRateUnit")}`}
            />
            <output htmlFor="stream-display-rate" className="shrink-0 font-mono tabular-nums text-book-text-main">
              {rate}
            </output>
            <span className="shrink-0 text-book-text-muted">{t("settingsAdvanced.streamDisplayRateUnit")}</span>
          </div>
        </div>
        <p className="mt-2 text-small leading-relaxed text-book-text-muted">
          {t("settingsAdvanced.streamDisplayRateHelp", {
            min: MIN_STREAM_DISPLAY_RATE,
            max: MAX_STREAM_DISPLAY_RATE,
          })}
        </p>
      </div>
    </section>
  );
};
