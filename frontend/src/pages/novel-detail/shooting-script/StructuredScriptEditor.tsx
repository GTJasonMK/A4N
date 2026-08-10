import React, { useEffect, useState } from "react";
import { ChevronRight } from "lucide-react";

import type {
  ChapterShootingScriptV1,
  ShootingPerformanceBeatV1,
  ShootingSceneV1,
  ShootingShotV1,
} from "../../../api/shootingScript";
import {
  shootingCameraAngleLabel,
  shootingCameraMovementLabel,
  shootingCoverageLabel,
  shootingShotSizeLabel,
} from "../../../utils/shootingScriptPresentation";

type ValuePath = Array<string | number>;
type UpdateValue = (path: ValuePath, value: unknown) => void;

const EDITING_MODE_OPTIONS = [
  ["continuity", "连续剪辑"],
  ["intercut", "交叉剪辑"],
  ["montage", "蒙太奇"],
  ["flashback", "闪回"],
  ["parallel_action", "平行动作"],
  ["sound_bridge", "声音桥"],
] as const;

const VIEWPOINT_OPTIONS = [
  ["direct_subjective", "直接主观视点"],
  ["modified_subjective", "修正主观视点"],
  ["limited_third_person", "限制性第三人称"],
  ["omniscient_objective", "全知客观视点"],
] as const;

const CONTINUITY_OPTIONS = [
  ["transparent", "清晰连续"],
  ["deliberate_disorientation", "有意制造迷失"],
  ["open_ambiguity", "保留空间歧义"],
] as const;

const CAMERA_HEIGHT_OPTIONS = [
  ["ground", "贴地机位"],
  ["low", "低机位"],
  ["eye", "视线高度"],
  ["high", "高机位"],
  ["overhead", "顶置机位"],
] as const;

const SHOT_SIZE_OPTIONS = [
  "extreme_wide",
  "wide",
  "medium_wide",
  "medium",
  "medium_close_up",
  "close_up",
  "extreme_close_up",
  "insert",
] as const;

const CAMERA_ANGLE_OPTIONS = [
  "eye_level",
  "high",
  "low",
  "overhead",
  "dutch",
  "subjective",
] as const;

const CAMERA_MOVEMENT_OPTIONS = [
  "locked",
  "pan",
  "tilt",
  "push_in",
  "pull_out",
  "track",
  "dolly",
  "crane",
  "handheld",
  "steadicam",
  "whip_pan",
] as const;

const COVERAGE_OPTIONS = ["must_show", "should_show", "optional"] as const;

interface StructuredScriptEditorProps {
  script: ChapterShootingScriptV1;
  onChange: (script: ChapterShootingScriptV1) => void;
  disabled?: boolean;
}

export const StructuredScriptEditor: React.FC<
  StructuredScriptEditorProps
> = ({ script, onChange, disabled = false }) => {
  const update: UpdateValue = (path, value) => {
    const next = structuredClone(script) as unknown as Record<string, unknown>;
    let cursor: unknown = next;
    for (const key of path.slice(0, -1)) {
      cursor = (cursor as Record<string | number, unknown>)[key];
    }
    (cursor as Record<string | number, unknown>)[path[path.length - 1]] = value;
    onChange(next as unknown as ChapterShootingScriptV1);
  };

  const directing = script.directing_statement;

  return (
    <fieldset
      disabled={disabled}
      aria-busy={disabled || undefined}
      className="m-0 min-w-0 space-y-3 border-0 p-0"
    >
      <section className="a4n-flat-surface overflow-hidden">
        <div className="border-b border-book-border/45 px-3 py-2.5 sm:px-4">
          <h3 className="text-sm font-semibold text-book-text-main">导演意图</h3>
          <p className="mt-0.5 text-small text-book-text-muted">
            统一整章的叙事视点、影像语言与节奏方向。
          </p>
        </div>
        <div className="grid gap-3 p-3 sm:grid-cols-2 sm:p-4">
          <TextField
            label="核心戏剧问题"
            value={directing.central_dramatic_question}
            path={["directing_statement", "central_dramatic_question"]}
            onChange={update}
          />
          <TextField
            label="主角行动主线"
            value={directing.protagonist_throughline}
            path={["directing_statement", "protagonist_throughline"]}
            onChange={update}
          />
          <TextField
            label="观众体验"
            value={directing.audience_experience}
            path={["directing_statement", "audience_experience"]}
            onChange={update}
          />
          <TextField
            label="视点策略"
            value={directing.viewpoint_strategy}
            path={["directing_statement", "viewpoint_strategy"]}
            onChange={update}
          />
          <TextField
            label="视觉策略"
            value={directing.visual_strategy}
            path={["directing_statement", "visual_strategy"]}
            onChange={update}
          />
          <TextField
            label="空间策略"
            value={directing.spatial_strategy}
            path={["directing_statement", "spatial_strategy"]}
            onChange={update}
          />
          <TextField
            label="声音策略"
            value={directing.sound_strategy}
            path={["directing_statement", "sound_strategy"]}
            onChange={update}
          />
          <TextField
            label="节奏策略"
            value={directing.rhythm_strategy}
            path={["directing_statement", "rhythm_strategy"]}
            onChange={update}
          />
        </div>
        <DiagnosticDetails
          rows={[["结构版本", script.schema_version]]}
        />
      </section>

      {script.scenes.map((scene, sceneOffset) => (
        <SceneEditor
          key={`${scene.scene_index}-${scene.scene_identity}`}
          scene={scene}
          sceneOffset={sceneOffset}
          onChange={update}
        />
      ))}
    </fieldset>
  );
};

const SceneEditor: React.FC<{
  scene: ShootingSceneV1;
  sceneOffset: number;
  onChange: UpdateValue;
}> = ({ scene, sceneOffset, onChange }) => {
  const base: ValuePath = ["scenes", sceneOffset];

  return (
    <article className="a4n-flat-surface overflow-hidden">
      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1 border-b border-book-border/45 px-3 py-2.5 sm:px-4">
        <h3 className="text-sm font-semibold text-book-text-main">
          场景 {scene.scene_index} · {scene.title || "未命名场景"}
        </h3>
        <span className="text-small text-book-text-muted">
          {scene.shots.length} 个镜头
        </span>
      </div>

      <div className="space-y-4 p-3 sm:p-4">
        <section className="grid gap-3 sm:grid-cols-2">
          <TextField
            label="场景标题"
            value={scene.title}
            path={[...base, "title"]}
            onChange={onChange}
          />
          <TextField
            label="场景标头"
            value={scene.slugline}
            path={[...base, "slugline"]}
            onChange={onChange}
          />
          <TextField
            label="核心悬念"
            value={scene.dramatic_question}
            path={[...base, "dramatic_question"]}
            onChange={onChange}
          />
          <TextField
            label="场景目标"
            value={scene.scene_objective}
            path={[...base, "scene_objective"]}
            onChange={onChange}
          />
          <TextField
            label="主要阻碍"
            value={scene.scene_obstacle}
            path={[...base, "scene_obstacle"]}
            onChange={onChange}
          />
          <TextField
            label="声音策略"
            value={scene.sound_strategy}
            path={[...base, "sound_strategy"]}
            onChange={onChange}
          />
        </section>

        <section className="border-t border-book-border/40 pt-3">
          <h4 className="mb-2 text-xs font-semibold text-book-text-main">
            人物与时空
          </h4>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <TextField
              label="地点"
              value={scene.location}
              path={[...base, "location"]}
              onChange={onChange}
              nullable
              compact
            />
            <TextField
              label="时间"
              value={scene.time_of_day}
              path={[...base, "time_of_day"]}
              onChange={onChange}
              nullable
              compact
            />
            <StringListField
              label="参与人物"
              value={scene.participants}
              path={[...base, "participants"]}
              onChange={onChange}
              compact
            />
            <TextField
              label="焦点人物"
              value={scene.focal_character}
              path={[...base, "focal_character"]}
              onChange={onChange}
              nullable
              compact
            />
          </div>
        </section>

        <AdvancedDetails title="场景策略与连续性">
          <div className="grid gap-3 sm:grid-cols-2">
            <SelectField
              label="剪辑方式"
              value={scene.editing_mode}
              options={EDITING_MODE_OPTIONS}
              path={[...base, "editing_mode"]}
              onChange={onChange}
            />
            <SelectField
              label="视点方式"
              value={scene.viewpoint.mode}
              options={VIEWPOINT_OPTIONS}
              path={[...base, "viewpoint", "mode"]}
              onChange={onChange}
            />
            <TextField
              label="视点人物"
              value={scene.viewpoint.anchor_character}
              path={[...base, "viewpoint", "anchor_character"]}
              onChange={onChange}
              nullable
            />
            <TextField
              label="信息控制"
              value={scene.viewpoint.information_strategy}
              path={[...base, "viewpoint", "information_strategy"]}
              onChange={onChange}
            />
            <TextField
              label="关系起点"
              value={scene.relationship_before}
              path={[...base, "relationship_before"]}
              onChange={onChange}
            />
            <TextField
              label="关系终点"
              value={scene.relationship_after}
              path={[...base, "relationship_after"]}
              onChange={onChange}
            />
            <TextField
              label="进入状态"
              value={scene.entry_state}
              path={[...base, "entry_state"]}
              onChange={onChange}
            />
            <TextField
              label="结束状态"
              value={scene.exit_state}
              path={[...base, "exit_state"]}
              onChange={onChange}
            />
            <StringListField
              label="观众已知信息"
              value={scene.audience_knowledge_before}
              path={[...base, "audience_knowledge_before"]}
              onChange={onChange}
            />
            <StringListField
              label="场景新增信息"
              value={scene.audience_knowledge_after}
              path={[...base, "audience_knowledge_after"]}
              onChange={onChange}
            />
            <StringListField
              label="暂不揭示的信息"
              value={scene.withheld_information}
              path={[...base, "withheld_information"]}
              onChange={onChange}
            />
            <SelectField
              label="空间连续性"
              value={scene.spatial_continuity.continuity_intent}
              options={CONTINUITY_OPTIONS}
              path={[...base, "spatial_continuity", "continuity_intent"]}
              onChange={onChange}
            />
            <TextField
              label="主要轴线"
              value={scene.spatial_continuity.primary_axis}
              path={[...base, "spatial_continuity", "primary_axis"]}
              onChange={onChange}
              nullable
            />
            <TextField
              label="画面运动方向"
              value={scene.spatial_continuity.screen_direction}
              path={[...base, "spatial_continuity", "screen_direction"]}
              onChange={onChange}
              nullable
            />
            <TextField
              label="重新建立空间方式"
              value={scene.spatial_continuity.reorientation_strategy}
              path={[...base, "spatial_continuity", "reorientation_strategy"]}
              onChange={onChange}
              nullable
            />
          </div>

          {scene.transition_contracts.length > 0 ? (
            <div className="mt-4 border-t border-book-border/40 pt-3">
              <h5 className="mb-1 text-xs font-semibold text-book-text-main">
                转场约定
              </h5>
              <div className="divide-y divide-book-border/40">
                {scene.transition_contracts.map((transition, offset) => {
                  const path = [...base, "transition_contracts", offset];
                  return (
                    <div key={offset} className="grid gap-3 py-3 sm:grid-cols-2">
                      <NumberField
                        label="目标叙事单元"
                        value={transition.to_narrative_unit_index}
                        path={[...path, "to_narrative_unit_index"]}
                        onChange={onChange}
                      />
                      <TextField
                        label="转场方式"
                        value={transition.transition_type}
                        path={[...path, "transition_type"]}
                        onChange={onChange}
                      />
                      <TextField
                        label="延续动作"
                        value={transition.preserved_action}
                        path={[...path, "preserved_action"]}
                        onChange={onChange}
                        nullable
                      />
                      <TextField
                        label="声音桥"
                        value={transition.preserved_sound}
                        path={[...path, "preserved_sound"]}
                        onChange={onChange}
                        nullable
                      />
                      <StringListField
                        label="保持的观众认知"
                        value={transition.preserved_knowledge}
                        path={[...path, "preserved_knowledge"]}
                        onChange={onChange}
                      />
                      <TextField
                        label="强制切点"
                        value={transition.required_cut_point}
                        path={[...path, "required_cut_point"]}
                        onChange={onChange}
                        nullable
                      />
                    </div>
                  );
                })}
              </div>
            </div>
          ) : null}
        </AdvancedDetails>

        <section className="border-t border-book-border/45 pt-3">
          <div className="flex items-baseline justify-between gap-2">
            <h4 className="text-xs font-semibold text-book-text-main">镜头</h4>
            <span className="text-caption text-book-text-muted">
              同一场景内按拍摄顺序排列
            </span>
          </div>
          <div className="mt-1 divide-y divide-book-border/45">
            {scene.shots.map((shot, shotOffset) => (
              <ShotEditor
                key={shot.shot_id}
                shot={shot}
                path={[...base, "shots", shotOffset]}
                onChange={onChange}
              />
            ))}
          </div>
        </section>
      </div>

      <DiagnosticDetails
        rows={[
          ["场景身份", scene.scene_identity],
          ["段落序号", scene.sequence_index],
          ["叙事单元序号", scene.narrative_unit_index],
          ["时间线位置", scene.timeline_position],
          ["交叉剪辑组", scene.intercut_group_id ?? "—"],
          ["来源定位", scene.source_span_ids.join("、")],
        ]}
      />
    </article>
  );
};

const ShotEditor: React.FC<{
  shot: ShootingShotV1;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ shot, path, onChange }) => (
  <details className="group">
    <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-3 py-3 [&::-webkit-details-marker]:hidden">
      <div className="min-w-0">
        <div className="truncate text-xs font-semibold text-book-text-main">
          镜头 {shot.shot_index} · {shot.title || "未命名镜头"}
        </div>
        <div className="mt-0.5 flex flex-wrap gap-x-2 text-caption text-book-text-muted">
          <span>{shootingShotSizeLabel(shot.shot_size)}</span>
          <span>{shootingCameraMovementLabel(shot.camera_movement)}</span>
          <span>{shot.performance_beats.length} 个节拍</span>
        </div>
      </div>
      <ChevronRight
        size={14}
        className="text-book-text-muted transition-transform group-open:rotate-90"
      />
    </summary>

    <div className="space-y-4 pb-4 pl-2 sm:pl-3">
      <div className="grid gap-3 sm:grid-cols-2">
        <TextField
          label="镜头标题"
          value={shot.title}
          path={[...path, "title"]}
          onChange={onChange}
        />
        <TextField
          label="叙事目的"
          value={shot.narrative_purpose}
          path={[...path, "narrative_purpose"]}
          onChange={onChange}
        />
        <TextField
          label="主体"
          value={shot.subject}
          path={[...path, "subject"]}
          onChange={onChange}
        />
        <TextField
          label="构图"
          value={shot.composition}
          path={[...path, "composition"]}
          onChange={onChange}
        />
        <TextField
          label="镜头意图"
          value={shot.lens_intent}
          path={[...path, "lens_intent"]}
          onChange={onChange}
        />
        <NumberField
          label="自然时长（秒）"
          value={shot.estimated_natural_seconds}
          path={[...path, "estimated_natural_seconds"]}
          onChange={onChange}
        />
      </div>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
        <SelectField
          label="景别"
          value={shot.shot_size}
          options={SHOT_SIZE_OPTIONS.map((value) => [value, shootingShotSizeLabel(value)] as const)}
          path={[...path, "shot_size"]}
          onChange={onChange}
        />
        <SelectField
          label="角度"
          value={shot.camera_angle}
          options={CAMERA_ANGLE_OPTIONS.map((value) => [value, shootingCameraAngleLabel(value)] as const)}
          path={[...path, "camera_angle"]}
          onChange={onChange}
        />
        <SelectField
          label="机位高度"
          value={shot.camera_height}
          options={CAMERA_HEIGHT_OPTIONS}
          path={[...path, "camera_height"]}
          onChange={onChange}
        />
        <SelectField
          label="运动"
          value={shot.camera_movement}
          options={CAMERA_MOVEMENT_OPTIONS.map((value) => [value, shootingCameraMovementLabel(value)] as const)}
          path={[...path, "camera_movement"]}
          onChange={onChange}
        />
        <SelectField
          label="覆盖等级"
          value={shot.coverage}
          options={COVERAGE_OPTIONS.map((value) => [value, shootingCoverageLabel(value)] as const)}
          path={[...path, "coverage"]}
          onChange={onChange}
        />
      </div>

      <AdvancedDetails title="调度、声音与连续性">
        <div className="grid gap-3 sm:grid-cols-3">
          <TextField label="调度起点" value={shot.blocking.start} path={[...path, "blocking", "start"]} onChange={onChange} />
          <TextField label="移动过程" value={shot.blocking.movement} path={[...path, "blocking", "movement"]} onChange={onChange} />
          <TextField label="调度终点" value={shot.blocking.end} path={[...path, "blocking", "end"]} onChange={onChange} />
        </div>
        <div className="mt-3 grid gap-3 sm:grid-cols-2">
          <StringListField label="空间锚点" value={shot.spatial_anchors} path={[...path, "spatial_anchors"]} onChange={onChange} />
          <StringListField label="连续性要求" value={shot.continuity_requirements} path={[...path, "continuity_requirements"]} onChange={onChange} />
          <TextField label="摄影机起始状态" value={shot.camera_start_state} path={[...path, "camera_start_state"]} onChange={onChange} />
          <TextField label="摄影机结束状态" value={shot.camera_end_state} path={[...path, "camera_end_state"]} onChange={onChange} />
          <TextField label="画面进入状态" value={shot.start_state} path={[...path, "start_state"]} onChange={onChange} />
          <TextField label="画面结束状态" value={shot.end_state} path={[...path, "end_state"]} onChange={onChange} />
          <TextField label="入场转场" value={shot.transition_in} path={[...path, "transition_in"]} onChange={onChange} />
          <TextField label="出场转场" value={shot.transition_out} path={[...path, "transition_out"]} onChange={onChange} />
          <StringListField label="对白" value={shot.sound.dialogue} path={[...path, "sound", "dialogue"]} onChange={onChange} />
          <StringListField label="环境声" value={shot.sound.environment} path={[...path, "sound", "environment"]} onChange={onChange} />
          <StringListField label="音效" value={shot.sound.effects} path={[...path, "sound", "effects"]} onChange={onChange} />
          <StringListField label="音乐" value={shot.sound.music} path={[...path, "sound", "music"]} onChange={onChange} />
          <StringListField label="静默设计" value={shot.sound.silence} path={[...path, "sound", "silence"]} onChange={onChange} />
        </div>
      </AdvancedDetails>

      <section className="border-t border-book-border/40 pt-3">
        <h5 className="text-xs font-semibold text-book-text-main">表演节拍</h5>
        <div className="mt-1 divide-y divide-book-border/40 border-b border-book-border/40">
          {shot.performance_beats.map((beat, beatOffset) => (
            <BeatEditor
              key={beat.beat_id}
              beat={beat}
              path={[...path, "performance_beats", beatOffset]}
              onChange={onChange}
            />
          ))}
        </div>
      </section>

      {shot.dialogue_cues.length > 0 ? (
        <AdvancedDetails title="对白提示">
          <div className="divide-y divide-book-border/40">
            {shot.dialogue_cues.map((cue, offset) => {
              const cuePath = [...path, "dialogue_cues", offset];
              return (
                <div key={`${cue.beat_id}-${offset}`} className="grid gap-3 py-3 sm:grid-cols-2">
                  <TextField label="说话人" value={cue.speaker} path={[...cuePath, "speaker"]} onChange={onChange} />
                  <TextField label="台词或意图" value={cue.line_or_intent} path={[...cuePath, "line_or_intent"]} onChange={onChange} />
                  <TextField label="听者反应" value={cue.listener_reaction} path={[...cuePath, "listener_reaction"]} onChange={onChange} nullable />
                </div>
              );
            })}
          </div>
        </AdvancedDetails>
      ) : null}

      <DiagnosticDetails
        nested
        rows={[
          ["镜头 ID", shot.shot_id],
          ["来源定位", shot.source_span_ids.join("、")],
        ]}
      />
    </div>
  </details>
);

const BeatEditor: React.FC<{
  beat: ShootingPerformanceBeatV1;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ beat, path, onChange }) => (
  <details className="group">
    <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-3 py-2.5 [&::-webkit-details-marker]:hidden">
      <div className="min-w-0">
        <div className="text-small font-medium text-book-text-main">
          节拍 {beat.beat_index}{beat.actor ? ` · ${beat.actor}` : ""}
        </div>
        <div className="mt-0.5 truncate text-caption text-book-text-muted">
          {beat.visible_action}
        </div>
      </div>
      <ChevronRight size={13} className="text-book-text-muted transition-transform group-open:rotate-90" />
    </summary>
    <div className="grid gap-3 pb-3 pl-2 sm:grid-cols-2 sm:pl-3">
      <TextField label="演员" value={beat.actor} path={[...path, "actor"]} onChange={onChange} nullable />
      <TextField label="可见动作" value={beat.visible_action} path={[...path, "visible_action"]} onChange={onChange} />
      <TextField label="行动目标" value={beat.objective} path={[...path, "objective"]} onChange={onChange} />
      <TextField label="行动阻力" value={beat.obstacle} path={[...path, "obstacle"]} onChange={onChange} />
      <TextField label="反应" value={beat.reaction} path={[...path, "reaction"]} onChange={onChange} nullable />
      <TextField label="表情" value={beat.expression} path={[...path, "expression"]} onChange={onChange} nullable />
      <TextField label="视线" value={beat.gaze} path={[...path, "gaze"]} onChange={onChange} nullable />
      <TextField label="姿态与移动" value={beat.pose_or_movement} path={[...path, "pose_or_movement"]} onChange={onChange} nullable />
      <TextField label="对白意图" value={beat.dialogue_intent} path={[...path, "dialogue_intent"]} onChange={onChange} nullable />
      <TextField label="观众此刻的问题" value={beat.audience_question} path={[...path, "audience_question"]} onChange={onChange} />
      <TextField label="回答或延迟" value={beat.answer_or_delay} path={[...path, "answer_or_delay"]} onChange={onChange} />
      <StringListField label="新增认知" value={beat.knowledge_delta} path={[...path, "knowledge_delta"]} onChange={onChange} />
      <TextField label="关系变化" value={beat.relationship_delta} path={[...path, "relationship_delta"]} onChange={onChange} nullable />
      <TextField label="空间变化" value={beat.spatial_change} path={[...path, "spatial_change"]} onChange={onChange} nullable />
      <TextField label="进入状态" value={beat.entry_state} path={[...path, "entry_state"]} onChange={onChange} />
      <TextField label="结束状态" value={beat.exit_state} path={[...path, "exit_state"]} onChange={onChange} />
      <NumberField label="预计秒数" value={beat.estimated_seconds} path={[...path, "estimated_seconds"]} onChange={onChange} />
      <SwitchField label="必须拍摄" value={beat.required} path={[...path, "required"]} onChange={onChange} />
      <div className="sm:col-span-2">
        <DiagnosticDetails
          nested
          rows={[
            ["节拍 ID", beat.beat_id],
            ["来源定位", beat.source_span_ids.join("、")],
          ]}
        />
      </div>
    </div>
  </details>
);

const TextField: React.FC<{
  label: string;
  value: string | null;
  path: ValuePath;
  onChange: UpdateValue;
  nullable?: boolean;
  compact?: boolean;
}> = ({ label, value, path, onChange, nullable = false, compact = false }) => (
  <label className="block min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <textarea
      value={value ?? ""}
      onChange={(event) =>
        onChange(path, nullable && event.target.value === "" ? null : event.target.value)
      }
      rows={compact ? 1 : (value?.length ?? 0) > 80 ? 3 : 2}
      className="pyqt-input w-full resize-y text-xs"
    />
  </label>
);

const NumberField: React.FC<{
  label: string;
  value: number;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ label, value, path, onChange }) => (
  <label className="block min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <input
      type="number"
      step="any"
      value={value}
      onChange={(event) => onChange(path, Number(event.target.value))}
      className="pyqt-input w-full text-xs"
    />
  </label>
);

const SelectField: React.FC<{
  label: string;
  value: string;
  options: ReadonlyArray<readonly [string, string]>;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ label, value, options, path, onChange }) => (
  <label className="block min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <select
      value={value}
      onChange={(event) => onChange(path, event.target.value)}
      className="pyqt-input w-full text-xs"
    >
      {options.map(([optionValue, optionLabel]) => (
        <option key={optionValue} value={optionValue}>
          {optionLabel}
        </option>
      ))}
    </select>
  </label>
);

const StringListField: React.FC<{
  label: string;
  value: string[];
  path: ValuePath;
  onChange: UpdateValue;
  compact?: boolean;
}> = ({ label, value, path, onChange, compact = false }) => {
  const serialized = value.join("\n");
  const [draft, setDraft] = useState(serialized);

  useEffect(() => {
    setDraft(serialized);
  }, [serialized]);

  const commit = () => {
    const next = draft
      .split("\n")
      .map((item) => item.trim())
      .filter(Boolean);
    if (next.join("\n") !== serialized) onChange(path, next);
  };

  return (
    <label className="block min-w-0">
      <span className="mb-1 block text-small text-book-text-muted">{label}</span>
      <textarea
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        rows={compact ? 1 : Math.max(2, Math.min(4, value.length))}
        placeholder="每行一项"
        className="pyqt-input w-full resize-y text-xs"
      />
    </label>
  );
};

const SwitchField: React.FC<{
  label: string;
  value: boolean;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ label, value, path, onChange }) => (
  <div className="min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <button
      type="button"
      role="switch"
      aria-checked={value}
      onClick={() => onChange(path, !value)}
      className="inline-flex min-h-9 items-center gap-2 text-small text-book-text-sub"
    >
      <span>{value ? "是" : "否"}</span>
      <span className={`relative inline-flex h-[18px] w-8 rounded-full transition-colors ${value ? "bg-book-primary" : "bg-book-text-muted/25"}`}>
        <span className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${value ? "translate-x-4" : "translate-x-[2px]"}`} />
      </span>
    </button>
  </div>
);

const AdvancedDetails: React.FC<React.PropsWithChildren<{ title: string }>> = ({
  title,
  children,
}) => (
  <details className="group border-t border-book-border/40 pt-2">
    <summary className="flex min-h-9 w-fit cursor-pointer list-none items-center gap-1.5 rounded-lg px-2 text-xs text-book-text-muted transition-colors hover:bg-book-bg-tertiary/60 hover:text-book-text-main [&::-webkit-details-marker]:hidden">
      <ChevronRight size={13} className="transition-transform group-open:rotate-90" />
      {title}
    </summary>
    <div className="pt-2">{children}</div>
  </details>
);

const DiagnosticDetails: React.FC<{
  rows: Array<readonly [string, string | number]>;
  nested?: boolean;
}> = ({ rows, nested = false }) => (
  <details
    className={
      nested
        ? "text-caption text-book-text-muted"
        : "border-t border-book-border/40 px-3 py-2 text-caption text-book-text-muted sm:px-4"
    }
  >
    <summary className="w-fit cursor-pointer rounded px-1 py-0.5 transition-colors hover:bg-book-bg-tertiary hover:text-book-text-main">
      诊断信息
    </summary>
    <dl className="mt-2 grid grid-cols-[5rem_minmax(0,1fr)] gap-x-2 gap-y-1 font-mono">
      {rows.map(([label, value]) => (
        <React.Fragment key={label}>
          <dt>{label}</dt>
          <dd className="min-w-0 break-all">{value}</dd>
        </React.Fragment>
      ))}
    </dl>
  </details>
);
