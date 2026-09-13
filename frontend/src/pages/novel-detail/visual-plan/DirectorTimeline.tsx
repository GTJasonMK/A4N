import React, { useEffect, useRef, useState } from "react";
import { useStore } from "zustand";
import { Clock3, Plus, Scissors, ZoomIn, ZoomOut } from "lucide-react";
import type { ChapterVisualPlanV3, VisualPlanEditOperation } from "../../../api/chapterVisualPlan";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { durationMs, formatTime, segmentBackground, sceneLabel, segmentTimes, type DirectorSelection } from "./timeline";
import type { DirectorPlayback } from "./playback";

interface Props {
  plan: ChapterVisualPlanV3;
  selection: DirectorSelection | null;
  clock: DirectorPlayback;
  editable: boolean;
  error: string | null;
  sceneNames: ReadonlyMap<number, string>;
  onSelect: (selection: DirectorSelection, atMs: number) => void;
  onEdit: (operation: VisualPlanEditOperation) => Promise<void>;
  onSplit: () => void;
}

type Drag = { kind: "point" | "boundary"; id: string; startX: number; startTime: number; width: number; value: number };

export function DirectorTimeline({ plan, selection, clock, editable, error, sceneNames, onSelect, onEdit, onSplit }: Props) {
  const [zoom, setZoom] = useState(1);
  const [fit, setFit] = useState(true);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [duration, setDuration] = useState(String(durationMs(plan) / 1000));
  const [scale, setScale] = useState(100);
  const [timingOpen, setTimingOpen] = useState(false);
  const rail = useRef<HTMLDivElement>(null);
  const scaleStart = useRef(durationMs(plan));
  const total = durationMs(plan);
  const percent = (time: number) => `${Math.max(0, Math.min(100, time / total * 100))}%`;
  useEffect(() => { setDuration(String(total / 1000)); setScale(100); }, [total]);

  const seek = (event: React.PointerEvent<HTMLElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    clock.getState().seek(Math.round(Math.max(0, Math.min(1, (event.clientX - box.left) / box.width)) * total));
  };
  const startDrag = (event: React.PointerEvent<HTMLButtonElement>, kind: Drag["kind"], id: string, time: number) => {
    if (!editable || event.button !== 0 || !rail.current) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDrag({ kind, id, startX: event.clientX, startTime: time, width: rail.current.getBoundingClientRect().width, value: time });
  };
  const move = (event: React.PointerEvent<HTMLButtonElement>) => {
    if (!drag) return;
    let value = Math.round(drag.startTime + (event.clientX - drag.startX) / drag.width * total);
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
    event.currentTarget.releasePointerCapture(event.pointerId);
    if (Math.abs(event.clientX - drag.startX) > 3 && drag.value !== drag.startTime) {
      if (drag.kind === "point") void onEdit({ kind: "move_point", point_id: drag.id, at_ms: drag.value });
      else {
        const point = plan.points.find((point) => point.at_ms === drag.value);
        if (point) void onEdit({ kind: "move_segment_boundary", left_segment_id: drag.id, point_id: point.id });
      }
    }
    setDrag(null);
  };
  const insertPoint = () => void onEdit({ kind: "insert_point", point: {
    id: crypto.randomUUID(), at_ms: Math.round(clock.getState().time), label: "分镜", image_id: null, description: "", reference_image_ids: [],
  } });
  const scaleTimeline = (target: number) => {
    if (Number.isSafeInteger(target) && target > 0 && target !== total) void onEdit({ kind: "scale_timeline", duration_ms: target });
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

  return <section className="director-timeline" aria-label="导演时间线">
    <div className="director-timeline-heading">
      <div className="director-timeline-summary"><h3>时间线</h3><span>{plan.segments.length} 个镜头 · {plan.points.length} 个图片点</span>
        <BookButton variant="ghost" size="sm" aria-label="调整整体时长" aria-haspopup="dialog" aria-expanded={timingOpen} title="调整整体时长" onClick={() => setTimingOpen(true)}><Clock3 size={14} />时长 {formatTime(total)}</BookButton>
      </div>
      <div className="director-inline-actions">
        <AddPointButton clock={clock} plan={plan} editable={editable} onClick={insertPoint} />
        <button type="button" onClick={onSplit} disabled={!editable}><Scissors size={15} />拆分镜头</button>
        <button type="button" aria-label="缩小视图" onClick={() => { if (zoom <= 1) setFit(true); else setZoom((value) => Math.max(1, value / 1.5)); }} disabled={fit}><ZoomOut size={16} /></button>
        <button type="button" aria-label="放大视图" onClick={() => { setFit(false); setZoom((value) => fit ? 1 : Math.min(12, value * 1.5)); }} disabled={!fit && zoom === 12}><ZoomIn size={16} />放大编辑</button>
      </div>
    </div>
    <div className="director-timeline-scroll">
      <div className="director-timeline-track" style={{ width: fit ? "100%" : `max(${zoom * 100}%, ${Math.max(640, plan.segments.length * 96) * zoom}px)` }} ref={rail}>
        <div className="director-ruler" onPointerDown={seek} role="presentation">
          {[0, 0.25, 0.5, 0.75, 1].map((ratio) => <span key={ratio} style={{ left: `${ratio * 100}%` }}>{formatTime(total * ratio)}</span>)}
        </div>
        <div className="director-points">
          {plan.points.map((point, index) => {
            const fixed = index === 0 || index === plan.points.length - 1;
            const at = drag?.kind === "point" && drag.id === point.id ? drag.value : point.at_ms;
            const selected = selection?.kind === "point" && selection.id === point.id;
            return <button key={point.id} type="button" className={`director-point ${selected ? "is-selected" : ""} ${point.image_id ? "has-image" : ""} ${fixed ? "is-fixed" : ""}`}
              style={{ left: percent(at) }} aria-label={`${fixed ? "固定端点" : "图片点"} ${formatTime(at)} · ${point.label || "未命名"}${point.image_id ? " · 已绑定图片" : " · 未绑定图片"}`}
              aria-pressed={selected} title={`${formatTime(at)} · ${point.label || "图片点"}`}
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
            return <React.Fragment key={segment.id}>
              <button type="button" className={`director-segment ${selected ? "is-selected" : ""}`} style={{ left: percent(start), width: percent(end - start), background: segmentBackground(segment.scene_binding) }}
                aria-label={`${segment.title} · ${sceneLabel(segment.scene_binding, sceneNames)} · ${formatTime(end - start)}`} aria-pressed={selected}
                title={`${segment.title || `镜头 ${index + 1}`} · ${sceneLabel(segment.scene_binding, sceneNames)} · ${formatTime(end - start)}`}
                onClick={() => onSelect({ kind: "segment", id: segment.id }, start)}>
                <strong>{segment.title || `镜头 ${index + 1}`}</strong>
              </button>
              {index < plan.segments.length - 1 && <button type="button" className="director-segment-boundary" style={{ left: percent(drag?.kind === "boundary" && drag.id === segment.id ? drag.value : end) }}
                aria-label={`调整 ${segment.title} 的结束边界`} disabled={!editable}
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
    <Modal isOpen={timingOpen} title="调整时长" onClose={closeTiming} footer={<BookButton onClick={closeTiming}>完成</BookButton>}>
      {error && <p className="director-error" role="alert">{error}</p>}
      <div className="director-timing-fields">
        <label>整体时长 <input className="book-control w-24 px-2 py-1" aria-label="整体时长（秒）" type="number" min="0.001" step="0.001" value={duration} disabled={!editable}
          onChange={(event) => setDuration(event.target.value)} onBlur={() => scaleTimeline(Math.round(Number(duration) * 1000))} onKeyDown={(event) => { if (event.key === "Enter") event.currentTarget.blur(); }} /> 秒</label>
        <label className="director-scale-label">等比伸缩 <input type="range" aria-label="等比伸缩全部时间点" min={25} max={200} step={1} value={scale} disabled={!editable}
          onPointerDown={() => { scaleStart.current = total; }} onChange={(event) => { if (scale === 100) scaleStart.current = total; setScale(Number(event.target.value)); }}
          onPointerUp={scaleCommit} onKeyUp={(event) => { if (event.key.startsWith("Arrow") || event.key === "Home" || event.key === "End") scaleCommit(); }}
          onPointerCancel={() => setScale(100)} /><output>{formatTime(Math.round(total * scale / 100))}</output></label>
        <p className="director-context-line">调整时长会等比移动全部图片点，镜头顺序保持不变。</p>
      </div>
    </Modal>
  </section>;
}

function Playhead({ clock, total }: { clock: DirectorPlayback; total: number }) {
  const time = useStore(clock, (state) => state.time);
  return <div className="director-playhead" aria-hidden="true" style={{ left: `${time / total * 100}%` }}><span /></div>;
}

export function DirectorScrubber({ clock, total }: { clock: DirectorPlayback; total: number }) {
  const time = useStore(clock, (state) => state.time);
  return <input className="director-scrubber" type="range" aria-label="播放位置" min={0} max={total} step={1} value={Math.min(time, total)} onChange={(event) => clock.getState().seek(Number(event.target.value))} />;
}

function AddPointButton({ clock, plan, editable, onClick }: { clock: DirectorPlayback; plan: ChapterVisualPlanV3; editable: boolean; onClick: () => void }) {
  const unavailable = useStore(clock, (state) => plan.points.some((point) => point.at_ms === Math.round(state.time)));
  return <button type="button" onClick={onClick} disabled={!editable || unavailable}><Plus size={15} />添加图片点</button>;
}
