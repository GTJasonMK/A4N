import React from "react";
import type { Locale, MessageTranslator } from "../../../i18n";
interface SelectedRange {
  start: number;
  end: number;
}
interface VideoPostProcessTimelineProps {
  locale: Locale;
  t: MessageTranslator;
  duration: number;
  timelineReadout: number;
  formatPreciseSeconds: (value: number, locale: Locale, t: MessageTranslator) => string;
  timelineRef: React.Ref<HTMLDivElement>;
  markers: number[];
  videoPlaying: boolean;
  playbackTimeSecs: number;
  selectedMarkerIndex: number | null;
  draggingMarkerIndex: number | null;
  selectedRange: SelectedRange | null;
  markerValueFromClientX: (clientX: number) => number | null;
  addMarker: () => void;
  constrainMarkerValue: (index: number, value: number) => number;
  beginScrub: (dragging?: boolean) => void;
  seekVideo: (value: number) => void;
  finishScrub: () => void;
  handleTimelineClick: (event: React.MouseEvent<HTMLDivElement>) => void;
  setMarkers: React.Dispatch<React.SetStateAction<number[]>>;
  setSelectedMarkerIndex: React.Dispatch<React.SetStateAction<number | null>>;
  setDraggingMarkerIndex: React.Dispatch<React.SetStateAction<number | null>>;
}
export const VideoPostProcessTimeline: React.FC<VideoPostProcessTimelineProps> = ({
  locale,
  t,
  duration,
  timelineReadout,
  formatPreciseSeconds,
  timelineRef,
  markers,
  videoPlaying,
  playbackTimeSecs,
  selectedMarkerIndex,
  draggingMarkerIndex,
  selectedRange,
  markerValueFromClientX,
  addMarker,
  constrainMarkerValue,
  beginScrub,
  seekVideo,
  finishScrub,
  handleTimelineClick,
  setMarkers,
  setSelectedMarkerIndex,
  setDraggingMarkerIndex,
}) => {
  return (
            <div>
              <div className="grid grid-cols-3 text-caption tabular-nums text-book-text-muted">
                <span>{formatPreciseSeconds(0, locale, t)}</span>
                <span className="justify-self-center">
                  {formatPreciseSeconds(timelineReadout, locale, t)}
                </span>
                <span className="justify-self-end">{formatPreciseSeconds(duration, locale, t)}</span>
              </div>
              <div
                ref={timelineRef}
                role="button"
                aria-label={t("video.postprocess.timelineAria")}
                tabIndex={0}
                aria-disabled={markers.length >= 2 || undefined}
                title={
                  markers.length >= 2
                    ? t("video.postprocess.twoMarkersClearFirst")
                    : t("video.postprocess.clickToAddMarker")
                }
                onClick={handleTimelineClick}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    addMarker();
                  }
                }}
                className={`relative mt-1 h-12 rounded-lg bg-book-bg-tertiary ${
                  markers.length < 2 ? "cursor-crosshair" : "cursor-default"
                }`}
              >
                {videoPlaying ? (
                  <span
                    aria-hidden="true"
                    data-playback-head="true"
                    className="pointer-events-none absolute -top-1 z-[1] h-10 w-px bg-white/80"
                    style={{
                      left: `${Math.min(
                        100,
                        Math.max(0, (playbackTimeSecs / duration) * 100),
                      )}%`,
                    }}
                  />
                ) : null}
                {markers.map((marker, index) => (
                  <span
                    key={`marker-${index}`}
                    role="slider"
                    tabIndex={0}
                    aria-label={t("video.postprocess.markerAria", { number: index + 1 })}
                    aria-keyshortcuts="Delete Backspace"
                    aria-valuemin={0}
                    aria-valuemax={duration}
                    aria-valuenow={Number(marker.toFixed(2))}
                    onFocus={() => setSelectedMarkerIndex(index)}
                    onPointerDown={(event) => {
                      event.preventDefault();
                      event.stopPropagation();
                      if (event.isTrusted) {
                        event.currentTarget.setPointerCapture(event.pointerId);
                      }
                      setSelectedMarkerIndex(index);
                      beginScrub(true);
                      setDraggingMarkerIndex(index);
                      seekVideo(marker);
                    }}
                    onPointerMove={(event) => {
                      if (draggingMarkerIndex !== index) return;
                      const value = markerValueFromClientX(event.clientX);
                      if (value == null) return;
                      const constrainedValue = constrainMarkerValue(
                        index,
                        value,
                      );
                      seekVideo(constrainedValue);
                      setMarkers((prev) =>
                        prev.map((item, itemIndex) =>
                          itemIndex === index ? constrainedValue : item,
                        ),
                      );
                    }}
                    onPointerUp={(event) => {
                      if (
                        event.currentTarget.hasPointerCapture(event.pointerId)
                      ) {
                        event.currentTarget.releasePointerCapture(event.pointerId);
                      }
                      if (draggingMarkerIndex !== index) return;
                      const value = markerValueFromClientX(event.clientX);
                      if (value != null) {
                        const constrainedValue = constrainMarkerValue(
                          index,
                          value,
                        );
                        seekVideo(constrainedValue);
                        setMarkers((prev) =>
                          prev.map((item, itemIndex) =>
                            itemIndex === index ? constrainedValue : item,
                          ),
                        );
                      }
                      setDraggingMarkerIndex(null);
                      finishScrub();
                    }}
                    onPointerCancel={(event) => {
                      if (
                        event.currentTarget.hasPointerCapture(event.pointerId)
                      ) {
                        event.currentTarget.releasePointerCapture(
                          event.pointerId,
                        );
                      }
                      if (draggingMarkerIndex !== index) return;
                      setDraggingMarkerIndex(null);
                      finishScrub();
                    }}
                    onKeyDown={(event) => {
                      const step = event.shiftKey ? 1 : 0.05;
                      if (
                        event.key !== "ArrowLeft" &&
                        event.key !== "ArrowRight"
                      )
                        return;
                      event.preventDefault();
                      const direction = event.key === "ArrowLeft" ? -1 : 1;
                      const value = constrainMarkerValue(
                        index,
                        marker + direction * step,
                      );
                      beginScrub();
                      seekVideo(value);
                      finishScrub();
                      setMarkers((prev) =>
                        prev.map((item, itemIndex) =>
                          itemIndex === index ? value : item,
                        ),
                      );
                    }}
                    className="absolute -top-1 h-14 w-8 -translate-x-1/2 cursor-ew-resize touch-none"
                    style={{
                      left: `${Math.min(100, Math.max(0, (marker / duration) * 100))}%`,
                    }}
                    title={formatPreciseSeconds(marker, locale, t)}
                  >
                    <span className="absolute left-1/2 top-1 h-12 w-0.5 -translate-x-1/2 bg-book-primary" />
                    <span
                      className={`absolute left-1/2 top-1.5 h-3 w-3 -translate-x-1/2 rounded-full border border-book-primary ${
                        selectedMarkerIndex === index
                          ? "bg-book-primary"
                          : "bg-book-bg-paper"
                      }`}
                    />
                  </span>
                ))}
                {selectedRange && (
                  <span
                    className="pointer-events-none absolute top-2 h-4 rounded-md bg-book-primary/25"
                    style={{
                      left: `${(selectedRange.start / duration) * 100}%`,
                      width: `${((selectedRange.end - selectedRange.start) / duration) * 100}%`,
                    }}
                  />
                )}
              </div>
            </div>
  );
};
