import React, { useState } from "react";
import { Plus, X } from "lucide-react";
import {
  durationChoicesForLimits,
  isDurationAllowed,
  MAX_VIDEO_DURATION_OPTION_COUNT,
  mergeDurationOptions,
  parseNumberInput,
  type VideoDurationLimits,
  type VideoGenOptions,
} from "./videoTabUtils";
import { useI18n } from "../../../i18n";

interface VideoDurationOptionsPickerProps {
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  disabled?: boolean;
  onOptionsChange: React.Dispatch<React.SetStateAction<VideoGenOptions>>;
}

export const VideoDurationOptionsPicker: React.FC<VideoDurationOptionsPickerProps> = ({
  options,
  durationLimits,
  disabled = false,
  onOptionsChange,
}) => {
  const { t } = useI18n();
  const [customText, setCustomText] = useState("");
  const choices = durationChoicesForLimits(durationLimits);
  const selected = options.durationOptions;
  const selectedSet = new Set(selected);
  const displayChoices = mergeDurationOptions(options.durationSecs, [
    ...choices,
    ...selected,
  ]);
  const customDuration = parseNumberInput(customText);
  const customAddBlocked =
    disabled ||
    customDuration == null ||
    selectedSet.has(customDuration) ||
    selected.length >= MAX_VIDEO_DURATION_OPTION_COUNT ||
    !isDurationAllowed(customDuration, durationLimits);

  const toggleDuration = (duration: number) => {
    if (disabled) return;
    const active = selectedSet.has(duration);
    if (active && selected.length <= 1) return;
    const next = active
      ? selected.filter((item) => item !== duration)
      : [...selected, duration];
    const nextDefaultDuration =
      active && duration === options.durationSecs
        ? next[0]
        : options.durationSecs;
    onOptionsChange((prev) => ({
      ...prev,
      durationSecs: nextDefaultDuration,
      durationOptions: mergeDurationOptions(nextDefaultDuration, next),
    }));
  };

  const addCustomDuration = () => {
    if (customAddBlocked || customDuration == null) return;
    onOptionsChange((prev) => ({
      ...prev,
      durationOptions: mergeDurationOptions(prev.durationSecs, [
        ...prev.durationOptions,
        customDuration,
      ]),
    }));
    setCustomText("");
  };

  if (choices.length === 0) {
    return (
      <div className="book-control min-h-[30px] rounded-xl px-2.5 py-1.5 text-small text-book-text-muted">
        {t("video.chapter.noDurationChoices")}
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-book-border/50 bg-book-bg-paper/40 p-1">
      <div className="flex min-h-[26px] flex-wrap gap-0.5">
        {displayChoices.map((duration) => {
          const active = selectedSet.has(duration);
          const onlySelected = active && selected.length <= 1;
          return (
            <button
              key={duration}
              type="button"
              onClick={() => toggleDuration(duration)}
              disabled={disabled || onlySelected}
              data-active={active ? "1" : undefined}
              className="inline-flex min-w-9 items-center justify-center gap-0.5 rounded-md border border-book-border/50 px-1.5 py-0.5 text-caption tabular-nums text-book-text-sub transition-colors hover:border-book-primary/60 hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-70 data-[active='1']:border-book-primary/70 data-[active='1']:bg-book-primary/10 data-[active='1']:text-book-primary"
              title={
                onlySelected
                  ? t("video.chapter.keepOneDuration")
                  : active && duration === options.durationSecs
                    ? t("video.chapter.removeDurationAndSwitchDefault")
                    : active
                      ? t("video.chapter.removeDuration")
                      : t("video.chapter.addDuration")
              }
            >
              <span>{duration}s</span>
              {active && !onlySelected && (
                <X size={10} strokeWidth={2.2} className="shrink-0" aria-hidden="true" />
              )}
            </button>
          );
        })}
      </div>
      {durationLimits.durationMode === "range" && (
        <div className="mt-1 flex items-center gap-1 border-t border-book-border/30 pt-1">
          <span className="shrink-0 text-caption text-book-text-muted">{t("video.chapter.customDuration")}</span>
          <input
            type="number"
            min={durationLimits.minDurationSecs}
            max={durationLimits.maxDurationSecs}
            value={customText}
            onChange={(event) => setCustomText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                addCustomDuration();
              }
            }}
            disabled={disabled || selected.length >= MAX_VIDEO_DURATION_OPTION_COUNT}
            className="pyqt-input ml-auto h-6 w-14 !px-1 !py-0 text-caption"
            placeholder={t("video.chapter.secondsPlaceholder")}
            aria-label={t("video.chapter.customDuration")}
          />
          <button
            type="button"
            onClick={addCustomDuration}
            disabled={customAddBlocked}
            className="inline-flex h-6 w-6 items-center justify-center rounded-md border border-book-border/60 text-book-text-sub transition-colors hover:border-book-primary/60 hover:text-book-primary disabled:cursor-not-allowed disabled:opacity-45"
            title={t("video.chapter.addDuration")}
            aria-label={t("video.chapter.addDuration")}
          >
            <Plus size={12} />
          </button>
        </div>
      )}
    </div>
  );
};
