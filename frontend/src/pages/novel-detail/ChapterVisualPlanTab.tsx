import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Box,
  CheckCircle2,
  ChevronRight,
  ChevronDown,
  ChevronUp,
  CircleDot,
  Flame,
  Film,
  Grid2X2,
  Layers3,
  Mountain,
  Pause,
  Play,
  Plus,
  PersonStanding,
  Redo2,
  Save,
  Shapes,
  Settings2,
  Trash2,
  Undo2,
  Volume2,
  X,
  type LucideIcon,
} from "lucide-react";

import { extractErrorMessage } from "../../api";
import type { ChapterSummary } from "../../api/chapter";
import {
  confirmChapterVisualPlan,
  discardChapterVisualPlanDraft,
  getChapterVisualPlan,
  saveChapterVisualPlanDraft,
  type ChapterVisualPlanSetView,
  type ChapterVisualPlanV2,
  type ChapterVisualPlanView,
  type VisualCameraEventKindV1,
  type VisualCameraEventV2,
  type VisualCameraStateV1,
  type VisualEntityEventKindV1,
  type VisualEntityEventV2,
  type VisualEntityKindV1,
  type VisualEntityStateV1,
  type VisualFacingV1,
  type VisualInterpolationV1,
  type VisualKeyframeV2,
  type VisualRelationKindV1,
  type VisualSceneV2,
  type VisualShotV2,
  type VisualSoundEventKindV1,
  type VisualSoundEventV2,
  type VisualTechniqueEventV2,
  type VisualTechniqueKindV1,
} from "../../api/chapterVisualPlan";
import type {
  ShootingCameraAngleV1,
  ShootingCameraHeightV1,
  ShootingCoverageV1,
  ShootingEditingModeV1,
  ShootingShotSizeV1,
  ShootingTransitionTypeV1,
} from "../../api/shootingScript";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { DomainAgentConsole } from "../../components/agent/DomainAgentConsole";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../components/ui/Dropdown";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { Modal } from "../../components/ui/Modal";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { useI18n, type MessageKey } from "../../i18n";
import { EmptyHint } from "./shared";
import {
  clonePlan,
  collectVisualPlanIssues,
  eventText,
  availableVisualKeyframeTime,
  formatPlanTime,
  isVisualKeyframeTimeAvailable,
  moveVisualScene,
  moveVisualShot,
  nearestKeyframe,
  parseVisualEventParameters,
  planDurationMs,
  removeVisualEntity,
  removeVisualKeyframe,
  removeVisualScene,
  removeVisualPlanTimedEvent,
  removeVisualShot,
  resizeVisualShot,
  translateVisualPlanIssue,
  updateVisualPlanTimedEventRange,
  visualSceneStartMs,
  visualShotAt,
  visualShots,
  type VisualPlanIssue,
  type VisualPlanTimedEventV2,
  type VisualShotRef,
} from "./visual-plan/editing";
import {
  clampFrameValue,
  defaultRenderBounds,
  entityControlFramePosition,
  entityControlPosition,
  entitySelectionTransform,
  ENTITY_RENDER_Z_INDEX_OFFSET,
  finiteOr,
  ENTITY_CONTROL_SIZE,
  ENTITY_CONTROL_LAYER_Z_INDEX,
  measuredEntityBounds,
  normalizeEntityBounds,
  normalizeEntityTransform,
  renderOpacity,
  renderRotation,
  renderZIndex,
  resizeEntitySelectionByScreenDelta,
  rotatedBoundsSize,
  type EntityFrameSize,
  type EntityResizeHandle,
  type EntityTransform,
} from "./visual-plan/geometry";

interface ChapterVisualPlanTabProps {
  projectId: number;
  canEdit: boolean;
  chapter: ChapterSummary | null;
}

type ModalName =
  | "frame"
  | "sceneLibrary"
  | "summary"
  | "scene"
  | "shot"
  | "keyframe"
  | "keyframeLibrary"
  | "entityLibrary"
  | "entity"
  | "entityState"
  | "event"
  | "eventDetails"
  | "source"
  | "director"
  | null;

type EventCategory = "entity" | "camera" | "sound" | "technique";
type TransitionOption = ShootingTransitionTypeV1 | "none";

interface PendingDraftSave {
  plan: ChapterVisualPlanV2;
  notify: boolean;
}

interface ChapterEditorRuntime {
  projectId: number;
  chapterId: number;
  record: ChapterVisualPlanView | null;
  pendingSave: PendingDraftSave | null;
  savePromise: Promise<ChapterVisualPlanView | null> | null;
  externalUpdatePending: boolean;
}

interface FrameDraft {
  width: number;
  height: number;
  background: string;
}

interface ShotDraft {
  shot: VisualShotV2;
  initialDurationMs: number;
  transitionKind: TransitionOption;
  transitionDurationMs: number;
  transitionPreservedAction: string;
  transitionPreservedSound: string;
  transitionNotes: string;
  continuityNotes: string;
}

interface KeyframeDraft {
  shotId: string;
  keyframe: VisualKeyframeV2;
  relativeTimeMs: number;
}

interface EntityLibraryDraft {
  id: string | null;
  label: string;
  kind: VisualEntityKindV1;
  description: string;
}

interface EntityStateDraft {
  shotId: string;
  keyframeId: string;
  entityId: string;
  visible: boolean;
  action: string;
  expression: string;
  pose: string;
  dialogue: string;
  notes: string;
  x: number;
  y: number;
  width: number;
  height: number;
  opacity: number;
  rotation: number;
  zIndex: number;
  facing: VisualFacingV1;
  gazeTarget: string;
}

interface EventDraft {
  shotId: string;
  category: EventCategory;
  id: string | null;
  kind: string;
  startMs: number;
  endMs: number;
  entityId: string;
  targetEntityId: string;
  speakerEntityId: string;
  text: string;
  parametersText: string;
}

type PendingCreate =
  | { kind: "scene"; id: string }
  | { kind: "shot"; id: string; sceneId: string }
  | { kind: "keyframe"; id: string; shotId: string };

const EDITING_MODE_OPTIONS: ReadonlyArray<readonly [ShootingEditingModeV1, MessageKey]> = [
  ["continuity", "visualPlan.editingContinuity"],
  ["intercut", "visualPlan.editingIntercut"],
  ["montage", "visualPlan.editingMontage"],
  ["flashback", "visualPlan.editingFlashback"],
  ["parallel_action", "visualPlan.editingParallelAction"],
  ["sound_bridge", "visualPlan.editingSoundBridge"],
];

const COVERAGE_OPTIONS: ReadonlyArray<readonly [ShootingCoverageV1, MessageKey]> = [
  ["must_show", "visualPlan.coverageMustShow"],
  ["should_show", "visualPlan.coverageShouldShow"],
  ["optional", "visualPlan.coverageOptional"],
];

const SHOT_SIZE_OPTIONS: ReadonlyArray<readonly [ShootingShotSizeV1, MessageKey]> = [
  ["extreme_wide", "visualPlan.shotSizeExtremeWide"],
  ["wide", "visualPlan.shotSizeWide"],
  ["medium_wide", "visualPlan.shotSizeMediumWide"],
  ["medium", "visualPlan.shotSizeMedium"],
  ["medium_close_up", "visualPlan.shotSizeMediumCloseUp"],
  ["close_up", "visualPlan.shotSizeCloseUp"],
  ["extreme_close_up", "visualPlan.shotSizeExtremeCloseUp"],
  ["insert", "visualPlan.shotSizeInsert"],
];

const CAMERA_ANGLE_OPTIONS: ReadonlyArray<readonly [ShootingCameraAngleV1, MessageKey]> = [
  ["eye_level", "visualPlan.angleEyeLevel"],
  ["high", "visualPlan.angleHigh"],
  ["low", "visualPlan.angleLow"],
  ["overhead", "visualPlan.angleOverhead"],
  ["dutch", "visualPlan.angleDutch"],
  ["subjective", "visualPlan.angleSubjective"],
];

const CAMERA_HEIGHT_OPTIONS: ReadonlyArray<readonly [ShootingCameraHeightV1, MessageKey]> = [
  ["ground", "visualPlan.heightGround"],
  ["low", "visualPlan.heightLow"],
  ["eye", "visualPlan.heightEye"],
  ["high", "visualPlan.heightHigh"],
  ["overhead", "visualPlan.heightOverhead"],
];

const INTERPOLATION_OPTIONS: ReadonlyArray<readonly [VisualInterpolationV1, MessageKey]> = [
  ["hold", "visualPlan.interpolationHold"],
  ["linear", "visualPlan.interpolationLinear"],
  ["ease_in", "visualPlan.interpolationEaseIn"],
  ["ease_out", "visualPlan.interpolationEaseOut"],
  ["ease_in_out", "visualPlan.interpolationEaseInOut"],
];

const FACING_OPTIONS: ReadonlyArray<readonly [VisualFacingV1, MessageKey]> = [
  ["left", "visualPlan.facingLeft"],
  ["right", "visualPlan.facingRight"],
  ["front", "visualPlan.facingFront"],
  ["back", "visualPlan.facingBack"],
  ["profile_left", "visualPlan.facingProfileLeft"],
  ["profile_right", "visualPlan.facingProfileRight"],
  ["unspecified", "visualPlan.facingUnspecified"],
];

const TRANSITION_OPTIONS: ReadonlyArray<readonly [TransitionOption, MessageKey]> = [
  ["none", "visualPlan.transitionNone"],
  ["continuous", "visualPlan.transitionContinuous"],
  ["hard_cut", "visualPlan.transitionHardCut"],
  ["match_cut", "visualPlan.transitionMatchCut"],
  ["whip_pan", "visualPlan.transitionWhipPan"],
  ["action_wipe", "visualPlan.transitionActionWipe"],
  ["sound_bridge", "visualPlan.transitionSoundBridge"],
  ["dissolve", "visualPlan.transitionDissolve"],
  ["fade", "visualPlan.transitionFade"],
  ["focus_transition", "visualPlan.transitionFocus"],
  ["morph", "visualPlan.transitionMorph"],
];

const ENTITY_KIND_OPTIONS: ReadonlyArray<readonly [VisualEntityKindV1, MessageKey]> = [
  ["character", "visualPlan.character"],
  ["object", "visualPlan.object"],
  ["environment", "visualPlan.environment"],
  ["text", "visualPlan.text"],
  ["effect", "visualPlan.effect"],
];

const RELATION_OPTIONS: ReadonlyArray<readonly [VisualRelationKindV1, MessageKey]> = [
  ["left_of", "visualPlan.relationLeftOf"],
  ["right_of", "visualPlan.relationRightOf"],
  ["above", "visualPlan.relationAbove"],
  ["below", "visualPlan.relationBelow"],
  ["in_front_of", "visualPlan.relationInFrontOf"],
  ["behind", "visualPlan.relationBehind"],
  ["inside", "visualPlan.relationInside"],
  ["overlapping", "visualPlan.relationOverlapping"],
  ["facing", "visualPlan.relationFacing"],
  ["looking_at", "visualPlan.relationLookingAt"],
  ["touching", "visualPlan.relationTouching"],
  ["holding", "visualPlan.relationHolding"],
  ["following", "visualPlan.relationFollowing"],
  ["custom", "visualPlan.relationCustom"],
];

const ENTITY_EVENT_OPTIONS: ReadonlyArray<readonly [VisualEntityEventKindV1, MessageKey]> = [
  ["enter", "visualPlan.entityEnter"],
  ["exit", "visualPlan.entityExit"],
  ["move", "visualPlan.entityMove"],
  ["action", "visualPlan.entityAction"],
  ["dialogue", "visualPlan.entityDialogue"],
  ["expression", "visualPlan.entityExpression"],
  ["gaze", "visualPlan.entityGaze"],
  ["pose", "visualPlan.entityPose"],
  ["interaction", "visualPlan.entityInteraction"],
  ["transform", "visualPlan.entityTransform"],
  ["custom", "visualPlan.entityCustom"],
];

const CAMERA_EVENT_OPTIONS: ReadonlyArray<readonly [VisualCameraEventKindV1, MessageKey]> = [
  ["cut", "visualPlan.cameraCut"],
  ["pan", "visualPlan.cameraPan"],
  ["tilt", "visualPlan.cameraTilt"],
  ["push_in", "visualPlan.cameraPushIn"],
  ["pull_out", "visualPlan.cameraPullOut"],
  ["track", "visualPlan.cameraTrack"],
  ["dolly", "visualPlan.cameraDolly"],
  ["crane", "visualPlan.cameraCrane"],
  ["handheld", "visualPlan.cameraHandheld"],
  ["whip_pan", "visualPlan.cameraWhipPan"],
  ["zoom", "visualPlan.cameraZoom"],
  ["focus", "visualPlan.cameraFocus"],
  ["shake", "visualPlan.cameraShake"],
  ["orbit", "visualPlan.cameraOrbit"],
  ["reframe", "visualPlan.cameraReframe"],
  ["custom", "visualPlan.cameraCustom"],
];

const SOUND_EVENT_OPTIONS: ReadonlyArray<readonly [VisualSoundEventKindV1, MessageKey]> = [
  ["dialogue", "visualPlan.soundDialogue"],
  ["voice_over", "visualPlan.soundVoiceOver"],
  ["narration", "visualPlan.soundNarration"],
  ["ambient", "visualPlan.soundAmbient"],
  ["effect", "visualPlan.soundEffect"],
  ["music", "visualPlan.soundMusic"],
  ["silence", "visualPlan.soundSilence"],
];

const TECHNIQUE_EVENT_OPTIONS: ReadonlyArray<readonly [VisualTechniqueKindV1, MessageKey]> = [
  ["montage", "visualPlan.techniqueMontage"],
  ["split_screen", "visualPlan.techniqueSplitScreen"],
  ["slow_motion", "visualPlan.techniqueSlowMotion"],
  ["fast_motion", "visualPlan.techniqueFastMotion"],
  ["freeze_frame", "visualPlan.techniqueFreezeFrame"],
  ["flashback", "visualPlan.techniqueFlashback"],
  ["dream", "visualPlan.techniqueDream"],
  ["subjective", "visualPlan.techniqueSubjective"],
  ["symbolic", "visualPlan.techniqueSymbolic"],
  ["silhouette", "visualPlan.techniqueSilhouette"],
  ["reflection", "visualPlan.techniqueReflection"],
  ["overlay", "visualPlan.techniqueOverlay"],
  ["text_on_screen", "visualPlan.techniqueTextOnScreen"],
  ["lighting", "visualPlan.techniqueLighting"],
  ["color_grade", "visualPlan.techniqueColorGrade"],
  ["match_action", "visualPlan.techniqueMatchAction"],
  ["offscreen", "visualPlan.techniqueOffscreen"],
  ["custom", "visualPlan.techniqueCustom"],
];

const makeId = (prefix: string) =>
  `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;

const defaultCamera = (): VisualCameraStateV1 => ({
  center_x: 0.5,
  center_y: 0.5,
  focus_entity_id: null,
  framing_note: null,
  rotation_degrees: 0,
  zoom: 1,
});

const defaultEntityState = (entityId: string, index: number): VisualEntityStateV1 => ({
  entity_id: entityId,
  visible: true,
  bounds: {
    x: Math.min(0.74, 0.08 + (index % 4) * 0.22),
    y: Math.min(0.68, 0.16 + Math.floor(index / 4) * 0.24),
    width: 0.16,
    height: 0.2,
  },
  rotation_degrees: 0,
  opacity: 1,
  z_index: index,
  facing: "unspecified",
  expression: null,
  pose: null,
  action: null,
  gaze_target_entity_id: null,
  dialogue: null,
  notes: null,
});

const createShot = (id: string, entities: ChapterVisualPlanV2["entities"]): VisualShotV2 => {
  const durationMs = 3000;
  const states = entities.slice(0, 6).map((entity, index) => defaultEntityState(entity.id, index));
  return {
    id,
    title: "新镜头",
    duration_ms: durationMs,
    narrative_purpose: "推进当前场景",
    coverage: "must_show",
    shot_size: "medium",
    camera_angle: "eye_level",
    camera_height: "eye",
    lens_intent: null,
    composition: null,
    keyframes: [
      {
        id: makeId("keyframe"),
        at_ms: 0,
        label: "起始画面",
        interpolation_to_next: "linear",
        camera: defaultCamera(),
        entity_states: structuredClone(states),
        relations: [],
      },
      {
        id: makeId("keyframe"),
        at_ms: durationMs,
        label: "结束画面",
        interpolation_to_next: "hold",
        camera: defaultCamera(),
        entity_states: structuredClone(states),
        relations: [],
      },
    ],
    entity_events: [],
    camera_events: [],
    sound_events: [],
    technique_events: [],
    continuity_notes: [],
    transition_to_next: null,
  };
};

const createScene = (id: string, entities: ChapterVisualPlanV2["entities"]): VisualSceneV2 => {
  const participantEntities = entities.slice(0, 6);
  const shot = createShot(makeId("shot"), participantEntities);
  return {
    id,
    title: "新场景",
    narrative_purpose: "承载一段连续行动",
    editing_mode: "continuity",
    intercut_group_id: null,
    location: null,
    time_of_day: null,
    participant_entity_ids: participantEntities.map((entity) => entity.id),
    focal_entity_id: participantEntities[0]?.id ?? null,
    shots: [shot],
  };
};

const toText = (value: string | null | undefined) => value ?? "";

const parseNumericInput = (value: string): number => {
  const normalized = value.trim();
  return normalized ? Number(normalized) : Number.NaN;
};

const isVisualPlanRevisionConflict = (error: unknown) =>
  extractErrorMessage(error, "").includes("visual_plan_revision_conflict:");

const numericInputValue = (value: number, divisor = 1): number | "" =>
  Number.isFinite(value) ? value / divisor : "";

const shotDraftFrom = (shot: VisualShotV2): ShotDraft => ({
  shot: structuredClone(shot),
  initialDurationMs: shot.duration_ms,
  transitionKind: shot.transition_to_next?.kind ?? "none",
  transitionDurationMs: shot.transition_to_next?.duration_ms ?? 0,
  transitionPreservedAction: toText(shot.transition_to_next?.preserved_action),
  transitionPreservedSound: toText(shot.transition_to_next?.preserved_sound),
  transitionNotes: toText(shot.transition_to_next?.notes),
  continuityNotes: shot.continuity_notes.join("\n"),
});

const entityLibraryDraftFrom = (
  entity: ChapterVisualPlanV2["entities"][number],
): EntityLibraryDraft => ({
  id: entity.id,
  label: entity.label,
  kind: entity.kind,
  description: entity.description,
});

const entityStateDraftFrom = (
  state: VisualEntityStateV1,
  shotId: string,
  keyframeId: string,
): EntityStateDraft => {
  const bounds = normalizeEntityBounds(state.bounds);
  return {
    shotId,
    keyframeId,
    entityId: state.entity_id,
    visible: state.visible,
    action: toText(state.action),
    expression: toText(state.expression),
    pose: toText(state.pose),
    dialogue: toText(state.dialogue),
    notes: toText(state.notes),
    x: Math.round(bounds.x * 100),
    y: Math.round(bounds.y * 100),
    width: Math.max(1, Math.round(bounds.width * 100)),
    height: Math.max(1, Math.round(bounds.height * 100)),
    opacity: Math.round(renderOpacity(state.opacity) * 100),
    rotation: Math.round(renderRotation(state.rotation_degrees) * 100) / 100,
    zIndex: renderZIndex(state.z_index),
    facing: state.facing,
    gazeTarget: toText(state.gaze_target_entity_id),
  };
};

const eventDraftFrom = (
  event: VisualPlanTimedEventV2,
  category: EventCategory,
  shotId: string,
): EventDraft => ({
  shotId,
  category,
  id: event.id,
  kind: event.kind,
  startMs: event.start_ms,
  endMs: event.end_ms,
  entityId: "entity_id" in event ? event.entity_id : "",
  targetEntityId: "target_entity_id" in event ? toText(event.target_entity_id) : "",
  speakerEntityId: "speaker_entity_id" in event ? toText(event.speaker_entity_id) : "",
  text: eventText(event),
  parametersText: JSON.stringify(event.parameters, null, 2),
});

const optionItems = <T extends string>(
  options: ReadonlyArray<readonly [T, MessageKey]>,
  t: (key: MessageKey) => string,
) => options.map(([value, label]) => ({ value, label: t(label) }));

const findPlanShot = (
  value: ChapterVisualPlanV2 | null | undefined,
  shotId: string | null | undefined,
): VisualShotV2 | null => value && shotId
  ? visualShots(value).find(({ shot }) => shot.id === shotId)?.shot ?? null
  : null;

const findPlanKeyframe = (
  value: ChapterVisualPlanV2 | null | undefined,
  shotId: string | null | undefined,
  keyframeId: string | null | undefined,
): VisualKeyframeV2 | null => findPlanShot(value, shotId)?.keyframes.find(({ id }) => id === keyframeId) ?? null;

const Field: React.FC<{ label: string; children: React.ReactNode; className?: string }> = ({ label, children, className = "" }) => (
  <label className={`block min-w-0 ${className}`}>
    <span className="mb-1 block text-caption font-semibold text-book-text-muted">{label}</span>
    {children}
  </label>
);

const TextInput: React.FC<React.InputHTMLAttributes<HTMLInputElement>> = ({ className = "", ...props }) => (
  <input {...props} className={`pyqt-input w-full ${className}`} />
);

const TextArea: React.FC<React.TextareaHTMLAttributes<HTMLTextAreaElement>> = ({ className = "", ...props }) => (
  <textarea {...props} className={`pyqt-input min-h-24 w-full resize-y ${className}`} />
);

const SelectField: React.FC<{
  label: string;
  value: string;
  options: ReadonlyArray<{ value: string; label: string }>;
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
}> = ({ label, value, options, onChange, disabled = false, className = "" }) => (
  <Field label={label} className={className}>
    <ThemedSelect
      value={value}
      disabled={disabled}
      aria-label={label}
      className="pyqt-input w-full"
      onChange={(event) => onChange(event.currentTarget.value)}
    >
      {options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
    </ThemedSelect>
  </Field>
);

export const ChapterVisualPlanTab: React.FC<ChapterVisualPlanTabProps> = ({
  projectId,
  canEdit,
  chapter,
}) => {
  const { t } = useI18n();
  const { addToast } = useToast();
  const chapterId = chapter?.id ?? null;
  const [planSet, setPlanSet] = useState<ChapterVisualPlanSetView | null>(null);
  const [plan, setPlan] = useState<ChapterVisualPlanV2 | null>(null);
  const [baseline, setBaseline] = useState("");
  const [editing, setEditing] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [locking, setLocking] = useState(false);
  const [selectedSceneId, setSelectedSceneId] = useState<string | null>(null);
  const [selectedShotId, setSelectedShotId] = useState<string | null>(null);
  const [selectedKeyframeId, setSelectedKeyframeId] = useState<string | null>(null);
  const [selectedEntityId, setSelectedEntityId] = useState<string | null>(null);
  const [playheadMs, setPlayheadMs] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [undoStack, setUndoStack] = useState<ChapterVisualPlanV2[]>([]);
  const [redoStack, setRedoStack] = useState<ChapterVisualPlanV2[]>([]);
  const [modal, setModal] = useState<ModalName>(null);
  const [frameDraft, setFrameDraft] = useState<FrameDraft | null>(null);
  const [sceneDraft, setSceneDraft] = useState<VisualSceneV2 | null>(null);
  const [shotDraft, setShotDraft] = useState<ShotDraft | null>(null);
  const [keyframeDraft, setKeyframeDraft] = useState<KeyframeDraft | null>(null);
  const [pendingCreate, setPendingCreate] = useState<PendingCreate | null>(null);
  const [entityLibraryDraft, setEntityLibraryDraft] = useState<EntityLibraryDraft | null>(null);
  const [entityStateDraft, setEntityStateDraft] = useState<EntityStateDraft | null>(null);
  const [eventDraft, setEventDraft] = useState<EventDraft | null>(null);
  const [relationKind, setRelationKind] = useState<VisualRelationKindV1>("looking_at");
  const [relationTarget, setRelationTarget] = useState("");
  const [relationLabel, setRelationLabel] = useState("");
  const [relationEditingId, setRelationEditingId] = useState<string | null>(null);
  const planRef = useRef<ChapterVisualPlanV2 | null>(null);
  const runtimeRef = useRef<ChapterEditorRuntime | null>(null);
  const lockingRef = useRef(false);
  const editableRef = useRef(false);

  const dismissModal = useCallback(() => {
    setModal(null);
    setPendingCreate(null);
    setFrameDraft(null);
    setSceneDraft(null);
    setShotDraft(null);
    setKeyframeDraft(null);
    setEntityLibraryDraft(null);
    setEntityStateDraft(null);
    setEventDraft(null);
    setRelationEditingId(null);
    setRelationTarget("");
    setRelationLabel("");
  }, []);

  const openModal = (name: Exclude<ModalName, null>) => {
    setPlaying(false);
    setModal(name);
  };

  const record = planSet?.draft ?? null;
  const shotRefs = useMemo(() => plan ? visualShots(plan) : [], [plan]);
  const totalDuration = plan ? planDurationMs(plan) : 0;
  const selectedRef = useMemo(() => {
    if (!plan) return null;
    const selectedShot = shotRefs.find((ref) => ref.shot.id === selectedShotId);
    if (selectedShot) return selectedShot;
    const selectedScene = shotRefs.find((ref) => ref.scene.id === selectedSceneId);
    if (selectedScene) return selectedScene;
    return selectedSceneId == null ? shotRefs[0] ?? null : null;
  }, [plan, selectedSceneId, selectedShotId, shotRefs]);
  const selectedScene = selectedRef?.scene ?? plan?.scenes.find((scene) => scene.id === selectedSceneId) ?? null;
  const selectedShot = selectedRef?.shot ?? null;
  const selectedKeyframe = selectedShot
    ? selectedShot.keyframes.find((keyframe) => keyframe.id === selectedKeyframeId) ??
      nearestKeyframe(selectedShot, Math.max(0, playheadMs - (selectedRef?.start_ms ?? 0)))
    : null;
  const selectedEntityState = selectedKeyframe?.entity_states.find((state) => state.entity_id === selectedEntityId) ?? null;
  const entityById = useMemo(
    () => new Map((plan?.entities ?? []).map((entity) => [entity.id, entity] as const)),
    [plan],
  );
  const strictIssues = useMemo(
    () => plan ? collectVisualPlanIssues(plan, true) : [],
    [plan],
  );
  const serializedPlan = plan ? JSON.stringify(plan) : "";
  const planDirty = Boolean(plan && serializedPlan !== baseline);
  const editable = canEdit && editing && !locking;
  editableRef.current = editable;
  const editorDraftHasChanges = (() => {
    if (modal === "frame" && frameDraft && plan) {
      return Math.max(1, Math.round(frameDraft.width)) !== plan.frame.width
        || Math.max(1, Math.round(frameDraft.height)) !== plan.frame.height
        || (frameDraft.background.trim() || null) !== plan.frame.background;
    }
    if (modal === "scene" && sceneDraft) {
      if (pendingCreate?.kind === "scene" && pendingCreate.id === sceneDraft.id) return true;
      const current = plan?.scenes.find((scene) => scene.id === sceneDraft.id);
      return !current || JSON.stringify(current) !== JSON.stringify(sceneDraft);
    }
    if (modal === "shot" && shotDraft) {
      if (pendingCreate?.kind === "shot" && pendingCreate.id === shotDraft.shot.id) return true;
      const current = plan
        ? visualShots(plan).find(({ shot }) => shot.id === shotDraft.shot.id)?.shot
        : null;
      return !current || JSON.stringify(shotDraft) !== JSON.stringify(shotDraftFrom(current));
    }
    if (modal === "keyframe" && keyframeDraft) {
      if (pendingCreate?.kind === "keyframe" && pendingCreate.id === keyframeDraft.keyframe.id) return true;
      const current = findPlanKeyframe(plan, keyframeDraft.shotId, keyframeDraft.keyframe.id);
      if (!current) return true;
      return JSON.stringify({ ...keyframeDraft.keyframe, at_ms: Math.round(keyframeDraft.relativeTimeMs) }) !== JSON.stringify(current);
    }
    if (modal === "entity" && entityLibraryDraft) {
      if (entityLibraryDraft.id == null) {
        return Boolean(entityLibraryDraft.label.trim() || entityLibraryDraft.description.trim() || entityLibraryDraft.kind !== "character");
      }
      const current = plan?.entities.find((entity) => entity.id === entityLibraryDraft.id);
      return !current
        || JSON.stringify(entityLibraryDraft) !== JSON.stringify(entityLibraryDraftFrom(current));
    }
    if (modal === "entityState" && entityStateDraft) {
      const current = findPlanKeyframe(plan, entityStateDraft.shotId, entityStateDraft.keyframeId)
        ?.entity_states.find((state) => state.entity_id === entityStateDraft.entityId);
      return !current
        || JSON.stringify(entityStateDraft) !== JSON.stringify(entityStateDraftFrom(current, entityStateDraft.shotId, entityStateDraft.keyframeId));
    }
    if (modal === "event" && eventDraft) {
      if (eventDraft.id == null) return Boolean(eventDraft.text.trim() || eventDraft.parametersText.trim() !== "{}");
      const draftShot = findPlanShot(plan, eventDraft.shotId);
      const events = eventDraft.category === "entity"
        ? draftShot?.entity_events
        : eventDraft.category === "camera"
          ? draftShot?.camera_events
          : eventDraft.category === "sound"
            ? draftShot?.sound_events
            : draftShot?.technique_events;
      const current = events?.find((event) => event.id === eventDraft.id);
      return !current || JSON.stringify(eventDraft) !== JSON.stringify(eventDraftFrom(current, eventDraft.category, eventDraft.shotId));
    }
    return false;
  })();

  const resetEditor = useCallback((value: ChapterVisualPlanSetView, runtime: ChapterEditorRuntime) => {
    const source = value.draft ?? value.confirmed;
    const next = clonePlan(source?.plan ?? value.starter_plan);
    runtime.record = value.draft;
    runtime.pendingSave = null;
    runtime.savePromise = null;
    runtime.externalUpdatePending = false;
    lockingRef.current = false;
    setLocking(false);
    setPlanSet(value);
    setPlan(next);
    planRef.current = next;
    setBaseline(JSON.stringify(next));
    setEditing(Boolean(value.draft) || !value.confirmed);
    setUndoStack([]);
    setRedoStack([]);
    const first = visualShots(next)[0];
    setSelectedSceneId(first?.scene.id ?? next.scenes[0]?.id ?? null);
    setSelectedShotId(first?.shot.id ?? null);
    setSelectedKeyframeId(first?.shot.keyframes[0]?.id ?? null);
    setSelectedEntityId(null);
    setPlayheadMs(first?.start_ms ?? 0);
    setPlaying(false);
  }, []);

  const persistRuntimeDraft = useCallback(async (
    runtime: ChapterEditorRuntime,
    currentPlan: ChapterVisualPlanV2,
    notify = false,
  ) => {
    if (!canEdit || runtime.externalUpdatePending) return null;
    const pending = runtime.pendingSave;
    runtime.pendingSave = {
      plan: clonePlan(currentPlan),
      notify: notify || pending?.notify === true,
    };
    if (runtime.savePromise) return runtime.savePromise;
    if (runtimeRef.current === runtime) setSaving(true);
    runtime.savePromise = (async () => {
      let lastSaved: ChapterVisualPlanView | null = null;
      while (runtime.pendingSave) {
        const request = runtime.pendingSave;
        runtime.pendingSave = null;
        try {
          const saved = await saveChapterVisualPlanDraft(runtime.projectId, runtime.chapterId, runtime.record, request.plan);
          runtime.record = saved;
          lastSaved = saved;
          if (runtimeRef.current === runtime) {
            setPlanSet((current) => current ? { ...current, draft: saved } : current);
            setBaseline(JSON.stringify(request.plan));
            if (request.notify) addToast(t("visualPlan.draftSaved"), "success");
          }
        } catch (error) {
          runtime.pendingSave = null;
          if (runtimeRef.current === runtime && !(runtime.externalUpdatePending && isVisualPlanRevisionConflict(error))) {
            addToast(extractErrorMessage(error, t("visualPlan.draftSaveFailed")), "error");
          }
          return null;
        }
      }
      return lastSaved;
    })().finally(() => {
      runtime.savePromise = null;
      if (runtimeRef.current === runtime && !lockingRef.current) setSaving(false);
    });
    return runtime.savePromise;
  }, [addToast, canEdit, t]);

  const loadPlan = useCallback(async ({ persistPrevious = true }: { persistPrevious?: boolean } = {}) => {
    if (chapterId == null) return;
    const previousRuntime = runtimeRef.current;
    const previousPlan = planRef.current;
    if (persistPrevious && previousRuntime && previousPlan && editableRef.current) {
      await persistRuntimeDraft(previousRuntime, previousPlan, false);
      if (runtimeRef.current !== previousRuntime) return;
    }
    const runtime: ChapterEditorRuntime = {
      projectId,
      chapterId,
      record: null,
      pendingSave: null,
      savePromise: null,
      externalUpdatePending: false,
    };
    runtimeRef.current = runtime;
    lockingRef.current = false;
    setLoading(true);
    setSaving(false);
    setLocking(false);
    setLoadError(null);
    setPlanSet(null);
    setPlan(null);
    planRef.current = null;
    try {
      const value = await getChapterVisualPlan(projectId, chapterId);
      if (runtimeRef.current !== runtime) return;
      resetEditor(value, runtime);
    } catch (error) {
      if (runtimeRef.current !== runtime) return;
      const message = extractErrorMessage(error, t("visualPlan.loadFailed"));
      setLoadError(message);
      addToast(message, "error");
    } finally {
      if (runtimeRef.current === runtime) setLoading(false);
    }
  }, [chapterId, persistRuntimeDraft, projectId, resetEditor, t]);

  const handleDirectorResourceChanged = useCallback(async () => {
    const runtime = runtimeRef.current;
    if (runtime) {
      runtime.externalUpdatePending = true;
      runtime.pendingSave = null;
      if (runtime.savePromise) await runtime.savePromise;
      if (runtimeRef.current !== runtime) return;
    }
    const currentPlan = planRef.current;
    const hasLocalChanges = runtime?.record
      ? Boolean(currentPlan && JSON.stringify(currentPlan) !== JSON.stringify(runtime.record.plan))
      : planDirty;
    if (hasLocalChanges) {
      addToast(t("visualPlan.externalUpdateWhileEditing"), "info");
      return;
    }
    await loadPlan({ persistPrevious: false });
  }, [addToast, loadPlan, planDirty, t]);

  useEffect(() => {
    if (chapterId == null) {
      dismissModal();
      runtimeRef.current = null;
      setPlanSet(null);
      setPlan(null);
      planRef.current = null;
      setLoadError(null);
      setLoading(false);
      setSaving(false);
      setLocking(false);
      return;
    }
    dismissModal();
    void loadPlan();
  }, [chapterId, dismissModal, loadPlan]);

  useEffect(() => {
    planRef.current = plan;
  }, [plan]);

  useEffect(() => {
    if (!plan) return;
    const ref = selectedRef;
    if (!ref) {
      if (!selectedSceneId || !plan.scenes.some((scene) => scene.id === selectedSceneId)) setSelectedSceneId(null);
      setSelectedShotId(null);
      setSelectedKeyframeId(null);
      setSelectedEntityId(null);
      return;
    }
    if (selectedSceneId !== ref.scene.id) setSelectedSceneId(ref.scene.id);
    if (selectedShotId !== ref.shot.id) setSelectedShotId(ref.shot.id);
    const keyframe = ref.shot.keyframes.find((item) => item.id === selectedKeyframeId)
      ?? nearestKeyframe(ref.shot, Math.max(0, playheadMs - ref.start_ms));
    if (keyframe?.id !== selectedKeyframeId) setSelectedKeyframeId(keyframe?.id ?? null);
    if (selectedEntityId && !keyframe?.entity_states.some((state) => state.entity_id === selectedEntityId)) setSelectedEntityId(null);
  }, [plan, playheadMs, selectedEntityId, selectedKeyframeId, selectedRef, selectedSceneId, selectedShotId]);

  useEffect(() => {
    if (totalDuration <= 0) {
      setPlayheadMs(0);
      setPlaying(false);
      return;
    }
    setPlayheadMs((current) => Math.max(0, Math.min(current, totalDuration)));
  }, [totalDuration]);

  useEffect(() => {
    if (!playing || !plan || totalDuration <= 0) return undefined;
    const timer = window.setInterval(() => {
      setPlayheadMs((current) => {
        const next = current + 50;
        if (next >= totalDuration) {
          setPlaying(false);
          return totalDuration;
        }
        return next;
      });
    }, 50);
    return () => window.clearInterval(timer);
  }, [plan, playing, totalDuration]);

  useEffect(() => {
    if (!plan) return;
    const selectedSceneState = selectedSceneId == null
      ? null
      : plan.scenes.find((scene) => scene.id === selectedSceneId);
    const selectedShotExists = selectedShotId != null && shotRefs.some(({ shot }) => shot.id === selectedShotId);
    if (selectedSceneState?.shots.length === 0 && !selectedShotExists) return;
    const active = visualShotAt(plan, playheadMs, shotRefs);
    if (!active) return;
    if (active.shot.id !== selectedShotId) setSelectedShotId(active.shot.id);
    const nextKeyframe = nearestKeyframe(active.shot, playheadMs - active.start_ms);
    if (nextKeyframe && nextKeyframe.id !== selectedKeyframeId) setSelectedKeyframeId(nextKeyframe.id);
  }, [plan, playheadMs, selectedKeyframeId, selectedShotId, shotRefs]);

  const acquireLock = useCallback(() => {
    if (lockingRef.current || !runtimeRef.current) return false;
    lockingRef.current = true;
    setLocking(true);
    return true;
  }, []);

  const releaseLock = useCallback(() => {
    lockingRef.current = false;
    setLocking(false);
  }, []);

  const commit = useCallback((edit: ((next: ChapterVisualPlanV2) => void) | ChapterVisualPlanV2): ChapterVisualPlanV2 | null => {
    const current = planRef.current;
    if (!current || !editable) return null;
    const next = typeof edit === "function" ? clonePlan(current) : edit;
    if (typeof edit === "function") edit(next);
    if (JSON.stringify(next) === JSON.stringify(current)) return null;
    setUndoStack((stack) => [...stack.slice(-39), current]);
    setRedoStack([]);
    planRef.current = next;
    setPlan(next);
    return next;
  }, [editable]);

  const requestModalClose = async () => {
    if (!editorDraftHasChanges) {
      dismissModal();
      return;
    }
    const name = modal === "frame"
      ? t("visualPlan.frameSettings")
      : modal === "scene"
        ? t("visualPlan.scene")
        : modal === "shot"
          ? t("visualPlan.shot")
          : modal === "keyframe"
            ? t("visualPlan.keyframes")
            : modal === "entity"
              ? t("visualPlan.entity")
              : modal === "entityState"
                ? t("visualPlan.entityState")
                : t("visualPlan.eventDetails");
    const accepted = await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name, action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    });
    if (accepted) dismissModal();
  };

  const persistDraft = useCallback(async (notify = false) => {
    const runtime = runtimeRef.current;
    const currentPlan = planRef.current;
    if (!runtime || chapterId == null || runtime.projectId !== projectId || runtime.chapterId !== chapterId || !currentPlan || !editing) return null;
    return persistRuntimeDraft(runtime, currentPlan, notify);
  }, [chapterId, editing, persistRuntimeDraft, projectId]);

  useEffect(() => () => {
    const runtime = runtimeRef.current;
    const currentPlan = planRef.current;
    if (editableRef.current && runtime && currentPlan) void persistRuntimeDraft(runtime, currentPlan, false);
  }, [persistRuntimeDraft]);

  useEffect(() => {
    if (!planDirty || !editable) return undefined;
    const timer = window.setTimeout(() => {
      if (!lockingRef.current) void persistDraft(false);
    }, 800);
    return () => window.clearTimeout(timer);
  }, [editable, persistDraft, plan, planDirty]);

  const ensureLatestDraft = useCallback(async (runtime: ChapterEditorRuntime) => {
    if (runtime.savePromise) await runtime.savePromise;
    if (runtimeRef.current !== runtime || !planRef.current) return null;
    if (runtime.record && JSON.stringify(planRef.current) === JSON.stringify(runtime.record.plan)) return runtime.record;
    return persistDraft(false);
  }, [persistDraft]);

  const undo = useCallback(() => {
    const current = planRef.current;
    const previous = undoStack.at(-1);
    if (!current || !previous || !editable || lockingRef.current) return;
    setUndoStack((stack) => stack.slice(0, -1));
    setRedoStack((stack) => [...stack, current]);
    const next = clonePlan(previous);
    planRef.current = next;
    setPlan(next);
  }, [editable, undoStack]);

  const redo = useCallback(() => {
    const current = planRef.current;
    const nextValue = redoStack.at(-1);
    if (!current || !nextValue || !editable || lockingRef.current) return;
    setRedoStack((stack) => stack.slice(0, -1));
    setUndoStack((stack) => [...stack, current]);
    const next = clonePlan(nextValue);
    planRef.current = next;
    setPlan(next);
  }, [editable, redoStack]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target;
      if (target instanceof Element && target.matches("input, textarea, select, [contenteditable=true]")) return;
      if (event.key === "Escape") {
        if (!modal && selectedEntityId) setSelectedEntityId(null);
        return;
      }
      if (!editable) return;
      if (!(event.ctrlKey || event.metaKey)) return;
      if (event.key.toLowerCase() === "z") {
        event.preventDefault();
        event.shiftKey ? redo() : undo();
      } else if (event.key.toLowerCase() === "y") {
        event.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [editable, modal, redo, selectedEntityId, undo]);

  const selectShot = (ref: VisualShotRef) => {
    setPlaying(false);
    setSelectedSceneId(ref.scene.id);
    setSelectedShotId(ref.shot.id);
    setSelectedKeyframeId(ref.shot.keyframes[0]?.id ?? null);
    setSelectedEntityId(null);
    setPlayheadMs(ref.start_ms);
  };

  const selectKeyframe = (keyframe: VisualKeyframeV2, ref: VisualShotRef) => {
    setPlaying(false);
    setSelectedSceneId(ref.scene.id);
    setSelectedShotId(ref.shot.id);
    setSelectedKeyframeId(keyframe.id);
    setSelectedEntityId(null);
    setPlayheadMs(ref.start_ms + keyframe.at_ms);
  };

  const selectScene = (sceneId: string) => {
    const ref = shotRefs.find((candidate) => candidate.scene.id === sceneId);
    if (ref) selectShot(ref);
    else {
      const scene = plan?.scenes.find((candidate) => candidate.id === sceneId);
      setSelectedSceneId(sceneId);
      setPlaying(false);
      setSelectedShotId(null);
      setSelectedKeyframeId(null);
      setSelectedEntityId(null);
      setPlayheadMs(plan && scene ? visualSceneStartMs(plan, scene.id) ?? 0 : 0);
    }
  };

  const seekPlayhead = (value: number) => {
    if (!plan || !Number.isFinite(value)) return;
    const nextValue = Math.max(0, Math.min(totalDuration, value));
    const ref = visualShotAt(plan, nextValue, shotRefs);
    setPlaying(false);
    setPlayheadMs(nextValue);
    if (!ref) {
      setSelectedShotId(null);
      setSelectedKeyframeId(null);
      setSelectedEntityId(null);
      return;
    }
    setSelectedSceneId(ref.scene.id);
    setSelectedShotId(ref.shot.id);
    const keyframe = nearestKeyframe(ref.shot, nextValue - ref.start_ms);
    setSelectedKeyframeId(keyframe?.id ?? null);
    setSelectedEntityId(null);
  };

  const openFrameEditor = () => {
    if (!plan) return;
    setFrameDraft({ width: plan.frame.width, height: plan.frame.height, background: toText(plan.frame.background) });
    openModal("frame");
  };

  const openSceneEditor = (scene = selectedScene) => {
    if (!scene) return;
    selectScene(scene.id);
    setSceneDraft(structuredClone(scene));
    openModal("scene");
  };

  const openShotEditor = (ref = selectedRef) => {
    if (!ref) return;
    const shot = ref.shot;
    selectShot(ref);
    setShotDraft(shotDraftFrom(shot));
    openModal("shot");
  };

  const openKeyframeEditor = (keyframe = selectedKeyframe, ref = selectedRef) => {
    if (!keyframe || !ref) return;
    selectKeyframe(keyframe, ref);
    setKeyframeDraft({ shotId: ref.shot.id, keyframe: structuredClone(keyframe), relativeTimeMs: keyframe.at_ms });
    openModal("keyframe");
  };

  const openEntityLibrary = (entity?: ChapterVisualPlanV2["entities"][number]) => {
    setEntityLibraryDraft(entity ? entityLibraryDraftFrom(entity) : { id: null, label: "", kind: "character", description: "" });
    openModal(entity ? "entity" : "entityLibrary");
  };

  const openNewEntity = () => {
    setEntityLibraryDraft({ id: null, label: "", kind: "character", description: "" });
    openModal("entity");
  };

  const openEntityState = (state = selectedEntityState) => {
    if (!state || !selectedRef || !selectedKeyframe) return;
    setSelectedEntityId(state.entity_id);
    setRelationTarget("");
    setRelationLabel("");
    setRelationEditingId(null);
    setEntityStateDraft(entityStateDraftFrom(state, selectedRef.shot.id, selectedKeyframe.id));
    openModal("entityState");
  };

  const handleEntityClick = (state: VisualEntityStateV1) => {
    if (selectedEntityId === state.entity_id) {
      openEntityState(state);
      return;
    }
    setSelectedEntityId(state.entity_id);
  };

  const defaultEventKind = (category: EventCategory) => category === "entity" ? "action" : category === "camera" ? "reframe" : category === "sound" ? "effect" : "symbolic";

  const eventOptions = (category: EventCategory): ReadonlyArray<readonly [string, MessageKey]> => {
    if (category === "entity") return ENTITY_EVENT_OPTIONS;
    if (category === "camera") return CAMERA_EVENT_OPTIONS;
    if (category === "sound") return SOUND_EVENT_OPTIONS;
    return TECHNIQUE_EVENT_OPTIONS;
  };

  const openNewEvent = (category: EventCategory) => {
    if (!selectedShot || !selectedRef) return;
    const relative = Math.max(0, Math.min(selectedShot.duration_ms - 1, playheadMs - (selectedRef?.start_ms ?? 0)));
    const entityStates = nearestKeyframe(selectedShot, relative)?.entity_states.filter((state) => state.visible) ?? [];
    const selectedEventEntityId = selectedEntityId && entityStates.some((state) => state.entity_id === selectedEntityId)
      ? selectedEntityId
      : entityStates[0]?.entity_id ?? "";
    setEventDraft({
      shotId: selectedRef.shot.id,
      category,
      id: null,
      kind: defaultEventKind(category),
      startMs: relative,
      endMs: Math.min(selectedShot.duration_ms, relative + 500),
      entityId: selectedEventEntityId,
      targetEntityId: "",
      speakerEntityId: selectedEventEntityId,
      text: "",
      parametersText: "{}",
    });
    openModal("event");
  };

  const openEditEvent = (event: VisualPlanTimedEventV2, category: EventCategory) => {
    if (!selectedRef) return;
    setEventDraft(eventDraftFrom(event, category, selectedRef.shot.id));
    openModal("event");
  };

  const addScene = () => {
    if (!plan || !editable) return;
    const scene = createScene(makeId("scene"), plan.entities);
    setPendingCreate({ kind: "scene", id: scene.id });
    setSceneDraft(scene);
    openModal("scene");
  };

  const deleteScene = async (sceneId = selectedScene?.id) => {
    if (!sceneId || !editable) return;
    if (pendingCreate?.kind === "scene" && pendingCreate.id === sceneId) {
      dismissModal();
      return;
    }
    const current = planRef.current;
    if (!current) return;
    const sceneIndex = current.scenes.findIndex((scene) => scene.id === sceneId);
    if (sceneIndex < 0) return;
    const adjacentScene = current.scenes[sceneIndex - 1] ?? current.scenes[sceneIndex + 1];
    const selectionWasRemoved = selectedSceneId === sceneId || selectedRef?.scene.id === sceneId;
    const accepted = await confirmDialog({
      title: t("visualPlan.deleteScene"),
      message: t("visualPlan.deleteSceneMessage"),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!accepted) return;
    const changed = commit(removeVisualScene(current, sceneId));
    if (!changed) return;
    if (selectionWasRemoved) {
      const nextRef = adjacentScene
        ? visualShots(changed).find(({ scene }) => scene.id === adjacentScene.id)
        : null;
      if (nextRef) {
        selectShot(nextRef);
      } else {
        const nextSceneId = adjacentScene && changed.scenes.some((scene) => scene.id === adjacentScene.id)
          ? adjacentScene.id
          : null;
        setSelectedSceneId(nextSceneId);
        setSelectedShotId(null);
        setSelectedKeyframeId(null);
        setSelectedEntityId(null);
        setPlayheadMs(nextSceneId ? visualSceneStartMs(changed, nextSceneId) ?? 0 : 0);
      }
    }
    if (modal === "scene") dismissModal();
  };

  const addShot = () => {
    if (!selectedScene || !editable) return;
    const sceneEntityIds = new Set(selectedScene.participant_entity_ids);
    const sceneEntities = (plan?.entities ?? []).filter((entity) => sceneEntityIds.has(entity.id));
    const shot = createShot(makeId("shot"), sceneEntities);
    const sceneId = selectedScene.id;
    setPendingCreate({ kind: "shot", id: shot.id, sceneId });
    setShotDraft(shotDraftFrom(shot));
    openModal("shot");
  };

  const deleteShot = async (shotId = selectedShot?.id) => {
    if (!shotId || !editable) return;
    if (pendingCreate?.kind === "shot" && pendingCreate.id === shotId) {
      dismissModal();
      return;
    }
    const current = planRef.current;
    if (!current) return;
    const targetRef = visualShots(current).find(({ shot }) => shot.id === shotId);
    if (!targetRef) return;
    const adjacentShot = targetRef.scene.shots[targetRef.shotIndex - 1] ?? targetRef.scene.shots[targetRef.shotIndex + 1];
    const selectionWasRemoved = selectedShotId === shotId || selectedRef?.shot.id === shotId;
    const accepted = await confirmDialog({
      title: t("visualPlan.deleteShot"),
      message: t("visualPlan.deleteShotMessage"),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!accepted) return;
    const changed = commit(removeVisualShot(current, shotId));
    const nextRef = changed && adjacentShot
      ? visualShots(changed).find(({ shot }) => shot.id === adjacentShot.id)
      : null;
    if (selectionWasRemoved) {
      if (nextRef) {
        selectShot(nextRef);
      } else {
        setSelectedSceneId(targetRef.scene.id);
        setSelectedShotId(null);
        setSelectedKeyframeId(null);
        setSelectedEntityId(null);
        setPlayheadMs(targetRef.start_ms);
      }
    }
    if (modal === "shot") dismissModal();
  };

  const saveFrame = () => {
    if (!frameDraft || !editable) return;
    if (!Number.isFinite(frameDraft.width) || !Number.isFinite(frameDraft.height) || frameDraft.width <= 0 || frameDraft.height <= 0) {
      addToast(t("visualPlan.validation.frameDimensionsPositive"), "error");
      return;
    }
    commit((next) => {
      next.frame.width = Math.max(1, Math.round(finiteOr(frameDraft.width, next.frame.width)));
      next.frame.height = Math.max(1, Math.round(finiteOr(frameDraft.height, next.frame.height)));
      next.frame.background = frameDraft.background.trim() || null;
      const frame = { width: next.frame.width, height: next.frame.height };
      next.scenes.forEach((scene) => scene.shots.forEach((shot) => shot.keyframes.forEach((keyframe) => keyframe.entity_states.forEach((state) => {
        const transform = normalizeEntityTransform(state.bounds, state.rotation_degrees, state.bounds, frame);
        state.bounds = transform.bounds;
        state.rotation_degrees = transform.rotation;
      }))));
    });
    dismissModal();
  };

  const saveScene = () => {
    if (!sceneDraft || !editable) return;
    if (!sceneDraft.title.trim()) {
      addToast(t("visualPlan.validation.textRequired", { label: t("visualPlan.titleLabel") }), "error");
      return;
    }
    if (!sceneDraft.narrative_purpose.trim()) {
      addToast(t("visualPlan.validation.textRequired", { label: t("visualPlan.narrativePurpose") }), "error");
      return;
    }
    const draft = structuredClone(sceneDraft);
    const participants = [...new Set(draft.participant_entity_ids)];
    const supportsIntercutGroup = draft.editing_mode === "intercut" || draft.editing_mode === "parallel_action";
    Object.assign(draft, {
      title: draft.title.trim(),
      narrative_purpose: draft.narrative_purpose.trim(),
      location: draft.location?.trim() || null,
      time_of_day: draft.time_of_day?.trim() || null,
      intercut_group_id: supportsIntercutGroup ? draft.intercut_group_id?.trim() || null : null,
      participant_entity_ids: participants,
      focal_entity_id: draft.focal_entity_id && participants.includes(draft.focal_entity_id)
        ? draft.focal_entity_id
        : null,
    });
    const isPending = pendingCreate?.kind === "scene" && pendingCreate.id === draft.id;
    const changed = commit((next) => {
      const target = next.scenes.find((scene) => scene.id === draft.id);
      if (target) Object.assign(target, draft);
      else if (isPending) next.scenes.push(draft);
    });
    if (!changed) {
      dismissModal();
      return;
    }
    const ref = visualShots(changed).find(({ scene }) => scene.id === draft.id);
    if (ref) {
      selectShot(ref);
    } else {
      setSelectedSceneId(draft.id);
      setSelectedShotId(null);
      setSelectedKeyframeId(null);
      setSelectedEntityId(null);
      setPlayheadMs(visualSceneStartMs(changed, draft.id) ?? 0);
    }
    dismissModal();
  };

  const saveShot = () => {
    if (!shotDraft || !editable) return;
    const draft = shotDraft;
    const current = planRef.current;
    if (!current) return;
    if (!draft.shot.title.trim()) {
      addToast(t("visualPlan.validation.textRequired", { label: t("visualPlan.titleLabel") }), "error");
      return;
    }
    if (!draft.shot.narrative_purpose.trim()) {
      addToast(t("visualPlan.validation.textRequired", { label: t("visualPlan.narrativePurpose") }), "error");
      return;
    }
    if (!Number.isFinite(draft.shot.duration_ms) || draft.shot.duration_ms < 250) {
      addToast(t("visualPlan.validation.durationPositive", { label: t("visualPlan.shot") }), "error");
      return;
    }
    const targetRef = visualShots(current).find(({ shot }) => shot.id === draft.shot.id);
    const pendingShot = pendingCreate?.kind === "shot"
      && pendingCreate.id === draft.shot.id
      && current.scenes.some((scene) => scene.id === pendingCreate.sceneId)
      ? pendingCreate
      : null;
    if (!targetRef && !pendingShot) return;
    const relativePlayheadMs = targetRef
      ? Math.max(0, playheadMs - targetRef.start_ms)
      : 0;
    const durationMs = Number.isFinite(draft.shot.duration_ms)
      ? Math.max(250, Math.round(draft.shot.duration_ms))
      : null;
    if (durationMs == null) return;
    let next: ChapterVisualPlanV2;
    if (pendingShot) {
      next = clonePlan(current);
      const scene = next.scenes.find((candidate) => candidate.id === pendingShot.sceneId);
      if (!scene) return;
      const newShot = structuredClone(draft.shot);
      newShot.duration_ms = draft.initialDurationMs;
      scene.shots.push(newShot);
      next = resizeVisualShot(next, newShot.id, durationMs);
    } else {
      next = resizeVisualShot(current, targetRef!.shot.id, durationMs);
    }
    const ref = visualShots(next).find((candidate) => candidate.shot.id === draft.shot.id);
    if (!ref) return;
    const title = draft.shot.title.trim();
    const narrativePurpose = draft.shot.narrative_purpose.trim();
    Object.assign(ref.shot, {
      title,
      narrative_purpose: narrativePurpose,
      coverage: draft.shot.coverage,
      shot_size: draft.shot.shot_size,
      camera_angle: draft.shot.camera_angle,
      camera_height: draft.shot.camera_height,
      lens_intent: draft.shot.lens_intent?.trim() || null,
      composition: draft.shot.composition?.trim() || null,
      continuity_notes: draft.continuityNotes.split("\n").map((line) => line.trim()).filter(Boolean),
    });
    const nextShot = ref.scene.shots[ref.shotIndex + 1];
    ref.shot.transition_to_next = draft.transitionKind === "none" || !nextShot
      ? null
      : {
          kind: draft.transitionKind,
          duration_ms: draft.transitionKind === "continuous" ? 0 : Math.min(Math.max(0, Math.round(draft.transitionDurationMs)), ref.shot.duration_ms, nextShot.duration_ms),
          preserved_action: draft.transitionPreservedAction.trim() || null,
          preserved_sound: draft.transitionPreservedSound.trim() || null,
          notes: draft.transitionNotes.trim() || null,
        };
    const changed = commit(next);
    if (!changed) {
      dismissModal();
      return;
    }
    const nextRef = visualShots(changed).find(({ shot }) => shot.id === draft.shot.id);
    if (nextRef) {
      selectShot(nextRef);
      const nextPlayheadMs = Math.min(nextRef.end_ms - 1, nextRef.start_ms + relativePlayheadMs);
      const nextKeyframe = nearestKeyframe(nextRef.shot, nextPlayheadMs - nextRef.start_ms);
      setSelectedKeyframeId(nextKeyframe?.id ?? null);
      setSelectedEntityId(null);
      setPlayheadMs(nextPlayheadMs);
    }
    dismissModal();
  };

  const saveKeyframe = () => {
    if (!keyframeDraft || !editable) return;
    const draft = keyframeDraft;
    const current = planRef.current;
    const pending = pendingCreate?.kind === "keyframe" && pendingCreate.id === draft.keyframe.id
      ? pendingCreate
      : null;
    const shotId = pending?.shotId ?? draft.shotId;
    const shot = findPlanShot(current, shotId);
    const atMs = Math.round(draft.relativeTimeMs);
    const label = draft.keyframe.label.trim();
    const camera = draft.keyframe.camera;
    if (!shot || !label || !Number.isFinite(draft.relativeTimeMs) || !isVisualKeyframeTimeAvailable(shot, draft.keyframe.id, draft.relativeTimeMs)) {
      addToast(t("visualPlan.validation.keyframeTimeInvalid", { label: label || t("visualPlan.keyframe") }), "error");
      return;
    }
    if (!Number.isFinite(camera.center_x) || !Number.isFinite(camera.center_y) || !Number.isFinite(camera.zoom) || !Number.isFinite(camera.rotation_degrees) || camera.zoom <= 0 || camera.center_x < 0 || camera.center_x > 1 || camera.center_y < 0 || camera.center_y > 1) {
      addToast(t("visualPlan.validation.cameraParamsInvalid", { label: t("visualPlan.keyframe") }), "error");
      return;
    }
    if (camera.focus_entity_id != null && !draft.keyframe.entity_states.some((state) => state.entity_id === camera.focus_entity_id)) {
      addToast(t("visualPlan.validation.missingReferenceId", { label: t("visualPlan.focusEntity"), value: camera.focus_entity_id }), "error");
      return;
    }
    const changed = pending
      ? commit((next) => {
          const targetShot = visualShots(next).find(({ shot: item }) => item.id === pending.shotId)?.shot;
          if (!targetShot) return;
          targetShot.keyframes.push({ ...structuredClone(draft.keyframe), at_ms: atMs, label });
          targetShot.keyframes.sort((left, right) => left.at_ms - right.at_ms);
        })
      : commit((next) => {
          const targetShot = visualShots(next).find(({ shot: item }) => item.id === shotId)?.shot;
          const target = targetShot?.keyframes.find((keyframe) => keyframe.id === draft.keyframe.id);
          if (!targetShot || !target) return;
          Object.assign(target, draft.keyframe, { at_ms: atMs, label });
          targetShot.keyframes.sort((left, right) => left.at_ms - right.at_ms);
        });
    if (!changed) {
      dismissModal();
      return;
    }
    const nextRef = visualShots(changed).find(({ shot: item }) => item.id === shotId);
    const nextKeyframe = nextRef?.shot.keyframes.find(({ id }) => id === draft.keyframe.id);
    if (nextRef && nextKeyframe) {
      selectKeyframe(nextKeyframe, nextRef);
    }
    dismissModal();
  };

  const addKeyframe = () => {
    if (!selectedShot || !selectedRef || !editable || selectedShot.duration_ms <= 0) return;
    const relative = availableVisualKeyframeTime(
      selectedShot,
      Math.max(0, Math.min(selectedShot.duration_ms, playheadMs - selectedRef.start_ms)),
    );
    if (relative == null) {
      addToast(t("visualPlan.keyframeNoAvailableTime"), "error");
      return;
    }
    const source = nearestKeyframe(selectedShot, relative);
    const keyframeId = makeId("keyframe");
    const label = t("novel.keyframeLabel", { time: formatPlanTime(relative) });
    const entityIds = selectedRef.scene.participant_entity_ids.filter((entityId) => entityById.has(entityId));
    const value: VisualKeyframeV2 = source
      ? {
          ...structuredClone(source),
          id: keyframeId,
          at_ms: relative,
          label,
          relations: source.relations.map((relation) => ({ ...structuredClone(relation), id: makeId("relation") })),
        }
      : {
          id: keyframeId,
          at_ms: relative,
          label,
          interpolation_to_next: "linear",
          camera: defaultCamera(),
          entity_states: entityIds.map((entityId, index) => defaultEntityState(entityId, index)),
          relations: [],
        };
    setPlaying(false);
    setPendingCreate({ kind: "keyframe", id: keyframeId, shotId: selectedShot.id });
    setKeyframeDraft({ shotId: selectedShot.id, keyframe: value, relativeTimeMs: relative });
    openModal("keyframe");
  };

  const removeKeyframe = async (
    keyframeId = selectedKeyframe?.id,
    shotId = selectedRef?.shot.id,
  ) => {
    if (!keyframeId || !editable) return;
    if (pendingCreate?.kind === "keyframe" && pendingCreate.id === keyframeId && pendingCreate.shotId === shotId) {
      dismissModal();
      return;
    }
    const current = planRef.current;
    const currentShot = findPlanShot(current, shotId);
    const keyframeIndex = currentShot?.keyframes.findIndex((keyframe) => keyframe.id === keyframeId) ?? -1;
    if (!current || !shotId || !currentShot || currentShot.keyframes.length <= 2 || keyframeIndex <= 0 || keyframeIndex >= currentShot.keyframes.length - 1) return;
    const nextKeyframe = currentShot.keyframes[keyframeIndex + 1] ?? currentShot.keyframes[keyframeIndex - 1];
    const accepted = await confirmDialog({
      title: t("visualPlan.deleteKeyframe"),
      message: t("visualPlan.deleteKeyframeMessage"),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!accepted) return;
    const changed = commit(removeVisualKeyframe(current, shotId, keyframeId));
    if (!changed) return;
    if (selectedShotId === shotId && selectedKeyframeId === keyframeId) {
      const nextRef = visualShots(changed).find(({ shot }) => shot.id === shotId);
      const next = nextRef?.shot.keyframes.find(({ id }) => id === nextKeyframe?.id);
      setSelectedKeyframeId(next?.id ?? null);
      setSelectedEntityId(null);
      setPlayheadMs(nextRef && next ? nextRef.start_ms + next.at_ms : nextRef?.start_ms ?? 0);
    }
    if (modal === "keyframe") dismissModal();
  };

  const saveEntityLibrary = () => {
    if (!entityLibraryDraft || !editable) return;
    const draft = entityLibraryDraft;
    const label = draft.label.trim();
    const description = draft.description.trim();
    if (!label) {
      addToast(t("visualPlan.validation.textRequired", { label: t("visualPlan.validation.labels.entityName", { id: draft.id ?? t("visualPlan.entity") }) }), "error");
      return;
    }
    if (!description) {
      addToast(t("visualPlan.validation.textRequired", { label: t("visualPlan.validation.labels.entityDescription", { id: draft.id ?? t("visualPlan.entity") }) }), "error");
      return;
    }
    const newId = draft.id ?? makeId("entity");
    const changed = commit((next) => {
      const existing = next.entities.find((entity) => entity.id === newId);
      if (existing) Object.assign(existing, { label, kind: draft.kind, description });
      else next.entities.push({ id: newId, label, kind: draft.kind, description });
      if (!existing && selectedRef && selectedKeyframe) {
        const shot = visualShots(next).find((ref) => ref.shot.id === selectedRef.shot.id)?.shot;
        const frame = shot?.keyframes.find((keyframe) => keyframe.id === selectedKeyframe.id);
        const scene = next.scenes.find((candidate) => candidate.id === selectedRef.scene.id);
        if (frame && !frame.entity_states.some((state) => state.entity_id === newId)) frame.entity_states.push(defaultEntityState(newId, frame.entity_states.length));
        if (scene && !scene.participant_entity_ids.includes(newId)) scene.participant_entity_ids.push(newId);
      }
    });
    if (changed) setSelectedEntityId(newId);
    dismissModal();
  };

  const deleteEntity = async (entityId = entityLibraryDraft?.id) => {
    if (!entityId || !editable) return;
    const current = planRef.current;
    if (!current || !current.entities.some((entity) => entity.id === entityId)) return;
    const accepted = await confirmDialog({
      title: t("visualPlan.deleteEntity"),
      message: t("visualPlan.deleteEntityMessage"),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!accepted) return;
    const changed = commit(removeVisualEntity(current, entityId));
    if (!changed) return;
    if (selectedEntityId === entityId) setSelectedEntityId(null);
    if (modal === "entity") dismissModal();
  };

  const addExistingEntity = (entityId: string) => {
    if (!selectedRef || !selectedKeyframe || !editable) return;
    if (commit((next) => {
      const shot = visualShots(next).find((ref) => ref.shot.id === selectedRef.shot.id)?.shot;
      const frame = shot?.keyframes.find((keyframe) => keyframe.id === selectedKeyframe.id);
      const scene = next.scenes.find((candidate) => candidate.id === selectedRef.scene.id);
      if (!frame) return;
      if (!frame.entity_states.some((state) => state.entity_id === entityId)) frame.entity_states.push(defaultEntityState(entityId, frame.entity_states.length));
      if (scene && !scene.participant_entity_ids.includes(entityId)) scene.participant_entity_ids.push(entityId);
    })) setSelectedEntityId(entityId);
  };

  const removeEntityFromFrame = (entityId: string) => {
    if (!selectedRef || !selectedKeyframe || !editable) return;
    if (commit((next) => {
      const shot = visualShots(next).find((ref) => ref.shot.id === selectedRef.shot.id)?.shot;
      const frame = shot?.keyframes.find((keyframe) => keyframe.id === selectedKeyframe.id);
      if (!frame) return;
      frame.entity_states = frame.entity_states.filter((state) => state.entity_id !== entityId);
      frame.relations = frame.relations.filter((relation) => relation.source_entity_id !== entityId && relation.target_entity_id !== entityId);
      if (frame.camera.focus_entity_id === entityId) frame.camera.focus_entity_id = null;
      frame.entity_states.forEach((state) => {
        if (state.gaze_target_entity_id === entityId) state.gaze_target_entity_id = null;
      });
    })) setSelectedEntityId(null);
  };

  const saveEntityState = () => {
    if (!entityStateDraft || !editable) return;
    const draft = entityStateDraft;
    const current = planRef.current;
    const draftKeyframe = findPlanKeyframe(current, draft.shotId, draft.keyframeId);
    const numericValues = [draft.x, draft.y, draft.width, draft.height, draft.opacity, draft.rotation, draft.zIndex];
    if (!numericValues.every(Number.isFinite) || draft.width < 1 || draft.height < 1 || draft.width > 100 || draft.height > 100 || draft.x < 0 || draft.y < 0 || draft.x + draft.width > 100 || draft.y + draft.height > 100 || draft.opacity < 0 || draft.opacity > 100 || !Number.isInteger(draft.zIndex)) {
      addToast(t("visualPlan.validation.entityBoundsOutOfBounds", { label: t("visualPlan.entityState"), entityId: draft.entityId }), "error");
      return;
    }
    if (!draftKeyframe) return;
    if (draft.gazeTarget && !draftKeyframe.entity_states.some((state) => state.entity_id === draft.gazeTarget)) {
      addToast(t("visualPlan.validation.missingReferenceId", { label: t("visualPlan.gazeTarget"), value: draft.gazeTarget }), "error");
      return;
    }
    const changed = commit((next) => {
      const frame = findPlanKeyframe(next, draft.shotId, draft.keyframeId);
      const state = frame?.entity_states.find((item) => item.entity_id === draft.entityId);
      if (!frame || !state) return;
      const width = clampFrameValue(finiteOr(draft.width, 1), 1, 100);
      const height = clampFrameValue(finiteOr(draft.height, 1), 1, 100);
      const x = clampFrameValue(finiteOr(draft.x, 0), 0, 100 - width);
      const y = clampFrameValue(finiteOr(draft.y, 0), 0, 100 - height);
      const nextTransform = normalizeEntityTransform({
        x: x / 100,
        y: y / 100,
        width: width / 100,
        height: height / 100,
      }, draft.rotation, state.bounds, {
          width: Math.max(1, finiteOr(next.frame.width, 16)),
          height: Math.max(1, finiteOr(next.frame.height, 9)),
      });
      Object.assign(state, {
        visible: draft.visible,
        action: draft.action.trim() || null,
        expression: draft.expression.trim() || null,
        pose: draft.pose.trim() || null,
        dialogue: draft.dialogue.trim() || null,
        notes: draft.notes.trim() || null,
        bounds: nextTransform.bounds,
        opacity: clampFrameValue(finiteOr(draft.opacity, 100), 0, 100) / 100,
        rotation_degrees: nextTransform.rotation,
        z_index: Math.round(clampFrameValue(finiteOr(draft.zIndex, state.z_index), -2147483648, 2147483647)),
        facing: draft.facing,
        gaze_target_entity_id: draft.gazeTarget && draft.gazeTarget !== draft.entityId ? draft.gazeTarget : null,
      });
      if (!draft.visible) frame.relations = frame.relations.filter((relation) => relation.source_entity_id !== draft.entityId && relation.target_entity_id !== draft.entityId);
      if (!draft.visible && frame.camera.focus_entity_id === draft.entityId) frame.camera.focus_entity_id = null;
    });
    if (changed) setSelectedEntityId(draft.entityId);
    dismissModal();
  };

  const transformEntity = (entityId: string, transform: EntityTransform) => {
    if (!selectedRef || !selectedKeyframe || !editable) return;
    const changed = commit((next) => {
      const shot = visualShots(next).find((ref) => ref.shot.id === selectedRef.shot.id)?.shot;
      const frame = shot?.keyframes.find((keyframe) => keyframe.id === selectedKeyframe.id);
      const state = frame?.entity_states.find((item) => item.entity_id === entityId);
      if (!state) return;
      const nextTransform = normalizeEntityTransform(
        transform.bounds,
        transform.rotation,
        state.bounds,
        {
          width: Math.max(1, finiteOr(next.frame.width, 16)),
          height: Math.max(1, finiteOr(next.frame.height, 9)),
        },
      );
      Object.assign(state, {
        bounds: nextTransform.bounds,
        rotation_degrees: nextTransform.rotation,
      });
    });
    if (changed) setSelectedEntityId(entityId);
  };

  const addRelation = () => {
    const draft = entityStateDraft;
    const sourceEntityId = selectedEntityId ?? draft?.entityId;
    const shotId = draft?.shotId ?? selectedRef?.shot.id;
    const keyframeId = draft?.keyframeId ?? selectedKeyframe?.id;
    const current = planRef.current;
    const frame = findPlanKeyframe(current, shotId, keyframeId);
    if (!frame || !sourceEntityId || !relationTarget || !editable) return;
    if (relationTarget === sourceEntityId) {
      addToast(t("visualPlan.validation.selfRelation", { label: t("visualPlan.entityState") }), "error");
      return;
    }
    if (!frame.entity_states.some((state) => state.entity_id === sourceEntityId && state.visible) || !frame.entity_states.some((state) => state.entity_id === relationTarget && state.visible)) {
      addToast(t("visualPlan.validation.relationTargetStateMissing", { label: t("visualPlan.entityState") }), "error");
      return;
    }
    const label = relationLabel.trim();
    if (relationKind === "custom" && !label) {
      addToast(t("visualPlan.validation.customRelationLabelRequired", { label: t("visualPlan.entityState") }), "error");
      return;
    }
    if (frame.relations.some((relation) => relation.id !== relationEditingId && relation.source_entity_id === sourceEntityId && relation.target_entity_id === relationTarget && relation.kind === relationKind)) {
      addToast(t("visualPlan.validation.duplicateId", { label: t("visualPlan.validation.labels.relation"), value: `${sourceEntityId} -> ${relationTarget}` }), "error");
      return;
    }
    const changed = commit((next) => {
      const targetFrame = findPlanKeyframe(next, shotId, keyframeId);
      if (!targetFrame) return;
      if (relationEditingId) {
        const relation = targetFrame.relations.find((item) => item.id === relationEditingId);
        if (relation) Object.assign(relation, {
          source_entity_id: sourceEntityId,
          target_entity_id: relationTarget,
          kind: relationKind,
          label: label || null,
        });
      } else {
        targetFrame.relations.push({ id: makeId("relation"), source_entity_id: sourceEntityId, target_entity_id: relationTarget, kind: relationKind, label: label || null });
      }
    });
    if (changed) {
      setRelationEditingId(null);
      setRelationLabel("");
    }
  };

  const editRelation = (relation: VisualKeyframeV2["relations"][number]) => {
    setSelectedEntityId(relation.source_entity_id);
    setRelationEditingId(relation.id);
    setRelationKind(relation.kind);
    setRelationTarget(relation.target_entity_id);
    setRelationLabel(toText(relation.label));
  };

  const cancelRelationEdit = () => {
    setRelationEditingId(null);
    setRelationTarget("");
    setRelationLabel("");
  };

  const deleteRelation = async (relationId: string) => {
    const shotId = entityStateDraft?.shotId ?? selectedRef?.shot.id;
    const keyframeId = entityStateDraft?.keyframeId ?? selectedKeyframe?.id;
    const relation = findPlanKeyframe(planRef.current, shotId, keyframeId)?.relations.find((item) => item.id === relationId);
    if (!relation || !editable) return;
    const accepted = await confirmDialog({
      title: t("visualPlan.deleteRelation"),
      message: t("visualPlan.deleteRelationMessage"),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!accepted) return;
    commit((next) => {
      const frame = findPlanKeyframe(next, shotId, keyframeId);
      if (frame) frame.relations = frame.relations.filter((relation) => relation.id !== relationId);
    });
    if (relationEditingId === relationId) cancelRelationEdit();
  };

  const saveEvent = () => {
    if (!eventDraft || !editable) return;
    const draft = eventDraft;
    if (!draft.text.trim()) {
      addToast(t("visualPlan.validation.textRequired", { label: t("visualPlan.eventDescription") }), "error");
      return;
    }
    const current = planRef.current;
    const shot = findPlanShot(current, draft.shotId);
    if (!current || !shot) return;
    const startMs = Math.round(draft.startMs);
    const endMs = Math.round(draft.endMs);
    if (!Number.isFinite(startMs) || !Number.isFinite(endMs) || startMs < 0 || endMs <= startMs || endMs > shot.duration_ms) {
      addToast(t("visualPlan.validation.eventOutOfShot", { label: shot.title || t("visualPlan.shot") }), "error");
      return;
    }
    const eventKeyframe = nearestKeyframe(shot, startMs);
    const visibleEntityIds = new Set(
      eventKeyframe?.entity_states.filter((state) => state.visible).map((state) => state.entity_id) ?? [],
    );
    if (draft.category === "entity" && (!draft.entityId || !visibleEntityIds.has(draft.entityId))) {
      addToast(t("visualPlan.validation.missingReferenceId", { label: t("visualPlan.entityState"), value: draft.entityId || "—" }), "error");
      return;
    }
    if ((draft.category === "entity" || draft.category === "camera") && draft.targetEntityId && !visibleEntityIds.has(draft.targetEntityId)) {
      addToast(t("visualPlan.validation.missingReferenceId", { label: t("visualPlan.eventTarget"), value: draft.targetEntityId }), "error");
      return;
    }
    if (draft.category === "sound" && draft.speakerEntityId && !visibleEntityIds.has(draft.speakerEntityId)) {
      addToast(t("visualPlan.validation.missingReferenceId", { label: t("visualPlan.eventSpeaker"), value: draft.speakerEntityId }), "error");
      return;
    }
    if (!eventOptions(draft.category).some(([kind]) => kind === draft.kind)) {
      addToast(t("visualPlan.validation.textRequired", { label: t("visualPlan.eventKind") }), "error");
      return;
    }
    const parameters = parseVisualEventParameters(draft.parametersText);
    if (!parameters) {
      addToast(t("visualPlan.eventParametersInvalid"), "error");
      return;
    }
    const changed = commit((next) => {
      const targetShot = findPlanShot(next, draft.shotId);
      if (!targetShot) return;
      if (draft.category === "entity") {
        const value: VisualEntityEventV2 = { id: draft.id ?? makeId("event"), kind: draft.kind as VisualEntityEventKindV1, start_ms: startMs, end_ms: endMs, entity_id: draft.entityId, target_entity_id: draft.targetEntityId || null, description: draft.text.trim(), parameters };
        const index = targetShot.entity_events.findIndex((event) => event.id === draft.id);
        if (index >= 0) targetShot.entity_events[index] = value;
        else targetShot.entity_events.push(value);
      } else if (draft.category === "camera") {
        const value: VisualCameraEventV2 = { id: draft.id ?? makeId("event"), kind: draft.kind as VisualCameraEventKindV1, start_ms: startMs, end_ms: endMs, target_entity_id: draft.targetEntityId || null, description: draft.text.trim(), parameters };
        const index = targetShot.camera_events.findIndex((event) => event.id === draft.id);
        if (index >= 0) targetShot.camera_events[index] = value;
        else targetShot.camera_events.push(value);
      } else if (draft.category === "sound") {
        const value: VisualSoundEventV2 = { id: draft.id ?? makeId("event"), kind: draft.kind as VisualSoundEventKindV1, start_ms: startMs, end_ms: endMs, speaker_entity_id: draft.speakerEntityId || null, content: draft.text.trim(), parameters };
        const index = targetShot.sound_events.findIndex((event) => event.id === draft.id);
        if (index >= 0) targetShot.sound_events[index] = value;
        else targetShot.sound_events.push(value);
      } else {
        const value: VisualTechniqueEventV2 = { id: draft.id ?? makeId("event"), kind: draft.kind as VisualTechniqueKindV1, start_ms: startMs, end_ms: endMs, description: draft.text.trim(), parameters };
        const index = targetShot.technique_events.findIndex((event) => event.id === draft.id);
        if (index >= 0) targetShot.technique_events[index] = value;
        else targetShot.technique_events.push(value);
      }
    });
    if (!changed) return;
    dismissModal();
  };

  const deleteEvent = async (eventId: string, shotId = selectedRef?.shot.id) => {
    if (!shotId || !editable) return;
    const targetShot = findPlanShot(planRef.current, shotId);
    const event = targetShot
      ? [targetShot.entity_events, targetShot.camera_events, targetShot.sound_events, targetShot.technique_events]
        .flat()
        .find((item) => item.id === eventId)
      : null;
    if (!event) return;
    const accepted = await confirmDialog({
      title: t("visualPlan.deleteEvent"),
      message: t("visualPlan.deleteEventMessage"),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!accepted) return;
    commit((next) => { removeVisualPlanTimedEvent(next, shotId, eventId); });
  };

  const moveShot = (direction: -1 | 1) => {
    if (!selectedShot || !editable || !selectedRef) return;
    const current = planRef.current;
    if (!current) return;
    const next = moveVisualShot(current, selectedShot.id, direction);
    const changed = commit(next);
    const ref = changed && visualShots(changed).find(({ shot }) => shot.id === selectedShot.id);
    if (ref) selectShot(ref);
  };

  const moveScene = (sceneId: string, direction: -1 | 1) => {
    if (!editable) return;
    const current = planRef.current;
    if (!current) return;
    const selectedShotIdBefore = selectedShotId;
    const changed = commit(moveVisualScene(current, sceneId, direction));
    if (!changed) return;
    if (selectedShotIdBefore) {
      const ref = visualShots(changed).find(({ shot }) => shot.id === selectedShotIdBefore);
      if (ref) selectShot(ref);
    } else if (selectedSceneId === sceneId) {
      setPlayheadMs(visualSceneStartMs(changed, sceneId) ?? 0);
    }
  };

  const startRevision = async () => {
    const runtime = runtimeRef.current;
    const current = planRef.current;
    if (!runtime || !current || !canEdit || !acquireLock()) return;
    setSaving(true);
    try {
      const saved = await saveChapterVisualPlanDraft(runtime.projectId, runtime.chapterId, null, clonePlan(current));
      if (runtimeRef.current !== runtime) return;
      runtime.record = saved;
      setPlanSet((value) => value ? { ...value, draft: saved } : value);
      setBaseline(JSON.stringify(current));
      setEditing(true);
      addToast(t("visualPlan.draftCreated"), "success");
    } catch (error) {
      if (runtimeRef.current === runtime) addToast(extractErrorMessage(error, t("visualPlan.draftSaveFailed")), "error");
    } finally {
      if (runtimeRef.current === runtime) setSaving(false);
      releaseLock();
    }
  };

  const confirmPlan = async () => {
    const runtime = runtimeRef.current;
    if (!runtime || !plan || strictIssues.length > 0 || !acquireLock()) return;
    setSaving(true);
    try {
      const saved = await ensureLatestDraft(runtime);
      if (!saved || runtimeRef.current !== runtime) return;
      const accepted = await confirmDialog({ title: t("visualPlan.confirmVisualPlan"), message: t("visualPlan.confirmVisualPlanMessage"), confirmText: t("visualPlan.confirmAndLock"), dialogType: "warning" });
      if (!accepted || runtimeRef.current !== runtime) return;
      const outcome = await confirmChapterVisualPlan(runtime.projectId, runtime.chapterId, saved);
      if (runtimeRef.current !== runtime) return;
      runtime.record = null;
      const next = clonePlan(outcome.visual_plan.plan);
      setPlan(next);
      planRef.current = next;
      setPlanSet((value) => value ? { ...value, draft: null, confirmed: outcome.visual_plan } : value);
      setBaseline(JSON.stringify(next));
      setEditing(false);
      setUndoStack([]);
      setRedoStack([]);
      addToast(outcome.invalidated_project_ids.length ? t("visualPlan.planConfirmedDownstream", { count: outcome.invalidated_project_ids.length }) : t("visualPlan.planConfirmed"), "success");
    } catch (error) {
      if (runtimeRef.current === runtime) addToast(extractErrorMessage(error, t("visualPlan.confirmPlanFailed")), "error");
    } finally {
      if (runtimeRef.current === runtime) setSaving(false);
      releaseLock();
    }
  };

  const discardDraft = async () => {
    const runtime = runtimeRef.current;
    if (!runtime?.record || !acquireLock()) return;
    setSaving(true);
    try {
      runtime.pendingSave = null;
      if (runtime.savePromise) await runtime.savePromise;
      const accepted = await confirmDialog({ title: t("visualPlan.discardDraft"), message: t("visualPlan.discardDraftMessage"), confirmText: t("visualPlan.discardDraftButton"), dialogType: "danger" });
      if (!accepted || runtimeRef.current !== runtime || !runtime.record) return;
      await discardChapterVisualPlanDraft(runtime.projectId, runtime.chapterId, runtime.record);
      if (runtimeRef.current !== runtime) return;
      runtimeRef.current = null;
      await loadPlan();
      addToast(t("visualPlan.draftDiscarded"), "success");
    } catch (error) {
      if (runtimeRef.current === runtime) addToast(extractErrorMessage(error, t("visualPlan.discardDraftFailed")), "error");
    } finally {
      if (runtimeRef.current === runtime) setSaving(false);
      releaseLock();
    }
  };

  if (!chapter) return <EmptyHint text={t("visualPlan.noChapter")} />;
  if (loadError) return (
    <div className="a4n-flat-surface flex flex-col items-center gap-3 p-8 text-center">
      <p className="text-small text-error">{loadError}</p>
      <BookButton size="sm" variant="secondary" onClick={() => void loadPlan()}>{t("visualPlan.retryLoad")}</BookButton>
    </div>
  );
  if (loading || !planSet || !plan) return <div className="a4n-flat-surface py-14"><LoadingSpinner size="md" text={t("visualPlan.loadingPlan")} inline={false} /></div>;

  const saveStatus = planSet.confirmed && !editing
    ? t("visualPlan.readOnly")
    : record && !planDirty
      ? t("visualPlan.draftSaved")
      : t("visualPlan.draftNotSaved");
  const hasScenes = plan.scenes.length > 0;
  const activeEntityStates = selectedKeyframe?.entity_states ?? [];
  const activeVisibleEntityStates = activeEntityStates.filter((state) => state.visible);
  const activeEvents: ReadonlyArray<readonly [EventCategory, MessageKey, VisualPlanTimedEventV2[]]> = selectedShot ? [
    ["entity", "visualPlan.entityEvents", selectedShot.entity_events],
    ["camera", "visualPlan.cameraEvents", selectedShot.camera_events],
    ["sound", "visualPlan.soundEvents", selectedShot.sound_events],
    ["technique", "visualPlan.techniques", selectedShot.technique_events],
  ] : [];
  const keyframeDraftShot = findPlanShot(plan, keyframeDraft?.shotId);
  const entityStateDraftKeyframe = findPlanKeyframe(plan, entityStateDraft?.shotId, entityStateDraft?.keyframeId);
  const eventDraftShot = findPlanShot(plan, eventDraft?.shotId);
  const eventDraftEntityStates = eventDraftShot
    ? nearestKeyframe(eventDraftShot, finiteOr(eventDraft?.startMs, 0))?.entity_states.filter((state) => state.visible) ?? []
    : [];
  const eventEntityOptions = eventDraftEntityStates.map((state) => ({
    value: state.entity_id,
    label: entityById.get(state.entity_id)?.label ?? state.entity_id,
  }));
  const shotDraftRef = shotDraft
    ? visualShots(plan).find(({ shot }) => shot.id === shotDraft.shot.id)
    : null;
  const shotDraftSceneId = pendingCreate?.kind === "shot" && shotDraft && pendingCreate.id === shotDraft.shot.id
    ? pendingCreate.sceneId
    : shotDraftRef?.scene.id;
  const shotDraftScene = shotDraftSceneId ? plan.scenes.find(({ id }) => id === shotDraftSceneId) : null;
  const shotDraftIndex = shotDraftScene && shotDraftRef?.scene.id === shotDraftScene.id
    ? shotDraftRef.shotIndex
    : -1;
  const selectedRelativePlayheadMs = selectedShot && selectedRef
    ? Math.max(0, Math.min(selectedShot.duration_ms, playheadMs - selectedRef.start_ms))
    : null;
  const canAddKeyframe = Boolean(
    selectedShot
    && selectedShot.duration_ms > 0
    && availableVisualKeyframeTime(
      selectedShot,
      selectedRelativePlayheadMs ?? 0,
    ) != null,
  );
  const keyframeDraftTimeValid = Boolean(
    keyframeDraft
    && keyframeDraftShot
    && isVisualKeyframeTimeAvailable(keyframeDraftShot, keyframeDraft.keyframe.id, keyframeDraft.relativeTimeMs),
  );
  const canDeleteKeyframe = Boolean(
    selectedShot
    && selectedKeyframe
    && selectedShot.keyframes.length > 2
    && selectedShot.keyframes[0]?.id !== selectedKeyframe.id
    && selectedShot.keyframes.at(-1)?.id !== selectedKeyframe.id,
  );
  const canDeleteDraftKeyframe = Boolean(
    keyframeDraftShot
    && keyframeDraftShot.keyframes.length > 2
    && keyframeDraftShot.keyframes[0]?.id !== keyframeDraft?.keyframe.id
    && keyframeDraftShot.keyframes.at(-1)?.id !== keyframeDraft?.keyframe.id,
  );
  const shotHasNext = Boolean(
    shotDraft
    && !(pendingCreate?.kind === "shot" && pendingCreate.id === shotDraft.shot.id)
    && shotDraftScene?.shots[shotDraftIndex + 1],
  );
  const togglePlayback = () => {
    if (playheadMs >= totalDuration) setPlayheadMs(0);
    setPlaying((value) => !value);
  };
  const primaryAction = !editing
    ? {
        label: t("common.edit"),
        icon: <Settings2 size={13} aria-hidden="true" />,
        disabled: !canEdit || saving || locking,
        onClick: () => void startRevision(),
      }
    : record
      ? {
          label: t("visualPlan.confirmPlan"),
          icon: <CheckCircle2 size={13} aria-hidden="true" />,
          disabled: !canEdit || saving || locking || strictIssues.length > 0,
          onClick: () => void confirmPlan(),
        }
      : {
          label: t("visualPlan.saveDraft"),
          icon: <Save size={13} aria-hidden="true" />,
          disabled: !canEdit || saving || locking || !planDirty,
          onClick: () => void persistDraft(true),
        };
  const planActionItems: DropdownItem[] = [
    {
      label: t("common.undo"),
      icon: <Undo2 size={14} aria-hidden="true" />,
      onClick: undo,
      disabled: !editable || undoStack.length === 0,
    },
    {
      label: t("common.redo"),
      icon: <Redo2 size={14} aria-hidden="true" />,
      onClick: redo,
      disabled: !editable || redoStack.length === 0,
    },
    { type: "divider" },
    {
      label: t("visualPlan.frameSettings"),
      icon: <Grid2X2 size={14} aria-hidden="true" />,
      onClick: openFrameEditor,
      disabled: !plan,
    },
    {
      label: t("visualPlan.openSource"),
      icon: <Box size={14} aria-hidden="true" />,
      onClick: () => openModal("source"),
    },
    ...(canEdit
      ? [{
          label: t("agent.domain.directorOpen"),
          icon: <Film size={14} aria-hidden="true" />,
          onClick: () => openModal("director"),
          disabled: saving || locking,
        }]
      : []),
    ...(canEdit && editing && record
      ? [{
          type: "divider" as const,
        }, {
          label: t("visualPlan.discardDraftButton"),
          icon: <Trash2 size={14} aria-hidden="true" />,
          onClick: () => void discardDraft(),
          disabled: saving || locking,
          danger: true,
        }]
      : []),
  ];
  const shotActionItems: DropdownItem[] = [
    {
      label: t("visualPlan.editScene"),
      icon: <Layers3 size={14} aria-hidden="true" />,
      onClick: () => openSceneEditor(),
      disabled: !selectedScene || !editable,
    },
    {
      label: t("visualPlan.editShot"),
      icon: <Settings2 size={14} aria-hidden="true" />,
      onClick: () => openShotEditor(),
      disabled: !selectedShot || !editable,
    },
    {
      label: t("visualPlan.moveShotEarlier"),
      icon: <ArrowLeft size={14} aria-hidden="true" />,
      onClick: () => moveShot(-1),
      disabled: !selectedRef || !editable || selectedRef.shotIndex === 0,
    },
    {
      label: t("visualPlan.moveShotLater"),
      icon: <ArrowRight size={14} aria-hidden="true" />,
      onClick: () => moveShot(1),
      disabled: !selectedRef || !editable || selectedRef.shotIndex === selectedRef.scene.shots.length - 1,
    },
    {
      label: t("visualPlan.addShot"),
      icon: <Plus size={14} aria-hidden="true" />,
      onClick: addShot,
      disabled: !selectedScene || !editable,
    },
  ];
  const contextViewItems: DropdownItem[] = [
    {
      label: t("visualPlan.scenes"),
      icon: <Layers3 size={14} aria-hidden="true" />,
      onClick: () => openModal("sceneLibrary"),
      disabled: !hasScenes,
    },
    {
      label: t("visualPlan.planSummary"),
      icon: <Grid2X2 size={14} aria-hidden="true" />,
      onClick: () => openModal("summary"),
    },
    {
      label: t("visualPlan.entities"),
      icon: <Box size={14} aria-hidden="true" />,
      onClick: () => openModal("entityLibrary"),
    },
    {
      label: t("visualPlan.keyframes"),
      icon: <CircleDot size={14} aria-hidden="true" />,
      onClick: () => openModal("keyframeLibrary"),
      disabled: !selectedShot,
    },
    {
      label: t("visualPlan.eventDetails"),
      icon: <CircleDot size={14} aria-hidden="true" />,
      onClick: () => openModal("eventDetails"),
      disabled: !selectedShot,
    },
  ];
  const contextActionItems: DropdownItem[] = hasScenes
    ? [...planActionItems, { type: "divider" }, ...contextViewItems, { type: "divider" }, ...shotActionItems]
    : planActionItems;

  return (
    <section className="a4n-visual-plan flex min-w-0 flex-col pb-10">
      <div className="a4n-visual-plan__workspace a4n-flat-surface min-w-0">
        <main className="min-w-0">
          <div className="a4n-visual-plan__context flex min-w-0 flex-wrap items-center gap-2 px-3 py-2">
            <div className="flex min-w-0 w-full items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="text-[10px] font-semibold uppercase tracking-wide text-book-text-muted">{hasScenes ? `${t("visualPlan.scene")} / ${t("visualPlan.shot")}` : t("visualPlan.title")}</p>
                <p className="truncate text-small font-semibold text-book-text-main">
                  {hasScenes ? <>
                    {selectedScene?.title ?? t("visualPlan.noScenes")}
                    <span className="px-1 text-book-text-muted" aria-hidden="true">›</span>
                    {selectedShot?.title ?? t("visualPlan.noShot")}
                    {selectedShot ? <span className="px-1 font-mono text-caption font-normal text-book-text-muted">· {formatPlanTime(selectedShot.duration_ms)}</span> : null}
                  </> : t("visualPlan.noScenes")}
                </p>
              </div>
              <div className="flex min-w-0 shrink-0 items-center gap-1.5">
                <BookIconButton label={playing ? t("visualPlan.pause") : t("visualPlan.play")} size="sm" onClick={togglePlayback} disabled={!hasScenes || totalDuration <= 0}>
                  {playing ? <Pause size={14} /> : <Play size={14} />}
                </BookIconButton>
                <BookButton size="sm" className="min-w-0 max-w-[14rem]" onClick={hasScenes ? primaryAction.onClick : addScene} disabled={hasScenes ? primaryAction.disabled : !editable}>
                  {hasScenes ? primaryAction.icon : <Plus size={13} aria-hidden="true" />}
                  <span className="truncate">{hasScenes ? primaryAction.label : t("visualPlan.addScene")}</span>
                </BookButton>
                <Dropdown items={contextActionItems} ariaLabel={t("common.more")} />
              </div>
            </div>
            {hasScenes ? <div className="a4n-visual-plan__context-secondary flex w-full min-w-0 flex-wrap items-center gap-1.5 border-t border-book-border/45 pt-2">
              <div className="a4n-visual-plan__context-mobile-selects grid w-full min-w-0 grid-cols-2 gap-2">
                <SelectField
                  label={t("visualPlan.scene")}
                  value={selectedScene?.id ?? ""}
                  options={plan.scenes.map((scene) => ({ value: scene.id, label: scene.title || t("visualPlan.scene") }))}
                  onChange={selectScene}
                />
                <SelectField
                  label={t("visualPlan.shot")}
                  value={selectedShot?.id ?? ""}
                  options={(selectedScene?.shots ?? []).map((shot) => ({ value: shot.id, label: shot.title || t("visualPlan.shot") }))}
                  disabled={!selectedScene || selectedScene.shots.length === 0}
                  onChange={(shotId) => {
                    const ref = shotRefs.find(({ shot }) => shot.id === shotId);
                    if (ref) selectShot(ref);
                  }}
                />
              </div>
            </div> : null}
            <div className="flex min-h-5 w-full min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-caption text-book-text-muted">
              <span className={saving ? "text-book-primary" : undefined}>{saving ? t("visualPlan.saving") : saveStatus}</span>
              {locking ? <span className="inline-flex items-center gap-1 text-book-primary"><CircleDot size={12} />{t("visualPlan.editorLocked")}</span> : null}
              {strictIssues.length > 0 ? <span className="text-warning">{t("visualPlan.confirmBlocked", { count: strictIssues.length })}</span> : null}
            </div>
          </div>
          {!hasScenes ? <EmptyWorkspace t={t} /> : <>
            <section className="a4n-visual-plan__viewer min-w-0 px-3 pb-3" aria-label={t("visualPlan.viewerFrame")}>
              <div className="flex min-w-0 items-center justify-between gap-2">
                <div className="min-w-0">
                  <h4 className="text-small font-semibold text-book-text-main">{t("visualPlan.viewerFrame")}</h4>
                  <p className="truncate text-caption text-book-text-muted">{selectedKeyframe ? `${selectedKeyframe.label} · ${formatPlanTime(selectedKeyframe.at_ms)}` : t("visualPlan.noKeyframes")}</p>
                </div>
                <span className="shrink-0 rounded-full bg-book-bg-tertiary px-2 py-1 font-mono text-[10px] text-book-text-muted">{plan.frame.width} × {plan.frame.height}</span>
              </div>
              <VisualFrame
                plan={plan}
                keyframe={selectedKeyframe}
                entityById={entityById}
                selectedEntityId={selectedEntityId}
                editable={editable}
                onSelectEntity={handleEntityClick}
                onClearSelection={() => setSelectedEntityId(null)}
                onTransformEntity={transformEntity}
                t={t}
              />
            </section>

            <Timeline
            plan={plan}
            shots={shotRefs}
            selectedShot={selectedShot}
            selectedKeyframeId={selectedKeyframe?.id ?? null}
            playheadMs={playheadMs}
              onSeek={seekPlayhead}
              onSelectShot={selectShot}
              onSelectKeyframe={selectKeyframe}
              t={t}
            />

          </>}
        </main>

      </div>

      <Modal
        isOpen={modal === "sceneLibrary"}
        onClose={dismissModal}
        title={t("visualPlan.scenes")}
        subtitle={selectedScene?.title}
        icon={<Layers3 size={15} />}
        maxWidthClassName="max-w-xl"
        footer={<div className="flex flex-wrap gap-2"><BookButton size="sm" variant="secondary" onClick={dismissModal}>{t("common.close")}</BookButton><BookButton size="sm" onClick={addScene} disabled={!editable}><Plus size={13} />{t("visualPlan.addScene")}</BookButton></div>}
      >
        <SceneNavigation
          plan={plan}
          selectedSceneId={selectedScene?.id ?? null}
          selectedShotId={selectedShot?.id ?? null}
          shotRefs={shotRefs}
          editable={editable}
          onAddShot={addShot}
          onMoveScene={moveScene}
          onDeleteScene={(sceneId) => { void deleteScene(sceneId); }}
          onDeleteShot={(shotId) => { void deleteShot(shotId); }}
          onSelectScene={(id) => { selectScene(id); dismissModal(); }}
          onSelectShot={(ref) => { selectShot(ref); dismissModal(); }}
          onEditScene={openSceneEditor}
          onEditShot={openShotEditor}
          t={t}
        />
      </Modal>

      <Modal
        isOpen={modal === "summary"}
        onClose={dismissModal}
        title={t("visualPlan.planSummary")}
        subtitle={selectedShot?.title}
        icon={<Box size={15} />}
        maxWidthClassName="max-w-xl"
        footer={<div className="flex flex-wrap gap-2"><BookButton size="sm" variant="secondary" onClick={dismissModal}>{t("common.close")}</BookButton></div>}
      >
        <SummaryPanel plan={plan} selectedScene={selectedScene} selectedShot={selectedShot} totalDuration={totalDuration} entityById={entityById} editable={editable} strictIssues={strictIssues} onOpenFrame={openFrameEditor} onOpenEntities={() => openModal("entityLibrary")} onOpenShot={() => openShotEditor()} onDeleteShot={() => void deleteShot()} t={t} />
      </Modal>

      <Modal isOpen={modal === "frame" && frameDraft != null} onClose={requestModalClose} title={t("visualPlan.frameSettings")} icon={<Grid2X2 size={15} />} footer={<ModalActions onCancel={requestModalClose} onSave={saveFrame} disabled={!editable} t={t} />}>
        {frameDraft ? <div className="grid gap-3 sm:grid-cols-2"><Field label={t("visualPlan.frameWidth")}><TextInput type="number" min={1} value={numericInputValue(frameDraft.width)} disabled={!editable} onChange={(event) => setFrameDraft({ ...frameDraft, width: parseNumericInput(event.currentTarget.value) })} /></Field><Field label={t("visualPlan.frameHeight")}><TextInput type="number" min={1} value={numericInputValue(frameDraft.height)} disabled={!editable} onChange={(event) => setFrameDraft({ ...frameDraft, height: parseNumericInput(event.currentTarget.value) })} /></Field><Field label={t("visualPlan.background")} className="sm:col-span-2"><TextArea value={frameDraft.background} disabled={!editable} onChange={(event) => setFrameDraft({ ...frameDraft, background: event.currentTarget.value })} /></Field></div> : null}
      </Modal>

      <Modal isOpen={modal === "scene" && sceneDraft != null} onClose={requestModalClose} title={t("visualPlan.editScene")} icon={<Layers3 size={15} />} footer={<ModalActions onCancel={requestModalClose} onSave={saveScene} onDelete={() => { if (sceneDraft) void deleteScene(sceneDraft.id); }} deleteDisabled={!editable || !sceneDraft} disabled={!editable || !sceneDraft?.title.trim() || !sceneDraft?.narrative_purpose.trim()} t={t} />}>
        {sceneDraft ? <div className="space-y-3"><div className="grid gap-3 sm:grid-cols-2"><Field label={t("visualPlan.sceneTitle")}><TextInput value={sceneDraft.title} disabled={!editable} onChange={(event) => setSceneDraft({ ...sceneDraft, title: event.currentTarget.value })} /></Field><SelectField label={t("visualPlan.editingMode")} value={sceneDraft.editing_mode} disabled={!editable} options={optionItems(EDITING_MODE_OPTIONS, t)} onChange={(value) => { const editingMode = value as ShootingEditingModeV1; setSceneDraft({ ...sceneDraft, editing_mode: editingMode, intercut_group_id: editingMode === "intercut" || editingMode === "parallel_action" ? sceneDraft.intercut_group_id : null }); }} /><Field label={t("visualPlan.sceneLocation")}><TextInput value={toText(sceneDraft.location)} disabled={!editable} onChange={(event) => setSceneDraft({ ...sceneDraft, location: event.currentTarget.value })} /></Field><Field label={t("visualPlan.timeOfDay")}><TextInput value={toText(sceneDraft.time_of_day)} disabled={!editable} onChange={(event) => setSceneDraft({ ...sceneDraft, time_of_day: event.currentTarget.value })} /></Field><Field label={t("visualPlan.intercutGroup")} className="sm:col-span-2"><TextInput value={toText(sceneDraft.intercut_group_id)} disabled={!editable || (sceneDraft.editing_mode !== "intercut" && sceneDraft.editing_mode !== "parallel_action")} onChange={(event) => setSceneDraft({ ...sceneDraft, intercut_group_id: event.currentTarget.value })} /></Field><Field label={t("visualPlan.scenePurpose")} className="sm:col-span-2"><TextArea value={sceneDraft.narrative_purpose} disabled={!editable} onChange={(event) => setSceneDraft({ ...sceneDraft, narrative_purpose: event.currentTarget.value })} /></Field></div><fieldset><legend className="mb-1 text-caption font-semibold text-book-text-muted">{t("visualPlan.participants")}</legend><div className="grid gap-1 sm:grid-cols-2">{plan.entities.map((entity) => <label key={entity.id} className="flex min-w-0 items-center gap-2 rounded-lg border border-book-border/55 bg-book-bg-paper px-2 py-2 text-caption text-book-text-sub"><input type="checkbox" checked={sceneDraft.participant_entity_ids.includes(entity.id)} disabled={!editable} onChange={(event) => setSceneDraft({ ...sceneDraft, participant_entity_ids: event.currentTarget.checked ? [...sceneDraft.participant_entity_ids, entity.id] : sceneDraft.participant_entity_ids.filter((id) => id !== entity.id) })} /><span className="min-w-0 truncate">{entity.label}</span></label>)}</div></fieldset><SelectField label={t("visualPlan.focalEntity")} value={sceneDraft.focal_entity_id ?? ""} disabled={!editable} options={[{ value: "", label: t("common.none") }, ...plan.entities.filter((entity) => sceneDraft.participant_entity_ids.includes(entity.id)).map((entity) => ({ value: entity.id, label: entity.label }))]} onChange={(value) => setSceneDraft({ ...sceneDraft, focal_entity_id: value || null })} /></div> : null}
      </Modal>

      <Modal isOpen={modal === "shot" && shotDraft != null} onClose={requestModalClose} title={t("visualPlan.shotDetails")} subtitle={shotDraft?.shot.title ?? selectedShot?.title} icon={<Film size={15} />} maxWidthClassName="max-w-2xl" footer={<ModalActions onCancel={requestModalClose} onSave={saveShot} onDelete={() => { if (shotDraft) void deleteShot(shotDraft.shot.id); }} deleteDisabled={!editable || !shotDraft} disabled={!editable || !shotDraft?.shot.title.trim() || !shotDraft?.shot.narrative_purpose.trim()} t={t} />}>
        {shotDraft ? <div className="space-y-3"><div className="grid gap-3 sm:grid-cols-2"><Field label={t("visualPlan.titleLabel")}><TextInput value={shotDraft.shot.title} disabled={!editable} onChange={(event) => setShotDraft({ ...shotDraft, shot: { ...shotDraft.shot, title: event.currentTarget.value } })} /></Field><Field label={t("visualPlan.durationRelative")}><TextInput type="number" min={0.25} step={0.05} value={numericInputValue(shotDraft.shot.duration_ms, 1000)} disabled={!editable} onChange={(event) => setShotDraft({ ...shotDraft, shot: { ...shotDraft.shot, duration_ms: parseNumericInput(event.currentTarget.value) * 1000 } })} /></Field><Field label={t("visualPlan.narrativePurpose")} className="sm:col-span-2"><TextArea value={shotDraft.shot.narrative_purpose} disabled={!editable} onChange={(event) => setShotDraft({ ...shotDraft, shot: { ...shotDraft.shot, narrative_purpose: event.currentTarget.value } })} /></Field><SelectField label={t("visualPlan.coverage")} value={shotDraft.shot.coverage} disabled={!editable} options={optionItems(COVERAGE_OPTIONS, t)} onChange={(value) => setShotDraft({ ...shotDraft, shot: { ...shotDraft.shot, coverage: value as ShootingCoverageV1 } })} /><SelectField label={t("visualPlan.shotSize")} value={shotDraft.shot.shot_size ?? ""} disabled={!editable} options={[{ value: "", label: t("common.none") }, ...optionItems(SHOT_SIZE_OPTIONS, t)]} onChange={(value) => setShotDraft({ ...shotDraft, shot: { ...shotDraft.shot, shot_size: (value || null) as ShootingShotSizeV1 | null } })} /><SelectField label={t("visualPlan.cameraAngle")} value={shotDraft.shot.camera_angle ?? ""} disabled={!editable} options={[{ value: "", label: t("common.none") }, ...optionItems(CAMERA_ANGLE_OPTIONS, t)]} onChange={(value) => setShotDraft({ ...shotDraft, shot: { ...shotDraft.shot, camera_angle: (value || null) as ShootingCameraAngleV1 | null } })} /><SelectField label={t("visualPlan.cameraHeight")} value={shotDraft.shot.camera_height ?? ""} disabled={!editable} options={[{ value: "", label: t("common.none") }, ...optionItems(CAMERA_HEIGHT_OPTIONS, t)]} onChange={(value) => setShotDraft({ ...shotDraft, shot: { ...shotDraft.shot, camera_height: (value || null) as ShootingCameraHeightV1 | null } })} /><Field label={t("visualPlan.lensIntent")}><TextInput value={toText(shotDraft.shot.lens_intent)} disabled={!editable} onChange={(event) => setShotDraft({ ...shotDraft, shot: { ...shotDraft.shot, lens_intent: event.currentTarget.value } })} /></Field><Field label={t("visualPlan.composition")}><TextInput value={toText(shotDraft.shot.composition)} disabled={!editable} onChange={(event) => setShotDraft({ ...shotDraft, shot: { ...shotDraft.shot, composition: event.currentTarget.value } })} /></Field><Field label={t("visualPlan.continuityNotes")} className="sm:col-span-2"><TextArea value={shotDraft.continuityNotes} disabled={!editable} onChange={(event) => setShotDraft({ ...shotDraft, continuityNotes: event.currentTarget.value })} /></Field></div><div className="grid gap-3 border-t border-book-border/55 pt-3 sm:grid-cols-2"><SelectField label={t("visualPlan.transitionType")} value={shotDraft.transitionKind} disabled={!editable || !shotHasNext} options={optionItems(TRANSITION_OPTIONS, t)} onChange={(value) => setShotDraft({ ...shotDraft, transitionKind: value as TransitionOption })} /><Field label={t("visualPlan.transitionDuration")}><TextInput type="number" min={0} value={numericInputValue(shotDraft.transitionDurationMs)} disabled={!editable || shotDraft.transitionKind === "none" || shotDraft.transitionKind === "continuous"} onChange={(event) => setShotDraft({ ...shotDraft, transitionDurationMs: parseNumericInput(event.currentTarget.value) })} /></Field><Field label={t("visualPlan.transitionPreservedAction")}><TextInput value={shotDraft.transitionPreservedAction} disabled={!editable || shotDraft.transitionKind === "none" || !shotHasNext} onChange={(event) => setShotDraft({ ...shotDraft, transitionPreservedAction: event.currentTarget.value })} /></Field><Field label={t("visualPlan.transitionPreservedSound")}><TextInput value={shotDraft.transitionPreservedSound} disabled={!editable || shotDraft.transitionKind === "none" || !shotHasNext} onChange={(event) => setShotDraft({ ...shotDraft, transitionPreservedSound: event.currentTarget.value })} /></Field><Field label={t("visualPlan.transitionNotes")} className="sm:col-span-2"><TextArea value={shotDraft.transitionNotes} disabled={!editable || shotDraft.transitionKind === "none" || !shotHasNext} onChange={(event) => setShotDraft({ ...shotDraft, transitionNotes: event.currentTarget.value })} /></Field></div></div> : null}
      </Modal>

      <Modal isOpen={modal === "keyframe" && keyframeDraft != null} onClose={requestModalClose} title={t("visualPlan.editKeyframe")} icon={<CircleDot size={15} />} footer={<ModalActions onCancel={requestModalClose} onSave={saveKeyframe} onDelete={() => { if (keyframeDraft) void removeKeyframe(keyframeDraft.keyframe.id, keyframeDraft.shotId); }} deleteDisabled={!editable || !keyframeDraft || (pendingCreate?.kind !== "keyframe" && !canDeleteDraftKeyframe)} disabled={!editable || !keyframeDraft?.keyframe.label.trim() || !keyframeDraftTimeValid} t={t} />}>
        {keyframeDraft ? <div className="space-y-3"><div className="grid gap-3 sm:grid-cols-2"><Field label={t("visualPlan.keyframe")}><TextInput value={keyframeDraft.keyframe.label} disabled={!editable} onChange={(event) => setKeyframeDraft({ ...keyframeDraft, keyframe: { ...keyframeDraft.keyframe, label: event.currentTarget.value } })} /></Field><Field label={t("visualPlan.keyframeTime")}><TextInput type="number" min={0} step={0.05} value={numericInputValue(keyframeDraft.relativeTimeMs, 1000)} disabled={!editable} onChange={(event) => setKeyframeDraft({ ...keyframeDraft, relativeTimeMs: parseNumericInput(event.currentTarget.value) * 1000 })} /></Field><SelectField label={t("visualPlan.interpolation")} value={keyframeDraft.keyframe.interpolation_to_next} disabled={!editable} options={optionItems(INTERPOLATION_OPTIONS, t)} onChange={(value) => setKeyframeDraft({ ...keyframeDraft, keyframe: { ...keyframeDraft.keyframe, interpolation_to_next: value as VisualInterpolationV1 } })} /><Field label={t("visualPlan.frameDescription")}><TextInput value={toText(keyframeDraft.keyframe.camera.framing_note)} disabled={!editable} onChange={(event) => setKeyframeDraft({ ...keyframeDraft, keyframe: { ...keyframeDraft.keyframe, camera: { ...keyframeDraft.keyframe.camera, framing_note: event.currentTarget.value } } })} /></Field><Field label={t("visualPlan.cameraCenterX")}><TextInput type="number" min={0} max={1} step={0.01} value={numericInputValue(keyframeDraft.keyframe.camera.center_x)} disabled={!editable} onChange={(event) => setKeyframeDraft({ ...keyframeDraft, keyframe: { ...keyframeDraft.keyframe, camera: { ...keyframeDraft.keyframe.camera, center_x: parseNumericInput(event.currentTarget.value) } } })} /></Field><Field label={t("visualPlan.cameraCenterY")}><TextInput type="number" min={0} max={1} step={0.01} value={numericInputValue(keyframeDraft.keyframe.camera.center_y)} disabled={!editable} onChange={(event) => setKeyframeDraft({ ...keyframeDraft, keyframe: { ...keyframeDraft.keyframe, camera: { ...keyframeDraft.keyframe.camera, center_y: parseNumericInput(event.currentTarget.value) } } })} /></Field><Field label={t("visualPlan.zoom")}><TextInput type="number" min={0.01} step={0.05} value={numericInputValue(keyframeDraft.keyframe.camera.zoom)} disabled={!editable} onChange={(event) => setKeyframeDraft({ ...keyframeDraft, keyframe: { ...keyframeDraft.keyframe, camera: { ...keyframeDraft.keyframe.camera, zoom: parseNumericInput(event.currentTarget.value) } } })} /></Field><SelectField label={t("visualPlan.focusEntity")} value={keyframeDraft.keyframe.camera.focus_entity_id ?? ""} disabled={!editable} options={[{ value: "", label: t("common.none") }, ...keyframeDraft.keyframe.entity_states.map((state) => ({ value: state.entity_id, label: entityById.get(state.entity_id)?.label ?? state.entity_id }))]} onChange={(value) => setKeyframeDraft({ ...keyframeDraft, keyframe: { ...keyframeDraft.keyframe, camera: { ...keyframeDraft.keyframe.camera, focus_entity_id: value || null } } })} /></div></div> : null}
        {keyframeDraft ? <Field label={t("visualPlan.cameraRotation")} className="mt-3"><TextInput type="number" step={1} value={numericInputValue(keyframeDraft.keyframe.camera.rotation_degrees)} disabled={!editable} onChange={(event) => setKeyframeDraft({ ...keyframeDraft, keyframe: { ...keyframeDraft.keyframe, camera: { ...keyframeDraft.keyframe.camera, rotation_degrees: parseNumericInput(event.currentTarget.value) } } })} /></Field> : null}
      </Modal>

      <Modal
        isOpen={modal === "keyframeLibrary"}
        onClose={dismissModal}
        title={t("visualPlan.keyframes")}
        subtitle={selectedShot?.title}
        icon={<CircleDot size={15} />}
        maxWidthClassName="max-w-xl"
        footer={<div className="flex flex-wrap gap-2"><BookButton size="sm" variant="secondary" onClick={dismissModal}>{t("common.close")}</BookButton></div>}
      >
        <div className="space-y-3">
          <div className="flex min-w-0 flex-wrap items-center justify-between gap-2">
            <div className="min-w-0">
              <p className="text-caption font-semibold text-book-text-muted">{selectedKeyframe ? `${selectedKeyframe.label} · ${formatPlanTime(selectedKeyframe.at_ms)}` : t("visualPlan.noKeyframes")}</p>
            </div>
            <div className="flex shrink-0 gap-1.5">
              <BookButton size="sm" variant="secondary" disabled={!canAddKeyframe || !editable} onClick={addKeyframe}><Plus size={13} />{t("visualPlan.addKeyframe")}</BookButton>
              <BookIconButton label={t("visualPlan.editKeyframe")} size="sm" disabled={!selectedKeyframe || !editable} onClick={() => openKeyframeEditor()}><Settings2 size={14} /></BookIconButton>
              <BookIconButton label={t("visualPlan.deleteKeyframe")} size="sm" disabled={!selectedKeyframe || !editable || !canDeleteKeyframe} onClick={() => void removeKeyframe()}><Trash2 size={14} /></BookIconButton>
            </div>
          </div>
          <div className="space-y-1.5">
            {(selectedShot?.keyframes ?? []).map((keyframe) => (
              <div
                key={keyframe.id}
                className={`flex min-h-11 w-full min-w-0 items-center gap-2 rounded-lg border px-2.5 py-2 transition-colors ${selectedKeyframe?.id === keyframe.id ? "border-book-primary/60 bg-book-primary/10" : "border-book-border/60 bg-book-bg-paper"}`}
              >
                <button
                  type="button"
                  aria-pressed={selectedKeyframe?.id === keyframe.id}
                  onClick={() => { if (selectedRef) selectKeyframe(keyframe, selectedRef); }}
                  className="min-w-0 flex-1 text-left hover:text-book-primary focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary"
                >
                  <span className="block truncate text-caption font-semibold text-book-text-main">{keyframe.label}</span>
                  <span className="block font-mono text-[10px] text-book-text-muted">{formatPlanTime(keyframe.at_ms)}</span>
                </button>
                <div className="flex shrink-0 items-center gap-0.5">
                  <BookIconButton label={t("visualPlan.editKeyframe")} size="sm" disabled={!editable || !selectedRef} onClick={() => { if (selectedRef) openKeyframeEditor(keyframe, selectedRef); }}>
                    <Settings2 size={13} />
                  </BookIconButton>
                  <BookIconButton label={t("visualPlan.deleteKeyframe")} size="sm" disabled={!editable || !selectedRef || (selectedShot?.keyframes?.length ?? 0) <= 2 || keyframe.id === selectedShot?.keyframes?.[0]?.id || keyframe.id === selectedShot?.keyframes?.at(-1)?.id} onClick={() => { if (selectedRef) { selectKeyframe(keyframe, selectedRef); void removeKeyframe(keyframe.id); } }}>
                    <Trash2 size={13} />
                  </BookIconButton>
                </div>
              </div>
            ))}
            {selectedShot?.keyframes.length === 0 ? <p className="py-3 text-caption text-book-text-muted">{t("visualPlan.noKeyframes")}</p> : null}
          </div>
        </div>
      </Modal>

      <Modal isOpen={modal === "entityLibrary"} onClose={dismissModal} title={t("visualPlan.entities")} icon={<Box size={15} />} maxWidthClassName="max-w-xl" footer={<div className="flex flex-wrap gap-2"><BookButton size="sm" variant="secondary" onClick={dismissModal}>{t("common.close")}</BookButton><BookButton size="sm" onClick={openNewEntity} disabled={!editable}><Plus size={13} />{t("visualPlan.add")}</BookButton></div>}>
        <div className="space-y-2">{plan.entities.map((entity) => { const inFrame = activeEntityStates.some((state) => state.entity_id === entity.id); return <article key={entity.id} className="flex min-w-0 items-center gap-2 rounded-lg border border-book-border/60 bg-book-bg-paper p-2"><button type="button" className="min-w-0 flex-1 text-left" onClick={() => openEntityLibrary(entity)}><span className="block truncate text-small font-semibold text-book-text-main">{entity.label}</span><span className="block truncate text-caption text-book-text-muted">{entity.description || t(ENTITY_KIND_OPTIONS.find(([kind]) => kind === entity.kind)?.[1] ?? "visualPlan.text")}</span></button>{selectedKeyframe && !inFrame ? <BookIconButton label={t("visualPlan.addExistingEntity")} size="sm" disabled={!editable} onClick={() => addExistingEntity(entity.id)}><Plus size={13} /></BookIconButton> : null}{selectedKeyframe && inFrame ? <BookIconButton label={t("visualPlan.removeEntityFromFrame")} size="sm" disabled={!editable} onClick={() => removeEntityFromFrame(entity.id)}><X size={13} /></BookIconButton> : null}<BookIconButton label={t("visualPlan.deleteEntity")} size="sm" disabled={!editable} onClick={() => { void deleteEntity(entity.id); }}><Trash2 size={13} /></BookIconButton></article>; })}{plan.entities.length === 0 ? <p className="py-4 text-center text-caption text-book-text-muted">{t("visualPlan.noEntitiesInKeyframe")}</p> : null}</div>
      </Modal>

      <Modal isOpen={modal === "entity" && entityLibraryDraft != null} onClose={requestModalClose} title={entityLibraryDraft?.id ? t("visualPlan.editEntity") : t("visualPlan.addEntity", { kind: t("visualPlan.entity") })} icon={<Box size={15} />} footer={<ModalActions onCancel={requestModalClose} onSave={saveEntityLibrary} onDelete={entityLibraryDraft?.id ? () => { void deleteEntity(entityLibraryDraft.id); } : undefined} deleteDisabled={!editable || !entityLibraryDraft?.id} disabled={!editable || !entityLibraryDraft?.label.trim() || !entityLibraryDraft?.description.trim()} t={t} />}>
        {entityLibraryDraft ? <div className="grid gap-3 sm:grid-cols-2"><Field label={t("visualPlan.entityNamePlaceholder")}><TextInput value={entityLibraryDraft.label} disabled={!editable} onChange={(event) => setEntityLibraryDraft({ ...entityLibraryDraft, label: event.currentTarget.value })} /></Field><SelectField label={t("visualPlan.newEntityType")} value={entityLibraryDraft.kind} disabled={!editable} options={optionItems(ENTITY_KIND_OPTIONS, t)} onChange={(value) => setEntityLibraryDraft({ ...entityLibraryDraft, kind: value as VisualEntityKindV1 })} /><Field label={t("visualPlan.entityDetails")} className="sm:col-span-2"><TextArea value={entityLibraryDraft.description} disabled={!editable} onChange={(event) => setEntityLibraryDraft({ ...entityLibraryDraft, description: event.currentTarget.value })} /></Field></div> : null}
      </Modal>

      <Modal isOpen={modal === "entityState" && entityStateDraft != null} onClose={requestModalClose} title={t("visualPlan.entityState")} subtitle={entityById.get(entityStateDraft?.entityId ?? "")?.label} icon={<Box size={15} />} maxWidthClassName="max-w-2xl" bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4 pb-8 sm:px-5" footer={<ModalActions onCancel={requestModalClose} onSave={saveEntityState} disabled={!editable} t={t} />}>
        {entityStateDraft ? <div className="a4n-visual-plan__entity-state-form space-y-3"><label className="flex items-center gap-2 text-small text-book-text-sub"><input type="checkbox" checked={entityStateDraft.visible} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, visible: event.currentTarget.checked })} />{t("visualPlan.visibleInFrame")}</label><div className="a4n-visual-plan__entity-state-fields grid grid-cols-2 gap-3"><Field label={t("visualPlan.action")}><TextInput value={entityStateDraft.action} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, action: event.currentTarget.value })} /></Field><Field label={t("visualPlan.expression")}><TextInput value={entityStateDraft.expression} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, expression: event.currentTarget.value })} /></Field><Field label={t("visualPlan.pose")}><TextInput value={entityStateDraft.pose} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, pose: event.currentTarget.value })} /></Field><Field label={t("visualPlan.dialogue")} className="col-span-2"><TextArea value={entityStateDraft.dialogue} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, dialogue: event.currentTarget.value })} /></Field><Field label={t("visualPlan.left")}><TextInput type="number" min={0} max={100} value={numericInputValue(entityStateDraft.x)} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, x: parseNumericInput(event.currentTarget.value) })} /></Field><Field label={t("visualPlan.top")}><TextInput type="number" min={0} max={100} value={numericInputValue(entityStateDraft.y)} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, y: parseNumericInput(event.currentTarget.value) })} /></Field><Field label={t("visualPlan.width")}><TextInput type="number" min={1} max={100} value={numericInputValue(entityStateDraft.width)} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, width: parseNumericInput(event.currentTarget.value) })} /></Field><Field label={t("visualPlan.height")}><TextInput type="number" min={1} max={100} value={numericInputValue(entityStateDraft.height)} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, height: parseNumericInput(event.currentTarget.value) })} /></Field><Field label={t("visualPlan.opacity")}><TextInput type="number" min={0} max={100} value={numericInputValue(entityStateDraft.opacity)} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, opacity: parseNumericInput(event.currentTarget.value) })} /></Field><Field label={t("visualPlan.rotation")}><TextInput type="number" value={numericInputValue(entityStateDraft.rotation)} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, rotation: parseNumericInput(event.currentTarget.value) })} /></Field><Field label={t("visualPlan.zIndex")}><TextInput type="number" step={1} value={numericInputValue(entityStateDraft.zIndex)} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, zIndex: parseNumericInput(event.currentTarget.value) })} /></Field><SelectField label={t("visualPlan.facing")} value={entityStateDraft.facing} disabled={!editable} options={optionItems(FACING_OPTIONS, t)} onChange={(value) => setEntityStateDraft({ ...entityStateDraft, facing: value as VisualFacingV1 })} /><SelectField label={t("visualPlan.gazeTarget")} value={entityStateDraft.gazeTarget} disabled={!editable} options={[{ value: "", label: t("common.none") }, ...(entityStateDraftKeyframe?.entity_states ?? []).filter((state) => state.entity_id !== entityStateDraft.entityId).map((state) => ({ value: state.entity_id, label: entityById.get(state.entity_id)?.label ?? state.entity_id }))]} onChange={(value) => setEntityStateDraft({ ...entityStateDraft, gazeTarget: value })} /><Field label={t("visualPlan.frameDescription")} className="col-span-2"><TextArea value={entityStateDraft.notes} disabled={!editable} onChange={(event) => setEntityStateDraft({ ...entityStateDraft, notes: event.currentTarget.value })} /></Field></div><div className="border-t border-book-border/55 pt-3"><h4 className="text-small font-semibold text-book-text-main">{t("visualPlan.positionRelations")}</h4><div className="mt-2 grid gap-2 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto]"><ThemedSelect aria-label={t("visualPlan.relationType")} value={relationKind} className="pyqt-input w-full" disabled={!editable} onChange={(event) => setRelationKind(event.currentTarget.value as VisualRelationKindV1)}>{optionItems(RELATION_OPTIONS, t).map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</ThemedSelect><ThemedSelect aria-label={t("visualPlan.relationTarget")} value={relationTarget} className="pyqt-input w-full" disabled={!editable} onChange={(event) => setRelationTarget(event.currentTarget.value)}><option value="">{t("visualPlan.selectTarget")}</option>{(entityStateDraftKeyframe?.entity_states ?? []).filter((state) => state.entity_id !== entityStateDraft.entityId && state.visible).map((state) => <option key={state.entity_id} value={state.entity_id}>{entityById.get(state.entity_id)?.label ?? state.entity_id}</option>)}</ThemedSelect><TextInput aria-label={t("visualPlan.relationDescription")} value={relationLabel} disabled={!editable} placeholder={t("visualPlan.relationLabelPlaceholder")} onChange={(event) => setRelationLabel(event.currentTarget.value)} /><BookIconButton label={t(relationEditingId ? "visualPlan.saveRelation" : "visualPlan.addRelation")} size="sm" disabled={!editable || !relationTarget} onClick={addRelation}>{relationEditingId ? <Save size={14} /> : <Plus size={14} />}</BookIconButton>{relationEditingId ? <BookIconButton label={t("visualPlan.cancelRelationEdit")} size="sm" disabled={!editable} onClick={cancelRelationEdit}><X size={14} /></BookIconButton> : null}</div><div className="mt-2 space-y-1">{(entityStateDraftKeyframe?.relations ?? []).map((relation) => <div key={relation.id} className="flex min-w-0 items-center gap-2 rounded-lg bg-book-bg-tertiary/60 px-2 py-1.5"><span className="min-w-0 flex-1 truncate text-caption text-book-text-sub">{entityById.get(relation.source_entity_id)?.label ?? relation.source_entity_id} → {entityById.get(relation.target_entity_id)?.label ?? relation.target_entity_id} · {relation.label ?? t(RELATION_OPTIONS.find(([kind]) => kind === relation.kind)?.[1] ?? "visualPlan.relationCustom")}</span><BookIconButton label={t("visualPlan.editRelation")} size="sm" disabled={!editable} onClick={() => editRelation(relation)}><Settings2 size={12} /></BookIconButton><BookIconButton label={t("common.delete")} size="sm" disabled={!editable} onClick={() => { void deleteRelation(relation.id); }}><Trash2 size={12} /></BookIconButton></div>)}</div></div></div> : null}
      </Modal>

      <Modal
        isOpen={modal === "eventDetails"}
        onClose={dismissModal}
        title={t("visualPlan.eventDetails")}
        subtitle={selectedShot?.title}
        icon={<CircleDot size={15} />}
        maxWidthClassName="max-w-3xl"
        footer={<div className="flex flex-wrap gap-2"><BookButton size="sm" variant="secondary" onClick={dismissModal}>{t("common.close")}</BookButton></div>}
      >
        <div className="space-y-3">
          <div className="flex min-w-0 flex-wrap items-center justify-between gap-2">
            <div className="min-w-0">
              <p className="text-caption font-semibold text-book-text-muted">{selectedShot?.title ?? t("visualPlan.noShot")}</p>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {(["entity", "camera", "sound", "technique"] as EventCategory[]).map((category) => <BookButton key={category} size="sm" variant="secondary" disabled={!selectedShot || !editable || (category === "entity" && activeVisibleEntityStates.length === 0)} onClick={() => openNewEvent(category)}>{category === "sound" ? <Volume2 size={13} /> : <Plus size={13} />}{t(category === "entity" ? "visualPlan.entityEvents" : category === "camera" ? "visualPlan.cameraEvents" : category === "sound" ? "visualPlan.soundEvents" : "visualPlan.techniques")}</BookButton>)}
            </div>
          </div>
          <div className="grid min-w-0 gap-2 md:grid-cols-2">
            {activeEvents.map(([category, label, events]) => <EventList key={category} category={category} label={t(label)} events={events} editable={editable} onEdit={openEditEvent} onDelete={deleteEvent} onRangeChange={(eventId, boundary, value) => {
              if (!selectedRef || !selectedShot) return false;
              const event = activeEvents.flatMap(([, , items]) => items).find((item) => item.id === eventId);
              const rounded = Math.round(value);
              const valid = event != null
                && Number.isFinite(rounded)
                && rounded >= 0
                && rounded <= selectedShot.duration_ms
                && (boundary === "start" ? rounded < event.end_ms : rounded > event.start_ms);
              if (!valid) {
                addToast(t("visualPlan.validation.eventOutOfShot", { label: selectedShot.title || t("visualPlan.shot") }), "error");
                return false;
              }
              return Boolean(commit((next) => {
                updateVisualPlanTimedEventRange(next, selectedRef.shot.id, eventId, boundary, rounded);
              }));
            }} t={t} />)}
          </div>
          {selectedShot && activeEvents.every(([, , events]) => events.length === 0) ? <p className="text-caption text-book-text-muted">{t("visualPlan.noEvents")}</p> : null}
        </div>
      </Modal>

      <Modal isOpen={modal === "event" && eventDraft != null} onClose={requestModalClose} title={eventDraft?.id ? t("visualPlan.editTimedEvent") : t("visualPlan.addTimedEvent")} subtitle={eventDraftShot?.title} icon={<CircleDot size={15} />} footer={<ModalActions onCancel={requestModalClose} onSave={saveEvent} disabled={!editable || !eventDraft?.text.trim()} t={t} />}>
        {eventDraft ? <div className="grid gap-3 sm:grid-cols-2"><SelectField label={t("visualPlan.eventCategory")} value={eventDraft.category} disabled={!editable || eventDraft.id != null} options={[{ value: "entity", label: t("visualPlan.entityEvents") }, { value: "camera", label: t("visualPlan.cameraEvents") }, { value: "sound", label: t("visualPlan.soundEvents") }, { value: "technique", label: t("visualPlan.techniques") }]} onChange={(value) => setEventDraft({ ...eventDraft, category: value as EventCategory, kind: defaultEventKind(value as EventCategory) })} /><SelectField label={t("visualPlan.eventKind")} value={eventDraft.kind} disabled={!editable} options={optionItems(eventOptions(eventDraft.category), t)} onChange={(value) => setEventDraft({ ...eventDraft, kind: value })} /><Field label={t("visualPlan.eventStart")}><TextInput type="number" min={0} step={0.05} value={numericInputValue(eventDraft.startMs, 1000)} disabled={!editable} onChange={(event) => setEventDraft({ ...eventDraft, startMs: parseNumericInput(event.currentTarget.value) * 1000 })} /></Field><Field label={t("visualPlan.eventEnd")}><TextInput type="number" min={0.001} step={0.05} value={numericInputValue(eventDraft.endMs, 1000)} disabled={!editable} onChange={(event) => setEventDraft({ ...eventDraft, endMs: parseNumericInput(event.currentTarget.value) * 1000 })} /></Field>{eventDraft.category === "entity" ? <SelectField label={t("visualPlan.entityState")} value={eventDraft.entityId} disabled={!editable} options={eventEntityOptions} onChange={(value) => setEventDraft({ ...eventDraft, entityId: value })} /> : null}{eventDraft.category === "entity" || eventDraft.category === "camera" ? <SelectField label={t("visualPlan.eventTarget")} value={eventDraft.targetEntityId} disabled={!editable} options={[{ value: "", label: t("common.none") }, ...eventEntityOptions]} onChange={(value) => setEventDraft({ ...eventDraft, targetEntityId: value })} /> : null}{eventDraft.category === "sound" ? <SelectField label={t("visualPlan.eventSpeaker")} value={eventDraft.speakerEntityId} disabled={!editable} options={[{ value: "", label: t("common.none") }, ...eventEntityOptions]} onChange={(value) => setEventDraft({ ...eventDraft, speakerEntityId: value })} /> : null}<Field label={eventDraft.category === "sound" ? t("visualPlan.eventContent") : t("visualPlan.eventDescription")} className="sm:col-span-2"><TextArea value={eventDraft.text} disabled={!editable} placeholder={eventDraft.category === "sound" ? t("visualPlan.soundPlaceholder") : eventDraft.category === "camera" ? t("visualPlan.cameraEventPlaceholder") : eventDraft.category === "technique" ? t("visualPlan.techniquePlaceholder") : t("visualPlan.eventChangePlaceholder", { kind: t("visualPlan.entityEvents") })} onChange={(event) => setEventDraft({ ...eventDraft, text: event.currentTarget.value })} /></Field><Field label={t("visualPlan.eventParameters")} className="sm:col-span-2"><TextArea value={eventDraft.parametersText} disabled={!editable} placeholder="{}" className="font-mono text-caption" onChange={(event) => setEventDraft({ ...eventDraft, parametersText: event.currentTarget.value })} /></Field></div> : null}
      </Modal>

      <Modal isOpen={modal === "source"} onClose={dismissModal} title={t("visualPlan.sourceReference")} icon={<Box size={15} />} maxWidthClassName="max-w-3xl">
        <div className="space-y-3"><div className="rounded-lg border border-book-border/60 bg-book-bg-paper p-3"><p className="text-small font-semibold text-book-text-main">{planSet.source.title || `${t("novel.chapter")} ${planSet.source.chapter_number}`}</p><p className="mt-2 whitespace-pre-wrap break-words text-small leading-7 text-book-text-sub">{planSet.source.content}</p></div><p className="text-caption leading-relaxed text-book-text-muted">{t("visualPlan.sourceReference")}</p></div>
      </Modal>

      <Modal
        isOpen={modal === "director"}
        onClose={dismissModal}
        title={t("agent.domain.directorTitle")}
        icon={<Film size={15} />}
        maxWidthClassName="max-w-3xl"
      >
        <div className="h-[min(42rem,75vh)] min-h-[28rem]">
          <DomainAgentConsole
            capability="director"
            projectId={projectId}
            chapterId={chapter.id}
            resourceLabel={chapter.title || `${t("novel.chapter")} ${chapter.chapter_number}`}
            fill
            onResourceChanged={handleDirectorResourceChanged}
          />
        </div>
      </Modal>
    </section>
  );
};

const ModalActions: React.FC<{
  onCancel: () => void;
  onSave: () => void;
  onDelete?: () => void;
  deleteDisabled?: boolean;
  disabled?: boolean;
  t: (key: MessageKey) => string;
}> = ({ onCancel, onSave, onDelete, deleteDisabled = false, disabled = false, t }) => (
  <div className="flex w-full flex-wrap items-center justify-end gap-2">
    {onDelete ? <BookButton size="sm" variant="danger" className="mr-auto" onClick={onDelete} disabled={deleteDisabled}><Trash2 size={13} />{t("common.delete")}</BookButton> : null}
    <BookButton size="sm" variant="secondary" onClick={onCancel}>{t("common.cancel")}</BookButton>
    <BookButton size="sm" onClick={onSave} disabled={disabled}>{t("common.save")}</BookButton>
  </div>
);

const EmptyWorkspace: React.FC<{ t: (key: MessageKey) => string }> = ({ t }) => (
  <section className="a4n-visual-plan__empty-workspace min-w-0" aria-labelledby="visual-plan-empty-title">
    <div className="a4n-visual-plan__empty-copy">
      <span className="a4n-visual-plan__empty-icon" aria-hidden="true"><Film size={24} /></span>
      <p className="text-caption font-semibold uppercase tracking-[0.16em] text-book-primary">{t("visualPlan.title")}</p>
      <h4 id="visual-plan-empty-title" className="mt-1 font-serif text-title font-semibold text-book-text-main">{t("visualPlan.noScenes")}</h4>
      <p className="mt-2 max-w-md text-small leading-relaxed text-book-text-muted">{t("visualPlan.workspaceHint")}</p>
    </div>
    <div className="a4n-visual-plan__empty-steps" aria-hidden="true">
      {["scene", "shot", "keyframes"].map((step, index) => (
        <div key={step} className="a4n-visual-plan__empty-step">
          <span>{String(index + 1).padStart(2, "0")}</span>
          <p>{t(`visualPlan.${step}` as MessageKey)}</p>
        </div>
      ))}
    </div>
  </section>
);

const SceneNavigation: React.FC<{
  plan: ChapterVisualPlanV2;
  selectedSceneId: string | null;
  selectedShotId: string | null;
  shotRefs: readonly VisualShotRef[];
  editable: boolean;
  onAddShot: () => void;
  onMoveScene: (sceneId: string, direction: -1 | 1) => void;
  onDeleteScene: (sceneId: string) => void;
  onDeleteShot: (shotId: string) => void;
  onSelectScene: (id: string) => void;
  onSelectShot: (ref: VisualShotRef) => void;
  onEditScene: (scene: VisualSceneV2) => void;
  onEditShot: (ref: VisualShotRef) => void;
  t: (key: MessageKey, params?: Record<string, string | number>) => string;
}> = ({ plan, selectedSceneId, selectedShotId, shotRefs, editable, onAddShot, onMoveScene, onDeleteScene, onDeleteShot, onSelectScene, onSelectShot, onEditScene, onEditShot, t }) => (
  <div className="min-w-0">
    <div className="mt-2 max-h-[42rem] space-y-1 overflow-y-auto pr-0.5 custom-scrollbar">
      {plan.scenes.map((scene, sceneIndex) => <div key={scene.id} className="min-w-0"><div className={`flex min-w-0 items-center gap-1 rounded-lg border px-1.5 py-1 ${selectedSceneId === scene.id ? "border-book-primary/45 bg-book-primary/8" : "border-transparent"}`}><button type="button" className="min-w-0 flex-1 text-left" onClick={() => onSelectScene(scene.id)}><span className="block truncate text-caption font-semibold text-book-text-main">{sceneIndex + 1}. {scene.title || t("visualPlan.scene")}</span><span className="block truncate text-[10px] text-book-text-muted">{t("visualPlan.shotCount", { count: scene.shots.length })}</span></button><div className="flex shrink-0 items-center gap-0.5"><BookIconButton label={t("visualPlan.moveSceneEarlier")} size="sm" disabled={!editable || sceneIndex === 0} onClick={() => onMoveScene(scene.id, -1)}><ChevronUp size={12} /></BookIconButton><BookIconButton label={t("visualPlan.moveSceneLater")} size="sm" disabled={!editable || sceneIndex === plan.scenes.length - 1} onClick={() => onMoveScene(scene.id, 1)}><ChevronDown size={12} /></BookIconButton><BookIconButton label={t("visualPlan.editScene")} size="sm" disabled={!editable} onClick={() => onEditScene(scene)}><Settings2 size={12} /></BookIconButton><BookIconButton label={t("visualPlan.deleteScene")} size="sm" disabled={!editable} onClick={() => onDeleteScene(scene.id)}><Trash2 size={12} /></BookIconButton></div></div>{selectedSceneId === scene.id ? <div className="ml-2 mt-1 space-y-1 border-l border-book-border/60 pl-1.5">{scene.shots.map((shot) => { const ref = shotRefs.find((candidate) => candidate.shot.id === shot.id); return <div key={shot.id} className={`flex min-w-0 items-center gap-1 rounded-md px-1.5 py-1 ${selectedShotId === shot.id ? "bg-book-bg-tertiary" : "hover:bg-book-bg-tertiary/70"}`}><button type="button" className="min-w-0 flex-1 text-left" onClick={() => ref && onSelectShot(ref)}><span className="block truncate text-caption text-book-text-sub">{shot.title || t("visualPlan.shot")}</span><span className="font-mono text-[10px] text-book-text-muted">{formatPlanTime(shot.duration_ms)}</span></button>{ref ? <BookIconButton label={t("visualPlan.editShot")} size="sm" disabled={!editable} onClick={() => onEditShot(ref)}><ChevronRight size={12} /></BookIconButton> : null}<BookIconButton label={t("visualPlan.deleteShot")} size="sm" disabled={!editable} onClick={() => onDeleteShot(shot.id)}><Trash2 size={12} /></BookIconButton></div>; })}{scene.shots.length === 0 ? <p className="px-2 py-2 text-[10px] text-book-text-muted">{t("visualPlan.noShots")}</p> : null}</div> : null}</div>)}
      {plan.scenes.length === 0 ? <p className="px-2 py-4 text-center text-caption text-book-text-muted">{t("visualPlan.noScenes")}</p> : null}
    </div>
    {plan.scenes.length > 0 ? <BookButton size="sm" variant="secondary" className="mt-2 w-full" disabled={!editable || !selectedSceneId} onClick={onAddShot}><Plus size={13} />{t("visualPlan.addShot")}</BookButton> : null}
  </div>
);

const ENTITY_SKETCHES: Record<VisualEntityKindV1, LucideIcon | null> = {
  character: PersonStanding,
  object: Shapes,
  environment: Mountain,
  text: null,
  effect: Flame,
};

const ENTITY_SKETCH_VIEW_BOX: Record<VisualEntityKindV1, string> = {
  character: "5.2 3.2 13.6 17.6",
  object: "2.2 2.2 19.6 19.6",
  environment: "1.2 2.2 21.6 19.6",
  text: "0 0 24 24",
  effect: "3.2 2.2 17.6 20.6",
};

const TEXT_MIN_FONT_SIZE = 14;
const TEXT_MAX_WIDTH_RATIO = 0.68;

const EntitySketch: React.FC<{
  kind: VisualEntityKindV1;
  label: string;
  dialogue?: string | null;
  facing?: VisualFacingV1;
  textFontSize?: number;
  textMaxWidth?: number;
  textExpanded?: boolean;
}> = ({ kind, label, dialogue, facing, textFontSize, textMaxWidth, textExpanded }) => {
  const Sketch = ENTITY_SKETCHES[kind];
  const displayLabel = kind === "text" ? dialogue?.trim() || label : label;

  return (
    <span className={`a4n-visual-plan__entity-art ${kind === "text" ? "a4n-visual-plan__entity-art--text" : ""} ${facing === "left" || facing === "profile_left" ? "a4n-visual-plan__entity-art--mirrored" : ""}`} aria-hidden="true">
      {Sketch ? <Sketch size="100%" strokeWidth={1.6} viewBox={ENTITY_SKETCH_VIEW_BOX[kind]} preserveAspectRatio="none" /> : null}
      {kind === "text" ? <span className="a4n-visual-plan__entity-label" data-text-expanded={textExpanded ? "true" : undefined} style={{
        ...(textFontSize ? { fontSize: `${textFontSize}px` } : {}),
        ...(textMaxWidth ? { maxWidth: `${textMaxWidth}px` } : {}),
      }}>{displayLabel}</span> : null}
    </span>
  );
};

type EntityInteractionMode = "move" | "resize" | "rotate";

const RESIZE_HANDLE_LABEL_KEYS: Record<EntityResizeHandle, MessageKey> = {
  nw: "visualPlan.resizeTopLeft",
  ne: "visualPlan.resizeTopRight",
  se: "visualPlan.resizeBottomRight",
  sw: "visualPlan.resizeBottomLeft",
};

interface EntityInteraction {
  entityId: string;
  state: VisualEntityStateV1;
  pointerId: number;
  mode: EntityInteractionMode;
  handle?: EntityResizeHandle;
  startX: number;
  startY: number;
  frame: { width: number; height: number };
  centerX: number;
  centerY: number;
  startAngle: number;
  initial: EntityTransform;
  current: EntityTransform;
  moved: boolean;
}

const entityTransformAtPointer = (
  interaction: EntityInteraction,
  clientX: number,
  clientY: number,
  snapRotation: boolean,
): EntityTransform => {
  const initial = interaction.initial.bounds;
  const dx = (clientX - interaction.startX) / interaction.frame.width;
  const dy = (clientY - interaction.startY) / interaction.frame.height;

  if (interaction.mode === "move") {
    const rotated = rotatedBoundsSize(initial, interaction.initial.rotation, interaction.frame);
    const centerX = clampFrameValue(
      initial.x + initial.width / 2 + dx,
      rotated.width / 2,
      1 - rotated.width / 2,
    );
    const centerY = clampFrameValue(
      initial.y + initial.height / 2 + dy,
      rotated.height / 2,
      1 - rotated.height / 2,
    );
    return normalizeEntityTransform({
      ...initial,
      x: centerX - initial.width / 2,
      y: centerY - initial.height / 2,
    }, interaction.initial.rotation, defaultRenderBounds, interaction.frame);
  }

  if (interaction.mode === "resize") {
    return resizeEntitySelectionByScreenDelta(
      interaction.initial,
      interaction.handle ?? "se",
      clientX - interaction.startX,
      clientY - interaction.startY,
      interaction.frame,
    );
  }

  const angle = Math.atan2(clientY - interaction.centerY, clientX - interaction.centerX);
  const angleDelta = Math.atan2(
    Math.sin(angle - interaction.startAngle),
    Math.cos(angle - interaction.startAngle),
  );
  const rotation = interaction.initial.rotation + angleDelta * 180 / Math.PI;
  return normalizeEntityTransform(
    initial,
    snapRotation ? Math.round(rotation / 15) * 15 : rotation,
    defaultRenderBounds,
    interaction.frame,
  );
};

const VisualFrame: React.FC<{
  plan: ChapterVisualPlanV2;
  keyframe: VisualKeyframeV2 | null;
  entityById: ReadonlyMap<string, ChapterVisualPlanV2["entities"][number]>;
  selectedEntityId: string | null;
  editable: boolean;
  onSelectEntity: (state: VisualEntityStateV1) => void;
  onClearSelection: () => void;
  onTransformEntity: (entityId: string, transform: EntityTransform) => void;
  t: (key: MessageKey) => string;
}> = ({ plan, keyframe, entityById, selectedEntityId, editable, onSelectEntity, onClearSelection, onTransformEntity, t }) => {
  const visibleStates = keyframe?.entity_states.filter((state) => state.visible) ?? [];
  const frameSize: EntityFrameSize = {
    width: Math.max(1, finiteOr(plan.frame.width, 16)),
    height: Math.max(1, finiteOr(plan.frame.height, 9)),
  };
  const frameRef = useRef<HTMLDivElement | null>(null);
  const interactionRef = useRef<EntityInteraction | null>(null);
  const suppressClickRef = useRef(false);
  const onSelectEntityRef = useRef(onSelectEntity);
  const onTransformEntityRef = useRef(onTransformEntity);
  const [interaction, setInteraction] = useState<EntityInteraction | null>(null);
  const [framePixels, setFramePixels] = useState<EntityFrameSize | null>(null);
  const [textFontSizeByEntityId, setTextFontSizeByEntityId] = useState<Record<string, number>>({});
  const [textVisualBoundsByEntityId, setTextVisualBoundsByEntityId] = useState<Record<string, VisualEntityStateV1["bounds"]>>({});
  const controlFrame = framePixels && framePixels.width > 0 && framePixels.height > 0 ? framePixels : null;
  const geometryFrame = controlFrame ?? frameSize;
  onSelectEntityRef.current = onSelectEntity;
  onTransformEntityRef.current = onTransformEntity;

  useLayoutEffect(() => {
    const frame = frameRef.current;
    if (!frame) return undefined;
    const updateFramePixels = () => {
      const next = { width: frame.clientWidth, height: frame.clientHeight };
      setFramePixels((current) => current?.width === next.width && current.height === next.height ? current : next);
    };
    updateFramePixels();
    if (typeof ResizeObserver === "undefined") return undefined;
    const observer = new ResizeObserver(updateFramePixels);
    observer.observe(frame);
    return () => observer.disconnect();
  }, []);

  useLayoutEffect(() => {
    const frame = frameRef.current;
    if (!frame || frame.clientWidth <= 0 || frame.clientHeight <= 0) return;
    const textEntities = new Map(
      [...frame.querySelectorAll<HTMLElement>(".a4n-visual-plan__entity--text")]
        .map((entity) => [entity.dataset.entityId, entity] as const),
    );
    const next: Record<string, VisualEntityStateV1["bounds"]> = {};
    const nextFontSizes: Record<string, number> = {};
    visibleStates.forEach((state) => {
      if (entityById.get(state.entity_id)?.kind !== "text") return;
      const entity = textEntities.get(state.entity_id);
      const label = entity?.querySelector<HTMLElement>(".a4n-visual-plan__entity-label");
      if (!label || label.offsetWidth <= 0 || label.offsetHeight <= 0) return;
      label.style.removeProperty("font-size");
      const availableHeight = label.parentElement?.clientHeight ?? 0;
      const baseFontSize = finiteOr(Number.parseFloat(getComputedStyle(label).fontSize), 14);
      let fittedFontSize = baseFontSize;
      for (let attempt = 0; attempt < 4 && availableHeight > 0 && label.scrollHeight > availableHeight + 0.5; attempt += 1) {
        fittedFontSize = Math.max(TEXT_MIN_FONT_SIZE, fittedFontSize * availableHeight / label.scrollHeight);
        label.style.fontSize = `${fittedFontSize}px`;
      }
      if (fittedFontSize < baseFontSize - 0.1) nextFontSizes[state.entity_id] = fittedFontSize;
      const transform = interaction?.entityId === state.entity_id
        ? interaction.current
        : normalizeEntityTransform(state.bounds, state.rotation_degrees, defaultRenderBounds, geometryFrame);
      const visual = label.parentElement;
      const labelLeft = label.dataset.textExpanded === "true"
        ? 0
        : visual ? (visual.clientWidth - label.offsetWidth) / 2 : label.offsetLeft;
      next[state.entity_id] = measuredEntityBounds(transform, {
        left: labelLeft,
        top: visual ? (visual.clientHeight - label.offsetHeight) / 2 : label.offsetTop,
        width: label.offsetWidth,
        height: label.offsetHeight,
      }, { width: frame.clientWidth, height: frame.clientHeight });
    });
    setTextFontSizeByEntityId((current) => {
      const currentKeys = Object.keys(current);
      const nextKeys = Object.keys(nextFontSizes);
      if (currentKeys.length === nextKeys.length && nextKeys.every((key) => Math.abs((current[key] ?? 0) - nextFontSizes[key]) < 0.1)) return current;
      return nextFontSizes;
    });
    setTextVisualBoundsByEntityId((current) => {
      const currentKeys = Object.keys(current);
      const nextKeys = Object.keys(next);
      if (currentKeys.length === nextKeys.length && nextKeys.every((key) => JSON.stringify(current[key]) === JSON.stringify(next[key]))) return current;
      return next;
    });
  }, [entityById, framePixels, geometryFrame, interaction, keyframe, textFontSizeByEntityId, visibleStates]);

  const beginInteraction = (
    event: React.PointerEvent<HTMLElement>,
    state: VisualEntityStateV1,
    mode: EntityInteractionMode,
    handle?: EntityResizeHandle,
  ) => {
    if (!editable || (event.pointerType === "mouse" && event.button !== 0)) return;
    if (interactionRef.current) return;
    const frame = frameRef.current;
    if (!frame) return;
    const rect = frame.getBoundingClientRect();
    const width = frame.clientWidth;
    const height = frame.clientHeight;
    if (width <= 0 || height <= 0) return;
    const initial = normalizeEntityTransform(state.bounds, state.rotation_degrees, defaultRenderBounds, geometryFrame);
    const originX = rect.left + frame.clientLeft;
    const originY = rect.top + frame.clientTop;
    const centerX = originX + (initial.bounds.x + initial.bounds.width / 2) * width;
    const centerY = originY + (initial.bounds.y + initial.bounds.height / 2) * height;
    const next: EntityInteraction = {
      entityId: state.entity_id,
      state,
      pointerId: event.pointerId,
      mode,
      handle,
      startX: event.clientX,
      startY: event.clientY,
      frame: { width, height },
      centerX,
      centerY,
      startAngle: Math.atan2(event.clientY - centerY, event.clientX - centerX),
      initial,
      current: initial,
      moved: false,
    };
    event.stopPropagation();
    if (mode !== "move") event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    interactionRef.current = next;
    setInteraction(next);
  };

  const updateInteraction = useCallback((event: PointerEvent) => {
    const active = interactionRef.current;
    if (!active || active.pointerId !== event.pointerId) return;
    const next: EntityInteraction = {
      ...active,
      current: entityTransformAtPointer(active, event.clientX, event.clientY, event.shiftKey),
      moved: active.moved || Math.hypot(event.clientX - active.startX, event.clientY - active.startY) >= 3,
    };
    interactionRef.current = next;
    setInteraction(next);
  }, []);

  const finishInteraction = useCallback((pointerId: number, commitChanges: boolean) => {
    const active = interactionRef.current;
    if (!active || active.pointerId !== pointerId) return;
    interactionRef.current = null;
    setInteraction(null);
    if (!commitChanges) return;
    if (!active.moved) {
      if (active.mode === "move") {
        suppressClickRef.current = true;
        onSelectEntityRef.current(active.state);
        window.setTimeout(() => {
          suppressClickRef.current = false;
        }, 0);
      }
      return;
    }
    suppressClickRef.current = true;
    window.setTimeout(() => {
      suppressClickRef.current = false;
    }, 0);
    onTransformEntityRef.current(active.entityId, active.current);
  }, []);

  const handleLostPointerCapture = (event: React.PointerEvent<HTMLElement>) => {
    if (interactionRef.current?.pointerId === event.pointerId) finishInteraction(event.pointerId, true);
  };

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent) => updateInteraction(event);
    const handlePointerUp = (event: PointerEvent) => finishInteraction(event.pointerId, true);
    const handlePointerCancel = (event: PointerEvent) => finishInteraction(event.pointerId, false);
    const handleWindowBlur = () => {
      const active = interactionRef.current;
      if (active) finishInteraction(active.pointerId, false);
    };
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("pointercancel", handlePointerCancel);
    window.addEventListener("blur", handleWindowBlur);
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointercancel", handlePointerCancel);
      window.removeEventListener("blur", handleWindowBlur);
    };
  }, [finishInteraction, updateInteraction]);

  const handleEntityKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, state: VisualEntityStateV1) => {
    if (!editable || event.altKey || event.ctrlKey || event.metaKey) return;
    const current = interaction?.entityId === state.entity_id
      ? interaction.current
      : normalizeEntityTransform(state.bounds, state.rotation_degrees, defaultRenderBounds, geometryFrame);
    const step = event.shiftKey ? 0.02 : 0.005;
    let next = current;
    if (event.key === "ArrowLeft") next = { ...current, bounds: { ...current.bounds, x: clampFrameValue(current.bounds.x - step, 0, 1 - current.bounds.width) } };
    else if (event.key === "ArrowRight") next = { ...current, bounds: { ...current.bounds, x: clampFrameValue(current.bounds.x + step, 0, 1 - current.bounds.width) } };
    else if (event.key === "ArrowUp") next = { ...current, bounds: { ...current.bounds, y: clampFrameValue(current.bounds.y - step, 0, 1 - current.bounds.height) } };
    else if (event.key === "ArrowDown") next = { ...current, bounds: { ...current.bounds, y: clampFrameValue(current.bounds.y + step, 0, 1 - current.bounds.height) } };
    else if (event.key === "[") next = { ...current, rotation: current.rotation - 5 };
    else if (event.key === "]") next = { ...current, rotation: current.rotation + 5 };
    else return;
    event.preventDefault();
    onTransformEntity(
      state.entity_id,
      normalizeEntityTransform(next.bounds, next.rotation, defaultRenderBounds, geometryFrame),
    );
  };

  const handleEntityControlKeyDown = (
    event: React.KeyboardEvent<HTMLButtonElement>,
    state: VisualEntityStateV1,
    mode: EntityInteractionMode,
    handle?: EntityResizeHandle,
  ) => {
    if (!editable || event.altKey || event.ctrlKey || event.metaKey) return;
    const current = interaction?.entityId === state.entity_id
      ? interaction.current
      : normalizeEntityTransform(state.bounds, state.rotation_degrees, defaultRenderBounds, geometryFrame);
    const step = event.shiftKey ? 0.02 : 0.005;
    let next = current;

    if (mode === "rotate") {
      const direction = event.key === "ArrowLeft" || event.key === "["
        ? -1
        : event.key === "ArrowRight" || event.key === "]"
          ? 1
          : 0;
      if (!direction) return;
      next = normalizeEntityTransform(
        current.bounds,
        current.rotation + direction * (event.shiftKey ? 15 : 5),
        defaultRenderBounds,
        geometryFrame,
      );
    } else {
      if (!handle) return;
      const delta = event.key === "ArrowLeft"
        ? [-step * geometryFrame.width, 0]
        : event.key === "ArrowRight"
          ? [step * geometryFrame.width, 0]
          : event.key === "ArrowUp"
            ? [0, -step * geometryFrame.height]
            : event.key === "ArrowDown"
              ? [0, step * geometryFrame.height]
              : null;
      if (!delta) return;
      next = resizeEntitySelectionByScreenDelta(current, handle, delta[0], delta[1], geometryFrame);
    }

    event.preventDefault();
    onTransformEntity(state.entity_id, next);
  };

  const frameStyle: React.CSSProperties = {
    aspectRatio: `${Math.max(1, finiteOr(plan.frame.width, 16))} / ${Math.max(1, finiteOr(plan.frame.height, 9))}`,
    backgroundColor: plan.frame.background?.trim() || undefined,
  };
  return <div ref={frameRef} className="a4n-visual-plan__frame relative mt-3 w-full overflow-hidden rounded-lg border border-book-border/70 bg-book-bg-tertiary/60" style={frameStyle} onClick={(event) => {
    if (suppressClickRef.current) {
      event.preventDefault();
      event.stopPropagation();
      suppressClickRef.current = false;
      return;
    }
    const target = event.target;
    if (!(target instanceof Element) || !target.closest(".a4n-visual-plan__entity-shell, .a4n-visual-plan__entity-handle, .a4n-visual-plan__entity-rotate")) onClearSelection();
  }} aria-label={t("visualPlan.viewerFrame")}>
    <div className="a4n-visual-plan__frame-layer" style={{ position: "absolute", inset: 0 }}>
      {visibleStates.map((state) => {
        const entity = entityById.get(state.entity_id);
        const kind = entity?.kind ?? "object";
        const label = entity?.label ?? state.entity_id;
        const accessibleLabel = kind === "text" ? state.dialogue?.trim() || label : label;
        const activeTransform = interaction?.entityId === state.entity_id
          ? interaction.current
          : normalizeEntityTransform(state.bounds, state.rotation_degrees, defaultRenderBounds, geometryFrame);
        const selected = selectedEntityId === state.entity_id || interaction?.entityId === state.entity_id;
        const boundsStyle: React.CSSProperties = {
          left: `${activeTransform.bounds.x * 100}%`,
          top: `${activeTransform.bounds.y * 100}%`,
          width: `${activeTransform.bounds.width * 100}%`,
          height: `${activeTransform.bounds.height * 100}%`,
        };
        const textBounds = kind === "text" ? textVisualBoundsByEntityId[state.entity_id] : undefined;
        const selectionTransform = textBounds
          ? { bounds: textBounds, rotation: activeTransform.rotation }
          : entitySelectionTransform(activeTransform);
        const selectionReady = kind !== "text" || Boolean(textBounds);
        const tightBounds = rotatedBoundsSize(selectionTransform.bounds, selectionTransform.rotation, geometryFrame);
        const compact = kind === "text" || (controlFrame
          ? tightBounds.width * controlFrame.width < ENTITY_CONTROL_SIZE * 2
            || tightBounds.height * controlFrame.height < ENTITY_CONTROL_SIZE * 2
          : tightBounds.width < 0.1 || tightBounds.height < 0.1);
        const selectionBoundsStyle: React.CSSProperties = {
          left: `${selectionTransform.bounds.x * 100}%`,
          top: `${selectionTransform.bounds.y * 100}%`,
          width: `${selectionTransform.bounds.width * 100}%`,
          height: `${selectionTransform.bounds.height * 100}%`,
        };
        const textExpanded = Boolean(textBounds && textBounds.width > activeTransform.bounds.width + 0.002);
        const textMaxWidth = kind === "text" && controlFrame
          ? Math.min(
            controlFrame.width * TEXT_MAX_WIDTH_RATIO,
            Math.max(1, controlFrame.width - activeTransform.bounds.x * controlFrame.width),
          )
          : undefined;
        const textHitStyle = textBounds && controlFrame
          ? {
            left: `${(textBounds.x + textBounds.width / 2 - activeTransform.bounds.x) * controlFrame.width}px`,
            top: `${(textBounds.y + textBounds.height / 2 - activeTransform.bounds.y) * controlFrame.height}px`,
          }
          : undefined;
        const hitPosition = compact && controlFrame
          ? entityControlPosition(activeTransform, "center", controlFrame)
          : null;
        const resizeHandles = compact ? (["se"] as const) : (["nw", "ne", "se", "sw"] as const);
        const rotatePosition = controlFrame
          ? entityControlFramePosition(selectionTransform, "rotate", controlFrame, ENTITY_CONTROL_SIZE)
          : null;
        return <React.Fragment key={state.entity_id}>
          <div className="a4n-visual-plan__entity-shell" style={{ ...boundsStyle, zIndex: ENTITY_RENDER_Z_INDEX_OFFSET + renderZIndex(state.z_index) }}>
            <span data-entity-id={state.entity_id} className={`a4n-visual-plan__entity a4n-visual-plan__entity--${kind} ${compact ? "a4n-visual-plan__entity--compact" : ""} ${selected ? "a4n-visual-plan__entity--selected" : ""}`}>
              <span className={`a4n-visual-plan__entity-transform ${interaction?.entityId === state.entity_id ? "a4n-visual-plan__entity-transform--interacting" : ""}`} style={{ transform: `rotate(${activeTransform.rotation}deg)` }}>
                <span className="a4n-visual-plan__entity-content">
                  <span className="a4n-visual-plan__entity-visual" style={{ opacity: renderOpacity(state.opacity) }}>
                    <EntitySketch kind={kind} label={label} dialogue={state.dialogue} facing={state.facing} textFontSize={textFontSizeByEntityId[state.entity_id]} textMaxWidth={textMaxWidth} textExpanded={textExpanded} />
                  </span>
                </span>
                <button type="button" title={accessibleLabel} aria-label={accessibleLabel} onPointerDown={(event) => beginInteraction(event, state, "move")} onClick={(event) => {
                  if (suppressClickRef.current) {
                    event.preventDefault();
                    event.stopPropagation();
                    suppressClickRef.current = false;
                    return;
                  }
                  onSelectEntity(state);
                }} onLostPointerCapture={handleLostPointerCapture} onKeyDown={(event) => handleEntityKeyDown(event, state)} className={`a4n-visual-plan__entity-hit ${selected ? "a4n-visual-plan__entity-hit--selected" : ""}`} style={kind === "text" ? textHitStyle : hitPosition ? { left: `${hitPosition.left}px`, top: `${hitPosition.top}px` } : undefined} />
              </span>
            </span>
          </div>
          {selected && selectionReady ? <div className="a4n-visual-plan__entity-controls" style={{ inset: 0, zIndex: ENTITY_CONTROL_LAYER_Z_INDEX }}>
            <span className={`a4n-visual-plan__entity-selection a4n-visual-plan__entity-selection--${kind}`} style={{ ...selectionBoundsStyle, transform: `rotate(${selectionTransform.rotation}deg)` }} aria-hidden="true" />
            {editable ? <>
              {resizeHandles.map((handle) => {
                if (!controlFrame) return null;
                const position = entityControlFramePosition(selectionTransform, handle, controlFrame, ENTITY_CONTROL_SIZE);
                return <button key={handle} type="button" aria-label={`${label} ${t(RESIZE_HANDLE_LABEL_KEYS[handle])}`} className={`a4n-visual-plan__entity-handle a4n-visual-plan__entity-handle--${handle}`} style={{ left: `${position.left}px`, top: `${position.top}px`, transform: "translate(-50%, -50%)" }} onPointerDown={(event) => beginInteraction(event, state, "resize", handle)} onLostPointerCapture={handleLostPointerCapture} onKeyDown={(event) => handleEntityControlKeyDown(event, state, "resize", handle)} />;
              })}
              {rotatePosition ? <button type="button" aria-label={`${label} ${t("visualPlan.rotation")}`} className="a4n-visual-plan__entity-rotate" style={{ left: `${rotatePosition.left}px`, top: `${rotatePosition.top}px`, transform: "translate(-50%, -50%)" }} onPointerDown={(event) => beginInteraction(event, state, "rotate")} onLostPointerCapture={handleLostPointerCapture} onKeyDown={(event) => handleEntityControlKeyDown(event, state, "rotate")} /> : null}
            </> : null}
          </div> : null}
        </React.Fragment>;
      })}
      {!keyframe ? <div className="absolute inset-0 grid place-items-center px-6 text-center text-small text-book-text-muted">{t("visualPlan.noKeyframes")}</div> : null}
      {keyframe && visibleStates.length === 0 ? <div className="absolute inset-0 grid place-items-center px-6 text-center text-small text-book-text-muted">{t("visualPlan.noEntitiesInKeyframe")}</div> : null}
    </div>
  </div>;
};

const Timeline: React.FC<{
  plan: ChapterVisualPlanV2;
  shots: readonly VisualShotRef[];
  selectedShot: VisualShotV2 | null;
  selectedKeyframeId: string | null;
  playheadMs: number;
  onSeek: (value: number) => void;
  onSelectShot: (ref: VisualShotRef) => void;
  onSelectKeyframe: (keyframe: VisualKeyframeV2, ref: VisualShotRef) => void;
  t: (key: MessageKey) => string;
}> = ({ plan, shots, selectedShot, selectedKeyframeId, playheadMs, onSeek, onSelectShot, onSelectKeyframe, t }) => {
  const total = planDurationMs(plan);
  const percent = (value: number) => total > 0 ? `${Math.max(0, Math.min(100, value / total * 100))}%` : "0%";
  const timelinePosition = (value: number) => {
    const ratio = total > 0 ? Math.max(0, Math.min(1, value / total)) : 0;
    const edgeOffsetRem = (1.375 * (1 - ratio * 2)).toFixed(4);
    return `calc(${(ratio * 100).toFixed(4)}% + ${edgeOffsetRem}rem)`;
  };
  const keyframeLanes = new Map<number, number>();
  const keyframes = shots.flatMap((ref) => ref.shot.keyframes.map((keyframe) => {
    const position = ref.start_ms + keyframe.at_ms;
    const lane = keyframeLanes.get(position) ?? 0;
    keyframeLanes.set(position, lane + 1);
    return { keyframe, ref, position, lane: lane % 2 };
  }));
  const rangeStyle = { "--range-percent": percent(playheadMs) } as React.CSSProperties;

  return (
    <section className="a4n-visual-plan__timeline min-w-0" aria-label={t("visualPlan.chapterTimeline")}>
      <div className="a4n-visual-plan__timeline-head">
        <div className="a4n-visual-plan__timeline-title">
          <h4 className="text-small font-semibold text-book-text-main">{t("visualPlan.timeline")}</h4>
          <span className="truncate text-caption text-book-text-muted">{selectedShot?.title ?? t("visualPlan.noShot")}</span>
        </div>
        <div className="a4n-visual-plan__timeline-time" aria-live="polite">
          <span className="font-mono text-caption text-book-text-sub">{formatPlanTime(playheadMs)}</span>
          <span className="font-mono text-caption text-book-text-muted">/ {formatPlanTime(total)}</span>
        </div>
      </div>
      <div className="a4n-visual-plan__timeline-scrubber">
        <input aria-label={t("visualPlan.playhead")} type="range" min={0} max={Math.max(1, total)} step={50} value={Math.min(playheadMs, Math.max(1, total))} onChange={(event) => onSeek(Number(event.currentTarget.value))} className="book-range a4n-visual-plan__timeline-range" style={rangeStyle} />
      </div>
      <div className="a4n-visual-plan__timeline-body">
        <div className="a4n-visual-plan__timeline-track-content" style={{ "--a4n-timeline-min-width": `${Math.max(0, shots.length * 8)}rem` } as React.CSSProperties}>
          <div className="a4n-visual-plan__timeline-shot-track">
            {shots.map((ref) => (
              <button key={ref.shot.id} type="button" aria-label={`${t("visualPlan.shot")} ${ref.shot.title || t("visualPlan.shot")} · ${formatPlanTime(ref.shot.duration_ms)}`} onClick={() => onSelectShot(ref)} className={`a4n-visual-plan__timeline-shot ${selectedShot?.id === ref.shot.id ? "a4n-visual-plan__timeline-shot--selected" : ""}`} style={{ flex: `${Math.max(1, ref.shot.duration_ms)} 1 0%` }}>
                <span className="block truncate font-semibold">{ref.shot.title || t("visualPlan.shot")}</span>
                <span className="a4n-visual-plan__timeline-shot-duration">{formatPlanTime(ref.shot.duration_ms)}</span>
              </button>
            ))}
          </div>
          <div className="a4n-visual-plan__timeline-marker-track">
            <div className="a4n-visual-plan__timeline-marker-rail" aria-hidden="true" />
            {keyframes.map(({ keyframe, ref, position, lane }) => {
              return <button key={keyframe.id} type="button" aria-label={`${formatPlanTime(position)} ${keyframe.label}`} title={`${formatPlanTime(position)} · ${keyframe.label}`} onClick={() => onSelectKeyframe(keyframe, ref)} className={`a4n-visual-plan__timeline-keyframe ${selectedKeyframeId === keyframe.id ? "a4n-visual-plan__timeline-keyframe--selected" : ""}`} style={{ left: timelinePosition(position), "--keyframe-lane": lane } as React.CSSProperties}>
                <span className="a4n-visual-plan__timeline-keyframe-dot" />
              </button>;
            })}
            <div aria-hidden="true" className="a4n-visual-plan__timeline-playhead" style={{ left: timelinePosition(playheadMs) }} />
          </div>
          <div className="a4n-visual-plan__timeline-scale">
            <span>0s</span>
            <span>{formatPlanTime(total)}</span>
          </div>
        </div>
      </div>
    </section>
  );
};

const EventList: React.FC<{
  category: EventCategory;
  label: string;
  events: readonly VisualPlanTimedEventV2[];
  editable: boolean;
  onEdit: (event: VisualPlanTimedEventV2, category: EventCategory) => void;
  onDelete: (eventId: string) => void;
  onRangeChange: (eventId: string, boundary: "start" | "end", value: number) => boolean;
  t: (key: MessageKey) => string;
}> = ({ category, label, events, editable, onEdit, onDelete, onRangeChange, t }) => <details className="min-w-0 rounded-lg border border-book-border/60 bg-book-bg-paper"><summary className="cursor-pointer px-2.5 py-2 text-caption font-semibold text-book-text-main">{label} · {events.length}</summary>{events.length > 0 ? <ul className="space-y-1.5 border-t border-book-border/50 p-2">{events.map((event) => <li key={event.id} className="min-w-0 rounded-md bg-book-bg-tertiary/60 p-2"><div className="flex min-w-0 items-start gap-2"><button type="button" className="min-w-0 flex-1 text-left" onClick={() => onEdit(event, category)}><span className="block text-caption font-semibold text-book-text-main">{event.kind}</span><span className="line-clamp-2 text-caption leading-relaxed text-book-text-muted">{eventText(event)}</span></button><BookIconButton label={t("common.delete")} size="sm" disabled={!editable} onClick={() => onDelete(event.id)}><Trash2 size={12} /></BookIconButton></div><div className="mt-2 grid grid-cols-2 gap-2"><Field label={t("visualPlan.eventStart")}><TextInput key={`${event.id}-start-${event.start_ms}`} type="number" min={0} step={0.05} defaultValue={event.start_ms / 1000} disabled={!editable} onBlur={(input) => { const accepted = onRangeChange(event.id, "start", parseNumericInput(input.currentTarget.value) * 1000); if (!accepted) input.currentTarget.value = String(event.start_ms / 1000); }} className="!min-h-8 !px-2 text-caption" /></Field><Field label={t("visualPlan.eventEnd")}><TextInput key={`${event.id}-end-${event.end_ms}`} type="number" min={0.001} step={0.05} defaultValue={event.end_ms / 1000} disabled={!editable} onBlur={(input) => { const accepted = onRangeChange(event.id, "end", parseNumericInput(input.currentTarget.value) * 1000); if (!accepted) input.currentTarget.value = String(event.end_ms / 1000); }} className="!min-h-8 !px-2 text-caption" /></Field></div></li>)}</ul> : null}</details>;

const SummaryPanel: React.FC<{
  plan: ChapterVisualPlanV2;
  selectedScene: VisualSceneV2 | null;
  selectedShot: VisualShotV2 | null;
  totalDuration: number;
  entityById: ReadonlyMap<string, ChapterVisualPlanV2["entities"][number]>;
  editable: boolean;
  strictIssues: readonly VisualPlanIssue[];
  onOpenFrame: () => void;
  onOpenEntities: () => void;
  onOpenShot: () => void;
  onDeleteShot: () => void;
  t: (key: MessageKey, params?: Record<string, string | number>) => string;
}> = ({ plan, selectedScene, selectedShot, totalDuration, entityById, editable, strictIssues, onOpenFrame, onOpenEntities, onOpenShot, onDeleteShot, t }) => (
  <div className="min-w-0">
    <div className="flex min-w-0 items-center justify-between gap-2">
      <h4 className="text-small font-semibold text-book-text-main">{t("visualPlan.planSummary")}</h4>
      <BookIconButton label={t("visualPlan.frameSettings")} size="sm" disabled={!editable} onClick={onOpenFrame}>
        <Settings2 size={14} />
      </BookIconButton>
    </div>
    <PlanSnapshot plan={plan} totalDuration={totalDuration} t={t} />
    <div className="mt-3 border-t border-book-border/55 pt-3">
      <p className="text-caption font-semibold text-book-text-muted">{t("visualPlan.selectedScene")}</p>
      <p className="mt-1 truncate text-small text-book-text-main">{selectedScene?.title ?? t("visualPlan.noScenes")}</p>
      <p className="mt-2 text-caption font-semibold text-book-text-muted">{t("visualPlan.shot")}</p>
      <p className="mt-1 truncate text-small text-book-text-main">{selectedShot?.title ?? t("visualPlan.noShot")}</p>
      <div className="mt-2 flex flex-wrap gap-1.5">
        <BookButton size="sm" variant="secondary" disabled={!selectedShot || !editable} onClick={onOpenShot}>
          {t("common.edit")}
        </BookButton>
        {selectedShot ? (
          <BookIconButton label={t("common.delete")} size="sm" disabled={!editable} onClick={onDeleteShot}>
            <Trash2 size={13} />
          </BookIconButton>
        ) : null}
      </div>
    </div>
    <div className="mt-3 border-t border-book-border/55 pt-3">
      <div className="flex items-center justify-between gap-2">
        <p className="text-caption font-semibold text-book-text-muted">{t("visualPlan.entities")}</p>
        <BookIconButton label={t("visualPlan.entities")} size="sm" onClick={onOpenEntities}>
          <ArrowRight size={13} />
        </BookIconButton>
      </div>
      <div className="mt-2 flex flex-wrap gap-1">
        {Array.from(entityById.values()).slice(0, 8).map((entity) => (
          <span key={entity.id} className="max-w-full truncate rounded-md bg-book-bg-tertiary px-1.5 py-1 text-[10px] text-book-text-sub">
            {entity.label}
          </span>
        ))}
      </div>
    </div>
    {strictIssues.length > 0 ? (
      <div className="mt-3 border-t border-book-border/55 pt-3">
        <p className="text-caption font-semibold text-warning">{t("visualPlan.confirmBlocked", { count: strictIssues.length })}</p>
        <ul className="mt-2 space-y-1">
          {strictIssues.slice(0, 6).map((issue, index) => (
            <li key={`${issue.key}-${index}`} className="line-clamp-2 text-[10px] leading-relaxed text-warning">
              {translateVisualPlanIssue(issue, t)}
            </li>
          ))}
        </ul>
      </div>
    ) : null}
  </div>
);

const PlanSnapshot: React.FC<{ plan: ChapterVisualPlanV2; totalDuration: number; t: (key: MessageKey, params?: Record<string, string | number>) => string }> = ({ plan, totalDuration, t }) => <div className="mt-3 grid grid-cols-2 gap-1.5"><div className="rounded-lg bg-book-bg-tertiary/60 p-2"><span className="block text-[10px] text-book-text-muted">{t("visualPlan.scenes")}</span><strong className="text-small text-book-text-main">{plan.scenes.length}</strong></div><div className="rounded-lg bg-book-bg-tertiary/60 p-2"><span className="block text-[10px] text-book-text-muted">{t("visualPlan.shot")}</span><strong className="text-small text-book-text-main">{plan.scenes.reduce((total, scene) => total + scene.shots.length, 0)}</strong></div><div className="col-span-2 rounded-lg bg-book-bg-tertiary/60 p-2"><span className="block text-[10px] text-book-text-muted">{t("visualPlan.totalDuration", { value: formatPlanTime(totalDuration) })}</span><span className="font-mono text-small text-book-text-main">{formatPlanTime(totalDuration)}</span></div></div>;
