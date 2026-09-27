import {
  AdvancedDetails,
  BeatEditor,
  DiagnosticDetails,
  SelectField,
  StringListField,
  TextField,
  type UpdateValue,
  type ValuePath,
} from "./StructuredScriptEditorControls";
import React, { useEffect, useRef } from "react";
import { ChevronRight, Plus, Trash2 } from "lucide-react";

import type {
  ChapterShootingScriptV2,
  ShootingDirectingStatementV1,
  ShootingDialogueCueV1,
  ShootingPerformanceBeatV2,
  ShootingSceneV2,
  ShootingShotV2,
  ShootingTransitionContractV1,
} from "../../../api/shootingScript";
import {
  shootingCameraAngleLabel,
  shootingCameraMovementLabel,
  shootingCoverageLabel,
  shootingShotSizeLabel,
  shootingShotDurationLabel,
} from "../../../utils/shootingScriptPresentation";
import { useI18n, type MessageKey } from "../../../i18n";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";

const EDITING_MODE_OPTIONS: ReadonlyArray<readonly [string, MessageKey]> = [
  ["continuity", "shootingScript.editor.editingContinuity"],
  ["intercut", "shootingScript.editor.editingIntercut"],
  ["montage", "shootingScript.editor.editingMontage"],
  ["flashback", "shootingScript.editor.editingFlashback"],
  ["parallel_action", "shootingScript.editor.editingParallelAction"],
  ["sound_bridge", "shootingScript.editor.editingSoundBridge"],
] as const;

const VIEWPOINT_OPTIONS: ReadonlyArray<readonly [string, MessageKey]> = [
  ["direct_subjective", "shootingScript.editor.viewpointDirectSubjective"],
  ["modified_subjective", "shootingScript.editor.viewpointModifiedSubjective"],
  ["limited_third_person", "shootingScript.editor.viewpointLimitedThirdPerson"],
  [
    "omniscient_objective",
    "shootingScript.editor.viewpointOmniscientObjective",
  ],
] as const;

const CONTINUITY_OPTIONS: ReadonlyArray<readonly [string, MessageKey]> = [
  ["transparent", "shootingScript.editor.continuityTransparent"],
  [
    "deliberate_disorientation",
    "shootingScript.editor.continuityDisorientation",
  ],
  ["open_ambiguity", "shootingScript.editor.continuityAmbiguity"],
] as const;

const TRANSITION_TYPE_OPTIONS: ReadonlyArray<readonly [string, MessageKey]> = [
  ["continuous", "shootingScript.editor.transitionContinuous"],
  ["hard_cut", "shootingScript.editor.transitionHardCut"],
  ["match_cut", "shootingScript.editor.transitionMatchCut"],
  ["whip_pan", "shootingScript.editor.transitionWhipPan"],
  ["action_wipe", "shootingScript.editor.transitionActionWipe"],
  ["sound_bridge", "shootingScript.editor.transitionSoundBridge"],
  ["dissolve", "shootingScript.editor.transitionDissolve"],
  ["fade", "shootingScript.editor.transitionFade"],
  ["focus_transition", "shootingScript.editor.transitionFocus"],
  ["morph", "shootingScript.editor.transitionMorph"],
] as const;

const CAMERA_HEIGHT_OPTIONS: ReadonlyArray<readonly [string, MessageKey]> = [
  ["ground", "shootingScript.editor.cameraGround"],
  ["low", "shootingScript.editor.cameraLow"],
  ["eye", "shootingScript.editor.cameraEye"],
  ["high", "shootingScript.editor.cameraHigh"],
  ["overhead", "shootingScript.editor.cameraOverhead"],
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
  script: ChapterShootingScriptV2;
  onChange: (script: ChapterShootingScriptV2) => void;
  disabled?: boolean;
  focusShotId?: string | null;
}

const newEntryId = (prefix: string) => `${prefix}-${crypto.randomUUID()}`;

const newTransition = (target: number): ShootingTransitionContractV1 => ({
  to_narrative_unit_index: target,
  transition_type: "continuous",
  preserved_action: null,
  preserved_sound: null,
  preserved_knowledge: [],
  required_cut_point: null,
});

const newBeat = (
  shot: ShootingShotV2,
  beatIndex: number,
): ShootingPerformanceBeatV2 => ({
  beat_index: beatIndex,
  beat_id: newEntryId("beat"),
  source_span_ids: [...shot.source_span_ids],
  user_addition_ids: [],
  actor: null,
  objective: "",
  obstacle: "",
  visible_action: "",
  reaction: null,
  expression: null,
  gaze: null,
  pose_or_movement: null,
  dialogue_intent: null,
  audience_question: "",
  answer_or_delay: "",
  knowledge_delta: [],
  relationship_delta: null,
  spatial_change: null,
  entry_state: "",
  exit_state: "",
  required: false,
});

const newDialogueCue = (
  beat: ShootingPerformanceBeatV2,
): ShootingDialogueCueV1 => ({
  beat_id: beat.beat_id,
  speaker: beat.actor ?? "",
  line_or_intent: "",
  listener_reaction: null,
});

const emptyDirectingStatement = (): ShootingDirectingStatementV1 => ({
  central_dramatic_question: "",
  protagonist_throughline: "",
  audience_experience: "",
  viewpoint_strategy: "",
  visual_strategy: "",
  spatial_strategy: "",
  sound_strategy: "",
  rhythm_strategy: "",
});

export const StructuredScriptEditor: React.FC<StructuredScriptEditorProps> = ({
  script,
  onChange,
  disabled = false,
  focusShotId = null,
}) => {
  const { t } = useI18n();
  const editorRef = useRef<HTMLFieldSetElement>(null);
  useEffect(() => {
    if (!focusShotId) return;
    const target = editorRef.current?.querySelector<HTMLElement>(`[data-shot-id="${CSS.escape(focusShotId)}"]`);
    if (!target) return;
    for (let details = target.closest("details"); details; details = details.parentElement?.closest("details") ?? null) {
      details.open = true;
    }
    target.querySelector<HTMLElement>("textarea, input:not([readonly])")?.focus();
  }, [focusShotId]);
  const update: UpdateValue = (path, value) => {
    const next = structuredClone(script);
    let cursor: object = next;
    for (const key of path.slice(0, -1)) {
      const child = Reflect.get(cursor, key);
      if (child == null || typeof child !== "object") {
        throw new Error(`Invalid shooting script edit path: ${path.join(".")}`);
      }
      cursor = child;
    }
    const leaf = path[path.length - 1];
    if (leaf == null || !Reflect.set(cursor, leaf, value)) {
      throw new Error(`Invalid shooting script edit path: ${path.join(".")}`);
    }
    onChange(next);
  };

  const directing = script.directing_statement;
  const deleteDirecting = async () => {
    if (
      !(await confirmDialog({
        title: t("shootingScript.editor.deleteDirectingIntentTitle"),
        message: t("shootingScript.editor.deleteDirectingIntentMessage"),
        confirmText: t("shootingScript.editor.deleteDirectingIntent"),
        dialogType: "danger",
      }))
    )
      return;
    onChange({ ...structuredClone(script), directing_statement: null });
  };
  const deleteScene = async (sceneOffset: number) => {
    const removed = script.scenes[sceneOffset];
    if (
      !removed ||
      !(await confirmDialog({
        title: t("shootingScript.editor.deleteSceneTitle"),
        message: t("shootingScript.editor.deleteSceneMessage"),
        confirmText: t("shootingScript.editor.deleteScene"),
        dialogType: "danger",
      }))
    )
      return;
    const next = structuredClone(script);
    next.scenes = next.scenes
      .filter((_, index) => index !== sceneOffset)
      .map((scene, index) => ({
        ...scene,
        scene_index: index + 1,
        sequence_index: index + 1,
        timeline_position: index + 1,
        transition_contracts: scene.transition_contracts.filter(
          (transition) =>
            transition.to_narrative_unit_index !== removed.narrative_unit_index,
        ),
      }));
    onChange(next);
  };

  return (
    <fieldset
      ref={editorRef}
      disabled={disabled}
      aria-busy={disabled || undefined}
      className="m-0 min-w-0 space-y-3 border-0 p-0"
    >
      <details className="a4n-flat-surface overflow-hidden">
        <summary className="group flex cursor-pointer list-none items-center gap-2 border-b border-book-border/45 px-3 py-2.5 [&::-webkit-details-marker]:hidden sm:px-4">
          <ChevronRight
            size={15}
            className="shrink-0 text-book-text-muted transition-transform group-open:rotate-90"
          />
          <span className="min-w-0 flex-1">
            <span className="block text-sm font-semibold text-book-text-main">
              {t("shootingScript.editor.directingIntent")}
            </span>
            <span className="mt-0.5 block text-small text-book-text-muted">
              {t("shootingScript.editor.directingIntentHint")}
            </span>
          </span>
          {directing ? (
            <BookIconButton
              label={t("shootingScript.editor.deleteDirectingIntent")}
              onClick={(event) => {
                event.preventDefault();
                void deleteDirecting();
              }}
              className="text-error hover:bg-error-bg"
            >
              <Trash2 size={14} />
            </BookIconButton>
          ) : null}
        </summary>
        {directing ? (
          <div className="grid gap-3 p-3 sm:grid-cols-2 sm:p-4">
            <TextField
              label={t("shootingScript.editor.centralDramaticQuestion")}
              value={directing.central_dramatic_question}
              path={["directing_statement", "central_dramatic_question"]}
              onChange={update}
            />
            <TextField
              label={t("shootingScript.editor.protagonistThroughline")}
              value={directing.protagonist_throughline}
              path={["directing_statement", "protagonist_throughline"]}
              onChange={update}
            />
            <TextField
              label={t("shootingScript.editor.audienceExperience")}
              value={directing.audience_experience}
              path={["directing_statement", "audience_experience"]}
              onChange={update}
            />
            <TextField
              label={t("shootingScript.editor.viewpointStrategy")}
              value={directing.viewpoint_strategy}
              path={["directing_statement", "viewpoint_strategy"]}
              onChange={update}
            />
            <TextField
              label={t("shootingScript.editor.visualStrategy")}
              value={directing.visual_strategy}
              path={["directing_statement", "visual_strategy"]}
              onChange={update}
            />
            <TextField
              label={t("shootingScript.editor.spatialStrategy")}
              value={directing.spatial_strategy}
              path={["directing_statement", "spatial_strategy"]}
              onChange={update}
            />
            <TextField
              label={t("shootingScript.editor.soundStrategy")}
              value={directing.sound_strategy}
              path={["directing_statement", "sound_strategy"]}
              onChange={update}
            />
            <TextField
              label={t("shootingScript.editor.rhythmStrategy")}
              value={directing.rhythm_strategy}
              path={["directing_statement", "rhythm_strategy"]}
              onChange={update}
            />
          </div>
        ) : (
          <div className="flex flex-wrap items-center justify-between gap-2 p-3 text-small text-book-text-muted sm:p-4">
            <span>{t("shootingScript.editor.directingIntentEmpty")}</span>
            <BookButton
              size="sm"
              variant="secondary"
              onClick={() =>
                onChange({
                  ...structuredClone(script),
                  directing_statement: emptyDirectingStatement(),
                })
              }
            >
              <Plus size={13} />
              {t("shootingScript.editor.addDirectingIntent")}
            </BookButton>
          </div>
        )}
        <DiagnosticDetails
          rows={[
            [t("shootingScript.editor.schemaVersion"), script.schema_version],
          ]}
        />
      </details>

      {script.scenes.map((scene, sceneOffset) => (
        <SceneEditor
          key={`${scene.scene_index}-${scene.scene_identity}`}
          scene={scene}
          sceneOffset={sceneOffset}
          narrativeUnitIndexes={script.scenes.map(
            ({ narrative_unit_index }) => narrative_unit_index,
          )}
          onChange={update}
          onDelete={() => void deleteScene(sceneOffset)}
        />
      ))}
    </fieldset>
  );
};

const SceneEditor: React.FC<{
  scene: ShootingSceneV2;
  sceneOffset: number;
  narrativeUnitIndexes: number[];
  onChange: UpdateValue;
  onDelete: () => void;
}> = ({ scene, sceneOffset, narrativeUnitIndexes, onChange, onDelete }) => {
  const { t } = useI18n();
  const base: ValuePath = ["scenes", sceneOffset];

  return (
    <details className="group/scene a4n-flat-surface overflow-hidden">
      <summary className="flex cursor-pointer list-none items-center gap-2 border-b border-book-border/45 px-3 py-2.5 [&::-webkit-details-marker]:hidden sm:px-4">
        <ChevronRight
          size={15}
          className="shrink-0 text-book-text-muted transition-transform group-open/scene:rotate-90"
        />
        <div className="flex min-w-0 flex-1 flex-wrap items-baseline gap-x-2 gap-y-1">
          <h3 className="text-sm font-semibold text-book-text-main">
            {t("shootingScript.editor.scene", { index: scene.scene_index })} ·{" "}
            {scene.title}
          </h3>
          <span className="text-small text-book-text-muted">
            {t("shootingScript.editor.shotsCount", {
              count: scene.shots.length,
            })}
          </span>
        </div>
        <BookIconButton
          label={t("shootingScript.editor.deleteScene")}
          onClick={(event) => {
            event.preventDefault();
            onDelete();
          }}
          className="text-error hover:bg-error-bg"
        >
          <Trash2 size={14} />
        </BookIconButton>
      </summary>

      <div className="space-y-4 p-3 sm:p-4">
        <section className="grid gap-3 sm:grid-cols-2">
          <TextField
            label={t("shootingScript.editor.sceneTitle")}
            value={scene.title}
            path={[...base, "title"]}
            onChange={onChange}
          />
          <TextField
            label={t("shootingScript.editor.sceneSlugline")}
            value={scene.slugline}
            path={[...base, "slugline"]}
            onChange={onChange}
          />
          <TextField
            label={t("shootingScript.editor.dramaticQuestion")}
            value={scene.dramatic_question}
            path={[...base, "dramatic_question"]}
            onChange={onChange}
          />
          <TextField
            label={t("shootingScript.editor.sceneObjective")}
            value={scene.scene_objective}
            path={[...base, "scene_objective"]}
            onChange={onChange}
          />
          <TextField
            label={t("shootingScript.editor.sceneObstacle")}
            value={scene.scene_obstacle}
            path={[...base, "scene_obstacle"]}
            onChange={onChange}
          />
          <TextField
            label={t("shootingScript.editor.soundStrategy")}
            value={scene.sound_strategy}
            path={[...base, "sound_strategy"]}
            onChange={onChange}
          />
        </section>

        <section className="border-t border-book-border/40 pt-3">
          <h4 className="mb-2 text-xs font-semibold text-book-text-main">
            {t("shootingScript.editor.charactersAndTime")}
          </h4>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <TextField
              label={t("shootingScript.editor.location")}
              value={scene.location}
              path={[...base, "location"]}
              onChange={onChange}
              nullable
              compact
            />
            <TextField
              label={t("shootingScript.editor.time")}
              value={scene.time_of_day}
              path={[...base, "time_of_day"]}
              onChange={onChange}
              nullable
              compact
            />
            <StringListField
              label={t("shootingScript.editor.participants")}
              value={scene.participants}
              path={[...base, "participants"]}
              onChange={onChange}
              compact
            />
            <TextField
              label={t("shootingScript.editor.focalCharacter")}
              value={scene.focal_character}
              path={[...base, "focal_character"]}
              onChange={onChange}
              nullable
              compact
            />
          </div>
        </section>

        <AdvancedDetails
          title={t("shootingScript.editor.sceneStrategyContinuity")}
        >
          <div className="grid gap-3 sm:grid-cols-2">
            <SelectField
              label={t("shootingScript.editor.editingMode")}
              value={scene.editing_mode}
              options={EDITING_MODE_OPTIONS.map(
                ([value, key]) => [value, t(key)] as const,
              )}
              path={[...base, "editing_mode"]}
              onChange={onChange}
            />
            <SelectField
              label={t("shootingScript.editor.viewpointMode")}
              value={scene.viewpoint.mode}
              options={VIEWPOINT_OPTIONS.map(
                ([value, key]) => [value, t(key)] as const,
              )}
              path={[...base, "viewpoint", "mode"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.anchorCharacter")}
              value={scene.viewpoint.anchor_character}
              path={[...base, "viewpoint", "anchor_character"]}
              onChange={onChange}
              nullable
            />
            <TextField
              label={t("shootingScript.editor.informationStrategy")}
              value={scene.viewpoint.information_strategy}
              path={[...base, "viewpoint", "information_strategy"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.relationshipBefore")}
              value={scene.relationship_before}
              path={[...base, "relationship_before"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.relationshipAfter")}
              value={scene.relationship_after}
              path={[...base, "relationship_after"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.entryState")}
              value={scene.entry_state}
              path={[...base, "entry_state"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.exitState")}
              value={scene.exit_state}
              path={[...base, "exit_state"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.intercutGroup")}
              value={scene.intercut_group_id}
              path={[...base, "intercut_group_id"]}
              onChange={onChange}
              nullable
            />
            <StringListField
              label={t("shootingScript.editor.audienceKnowledgeBefore")}
              value={scene.audience_knowledge_before}
              path={[...base, "audience_knowledge_before"]}
              onChange={onChange}
            />
            <StringListField
              label={t("shootingScript.editor.audienceKnowledgeAfter")}
              value={scene.audience_knowledge_after}
              path={[...base, "audience_knowledge_after"]}
              onChange={onChange}
            />
            <StringListField
              label={t("shootingScript.editor.withheldInformation")}
              value={scene.withheld_information}
              path={[...base, "withheld_information"]}
              onChange={onChange}
            />
            <SelectField
              label={t("shootingScript.editor.spatialContinuity")}
              value={scene.spatial_continuity.continuity_intent}
              options={CONTINUITY_OPTIONS.map(
                ([value, key]) => [value, t(key)] as const,
              )}
              path={[...base, "spatial_continuity", "continuity_intent"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.primaryAxis")}
              value={scene.spatial_continuity.primary_axis}
              path={[...base, "spatial_continuity", "primary_axis"]}
              onChange={onChange}
              nullable
            />
            <TextField
              label={t("shootingScript.editor.screenDirection")}
              value={scene.spatial_continuity.screen_direction}
              path={[...base, "spatial_continuity", "screen_direction"]}
              onChange={onChange}
              nullable
            />
            <TextField
              label={t("shootingScript.editor.reorientationStrategy")}
              value={scene.spatial_continuity.reorientation_strategy}
              path={[...base, "spatial_continuity", "reorientation_strategy"]}
              onChange={onChange}
              nullable
            />
          </div>

          {scene.transition_contracts.length > 0 ? (
            <div className="mt-4 border-t border-book-border/40 pt-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h5 className="text-xs font-semibold text-book-text-main">
                  {t("shootingScript.editor.transitionContracts")}
                </h5>
                <BookButton
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    const target = narrativeUnitIndexes.find(
                      (index) => index !== scene.narrative_unit_index,
                    );
                    if (target == null) return;
                    onChange(
                      [...base, "transition_contracts"],
                      [...scene.transition_contracts, newTransition(target)],
                    );
                  }}
                  disabled={narrativeUnitIndexes.length < 2}
                >
                  <Plus size={13} />
                  {t("shootingScript.editor.addTransition")}
                </BookButton>
              </div>
              <div className="divide-y divide-book-border/40">
                {scene.transition_contracts.map((transition, offset) => {
                  const path = [...base, "transition_contracts", offset];
                  return (
                    <div
                      key={offset}
                      className="grid gap-3 py-3 sm:grid-cols-2"
                    >
                      <SelectField
                        label={t("shootingScript.editor.narrativeUnitTarget")}
                        value={String(transition.to_narrative_unit_index)}
                        options={narrativeUnitIndexes
                          .filter(
                            (index) => index !== scene.narrative_unit_index,
                          )
                          .map(
                            (index) => [String(index), String(index)] as const,
                          )}
                        path={[...path, "to_narrative_unit_index"]}
                        onChange={(valuePath, value) =>
                          onChange(valuePath, Number(value))
                        }
                      />
                      <SelectField
                        label={t("shootingScript.editor.transitionType")}
                        value={transition.transition_type}
                        options={TRANSITION_TYPE_OPTIONS}
                        path={[...path, "transition_type"]}
                        onChange={onChange}
                      />
                      <TextField
                        label={t("shootingScript.editor.preservedAction")}
                        value={transition.preserved_action}
                        path={[...path, "preserved_action"]}
                        onChange={onChange}
                        nullable
                      />
                      <TextField
                        label={t("shootingScript.editor.preservedSound")}
                        value={transition.preserved_sound}
                        path={[...path, "preserved_sound"]}
                        onChange={onChange}
                        nullable
                      />
                      <StringListField
                        label={t("shootingScript.editor.preservedKnowledge")}
                        value={transition.preserved_knowledge}
                        path={[...path, "preserved_knowledge"]}
                        onChange={onChange}
                      />
                      <TextField
                        label={t("shootingScript.editor.requiredCutPoint")}
                        value={transition.required_cut_point}
                        path={[...path, "required_cut_point"]}
                        onChange={onChange}
                        nullable
                      />
                      <div className="flex items-end justify-end sm:col-span-2">
                        <BookIconButton
                          label={t("shootingScript.editor.deleteTransition")}
                          onClick={() =>
                            onChange(
                              [...base, "transition_contracts"],
                              scene.transition_contracts.filter(
                                (_, index) => index !== offset,
                              ),
                            )
                          }
                          className="text-error hover:bg-error-bg"
                        >
                          <Trash2 size={14} />
                        </BookIconButton>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ) : (
            <div className="mt-4 border-t border-book-border/40 pt-3">
              <BookButton
                size="sm"
                variant="ghost"
                onClick={() => {
                  const target = narrativeUnitIndexes.find(
                    (index) => index !== scene.narrative_unit_index,
                  );
                  if (target == null) return;
                  onChange(
                    [...base, "transition_contracts"],
                    [newTransition(target)],
                  );
                }}
                disabled={narrativeUnitIndexes.length < 2}
              >
                <Plus size={13} />
                {t("shootingScript.editor.addTransition")}
              </BookButton>
            </div>
          )}
        </AdvancedDetails>

        <section className="border-t border-book-border/45 pt-3">
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <h4 className="text-xs font-semibold text-book-text-main">
              {t("shootingScript.editor.shot")}
            </h4>
            <span className="text-caption text-book-text-muted">
              {t("shootingScript.editor.shotOrderHint")}
            </span>
          </div>
          <p className="mt-1 text-caption text-book-text-muted">
            {t("shootingScript.editor.shotStructureHint")}
          </p>
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
          [t("shootingScript.editor.sceneIdentity"), scene.scene_identity],
          [t("shootingScript.editor.paragraphIndex"), scene.sequence_index],
          [
            t("shootingScript.editor.narrativeUnitIndex"),
            scene.narrative_unit_index,
          ],
          [
            t("shootingScript.editor.timelinePosition"),
            scene.timeline_position,
          ],
          [
            t("shootingScript.editor.intercutGroup"),
            scene.intercut_group_id ?? "—",
          ],
          [
            t("shootingScript.editor.sceneSourceLocation"),
            scene.source_span_ids.join("、"),
          ],
        ]}
      />
    </details>
  );
};

const ShotEditor: React.FC<{
  shot: ShootingShotV2;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ shot, path, onChange }) => {
  const { t, locale } = useI18n();
  const updateShot = (next: ShootingShotV2) => onChange(path, next);
  const addBeat = () => {
    const next = structuredClone(shot);
    next.performance_beats.push(newBeat(shot, next.performance_beats.length + 1));
    updateShot(next);
  };
  const removeBeat = (beatOffset: number) => {
    const next = structuredClone(shot);
    const [removed] = next.performance_beats.splice(beatOffset, 1);
    next.performance_beats.forEach((beat, index) => {
      beat.beat_index = index + 1;
    });
    next.dialogue_cues = next.dialogue_cues.filter(
      (cue) => cue.beat_id !== removed?.beat_id,
    );
    updateShot(next);
  };
  const addDialogueCue = () => {
    const beat = shot.performance_beats[0];
    if (!beat) return;
    updateShot({
      ...structuredClone(shot),
      dialogue_cues: [...shot.dialogue_cues, newDialogueCue(beat)],
    });
  };
  const removeDialogueCue = (cueOffset: number) => {
    updateShot({
      ...structuredClone(shot),
      dialogue_cues: shot.dialogue_cues.filter(
        (_, index) => index !== cueOffset,
      ),
    });
  };
  return (
    <details className="group" data-shot-id={shot.shot_id}>
      <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-3 py-3 [&::-webkit-details-marker]:hidden">
        <div className="min-w-0">
          <div className="truncate text-xs font-semibold text-book-text-main">
            {t("shootingScript.editor.shot")} {shot.shot_index} · {shot.title}
          </div>
          <div className="mt-0.5 flex flex-wrap gap-x-2 text-caption text-book-text-muted">
            <span>{shootingShotSizeLabel(shot.shot_size, t)}</span>
            <span>{shootingCameraMovementLabel(shot.camera_movement, t)}</span>
            <span>
              {t("shootingScript.editor.beatCount", {
                count: shot.performance_beats.length,
              })}
            </span>
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
            label={t("shootingScript.editor.shotTitle")}
            value={shot.title}
            path={[...path, "title"]}
            onChange={onChange}
          />
          <TextField
            label={t("shootingScript.editor.narrativePurpose")}
            value={shot.narrative_purpose}
            path={[...path, "narrative_purpose"]}
            onChange={onChange}
          />
          <TextField
            label={t("shootingScript.editor.subject")}
            value={shot.subject}
            path={[...path, "subject"]}
            onChange={onChange}
          />
          <TextField
            label={t("shootingScript.editor.composition")}
            value={shot.composition}
            path={[...path, "composition"]}
            onChange={onChange}
          />
          <TextField
            label={t("shootingScript.editor.lensIntent")}
            value={shot.lens_intent}
            path={[...path, "lens_intent"]}
            onChange={onChange}
          />
          <div className="min-w-0">
            <span className="mb-1 block text-small text-book-text-muted">
              {t("shootingScript.editor.plannedDuration")}
            </span>
            <p className="text-xs text-book-text-main">
              {shootingShotDurationLabel(shot, locale, t)}
            </p>
          </div>
        </div>

        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
          <SelectField
            label={t("shootingScript.editor.shotSize")}
            value={shot.shot_size}
            options={SHOT_SIZE_OPTIONS.map(
              (value) => [value, shootingShotSizeLabel(value, t)] as const,
            )}
            path={[...path, "shot_size"]}
            onChange={onChange}
          />
          <SelectField
            label={t("shootingScript.editor.angle")}
            value={shot.camera_angle}
            options={CAMERA_ANGLE_OPTIONS.map(
              (value) => [value, shootingCameraAngleLabel(value, t)] as const,
            )}
            path={[...path, "camera_angle"]}
            onChange={onChange}
          />
          <SelectField
            label={t("shootingScript.editor.cameraHeight")}
            value={shot.camera_height}
            options={CAMERA_HEIGHT_OPTIONS.map(
              ([value, key]) => [value, t(key)] as const,
            )}
            path={[...path, "camera_height"]}
            onChange={onChange}
          />
          <SelectField
            label={t("shootingScript.editor.movement")}
            value={shot.camera_movement}
            options={CAMERA_MOVEMENT_OPTIONS.map(
              (value) =>
                [value, shootingCameraMovementLabel(value, t)] as const,
            )}
            path={[...path, "camera_movement"]}
            onChange={onChange}
          />
          <SelectField
            label={t("shootingScript.editor.coverage")}
            value={shot.coverage}
            options={COVERAGE_OPTIONS.map(
              (value) => [value, shootingCoverageLabel(value, t)] as const,
            )}
            path={[...path, "coverage"]}
            onChange={onChange}
          />
        </div>

        <AdvancedDetails
          title={t("shootingScript.editor.blockingSoundContinuity")}
        >
          <div className="grid gap-3 sm:grid-cols-3">
            <TextField
              label={t("shootingScript.editor.blockingStart")}
              value={shot.blocking.start}
              path={[...path, "blocking", "start"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.movementProcess")}
              value={shot.blocking.movement}
              path={[...path, "blocking", "movement"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.blockingEnd")}
              value={shot.blocking.end}
              path={[...path, "blocking", "end"]}
              onChange={onChange}
            />
          </div>
          <div className="mt-3 grid gap-3 sm:grid-cols-2">
            <StringListField
              label={t("shootingScript.editor.spatialAnchors")}
              value={shot.spatial_anchors}
              path={[...path, "spatial_anchors"]}
              onChange={onChange}
            />
            <StringListField
              label={t("shootingScript.editor.continuityRequirements")}
              value={shot.continuity_requirements}
              path={[...path, "continuity_requirements"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.cameraStartState")}
              value={shot.camera_start_state}
              path={[...path, "camera_start_state"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.cameraEndState")}
              value={shot.camera_end_state}
              path={[...path, "camera_end_state"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.frameStartState")}
              value={shot.start_state}
              path={[...path, "start_state"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.frameEndState")}
              value={shot.end_state}
              path={[...path, "end_state"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.transitionIn")}
              value={shot.transition_in}
              path={[...path, "transition_in"]}
              onChange={onChange}
            />
            <TextField
              label={t("shootingScript.editor.transitionOut")}
              value={shot.transition_out}
              path={[...path, "transition_out"]}
              onChange={onChange}
            />
            <StringListField
              label={t("shootingScript.editor.dialogue")}
              value={shot.sound.dialogue}
              path={[...path, "sound", "dialogue"]}
              onChange={onChange}
            />
            <StringListField
              label={t("shootingScript.editor.environmentSound")}
              value={shot.sound.environment}
              path={[...path, "sound", "environment"]}
              onChange={onChange}
            />
            <StringListField
              label={t("shootingScript.editor.soundEffect")}
              value={shot.sound.effects}
              path={[...path, "sound", "effects"]}
              onChange={onChange}
            />
            <StringListField
              label={t("shootingScript.editor.music")}
              value={shot.sound.music}
              path={[...path, "sound", "music"]}
              onChange={onChange}
            />
            <StringListField
              label={t("shootingScript.editor.silenceDesign")}
              value={shot.sound.silence}
              path={[...path, "sound", "silence"]}
              onChange={onChange}
            />
          </div>
        </AdvancedDetails>

        <section className="border-t border-book-border/40 pt-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h5 className="text-xs font-semibold text-book-text-main">
              {t("shootingScript.editor.performanceBeats")}
            </h5>
            <BookButton size="sm" variant="ghost" onClick={addBeat}>
              <Plus size={13} />
              {t("shootingScript.editor.addBeat")}
            </BookButton>
          </div>
          <div className="mt-1 divide-y divide-book-border/40 border-b border-book-border/40">
            {shot.performance_beats.map((beat, beatOffset) => (
              <BeatEditor
                key={beat.beat_id}
                beat={beat}
                path={[...path, "performance_beats", beatOffset]}
                onChange={onChange}
                onRemove={() => removeBeat(beatOffset)}
              />
            ))}
          </div>
        </section>

        <AdvancedDetails
          title={t("shootingScript.editor.dialogueCues")}
          action={
            <BookButton
              size="sm"
              variant="ghost"
              onClick={addDialogueCue}
              disabled={shot.performance_beats.length === 0}
            >
              <Plus size={13} />
              {t("shootingScript.editor.addDialogueCue")}
            </BookButton>
          }
        >
          {shot.dialogue_cues.length > 0 ? (
            <div className="divide-y divide-book-border/40">
              {shot.dialogue_cues.map((cue, offset) => {
                const cuePath = [...path, "dialogue_cues", offset];
                return (
                  <div
                    key={`${cue.beat_id}-${offset}`}
                    className="grid gap-3 py-3 sm:grid-cols-2"
                  >
                    <SelectField
                      label={t("shootingScript.editor.dialogueBeat")}
                      value={cue.beat_id}
                      options={shot.performance_beats.map(
                        (beat) =>
                          [
                            beat.beat_id,
                            t("shootingScript.editor.beat", {
                              index: beat.beat_index,
                            }),
                          ] as const,
                      )}
                      path={[...cuePath, "beat_id"]}
                      onChange={onChange}
                    />
                    <TextField
                      label={t("shootingScript.editor.speaker")}
                      value={cue.speaker}
                      path={[...cuePath, "speaker"]}
                      onChange={onChange}
                    />
                    <TextField
                      label={t("shootingScript.editor.lineOrIntent")}
                      value={cue.line_or_intent}
                      path={[...cuePath, "line_or_intent"]}
                      onChange={onChange}
                    />
                    <div className="flex items-end justify-end gap-2">
                      <TextField
                        label={t("shootingScript.editor.listenerReaction")}
                        value={cue.listener_reaction}
                        path={[...cuePath, "listener_reaction"]}
                        onChange={onChange}
                        nullable
                      />
                      <BookIconButton
                        label={t("shootingScript.editor.deleteDialogueCue")}
                        onClick={() => removeDialogueCue(offset)}
                        className="mb-0.5 text-error hover:bg-error-bg"
                      >
                        <Trash2 size={14} />
                      </BookIconButton>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <p className="py-3 text-small text-book-text-muted">
              {t("shootingScript.editor.noDialogueCues")}
            </p>
          )}
        </AdvancedDetails>

        <DiagnosticDetails
          nested
          rows={[
            [t("shootingScript.editor.shotId"), shot.shot_id],
            [
              t("shootingScript.editor.sceneSourceLocation"),
              shot.source_span_ids.join("、"),
            ],
          ]}
        />
      </div>
    </details>
  );
};
