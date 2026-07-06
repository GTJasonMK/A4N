import React, { useState } from "react";
import { Plus, X } from "lucide-react";
import {
  durationChoicesForLimits,
  isDurationAllowed,
  MAX_VIDEO_DURATION_OPTION_COUNT,
  normalizeDurationOptions,
  parseNumberInput,
  type VideoDurationLimits,
  type VideoGenOptions,
} from "./videoTabUtils";

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
  const [customText, setCustomText] = useState("");
  const choices = durationChoicesForLimits(durationLimits);
  const selected = normalizeDurationOptions(options.durationSecs, options.durationOptions);
  const selectedSet = new Set(selected);
  const displayChoices = normalizeDurationOptions(options.durationSecs, [...choices, ...selected]);
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
    const next = active ? selected.filter((item) => item !== duration) : [...selected, duration];
    const nextDefaultDuration =
      active && duration === options.durationSecs ? next[0] : options.durationSecs;
    onOptionsChange((prev) => ({
      ...prev,
      durationSecs: nextDefaultDuration,
      durationOptions: normalizeDurationOptions(nextDefaultDuration, next),
    }));
  };

  const addCustomDuration = () => {
    if (customAddBlocked || customDuration == null) return;
    onOptionsChange((prev) => ({
      ...prev,
      durationOptions: normalizeDurationOptions(prev.durationSecs, [
        ...prev.durationOptions,
        customDuration,
      ]),
    }));
    setCustomText("");
  };

  if (choices.length === 0) {
    return (
      <div className="book-control min-h-[30px] rounded-sm px-2 py-1.5 text-[11px] text-book-text-muted">
        无可用时长
      </div>
    );
  }

  return (
    <div className="rounded border border-book-border/50 bg-book-bg-paper/40 p-1.5">
      <div className="flex min-h-[28px] flex-wrap gap-1">
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
              className="inline-flex min-w-10 items-center justify-center gap-1 rounded border border-book-border/50 px-2 py-1 font-mono text-[11px] text-book-text-sub transition-colors hover:border-book-primary/60 hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-70 data-[active='1']:border-book-primary/70 data-[active='1']:bg-book-primary/10 data-[active='1']:text-book-primary"
              title={
                onlySelected
                  ? "至少保留一个候选时长"
                  : active && duration === options.durationSecs
                    ? "移除候选时长并切换默认时长"
                    : active
                      ? "移除候选时长"
                      : "加入候选时长"
              }
            >
              <span>{duration}s</span>
              {active && !onlySelected && (
                <X size={11} strokeWidth={2.2} className="shrink-0" aria-hidden="true" />
              )}
            </button>
          );
        })}
      </div>
      {durationLimits.durationMode === "range" && (
        <div className="mt-1.5 flex items-center gap-1 border-t border-book-border/30 pt-1.5">
          <span className="shrink-0 text-[10px] text-book-text-muted">自定义</span>
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
            className="pyqt-input ml-auto h-7 w-16 !px-1.5 !py-0 text-[11px]"
            placeholder="秒"
            aria-label="自定义候选时长"
          />
          <button
            type="button"
            onClick={addCustomDuration}
            disabled={customAddBlocked}
            className="inline-flex h-9 w-9 items-center justify-center rounded border border-book-border/60 text-book-text-sub transition-colors hover:border-book-primary/60 hover:text-book-primary disabled:cursor-not-allowed disabled:opacity-45 sm:h-7 sm:w-7"
            title="加入候选时长"
            aria-label="加入候选时长"
          >
            <Plus size={14} />
          </button>
        </div>
      )}
    </div>
  );
};
