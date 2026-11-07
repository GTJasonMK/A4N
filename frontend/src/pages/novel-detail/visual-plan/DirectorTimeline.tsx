import React, { useEffect, useImperativeHandle, useRef, useState } from "react";
import { useStore } from "zustand";
import { ArrowLeftRight, Clock3, Plus, Scissors, ZoomIn, ZoomOut } from "lucide-react";
import type { ChapterVisualPlanV3, ChapterVisualPlanView, VisualPlanEditOperation } from "../../../api/chapterVisualPlan";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { useI18n } from "../../../i18n";
import { durationMs, formatTime, segmentBackground, segmentTimes, timelineTrackWidth, type DirectorSelection } from "./timeline";
import type { DirectorPlayback } from "./playback";
import { isPositiveSafeInteger } from "../../../utils/numberRange";

interface Props {
  viewRef?: React.Ref<DirectorTimelineView>;
  plan: ChapterVisualPlanV3;
  record: ChapterVisualPlanView | null;
  selection: DirectorSelection | null;
  clock: DirectorPlayback;
  editable: boolean;
  sceneNames: ReadonlyMap<number, string>;
  onSelect: (selection: DirectorSelection, atMs: number) => void;
  onEdit: (operation: VisualPlanEditOperation, base?: ChapterVisualPlanView) => Promise<void>;
  onError: (error: Error) => void;
  onSplit: () => void;
}

export interface DirectorTimelineView {
  fit: () => void;
  locate: (atMs: number) => void;
  visibleRange: () => [number, number];
}

type Drag = { kind: "point" | "boundary"; id: string; startX: number; startTime: number; width: number; value: number; plan: ChapterVisualPlanV3; base: ChapterVisualPlanView | null };

export function DirectorTimeline({ plan, record, selection, clock, editable, sceneNames, onSelect, onEdit, onSplit, onError, viewRef }: Props) {
  const { t } = useI18n();
  const [zoom, setZoom] = useState(1);
  const [fit, setFit] = useState(true);
  const [scrollable, setScrollable] = useState(false);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [duration, setDuration] = useState(String(durationMs(plan) / 1000));
  const [scale, setScale] = useState(100);
  const [timingOpen, setTimingOpen] = useState(false);
  const rail = useRef<HTMLDivElement>(null);
  const timelineViewport = useRef<HTMLDivElement>(null);
  const scaleStart = useRef(durationMs(plan));
  const total = durationMs(plan);
  useEffect(() => {
    const viewport = timelineViewport.current;
    const track = rail.current;
    if (!viewport || !track) return;
    const update = () => setScrollable(viewport.scrollWidth > viewport.clientWidth + 1);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(viewport);
    observer.observe(track);
    return () => observer.disconnect();
  }, [fit, plan, zoom]);
  useImperativeHandle(viewRef, () => ({
    fit: () => { setFit(true); setZoom(1); },
    locate: (atMs) => {
      const track = rail.current;
      const viewport = track?.parentElement;
      if (!track || !viewport) throw new Error(t("visualPlan.timelineUi.viewNotReady"));
      viewport.scrollTo({ left: Math.max(0, atMs / total * track.clientWidth - viewport.clientWidth / 2) });
    },
    visibleRange: () => {
      const track = rail.current;
      const viewport = track?.parentElement;
      if (!track || !viewport || !track.clientWidth) return [0, total];
      return [Math.round(viewport.scrollLeft / track.clientWidth * total), Math.min(total, Math.round((viewport.scrollLeft + viewport.clientWidth) / track.clientWidth * total))];
    },
  }), [t, total]);
  const percent = (time: number) => `${Math.max(0, Math.min(100, time / total * 100))}%`;
  useEffect(() => { setDuration(String(total / 1000)); setScale(100); }, [total]);
  useEffect(() => {
    if (!drag) return;
    const items = drag.kind === "point" ? plan.points : plan.segments;
    if (!items.some((item) => item.id === drag.id)) {
      setDrag(null);
      onError(new Error(t("visualPlan.timelineUi.dragTargetDeleted")));
    }
  }, [drag, onError, plan, t]);

  const seek = (event: React.PointerEvent<HTMLElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    clock.getState().seek(Math.round(Math.max(0, Math.min(1, (event.clientX - box.left) / box.width)) * total));
  };
  const startDrag = (event: React.PointerEvent<HTMLButtonElement>, kind: Drag["kind"], id: string, time: number) => {
    if (!editable || event.button !== 0 || !rail.current) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDrag({ kind, id, startX: event.clientX, startTime: time, width: rail.current.getBoundingClientRect().width, value: time, plan, base: record });
  };
  const move = (event: React.PointerEvent<HTMLButtonElement>) => {
    if (!drag) return;
    const plan = drag.plan;
    let value = Math.round(drag.startTime + (event.clientX - drag.startX) / drag.width * durationMs(plan));
    if (drag.kind === "point") {
      const index = plan.points.findIndex((point) => point.id === drag.id);
      value = Math.max(plan.points[index - 1].at_ms + 1, Math.min(plan.points[index + 1].at_ms - 1, value));
    } else {
      const index = plan.segments.findIndex((segment) => segment.id === drag.id);
      const [start] = segmentTimes(plan, plan.segments[index]);
      const [, end] = segmentTimes(plan, plan.segments[index + 1]);
      const candidates = plan.points.filter((point) => point.at_ms > start && point.at_ms < end);
      value = candidates.reduce((closest, point) => Math.abs(point.at_ms - value) < Math.abs(closest.at_ms - value) ? point : closest, candidates[0]).at_ms;
    }
    setDrag({ ...drag, value });
  };
  const finish = (event: React.PointerEvent<HTMLButtonElement>) => {
    if (!drag) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    if (Math.abs(event.clientX - drag.startX) > 3 && drag.value !== drag.startTime) {
      if (drag.kind === "point") void onEdit({ kind: "move_point", point_id: drag.id, at_ms: drag.value }, drag.base ?? undefined);
      else {
        const point = drag.plan.points.find((point) => point.at_ms === drag.value);
        if (point) void onEdit({ kind: "move_segment_boundary", left_segment_id: drag.id, point_id: point.id }, drag.base ?? undefined);
      }
    }
    setDrag(null);
  };
  const insertPoint = () => void onEdit({ kind: "insert_point", point: {
    id: crypto.randomUUID(), at_ms: Math.round(clock.getState().time), label: t("visualPlan.timelineUi.defaultPointLabel"), image_id: null, description: "", reference_image_ids: [],
  } });
  const scaleTimeline = (target: number) => {
    if (isPositiveSafeInteger(target) && target !== total) void onEdit({ kind: "scale_timeline", duration_ms: target });
    else setDuration(String(total / 1000));
  };
  const scaleCommit = () => {
    const target = Math.round(scaleStart.current * scale / 100);
    setScale(100);
    scaleTimeline(target);
  };
  const closeTiming = () => {
    setTimingOpen(false);
    setDuration(String(total / 1000));
    setScale(100);
  };

  const sceneName = (id: number) => sceneNames.get(id) ?? t("visualPlan.timelineUi.sceneNumber", { id });
  const sceneLabelFor = (binding: ChapterVisualPlanV3["segments"][number]["scene_binding"]) => {
    if (!binding) return t("visualPlan.timelineUi.sceneUnbound");
    return binding.kind === "scene"
      ? sceneName(binding.scene_profile_id)
      : `${sceneName(binding.from_scene_profile_id)} → ${sceneName(binding.to_scene_profile_id)}`;
  };

  return <section className="director-timeline" aria-label={t("visualPlan.chapterTimeline")} data-scrollable={scrollable}>
    <div className="director-timeline-heading">
      <div className="director-timeline-summary"><h3>{t("visualPlan.timeline")}</h3><span>{t("visualPlan.timelineUi.summary", { shots: plan.segments.length, points: plan.points.length })}</span>
        <BookButton variant="ghost" size="sm" aria-label={t("visualPlan.timelineUi.adjustDuration")} aria-haspopup="dialog" aria-expanded={timingOpen} title={t("visualPlan.timelineUi.adjustDuration")} onClick={() => setTimingOpen(true)}><Clock3 size={14} />{t("visualPlan.timelineUi.durationButton", { duration: formatTime(total) })}</BookButton>
        <span className="director-timeline-scroll-hint"><ArrowLeftRight size={14} aria-hidden="true" />{t("visualPlan.timelineUi.scrollHint")}</span>
      </div>
      <div className="director-inline-actions">
        <AddPointButton clock={clock} plan={plan} editable={editable} onClick={insertPoint} />
        <button type="button" onClick={onSplit} disabled={!editable}><Scissors size={15} />{t("visualPlan.timelineUi.splitShot")}</button>
        <button type="button" aria-label={t("visualPlan.timelineUi.zoomOut")} onClick={() => { if (zoom <= 1) setFit(true); else setZoom((value) => Math.max(1, value / 1.5)); }} disabled={fit}><ZoomOut size={16} /></button>
        <button type="button" aria-label={t("visualPlan.timelineUi.zoomIn")} onClick={() => { setFit(false); setZoom((value) => fit ? 1 : Math.min(12, value * 1.5)); }} disabled={!fit && zoom === 12}><ZoomIn size={16} />{t("visualPlan.timelineUi.zoomInForEditing")}</button>
      </div>
    </div>
    <div className="director-timeline-scroll" ref={timelineViewport}>
      <div
        className="director-timeline-track"
        style={{ width: `max(100%, ${timelineTrackWidth(plan, zoom, fit)}px)` }}
        ref={rail}
      >
        <div className="director-ruler" onPointerDown={seek} role="presentation">
          {[0, 0.25, 0.5, 0.75, 1].map((ratio) => <span key={ratio} style={{ left: `${ratio * 100}%` }}>{formatTime(total * ratio)}</span>)}
        </div>
        <div className="director-points">
          {plan.points.map((point, index) => {
            const fixed = index === 0 || index === plan.points.length - 1;
            const at = drag?.kind === "point" && drag.id === point.id ? drag.value : point.at_ms;
            const selected = selection?.kind === "point" && selection.id === point.id;
            const pointLabel = point.label || t("visualPlan.timelineUi.unnamed");
            return <button key={point.id} type="button" className={`director-point ${selected ? "is-selected" : ""} ${point.image_id ? "has-image" : ""} ${fixed ? "is-fixed" : ""}`}
              style={{ left: percent(at) }} aria-label={`${t(fixed ? "visualPlan.timelineUi.fixedEndpoint" : "visualPlan.timelineUi.imagePoint")} ${formatTime(at)} · ${pointLabel}${point.image_id ? ` · ${t("visualPlan.timelineUi.imageBound")}` : ` · ${t("visualPlan.timelineUi.imageUnbound")}`}`}
              aria-pressed={selected} title={`${formatTime(at)} · ${point.label || t("visualPlan.timelineUi.imagePoint")}`}
              onClick={() => onSelect({ kind: "point", id: point.id }, at)}
              onPointerDown={(event) => { if (!fixed) startDrag(event, "point", point.id, point.at_ms); }}
              onPointerMove={move} onPointerUp={finish} onPointerCancel={() => setDrag(null)}
              onKeyDown={(event) => {
                if (!fixed && editable && (event.key === "ArrowLeft" || event.key === "ArrowRight")) {
                  event.preventDefault();
                  void onEdit({ kind: "move_point", point_id: point.id, at_ms: point.at_ms + (event.key === "ArrowLeft" ? -1 : 1) * (event.shiftKey ? 100 : 10) });
                }
              }}><span /></button>;
          })}
        </div>
        <div className="director-segments">
          {plan.segments.map((segment, index) => {
            const [start, end] = segmentTimes(plan, segment);
            const selected = selection?.kind === "segment" && selection.id === segment.id;
            const shotTitle = segment.title || t("visualPlan.timelineUi.shotNumber", { number: index + 1 });
            return <React.Fragment key={segment.id}>
              <button type="button" className={`director-segment ${selected ? "is-selected" : ""}`} style={{ left: percent(start), width: percent(end - start), background: segmentBackground(segment.scene_binding) }}
                aria-label={`${shotTitle} · ${sceneLabelFor(segment.scene_binding)} · ${formatTime(end - start)}`} aria-pressed={selected}
                title={`${shotTitle} · ${sceneLabelFor(segment.scene_binding)} · ${formatTime(end - start)}`}
                onClick={() => onSelect({ kind: "segment", id: segment.id }, start)}>
                <strong>{shotTitle}</strong>
              </button>
              {index < plan.segments.length - 1 && <button type="button" className="director-segment-boundary" style={{ left: percent(drag?.kind === "boundary" && drag.id === segment.id ? drag.value : end) }}
                aria-label={t("visualPlan.timelineUi.adjustShotBoundary", { title: shotTitle })} disabled={!editable}
                role="slider" aria-valuemin={start + 1} aria-valuemax={segmentTimes(plan, plan.segments[index + 1])[1] - 1} aria-valuenow={end} aria-valuetext={formatTime(end)}
                onPointerDown={(event) => startDrag(event, "boundary", segment.id, end)} onPointerMove={move} onPointerUp={finish} onPointerCancel={() => setDrag(null)}
                onKeyDown={(event) => {
                  if (!editable || !["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
                  event.preventDefault();
                  const limit = segmentTimes(plan, plan.segments[index + 1])[1];
                  const choices = plan.points.filter((point) => point.at_ms > start && point.at_ms < limit);
                  const current = choices.findIndex((point) => point.at_ms === end);
                  const target = event.key === "Home" ? choices[0] : event.key === "End" ? choices.at(-1) : choices[current + (event.key === "ArrowLeft" ? -1 : 1)];
                  if (target) void onEdit({ kind: "move_segment_boundary", left_segment_id: segment.id, point_id: target.id });
                }}
                onClick={() => onSelect({ kind: "segment", id: segment.id }, start)} />}
            </React.Fragment>;
          })}
        </div>
        <Playhead clock={clock} total={total} />
      </div>
    </div>
    <Modal isOpen={timingOpen} title={t("visualPlan.timelineUi.timingTitle")} onClose={closeTiming} footer={<BookButton onClick={closeTiming}>{t("visualPlan.timelineUi.complete")}</BookButton>}>
      <div className="director-timing-fields">
        <label>{t("visualPlan.timelineUi.overallDuration")} <input className="book-control w-24 px-2 py-1" aria-label={`${t("visualPlan.timelineUi.overallDuration")} (${t("visualPlan.timelineUi.seconds")})`} type="number" min="0.001" step="0.001" value={duration} disabled={!editable}
          onChange={(event) => setDuration(event.target.value)} onBlur={(event) => scaleTimeline(Math.round(Number(event.currentTarget.value) * 1000))} onKeyDown={(event) => { if (event.key === "Enter") event.currentTarget.blur(); }} /> {t("visualPlan.timelineUi.seconds")}</label>
        <label className="director-scale-label">{t("visualPlan.timelineUi.scalePoints")} <input type="range" aria-label={t("visualPlan.timelineUi.scalePoints")} min={25} max={200} step={1} value={scale} disabled={!editable}
          onPointerDown={() => { scaleStart.current = total; }} onChange={(event) => { if (scale === 100) scaleStart.current = total; setScale(Number(event.target.value)); }}
          onPointerUp={scaleCommit} onKeyUp={(event) => { if (event.key.startsWith("Arrow") || event.key === "Home" || event.key === "End") scaleCommit(); }}
          onPointerCancel={() => setScale(100)} /><output>{formatTime(Math.round(total * scale / 100))}</output></label>
        <p className="director-context-line">{t("visualPlan.timelineUi.scaleHint")}</p>
      </div>
    </Modal>
  </section>;
}

function Playhead({ clock, total }: { clock: DirectorPlayback; total: number }) {
  const time = useStore(clock, (state) => state.time);
  return <div className="director-playhead" aria-hidden="true" style={{ left: `${time / total * 100}%` }}><span /></div>;
}

export function DirectorScrubber({ clock, total }: { clock: DirectorPlayback; total: number }) {
  const { t } = useI18n();
  const time = useStore(clock, (state) => state.time);
  return <input className="director-scrubber" type="range" aria-label={t("visualPlan.timelineUi.playbackPosition")} min={0} max={total} step={1} value={Math.min(time, total)} onChange={(event) => clock.getState().seek(Number(event.target.value))} />;
}

function AddPointButton({ clock, plan, editable, onClick }: { clock: DirectorPlayback; plan: ChapterVisualPlanV3; editable: boolean; onClick: () => void }) {
  const { t } = useI18n();
  const unavailable = useStore(clock, (state) => plan.points.some((point) => point.at_ms === Math.round(state.time)));
  return <button type="button" onClick={onClick} disabled={!editable || unavailable}><Plus size={15} />{t("visualPlan.timelineUi.addImagePoint")}</button>;
}
