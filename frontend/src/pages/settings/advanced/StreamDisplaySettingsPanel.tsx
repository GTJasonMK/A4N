import React from "react";
import { useStreamDisplayRate } from "../../../hooks/usePacedStreamText";
import { useI18n } from "../../../i18n";
import {
  MAX_STREAM_DISPLAY_RATE,
  MIN_STREAM_DISPLAY_RATE,
} from "../../../utils/streamDisplaySettings";

export const StreamDisplaySettingsPanel: React.FC = () => {
  const { t } = useI18n();
  const [rate, setRate] = useStreamDisplayRate();

  return (
    <section aria-label={t("settingsAdvanced.streamDisplay")} className="w-full max-w-3xl">
      <h3 className="text-sm font-semibold text-book-text-main">{t("settingsAdvanced.streamDisplay")}</h3>
      <p className="mt-1 text-small leading-relaxed text-book-text-muted">
        {t("settingsAdvanced.streamDisplayHint")}
      </p>
      <div className="a4n-flat-surface mt-3 min-w-0 p-3">
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2 text-xs">
          <label htmlFor="stream-display-rate" className="font-medium text-book-text-sub">
            {t("settingsAdvanced.streamDisplayRate")}
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
          {t("settingsAdvanced.streamDisplayRateHelp", { min: MIN_STREAM_DISPLAY_RATE, max: MAX_STREAM_DISPLAY_RATE })}
        </p>
      </div>
    </section>
  );
};
