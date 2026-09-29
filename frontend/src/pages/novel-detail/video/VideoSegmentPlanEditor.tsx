import React, { useEffect, useRef, useState } from "react";
import { ChevronRight, Plus, Trash2 } from "lucide-react";
import type {
  ShootingEditingModeV1,
  ShootingTransitionTypeV1,
  VideoEmotionActionGoalV1,
  VideoPerformanceActionV1,
  VideoPerformanceTimeBlockV1,
  VideoSegmentPlanV2,
  VideoSpatialAnchorV1,
} from "../../../api/video";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { useI18n } from "../../../i18n";

type ValuePath = Array<string | number>;
type UpdateValue = (path: ValuePath, value: unknown) => void;

const TRANSITION_TYPES: readonly ShootingTransitionTypeV1[] = [
  "continuous",
  "hard_cut",
  "match_cut",
  "whip_pan",
  "action_wipe",
  "sound_bridge",
  "dissolve",
  "fade",
  "focus_transition",
  "morph",
];

const EDITING_MODES: readonly ShootingEditingModeV1[] = [
  "continuity",
  "intercut",
  "montage",
  "flashback",
  "parallel_action",
  "sound_bridge",
];

interface VideoSegmentPlanEditorProps {
  segments: VideoSegmentPlanV2[];
  onChange: (segments: VideoSegmentPlanV2[]) => void;
  disabled?: boolean;
}

const newTransition = () => ({
  transition_type: "continuous" as const,
  editing_mode: "continuity" as const,
  preserved_action: null,
  preserved_sound: null,
  required_cut_point: null,
});

export const VideoSegmentPlanEditor: React.FC<VideoSegmentPlanEditorProps> = ({
  segments,
  onChange,
  disabled = false,
}) => {
  const { t } = useI18n();
  const update: UpdateValue = (path, value) => {
    const next = structuredClone(segments);
    let cursor: object = next;
    for (const key of path.slice(0, -1)) {
      const child = Reflect.get(cursor, key);
      if (child == null || typeof child !== "object") {
        throw new Error(`Invalid video segment plan edit path: ${path.join(".")}`);
      }
      cursor = child;
    }
    const leaf = path[path.length - 1];
    if (leaf == null || !Reflect.set(cursor, leaf, value)) {
      throw new Error(`Invalid video segment plan edit path: ${path.join(".")}`);
    }
    onChange(next);
  };

  return (
    <fieldset
      disabled={disabled}
      aria-busy={disabled || undefined}
      className="m-0 min-w-0 space-y-3 border-0 p-0"
    >
      <p className="px-1 text-small leading-relaxed text-book-text-muted">
        {t("video.chapter.planEditHint")}
      </p>
      {segments.map((segment, index) => (
        <SegmentEditor
          key={`${segment.source_shot_id}:${segment.segment_index_in_shot}`}
          segment={segment}
          segmentIndex={index}
          update={update}
        />
      ))}
    </fieldset>
  );
};

const SegmentEditor: React.FC<{
  segment: VideoSegmentPlanV2;
  segmentIndex: number;
  update: UpdateValue;
}> = ({ segment, segmentIndex, update }) => {
  const { t } = useI18n();
  const base: ValuePath = [segmentIndex];
  const transitionPath = [...base, "transition_to_next"];

  return (
    <details className="group/segment a4n-flat-surface overflow-hidden">
      <summary className="flex cursor-pointer list-none items-center gap-2 border-b border-book-border/45 px-3 py-2.5 [&::-webkit-details-marker]:hidden sm:px-4">
        <ChevronRight
          size={15}
          className="shrink-0 text-book-text-muted transition-transform group-open/segment:rotate-90"
        />
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-1">
            <h3 className="text-sm font-semibold text-book-text-main">
              {t("video.chapter.segment", { number: segment.segment_number })}
            </h3>
            <span className="text-small text-book-text-muted">
              {segment.duration_secs}s · {segment.aspect_ratio}
            </span>
          </div>
          <div className="mt-0.5 truncate text-small text-book-text-muted">
            {segment.title}
          </div>
        </div>
      </summary>

      <div className="space-y-4 p-3 sm:p-4">
        <section className="grid gap-3 sm:grid-cols-2">
          <TextField
            label={t("video.chapter.planTitle")}
            value={segment.title}
            path={[...base, "title"]}
            onChange={update}
          />
          <TextField
            label={t("video.chapter.planSceneSummary")}
            value={segment.scene_summary}
            path={[...base, "scene_summary"]}
            onChange={update}
          />
          <div className="sm:col-span-2">
            <TextField
              label={t("video.chapter.planBlocking")}
              value={segment.blocking_summary}
              path={[...base, "blocking_summary"]}
              onChange={update}
            />
          </div>
          <SpatialAnchorsField
            value={segment.spatial_anchors}
            path={[...base, "spatial_anchors"]}
            onChange={update}
          />
        </section>

        <AdvancedDetails title={t("video.chapter.planEmotionGoals")}>
          <EmotionGoalsField
            value={segment.emotion_action_goals}
            path={[...base, "emotion_action_goals"]}
            onChange={update}
          />
        </AdvancedDetails>

        <AdvancedDetails title={t("video.chapter.planIntent")}>
          <div className="grid gap-3 sm:grid-cols-2">
            <TextField label={t("video.chapter.planHook")} value={segment.video_intent.hook} path={[...base, "video_intent", "hook"]} onChange={update} />
            <TextField label={t("video.chapter.planAudienceEffect")} value={segment.video_intent.audience_effect} path={[...base, "video_intent", "audience_effect"]} onChange={update} />
            <TextField label={t("video.chapter.planRhythm")} value={segment.video_intent.rhythm} path={[...base, "video_intent", "rhythm"]} onChange={update} />
            <TextField label={t("video.chapter.planVisualPriority")} value={segment.video_intent.visual_priority} path={[...base, "video_intent", "visual_priority"]} onChange={update} />
          </div>
        </AdvancedDetails>

        <AdvancedDetails title={t("video.chapter.planEmotionalArc")}>
          <div className="grid gap-3 sm:grid-cols-2">
            <TextField label={t("video.chapter.planArcStart")} value={segment.emotional_arc.start} path={[...base, "emotional_arc", "start"]} onChange={update} />
            <TextField label={t("video.chapter.planArcTurn")} value={segment.emotional_arc.turn} path={[...base, "emotional_arc", "turn"]} onChange={update} />
            <TextField label={t("video.chapter.planArcEnd")} value={segment.emotional_arc.end} path={[...base, "emotional_arc", "end"]} onChange={update} />
            <TextField label={t("video.chapter.planArcFunction")} value={segment.emotional_arc.function} path={[...base, "emotional_arc", "function"]} onChange={update} />
          </div>
        </AdvancedDetails>

        <AdvancedDetails title={t("video.chapter.planPerformance")}>
          <div className="grid gap-3 sm:grid-cols-2">
            <TextField label={t("video.chapter.planMovementPath")} value={segment.performance_plan.movement_path} path={[...base, "performance_plan", "movement_path"]} onChange={update} />
            <TextField label={t("video.chapter.planBodyPerformance")} value={segment.performance_plan.body_performance} path={[...base, "performance_plan", "body_performance"]} onChange={update} />
            <TextField label={t("video.chapter.planGazeAndHands")} value={segment.performance_plan.gaze_and_hands} path={[...base, "performance_plan", "gaze_and_hands"]} onChange={update} />
            <TextField label={t("video.chapter.planPauseReaction")} value={segment.performance_plan.pause_or_reaction} path={[...base, "performance_plan", "pause_or_reaction"]} onChange={update} />
            <TextField label={t("video.chapter.planCameraRelationship")} value={segment.performance_plan.camera_relationship} path={[...base, "performance_plan", "camera_relationship"]} onChange={update} />
          </div>
          <TimeBlocksField value={segment.performance_plan.time_blocks} path={[...base, "performance_plan", "time_blocks"]} onChange={update} />
          <ActionScriptField value={segment.performance_plan.action_script} path={[...base, "performance_plan", "action_script"]} onChange={update} />
        </AdvancedDetails>

        <AdvancedDetails title={t("video.chapter.planMaterials")}>
          <div className="grid gap-3 sm:grid-cols-2">
            <StringListField label={t("video.chapter.planRequiredEntities")} value={segment.material_coverage.required_entities} path={[...base, "material_coverage", "required_entities"]} onChange={update} />
            <StringListField label={t("video.chapter.planMissingReferences")} value={segment.material_coverage.missing_or_weak_references} path={[...base, "material_coverage", "missing_or_weak_references"]} onChange={update} />
            <TextField label={t("video.chapter.planRiskNote")} value={segment.material_coverage.risk_note} path={[...base, "material_coverage", "risk_note"]} onChange={update} />
          </div>
        </AdvancedDetails>

        <AdvancedDetails title={t("video.chapter.planTransition")}>
          {segment.director_constraints ? (
            <p className="text-small leading-relaxed text-book-text-muted">
              {t("video.chapter.planTransitionLocked")}
            </p>
          ) : segment.transition_to_next ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <SelectField
                label={t("video.chapter.planTransitionType")}
                value={segment.transition_to_next.transition_type}
                options={TRANSITION_TYPES.map((value) => [value, value] as const)}
                path={[...transitionPath, "transition_type"]}
                onChange={update}
              />
              <SelectField
                label={t("video.chapter.planEditingMode")}
                value={segment.transition_to_next.editing_mode}
                options={EDITING_MODES.map((value) => [value, value] as const)}
                path={[...transitionPath, "editing_mode"]}
                onChange={update}
              />
              <NullableTextField label={t("video.chapter.planPreservedAction")} value={segment.transition_to_next.preserved_action} path={[...transitionPath, "preserved_action"]} onChange={update} />
              <NullableTextField label={t("video.chapter.planPreservedSound")} value={segment.transition_to_next.preserved_sound} path={[...transitionPath, "preserved_sound"]} onChange={update} />
              <NullableTextField label={t("video.chapter.planCutPoint")} value={segment.transition_to_next.required_cut_point} path={[...transitionPath, "required_cut_point"]} onChange={update} />
              <div className="flex items-end justify-end sm:col-span-2">
                <BookButton size="sm" variant="ghost" onClick={() => update(transitionPath, null)}>
                  <Trash2 size={13} />
                  {t("video.chapter.planRemoveTransition")}
                </BookButton>
              </div>
            </div>
          ) : (
            <BookButton size="sm" variant="ghost" onClick={() => update(transitionPath, newTransition())}>
              <Plus size={13} />
              {t("video.chapter.planAddTransition")}
            </BookButton>
          )}
        </AdvancedDetails>

        <details className="border-t border-book-border/40 pt-2 text-caption text-book-text-muted">
          <summary className="w-fit cursor-pointer rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main">
            {t("video.chapter.planIdentity")}
          </summary>
          <dl className="mt-2 grid grid-cols-[7rem_minmax(0,1fr)] gap-x-2 gap-y-1 font-mono">
            <dt>{t("video.chapter.planSourceShot")}</dt><dd className="min-w-0 break-all">{segment.source_shot_id}</dd>
            <dt>{t("video.chapter.planSegmentIndex")}</dt><dd>{segment.segment_index_in_shot} / {segment.shot_segment_count}</dd>
            <dt>{t("video.chapter.planSourceSpans")}</dt><dd className="min-w-0 break-all">{segment.source_span_ids.join(", ")}</dd>
            <dt>{t("video.chapter.planDuration")}</dt><dd>{segment.duration_secs}s</dd>
            <dt>{t("video.chapter.planAspectRatio")}</dt><dd>{segment.aspect_ratio}</dd>
          </dl>
          <div className="mt-3">
            <StringListField label={t("video.chapter.planCoveredBeats")} value={segment.covered_beat_ids}
              path={[...base, "covered_beat_ids"]} onChange={update} />
          </div>
        </details>
      </div>
    </details>
  );
};

const EmotionGoalsField: React.FC<{
  value: VideoEmotionActionGoalV1[];
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ value, path, onChange }) => {
  const { t } = useI18n();
  return (
    <div className="space-y-2">
      {value.map((item, index) => (
        <div key={index} className="grid gap-2 rounded-lg border border-book-border/35 p-2 sm:grid-cols-[minmax(0,.8fr)_minmax(0,.8fr)_minmax(0,1.4fr)_auto]">
          <TextField label={t("video.chapter.planCharacter")} value={item.character} path={[...path, index, "character"]} onChange={onChange} />
          <TextField label={t("video.chapter.planEmotion")} value={item.emotion} path={[...path, index, "emotion"]} onChange={onChange} />
          <TextField label={t("video.chapter.planVisibleAction")} value={item.visible_action} path={[...path, index, "visible_action"]} onChange={onChange} />
          <div className="flex items-end justify-end">
            <BookIconButton label={t("video.chapter.planRemoveItem")} onClick={() => onChange(path, value.filter((_, itemIndex) => itemIndex !== index))} className="text-error hover:bg-error-bg"><Trash2 size={13} /></BookIconButton>
          </div>
        </div>
      ))}
      <BookButton size="sm" variant="ghost" onClick={() => onChange(path, [...value, { character: "", emotion: "", visible_action: "" }])}>
        <Plus size={12} />{t("video.chapter.planAddItem")}
      </BookButton>
    </div>
  );
};

const SpatialAnchorsField: React.FC<{
  value: VideoSpatialAnchorV1[];
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ value, path, onChange }) => {
  const { t } = useI18n();
  return (
    <div className="block min-w-0">
      <span className="mb-1 block text-small text-book-text-muted">
        {t("video.chapter.planSpatialAnchors")}
      </span>
      <div className="space-y-2">
        {value.map((item, index) => (
          <div key={index} className="grid min-w-0 gap-2 rounded-lg border border-book-border/35 p-2 sm:grid-cols-2">
            <TextField label={t("video.chapter.planAnchor")} value={item.anchor} path={[...path, index, "anchor"]} onChange={onChange} />
            <div className="flex min-w-0 items-end gap-1.5">
              <div className="min-w-0 flex-1">
                <TextField label={t("video.chapter.planUsage")} value={item.usage} path={[...path, index, "usage"]} onChange={onChange} />
              </div>
              <BookIconButton label={t("video.chapter.planRemoveItem")} onClick={() => onChange(path, value.filter((_, itemIndex) => itemIndex !== index))} className="mb-0.5 shrink-0 text-error hover:bg-error-bg"><Trash2 size={13} /></BookIconButton>
            </div>
          </div>
        ))}
      </div>
      <BookButton size="sm" variant="ghost" onClick={() => onChange(path, [...value, { anchor: "", usage: "" }])} className="mt-1">
        <Plus size={12} />{t("video.chapter.planAddItem")}
      </BookButton>
    </div>
  );
};

const TimeBlocksField: React.FC<{
  value: VideoPerformanceTimeBlockV1[];
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ value, path, onChange }) => {
  const { t } = useI18n();
  return (
    <AdvancedDetails title={t("video.chapter.planTimeBlocks")}>
      <div className="space-y-2">
        {value.map((item, index) => (
          <div key={index} className="grid gap-2 rounded-lg border border-book-border/35 p-2 sm:grid-cols-2">
            <NumberField label={t("video.chapter.planStartSec")} value={item.start_sec} path={[...path, index, "start_sec"]} onChange={onChange} />
            <NumberField label={t("video.chapter.planEndSec")} value={item.end_sec} path={[...path, index, "end_sec"]} onChange={onChange} />
            <TextField label={t("video.chapter.planCamera")} value={item.camera} path={[...path, index, "camera"]} onChange={onChange} />
            <TextField label={t("video.chapter.planSubjectAction")} value={item.subject_action} path={[...path, index, "subject_action"]} onChange={onChange} />
            <TextField label={t("video.chapter.planContinuityTarget")} value={item.continuity_target} path={[...path, index, "continuity_target"]} onChange={onChange} />
            <div className="flex items-end justify-end">
              <BookIconButton label={t("video.chapter.planRemoveItem")} onClick={() => onChange(path, value.filter((_, itemIndex) => itemIndex !== index))} className="text-error hover:bg-error-bg"><Trash2 size={13} /></BookIconButton>
            </div>
          </div>
        ))}
        <BookButton size="sm" variant="ghost" onClick={() => onChange(path, [...value, { start_sec: 0, end_sec: 1, camera: "", subject_action: "", continuity_target: "" }])}>
          <Plus size={12} />{t("video.chapter.planAddItem")}
        </BookButton>
      </div>
    </AdvancedDetails>
  );
};

const ActionScriptField: React.FC<{
  value: VideoPerformanceActionV1[];
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ value, path, onChange }) => {
  const { t } = useI18n();
  const fields: Array<[keyof VideoPerformanceActionV1, string]> = [
    ["character", t("video.chapter.planCharacter")],
    ["body", t("video.chapter.planBody")],
    ["movement", t("video.chapter.planMovement")],
    ["hands", t("video.chapter.planHands")],
    ["gaze", t("video.chapter.planGaze")],
    ["timing", t("video.chapter.planTiming")],
  ];
  return (
    <AdvancedDetails title={t("video.chapter.planActionScript")}>
      <div className="space-y-2">
        {value.map((item, index) => (
          <div key={index} className="grid gap-2 rounded-lg border border-book-border/35 p-2 sm:grid-cols-2">
            {fields.map(([field, label]) => <TextField key={field} label={label} value={item[field]} path={[...path, index, field]} onChange={onChange} />)}
            <div className="flex items-end justify-end sm:col-span-2">
              <BookIconButton label={t("video.chapter.planRemoveItem")} onClick={() => onChange(path, value.filter((_, itemIndex) => itemIndex !== index))} className="text-error hover:bg-error-bg"><Trash2 size={13} /></BookIconButton>
            </div>
          </div>
        ))}
        <BookButton size="sm" variant="ghost" onClick={() => onChange(path, [...value, { character: "", body: "", movement: "", hands: "", gaze: "", timing: "" }])}>
          <Plus size={12} />{t("video.chapter.planAddItem")}
        </BookButton>
      </div>
    </AdvancedDetails>
  );
};

const TextField: React.FC<{
  label: string;
  value: string;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ label, value, path, onChange }) => (
  <label className="block min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <textarea value={value} onChange={(event) => onChange(path, event.target.value)} rows={value.length > 80 ? 3 : 2} className="pyqt-input w-full resize-y text-xs" />
  </label>
);

const NullableTextField: React.FC<{
  label: string;
  value: string | null;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ label, value, path, onChange }) => (
  <label className="block min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <textarea value={value ?? ""} onChange={(event) => onChange(path, event.target.value || null)} rows={2} className="pyqt-input w-full resize-y text-xs" />
  </label>
);

const NumberField: React.FC<{
  label: string;
  value: number;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ label, value, path, onChange }) => {
  const [text, setText] = useState(String(value));
  const focused = useRef(false);

  useEffect(() => {
    if (!focused.current) setText(String(value));
  }, [value]);

  return (
    <label className="block min-w-0">
      <span className="mb-1 block text-small text-book-text-muted">{label}</span>
      <input
        type="number"
        step="any"
        required
        value={text}
        onFocus={() => { focused.current = true; }}
        onChange={(event) => {
          const nextText = event.currentTarget.value;
          setText(nextText);
          if (nextText.trim() && Number.isFinite(Number(nextText))) {
            onChange(path, Number(nextText));
          }
        }}
        onBlur={(event) => {
          focused.current = false;
          const nextText = event.currentTarget.value;
          const nextValue = Number(nextText);
          if (nextText.trim() && Number.isFinite(nextValue)) {
            onChange(path, nextValue);
            setText(String(nextValue));
          } else {
            setText(String(value));
          }
        }}
        className="pyqt-input w-full text-xs"
      />
    </label>
  );
};

const SelectField: React.FC<{
  label: string;
  value: string;
  options: ReadonlyArray<readonly [string, string]>;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ label, value, options, path, onChange }) => (
  <label className="block min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <ThemedSelect value={value} onChange={(selectedValue) => onChange(path, selectedValue)} className="pyqt-input w-full text-xs">
      {options.map(([optionValue, optionLabel]) => <option key={optionValue} value={optionValue}>{optionLabel}</option>)}
    </ThemedSelect>
  </label>
);

const StringListField: React.FC<{
  label: string;
  value: string[];
  path: ValuePath;
  onChange: UpdateValue;
  placeholder?: string;
}> = ({ label, value, path, onChange, placeholder }) => {
  const { t } = useI18n();
  return (
    <div className="block min-w-0">
      <span className="mb-1 block text-small text-book-text-muted">{label}</span>
      <div className="space-y-1.5">
        {value.map((item, index) => (
          <div key={index} className="flex min-w-0 items-start gap-1.5">
            <textarea value={item} placeholder={placeholder} onChange={(event) => onChange([...path, index], event.target.value)} rows={2} className="pyqt-input min-w-0 flex-1 resize-y text-xs" />
            <BookIconButton label={t("video.chapter.planRemoveItem")} onClick={() => onChange(path, value.filter((_, itemIndex) => itemIndex !== index))} className="shrink-0 text-error hover:bg-error-bg"><Trash2 size={13} /></BookIconButton>
          </div>
        ))}
      </div>
      <BookButton size="sm" variant="ghost" onClick={() => onChange(path, [...value, ""])} className="mt-1">
        <Plus size={12} />{t("video.chapter.planAddItem")}
      </BookButton>
    </div>
  );
};

const AdvancedDetails: React.FC<React.PropsWithChildren<{ title: string }>> = ({ title, children }) => (
  <details className="group border-t border-book-border/40 pt-2">
    <summary className="flex min-h-9 w-fit cursor-pointer list-none items-center gap-1.5 rounded-lg px-2 text-xs text-book-text-muted transition-colors hover:bg-book-bg-tertiary/60 hover:text-book-text-main [&::-webkit-details-marker]:hidden">
      <ChevronRight size={13} className="transition-transform group-open:rotate-90" />
      {title}
    </summary>
    <div className="pt-2">{children}</div>
  </details>
);
