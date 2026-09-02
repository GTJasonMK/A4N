import { ThemedSelect } from "../../components/ui/ThemedSelect";
import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Background,
  Controls,
  MarkerType,
  ReactFlow,
  useNodesState,
  type Edge,
  type Node,
  type NodeChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
  ArrowLeft,
  ArrowRight,
  Bot,
  Box,
  CheckCircle2,
  Copy,
  Film,
  Pause,
  Play,
  Plus,
  Redo2,
  Save,
  SlidersHorizontal,
  Sparkles,
  Trash2,
  Undo2,
  UserRound,
  Volume2,
} from "lucide-react";

import { extractErrorMessage } from "../../api";
import type { ChapterSummary } from "../../api/chapter";
import {
  applyChapterVisualPlanProposal,
  confirmChapterVisualPlan,
  discardChapterVisualPlanDraft,
  generateChapterVisualPlanDraft,
  getChapterVisualPlan,
  proposeChapterVisualPlanRevision,
  saveChapterVisualPlanDraft,
  type ChapterVisualPlanAuthoringMode,
  type ChapterVisualPlanProposalView,
  type ChapterVisualPlanSetView,
  type ChapterVisualPlanV1,
  type ChapterVisualPlanView,
  type VisualCameraEventKindV1,
  type VisualEntityEventKindV1,
  type VisualEntityKindV1,
  type VisualEntityStateV1,
  type VisualKeyframeV1,
  type VisualRelationKindV1,
  type VisualShotPlanV1,
  type VisualSoundEventKindV1,
  type VisualTechniqueKindV1,
} from "../../api/chapterVisualPlan";
import type { ShootingTransitionTypeV1 } from "../../api/shootingScript";
import { cancelWorkflowTask } from "../../api/streaming";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../components/feedback/PromptDialog";
import { useToast } from "../../components/feedback/Toast";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { Modal } from "../../components/ui/Modal";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { createWorkflowTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { EmptyHint } from "./shared";
import { useI18n, type MessageKey } from "../../i18n";
import {
  clonePlan,
  collectVisualPlanIssues,
  detachVisualPlanSourceSpanReferences,
  formatPlanTime,
  moveVisualShot,
  removeVisualPlanTimedEvent,
  resizeVisualShot,
  shouldAutoSaveVisualPlanDraft,
  translateVisualPlanIssue,
  updateVisualPlanTimedEventRange,
  visualShotAt,
  visualShots,
  type VisualPlanTimedEventV1,
  type VisualShotRef,
} from "./visual-plan/editing";

interface ChapterVisualPlanTabProps {
  projectId: number;
  canEdit: boolean;
  chapter: ChapterSummary | null;
}

type StageNode = Node<{ label: string }>;

interface PendingDraftSave {
  plan: ChapterVisualPlanV1;
  mode: ChapterVisualPlanAuthoringMode;
  notify: boolean;
}

interface ChapterEditorRuntime {
  projectId: number;
  chapterId: number;
  record: ChapterVisualPlanView | null;
  pendingSave: PendingDraftSave | null;
  savePromise: Promise<ChapterVisualPlanView | null> | null;
}

const STAGE_WIDTH = 960;
const STAGE_HEIGHT = 540;
const MODE_OPTIONS = [
  { id: "automatic", label: "visualPlan.modeAutomatic" },
  { id: "collaborative", label: "visualPlan.modeCollaborative" },
  { id: "manual", label: "visualPlan.modeManual" },
] satisfies ReadonlyArray<{ id: ChapterVisualPlanAuthoringMode; label: MessageKey }>;

const ENTITY_KINDS: ReadonlyArray<readonly [VisualEntityKindV1, MessageKey]> = [
  ["character", "visualPlan.character"],
  ["object", "visualPlan.object"],
  ["environment", "visualPlan.environment"],
  ["text", "visualPlan.text"],
  ["effect", "visualPlan.effect"],
];

const enumOptions = <T extends string>(labels: Record<T, MessageKey>) =>
  Object.entries(labels) as Array<[T, MessageKey]>;

const RELATION_LABELS = {
  left_of: "visualPlan.relationLeftOf",
  right_of: "visualPlan.relationRightOf",
  above: "visualPlan.relationAbove",
  below: "visualPlan.relationBelow",
  in_front_of: "visualPlan.relationInFrontOf",
  behind: "visualPlan.relationBehind",
  inside: "visualPlan.relationInside",
  overlapping: "visualPlan.relationOverlapping",
  facing: "visualPlan.relationFacing",
  looking_at: "visualPlan.relationLookingAt",
  touching: "visualPlan.relationTouching",
  holding: "visualPlan.relationHolding",
  following: "visualPlan.relationFollowing",
  custom: "visualPlan.relationCustom",
} satisfies Record<VisualRelationKindV1, MessageKey>;
const RELATION_KINDS = enumOptions(RELATION_LABELS);

const ENTITY_EVENT_LABELS = {
  enter: "visualPlan.entityEnter",
  exit: "visualPlan.entityExit",
  move: "visualPlan.entityMove",
  action: "visualPlan.entityAction",
  dialogue: "visualPlan.entityDialogue",
  expression: "visualPlan.entityExpression",
  gaze: "visualPlan.entityGaze",
  pose: "visualPlan.entityPose",
  interaction: "visualPlan.entityInteraction",
  transform: "visualPlan.entityTransform",
  custom: "visualPlan.entityCustom",
} satisfies Record<VisualEntityEventKindV1, MessageKey>;
const ENTITY_EVENT_KINDS = enumOptions(ENTITY_EVENT_LABELS);

const CAMERA_EVENT_LABELS = {
  cut: "visualPlan.cameraCut",
  pan: "visualPlan.cameraPan",
  tilt: "visualPlan.cameraTilt",
  push_in: "visualPlan.cameraPushIn",
  pull_out: "visualPlan.cameraPullOut",
  track: "visualPlan.cameraTrack",
  dolly: "visualPlan.cameraDolly",
  crane: "visualPlan.cameraCrane",
  handheld: "visualPlan.cameraHandheld",
  whip_pan: "visualPlan.cameraWhipPan",
  zoom: "visualPlan.cameraZoom",
  focus: "visualPlan.cameraFocus",
  shake: "visualPlan.cameraShake",
  orbit: "visualPlan.cameraOrbit",
  reframe: "visualPlan.cameraReframe",
  custom: "visualPlan.cameraCustom",
} satisfies Record<VisualCameraEventKindV1, MessageKey>;
const CAMERA_EVENT_KINDS = enumOptions(CAMERA_EVENT_LABELS);

const SOUND_EVENT_LABELS = {
  dialogue: "visualPlan.soundDialogue",
  voice_over: "visualPlan.soundVoiceOver",
  narration: "visualPlan.soundNarration",
  ambient: "visualPlan.soundAmbient",
  effect: "visualPlan.soundEffect",
  music: "visualPlan.soundMusic",
  silence: "visualPlan.soundSilence",
} satisfies Record<VisualSoundEventKindV1, MessageKey>;
const SOUND_EVENT_KINDS = enumOptions(SOUND_EVENT_LABELS);

const TECHNIQUE_LABELS = {
  montage: "visualPlan.techniqueMontage",
  split_screen: "visualPlan.techniqueSplitScreen",
  slow_motion: "visualPlan.techniqueSlowMotion",
  fast_motion: "visualPlan.techniqueFastMotion",
  freeze_frame: "visualPlan.techniqueFreezeFrame",
  flashback: "visualPlan.techniqueFlashback",
  dream: "visualPlan.techniqueDream",
  subjective: "visualPlan.techniqueSubjective",
  symbolic: "visualPlan.techniqueSymbolic",
  silhouette: "visualPlan.techniqueSilhouette",
  reflection: "visualPlan.techniqueReflection",
  overlay: "visualPlan.techniqueOverlay",
  text_on_screen: "visualPlan.techniqueTextOnScreen",
  lighting: "visualPlan.techniqueLighting",
  color_grade: "visualPlan.techniqueColorGrade",
  match_action: "visualPlan.techniqueMatchAction",
  offscreen: "visualPlan.techniqueOffscreen",
  custom: "visualPlan.techniqueCustom",
} satisfies Record<VisualTechniqueKindV1, MessageKey>;
const TECHNIQUE_KINDS = enumOptions(TECHNIQUE_LABELS);

const TRANSITION_KINDS: ReadonlyArray<readonly [ShootingTransitionTypeV1, MessageKey]> = [
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

const id = (prefix: string) =>
  `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;

const nearestKeyframe = (shot: VisualShotPlanV1, atMs: number) => {
  for (let index = shot.keyframes.length - 1; index >= 0; index -= 1) {
    const keyframe = shot.keyframes[index];
    if (keyframe.at_ms <= atMs) return keyframe;
  }
  return shot.keyframes[0] ?? null;
};

const currentShotRef = (
  shotId: string | null,
  shots: readonly VisualShotRef[],
) => shots.find(({ shot }) => shot.id === shotId) ?? shots[0] ?? null;

export const ChapterVisualPlanTab: React.FC<ChapterVisualPlanTabProps> = ({
  projectId,
  canEdit,
  chapter,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const { runProgressTask } = useProgressTask();
  const chapterId = chapter?.id ?? null;
  const [planSet, setPlanSet] = useState<ChapterVisualPlanSetView | null>(null);
  const [plan, setPlan] = useState<ChapterVisualPlanV1 | null>(null);
  const [mode, setMode] = useState<ChapterVisualPlanAuthoringMode>("manual");
  const [editing, setEditing] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [locking, setLocking] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [proposal, setProposal] = useState<ChapterVisualPlanProposalView | null>(null);
  const [baseline, setBaseline] = useState("");
  const [baselineMode, setBaselineMode] = useState<ChapterVisualPlanAuthoringMode>("manual");
  const [selectedShotId, setSelectedShotId] = useState<string | null>(null);
  const [selectedKeyframeId, setSelectedKeyframeId] = useState<string | null>(null);
  const [selectedEntityId, setSelectedEntityId] = useState<string | null>(null);
  const [playheadMs, setPlayheadMs] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [entityKind, setEntityKind] = useState<VisualEntityKindV1>("character");
  const [relationKind, setRelationKind] = useState<VisualRelationKindV1>("looking_at");
  const [relationTarget, setRelationTarget] = useState("");
  const [relationLabel, setRelationLabel] = useState("");
  const [entityEventKind, setEntityEventKind] = useState<VisualEntityEventKindV1>("action");
  const [cameraEventKind, setCameraEventKind] = useState<VisualCameraEventKindV1>("reframe");
  const [soundEventKind, setSoundEventKind] = useState<VisualSoundEventKindV1>("effect");
  const [techniqueKind, setTechniqueKind] = useState<VisualTechniqueKindV1>("symbolic");
  const [undoStack, setUndoStack] = useState<ChapterVisualPlanV1[]>([]);
  const [redoStack, setRedoStack] = useState<ChapterVisualPlanV1[]>([]);
  const [mobilePanel, setMobilePanel] = useState<"source" | "inspector" | null>(null);
  const [nodes, setNodes, applyNodeChanges] = useNodesState<StageNode>([]);
  const planRef = useRef<ChapterVisualPlanV1 | null>(null);
  const modeRef = useRef(mode);
  const runtimeRef = useRef<ChapterEditorRuntime | null>(null);
  const lockingRef = useRef(false);
  const record = planSet?.draft ?? null;

  useEffect(() => {
    setMobilePanel(null);
  }, [chapterId]);

  useEffect(() => {
    planRef.current = plan;
  }, [plan]);

  const acquireEditorLock = useCallback((runtime: ChapterEditorRuntime) => {
    if (lockingRef.current || runtimeRef.current !== runtime) return false;
    lockingRef.current = true;
    setLocking(true);
    return true;
  }, []);

  const releaseEditorLock = useCallback((runtime: ChapterEditorRuntime) => {
    if (runtimeRef.current !== runtime) return;
    lockingRef.current = false;
    setLocking(false);
  }, []);

  const resetEditor = useCallback((
    value: ChapterVisualPlanSetView,
    runtime: ChapterEditorRuntime,
  ) => {
    const source = value.draft ?? value.confirmed;
    const next = clonePlan(source?.plan ?? value.starter_plan);
    runtime.record = value.draft;
    runtime.pendingSave = null;
    runtime.savePromise = null;
    lockingRef.current = false;
    setLocking(false);
    setPlanSet(value);
    setPlan(next);
    planRef.current = next;
    const nextMode = value.draft?.record.authoring_mode ??
      value.confirmed?.record.authoring_mode ?? "manual";
    modeRef.current = nextMode;
    setMode(nextMode);
    setBaselineMode(nextMode);
    setEditing(Boolean(value.draft) || !value.confirmed);
    setProposal(null);
    setBaseline(JSON.stringify(next));
    setUndoStack([]);
    setRedoStack([]);
    const first = visualShots(next)[0]?.shot ?? null;
    setSelectedShotId(first?.id ?? null);
    setSelectedKeyframeId(first?.keyframes[0]?.id ?? null);
    setSelectedEntityId(first?.keyframes[0]?.entity_states[0]?.entity_id ?? null);
    setPlayheadMs(first?.start_ms ?? 0);
    setPlaying(false);
  }, []);

  const loadPlan = useCallback(async () => {
    if (chapterId == null) return;
    const runtime: ChapterEditorRuntime = {
      projectId,
      chapterId,
      record: null,
      pendingSave: null,
      savePromise: null,
    };
    runtimeRef.current = runtime;
    lockingRef.current = false;
    setLoading(true);
    setSaving(false);
    setLocking(false);
    setGenerating(false);
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
  }, [addToast, chapterId, projectId, resetEditor]);

  useEffect(() => {
    if (chapterId == null) {
      runtimeRef.current = null;
      lockingRef.current = false;
      setPlanSet(null);
      setPlan(null);
      planRef.current = null;
      setLoadError(null);
      setLoading(false);
      setSaving(false);
      setLocking(false);
      setGenerating(false);
      return;
    }
    void loadPlan();
  }, [chapterId, loadPlan]);

  const serializedPlan = useMemo(
    () => (plan == null ? "" : JSON.stringify(plan)),
    [plan],
  );
  const planDirty = plan != null && serializedPlan !== baseline;
  const modeDirty = mode !== baselineMode;
  const shouldAutoSave = shouldAutoSaveVisualPlanDraft({
    hasDraft: record != null,
    planDirty,
    modeDirty,
  });
  const editable = canEdit && editing && !generating && !locking;
  const issues = useMemo(() => (
    plan
      ? collectVisualPlanIssues(
        plan,
        planSet?.source.source_spans.map((span) => span.id) ?? [],
      )
      : []
  ), [plan, planSet?.source.source_spans]);
  const entityById = useMemo(
    () => new Map((plan?.entities ?? []).map((item) => [item.id, item] as const)),
    [plan],
  );
  const sourceDispositionBySpanId = useMemo(
    () => new Map((plan?.source_dispositions ?? []).map((item) => [item.source_span_id, item] as const)),
    [plan],
  );
  const shotRefs = useMemo(() => (plan ? visualShots(plan) : []), [plan]);
  const shotRef = plan ? currentShotRef(selectedShotId, shotRefs) : null;
  const shot = shotRef?.shot ?? null;
  const keyframe = shot?.keyframes.find((item) => item.id === selectedKeyframeId) ??
    (shot ? nearestKeyframe(shot, playheadMs) : null);
  const entityState = keyframe?.entity_states.find(
    (state) => state.entity_id === selectedEntityId,
  ) ?? null;
  const entity = selectedEntityId ? entityById.get(selectedEntityId) ?? null : null;
  const stageNodes = useMemo<StageNode[]>(() => {
    if (!plan || !keyframe) return [];
    return keyframe.entity_states
      .filter((state) => state.visible)
      .map((state) => {
        const value = entityById.get(state.entity_id);
        return {
          id: state.entity_id,
          position: {
            x: state.bounds.x * STAGE_WIDTH,
            y: state.bounds.y * STAGE_HEIGHT,
          },
          data: {
            label: [value?.label ?? state.entity_id, state.action, state.dialogue]
              .filter(Boolean)
              .join(" · "),
          },
          selected: state.entity_id === selectedEntityId,
          draggable: editable,
          style: {
            width: Math.max(76, state.bounds.width * STAGE_WIDTH),
            height: Math.max(42, state.bounds.height * STAGE_HEIGHT),
            borderRadius: 8,
            border: state.entity_id === selectedEntityId
              ? "2px solid rgb(var(--color-primary))"
              : "1px solid rgb(var(--color-border-dark))",
            color: "rgb(var(--color-text-primary))",
            background: "rgb(var(--color-bg-secondary) / 0.92)",
            fontSize: 12,
            overflow: "hidden",
          },
        } satisfies StageNode;
      });
  }, [editable, entityById, keyframe, plan, selectedEntityId]);

  const applyPlanEdit = useCallback((edit: (next: ChapterVisualPlanV1) => void) => {
    const current = planRef.current;
    if (!current || !canEdit || !editing || generating) return false;
    const next = clonePlan(current);
    edit(next);
    if (JSON.stringify(next) === JSON.stringify(current)) return false;
    setUndoStack((stack) => [...stack.slice(-39), current]);
    setRedoStack([]);
    setProposal(null);
    planRef.current = next;
    setPlan(next);
    return true;
  }, [canEdit, editing, generating]);

  const commit = useCallback((edit: (next: ChapterVisualPlanV1) => void) => {
    if (lockingRef.current) return false;
    return applyPlanEdit(edit);
  }, [applyPlanEdit]);

  const commitWhileLocked = useCallback((
    runtime: ChapterEditorRuntime,
    edit: (next: ChapterVisualPlanV1) => void,
  ) => {
    if (runtimeRef.current !== runtime || !lockingRef.current) return false;
    return applyPlanEdit(edit);
  }, [applyPlanEdit]);

  const persistDraft = useCallback(async (notify = false) => {
    const runtime = runtimeRef.current;
    const currentPlan = planRef.current;
    if (
      chapterId == null || !runtime || runtime.projectId !== projectId ||
      runtime.chapterId !== chapterId || !canEdit || !currentPlan || !editing
    ) return null;
    const pending = runtime.pendingSave;
    runtime.pendingSave = {
      plan: clonePlan(currentPlan),
      mode: modeRef.current,
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
          const saved = await saveChapterVisualPlanDraft(
            runtime.projectId,
            runtime.chapterId,
            request.mode,
            runtime.record,
            request.plan,
          );
          runtime.record = saved;
          lastSaved = saved;
          if (runtimeRef.current === runtime) {
            setPlanSet((current) => current ? { ...current, draft: saved } : current);
            setBaseline(JSON.stringify(request.plan));
            setBaselineMode(request.mode);
            if (request.notify) addToast(t("visualPlan.draftSaved"), "success");
          }
        } catch (error) {
          runtime.pendingSave = null;
          if (runtimeRef.current === runtime) {
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
  }, [addToast, canEdit, chapterId, editing, projectId]);

  useEffect(() => {
    if (!shouldAutoSave || !editable) return undefined;
    const timer = window.setTimeout(() => {
      if (!lockingRef.current) void persistDraft(false);
    }, 800);
    return () => window.clearTimeout(timer);
  }, [editable, mode, persistDraft, plan, shouldAutoSave]);

  const changeMode = useCallback((nextMode: ChapterVisualPlanAuthoringMode) => {
    modeRef.current = nextMode;
    setMode(nextMode);
  }, []);

  const ensureLatestDraft = useCallback(async (runtime: ChapterEditorRuntime) => {
    const activeSave = runtime.savePromise;
    if (activeSave) await activeSave;
    if (runtimeRef.current !== runtime) return null;

    const currentPlan = planRef.current;
    if (!currentPlan) return null;
    if (
      runtime.record &&
      JSON.stringify(currentPlan) === JSON.stringify(runtime.record.plan) &&
      modeRef.current === runtime.record.record.authoring_mode
    ) {
      return runtime.record;
    }
    return persistDraft(false);
  }, [persistDraft]);

  const undo = useCallback(() => {
    const current = planRef.current;
    const previous = undoStack.at(-1);
    if (!current || !previous || lockingRef.current) return;
    setUndoStack((stack) => stack.slice(0, -1));
    setRedoStack((stack) => [...stack, current]);
    const next = clonePlan(previous);
    planRef.current = next;
    setPlan(next);
  }, [undoStack]);

  const redo = useCallback(() => {
    const current = planRef.current;
    const nextValue = redoStack.at(-1);
    if (!current || !nextValue || lockingRef.current) return;
    setRedoStack((stack) => stack.slice(0, -1));
    setUndoStack((stack) => [...stack, current]);
    const next = clonePlan(nextValue);
    planRef.current = next;
    setPlan(next);
  }, [redoStack]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.matches("input, textarea, select, [contenteditable=true]")) return;
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
  }, [redo, undo]);

  useEffect(() => {
    if (!playing || !plan) return undefined;
    const timer = window.setInterval(() => {
      setPlayheadMs((current) => {
        const next = current + 50;
        if (next >= plan.frame.duration_ms) {
          setPlaying(false);
          return plan.frame.duration_ms;
        }
        return next;
      });
    }, 50);
    return () => window.clearInterval(timer);
  }, [plan, playing]);

  useEffect(() => {
    if (!plan) return;
    const active = visualShotAt(plan, playheadMs, shotRefs);
    if (!active) return;
    setSelectedShotId(active.shot.id);
    const nextKeyframe = nearestKeyframe(active.shot, playheadMs);
    setSelectedKeyframeId(nextKeyframe?.id ?? null);
    setSelectedEntityId((current) =>
      nextKeyframe?.entity_states.some((state) => state.entity_id === current)
        ? current
        : nextKeyframe?.entity_states[0]?.entity_id ?? null,
    );
  }, [plan, playheadMs, shotRefs]);

  useEffect(() => {
    if (!relationTarget) return;
    const valid = keyframe?.entity_states.some((state) =>
      state.entity_id === relationTarget && state.entity_id !== selectedEntityId && state.visible
    );
    if (!valid) setRelationTarget("");
  }, [keyframe, relationTarget, selectedEntityId]);

  useEffect(() => {
    setNodes((current) => sameStageNodes(current, stageNodes) ? current : stageNodes);
  }, [setNodes, stageNodes]);

  const edges = useMemo<Edge[]>(() => {
    const visibleNodeIds = new Set(stageNodes.map((node) => node.id));
    return (keyframe?.relations ?? [])
      .filter((relation) =>
        visibleNodeIds.has(relation.source_entity_id) &&
        visibleNodeIds.has(relation.target_entity_id),
      )
      .map((relation) => ({
      id: relation.id,
      source: relation.source_entity_id,
      target: relation.target_entity_id,
      label: relation.label ?? t(RELATION_LABELS[relation.kind]),
      markerEnd: { type: MarkerType.ArrowClosed },
      style: { stroke: "rgb(var(--color-primary))" },
      labelStyle: { fill: "rgb(var(--color-text-secondary))", fontSize: 10 },
      }));
  }, [keyframe, stageNodes, t]);

  const handleNodeChanges = useCallback((changes: NodeChange<StageNode>[]) => {
    applyNodeChanges(changes.filter((change) =>
      change.type === "position" || change.type === "dimensions" || change.type === "select"));
  }, [applyNodeChanges]);

  const persistNodePosition = useCallback((_: MouseEvent | TouchEvent, node: StageNode) => {
    if (!keyframe) return;
    commit((next) => {
      const target = visualShots(next)
        .flatMap(({ shot: item }) => item.keyframes)
        .find((item) => item.id === keyframe.id)
        ?.entity_states.find((state) => state.entity_id === node.id);
      if (!target) return;
      target.bounds.x = Math.max(0, Math.min(1 - target.bounds.width, node.position.x / STAGE_WIDTH));
      target.bounds.y = Math.max(0, Math.min(1 - target.bounds.height, node.position.y / STAGE_HEIGHT));
    });
  }, [commit, keyframe]);

  const selectShot = (ref: VisualShotRef) => {
    setPlaying(false);
    setSelectedShotId(ref.shot.id);
    setPlayheadMs(ref.shot.start_ms);
    setSelectedKeyframeId(ref.shot.keyframes[0]?.id ?? null);
    setSelectedEntityId(ref.shot.keyframes[0]?.entity_states[0]?.entity_id ?? null);
  };

  const startRevision = async () => {
    const runtime = runtimeRef.current;
    if (!plan || !canEdit || chapterId == null || !runtime || !acquireEditorLock(runtime)) return;
    const snapshot = clonePlan(plan);
    const snapshotMode = modeRef.current;
    setSaving(true);
    try {
      const saved = await saveChapterVisualPlanDraft(
        runtime.projectId,
        runtime.chapterId,
        snapshotMode,
        null,
        snapshot,
      );
      if (runtimeRef.current !== runtime) return;
      runtime.record = saved;
      setEditing(true);
      setBaseline(JSON.stringify(snapshot));
      setBaselineMode(snapshotMode);
      setPlanSet((current) => current ? { ...current, draft: saved } : current);
      addToast(t("visualPlan.draftCreated"), "success");
    } catch (error) {
      if (runtimeRef.current === runtime) {
        addToast(extractErrorMessage(error, t("visualPlan.draftSaveFailed")), "error");
      }
    } finally {
      if (runtimeRef.current === runtime) setSaving(false);
      releaseEditorLock(runtime);
    }
  };

  const generatePlan = async () => {
    const runtime = runtimeRef.current;
    if (planDirty) {
      addToast(t("visualPlan.waitForSaveBeforeAi"), "info");
      return;
    }
    if (chapterId == null || generating || !canEdit || !runtime || !acquireEditorLock(runtime)) return;
    try {
      if (runtime.record) {
      addToast(t("visualPlan.existingDraftUseAi"), "info");
        return;
      }
      const direction = await promptDialog({
        title: t("visualPlan.directorIntent"),
        message: t("visualPlan.directorIntentMessage"),
        placeholder: t("visualPlan.directorIntentPlaceholder"),
        multiline: true,
        confirmText: t("visualPlan.generatePlan"),
      });
      if (direction == null || runtimeRef.current !== runtime) return;
      const requestMode = modeRef.current;
      setGenerating(true);
      const taskId = createWorkflowTaskId("chapter-visual-plan", runtime.projectId);
      const generated = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: t("visualPlan.generateVisualPlan"),
          startMessage: t("visualPlan.understandingChapter"),
          successMessage: t("visualPlan.visualPlanGenerated"),
          errorMessage: t("visualPlan.visualPlanGenerationFailed"),
          total: 100,
          cancelLabel: t("workflow.stop"),
        },
        () => generateChapterVisualPlanDraft(
          runtime.projectId,
          runtime.chapterId,
          requestMode,
          direction.trim() || null,
          taskId,
        ),
      );
      if (runtimeRef.current !== runtime) return;
      runtime.record = generated;
      const next = clonePlan(generated.plan);
      setPlan(next);
      planRef.current = next;
      setBaseline(JSON.stringify(next));
      setBaselineMode(requestMode);
      changeMode(requestMode);
      setEditing(true);
      setPlanSet((current) => current ? { ...current, draft: generated } : current);
    } catch (error) {
      if (runtimeRef.current !== runtime) return;
      const message = extractErrorMessage(error, t("visualPlan.visualPlanGenerationFailed"));
      addToast(isCancelledOperation(message) ? t("workflow.cancelled") : message,
        isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (runtimeRef.current === runtime) setGenerating(false);
      releaseEditorLock(runtime);
    }
  };

  const askAiToRevise = async () => {
    const runtime = runtimeRef.current;
    if (!runtime?.record || chapterId == null || generating || !acquireEditorLock(runtime)) return;
    try {
      const latest = await ensureLatestDraft(runtime);
      if (!latest || runtimeRef.current !== runtime) return;
      const instruction = await promptDialog({
        title: t("visualPlan.proposeChanges"),
        message: t("visualPlan.proposeChangesMessage"),
        placeholder: t("visualPlan.proposeChangesPlaceholder"),
        multiline: true,
        required: true,
        confirmText: t("visualPlan.generateProposal"),
      });
      if (!instruction?.trim() || runtimeRef.current !== runtime) return;
      setGenerating(true);
      const taskId = createWorkflowTaskId("chapter-visual-plan-revision", runtime.projectId);
      const value = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: t("visualPlan.proposeChanges"),
          startMessage: t("visualPlan.comparingDraft"),
          successMessage: t("visualPlan.proposalGenerated"),
          errorMessage: t("visualPlan.proposalFailed"),
          total: 100,
          cancelLabel: t("workflow.stop"),
        },
        () => proposeChapterVisualPlanRevision(
          runtime.projectId,
          runtime.chapterId,
          runtime.record!,
          instruction,
          taskId,
        ),
      );
      if (runtimeRef.current !== runtime) return;
      setProposal(value);
    } catch (error) {
      if (runtimeRef.current === runtime) {
        addToast(extractErrorMessage(error, t("visualPlan.proposalFailed")), "error");
      }
    } finally {
      if (runtimeRef.current === runtime) {
        setGenerating(false);
        setSaving(false);
      }
      releaseEditorLock(runtime);
    }
  };

  const applyProposal = async () => {
    const runtime = runtimeRef.current;
    const currentProposal = proposal;
    if (!currentProposal || !runtime?.record || chapterId == null || saving || !acquireEditorLock(runtime)) return;
    setSaving(true);
    try {
      const latest = await ensureLatestDraft(runtime);
      if (!latest || runtimeRef.current !== runtime) return;
      if (latest.record.revision !== currentProposal.base_revision) {
        setProposal(null);
        addToast(t("visualPlan.proposalStale"), "info");
        return;
      }
      const saved = await applyChapterVisualPlanProposal(
        runtime.projectId,
        runtime.chapterId,
        currentProposal.proposal,
        currentProposal,
      );
      if (runtimeRef.current !== runtime) return;
      runtime.record = saved;
      const next = clonePlan(saved.plan);
      setPlan(next);
      planRef.current = next;
      setBaseline(JSON.stringify(next));
      setBaselineMode("collaborative");
      changeMode("collaborative");
      setProposal(null);
      setUndoStack([]);
      setRedoStack([]);
      setPlanSet((current) => current ? { ...current, draft: saved } : current);
      addToast(t("visualPlan.proposalApplied"), "success");
    } catch (error) {
      if (runtimeRef.current === runtime) {
        addToast(extractErrorMessage(error, t("visualPlan.applyProposalFailed")), "error");
      }
    } finally {
      if (runtimeRef.current === runtime) {
        setSaving(false);
      }
      releaseEditorLock(runtime);
    }
  };

  const confirmPlan = async () => {
    const runtime = runtimeRef.current;
    if (chapterId == null || !plan || issues.length > 0 || !runtime || !acquireEditorLock(runtime)) return;
    setSaving(true);
    try {
      const saved = await ensureLatestDraft(runtime);
      if (!saved || runtimeRef.current !== runtime) return;
      const accepted = await confirmDialog({
        title: t("visualPlan.confirmVisualPlan"),
        message: t("visualPlan.confirmVisualPlanMessage"),
        confirmText: t("visualPlan.confirmAndLock"),
        dialogType: "warning",
      });
      if (!accepted || runtimeRef.current !== runtime) return;
      const outcome = await confirmChapterVisualPlan(
        runtime.projectId,
        runtime.chapterId,
        saved,
      );
      if (runtimeRef.current !== runtime) return;
      runtime.record = null;
      const next = clonePlan(outcome.visual_plan.plan);
      setPlan(next);
      planRef.current = next;
      setBaseline(JSON.stringify(next));
      setBaselineMode(saved.record.authoring_mode);
      changeMode(saved.record.authoring_mode);
      setEditing(false);
      setPlanSet((current) => current ? {
        ...current,
        draft: null,
        confirmed: outcome.visual_plan,
      } : current);
      addToast(
        outcome.invalidated_project_ids.length
          ? t("visualPlan.planConfirmedDownstream", { count: outcome.invalidated_project_ids.length })
          : t("visualPlan.planConfirmed"),
        "success",
      );
    } catch (error) {
      if (runtimeRef.current === runtime) {
        addToast(extractErrorMessage(error, t("visualPlan.confirmPlanFailed")), "error");
      }
    } finally {
      if (runtimeRef.current === runtime) {
        setSaving(false);
      }
      releaseEditorLock(runtime);
    }
  };

  const discardDraft = async () => {
    const runtime = runtimeRef.current;
    if (!runtime || chapterId == null || !acquireEditorLock(runtime)) return;
    setSaving(true);
    try {
      runtime.pendingSave = null;
      if (runtime.savePromise) await runtime.savePromise;
      if (runtimeRef.current !== runtime) return;
      const currentRecord = runtime.record;
      if (!currentRecord) return;
      const accepted = await confirmDialog({
        title: t("visualPlan.discardDraft"),
        message: t("visualPlan.discardDraftMessage"),
        confirmText: t("visualPlan.discardDraftButton"),
        dialogType: "danger",
      });
      if (!accepted || runtimeRef.current !== runtime) return;
      await discardChapterVisualPlanDraft(
        runtime.projectId,
        runtime.chapterId,
        currentRecord,
      );
      if (runtimeRef.current !== runtime) return;
      await loadPlan();
      addToast(t("visualPlan.draftDiscarded"), "success");
    } catch (error) {
      if (runtimeRef.current === runtime) {
        addToast(extractErrorMessage(error, t("visualPlan.discardDraftFailed")), "error");
      }
    } finally {
      if (runtimeRef.current === runtime) {
        setSaving(false);
      }
      releaseEditorLock(runtime);
    }
  };

  const addEntity = async () => {
    const runtime = runtimeRef.current;
    if (!runtime || !shotRef || !keyframe || !acquireEditorLock(runtime)) return;
    try {
      const label = await promptDialog({
        title: t("visualPlan.addEntity", {
          kind: t(ENTITY_KINDS.find(([kind]) => kind === entityKind)?.[1] ?? "visualPlan.text"),
        }),
        message: t("visualPlan.addEntityMessage"),
        placeholder: t("visualPlan.entityNamePlaceholder"),
        required: true,
      });
      if (!label?.trim() || runtimeRef.current !== runtime) return;
      const entityId = id("entity");
      const additionId = id("addition");
      const added = commitWhileLocked(runtime, (next) => {
        const target = visualShots(next).find(({ shot: item }) => item.id === shotRef.shot.id);
        const frame = target?.shot.keyframes.find((item) => item.id === keyframe.id);
        if (!target || !frame) return;
        next.user_additions.push({
          id: additionId,
          kind: "entity",
          description: label.trim(),
          rationale: "用户在导演编排中添加",
          introduced_at_ms: Math.max(0, Math.min(playheadMs, next.frame.duration_ms - 1)),
        });
        next.entities.push({
          id: entityId,
          label: label.trim(),
          kind: entityKind,
          description: label.trim(),
          source_span_ids: [],
          user_addition_ids: [additionId],
        });
        frame.entity_states.push(defaultEntityState(entityId, frame.entity_states.length));
        if (!target.unit.participant_entity_ids.includes(entityId)) {
          target.unit.participant_entity_ids.push(entityId);
        }
      });
      if (added) setSelectedEntityId(entityId);
    } finally {
      releaseEditorLock(runtime);
    }
  };

  const addExistingEntityToFrame = (entityId: string) => {
    if (!shotRef || !keyframe) return;
    const added = commit((next) => {
      const target = visualShots(next).find(({ shot: item }) => item.id === shotRef.shot.id);
      const frame = target?.shot.keyframes.find((item) => item.id === keyframe.id);
      if (!target || !frame || frame.entity_states.some((state) => state.entity_id === entityId)) return;
      frame.entity_states.push(defaultEntityState(entityId, frame.entity_states.length));
      if (!target.unit.participant_entity_ids.includes(entityId)) {
        target.unit.participant_entity_ids.push(entityId);
      }
    });
    if (added) setSelectedEntityId(entityId);
  };

  const addKeyframe = () => {
    if (!shot || !keyframe || playheadMs <= shot.start_ms || playheadMs >= shot.end_ms) return;
    const keyframeId = id("kf");
    const added = commit((next) => {
      const target = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot;
      if (!target || target.keyframes.some((item) => item.at_ms === playheadMs)) return;
      target.keyframes.push({
        ...structuredClone(keyframe),
        id: keyframeId,
        at_ms: playheadMs,
        label: t("novel.keyframeLabel", { time: formatPlanTime(playheadMs) }),
        relations: keyframe.relations.map((relation) => ({
          ...structuredClone(relation),
          id: id("relation"),
        })),
      });
      target.keyframes.sort((left, right) => left.at_ms - right.at_ms);
    });
    if (added) setSelectedKeyframeId(keyframeId);
  };

  const removeKeyframe = () => {
    if (!shot || !keyframe || keyframe.at_ms === shot.start_ms || keyframe.at_ms === shot.end_ms) return;
    let fallbackId: string | null = null;
    const removed = commit((next) => {
      const target = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot;
      if (!target) return;
      target.keyframes = target.keyframes.filter((item) => item.id !== keyframe.id);
      fallbackId = nearestKeyframe(target, playheadMs)?.id ?? null;
    });
    if (removed) setSelectedKeyframeId(fallbackId);
  };

  const updateEntityState = (patch: Partial<VisualEntityStateV1>) => {
    if (!shot || !keyframe || !entityState) return;
    commit((next) => {
      const frame = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot
        .keyframes.find((item) => item.id === keyframe.id);
      const target = frame?.entity_states.find(
        (state) => state.entity_id === entityState.entity_id,
      );
      if (!frame || !target) return;
      Object.assign(target, patch);
      if (patch.visible === false) {
        frame.relations = frame.relations.filter((relation) =>
          relation.source_entity_id !== entityState.entity_id &&
          relation.target_entity_id !== entityState.entity_id
        );
      }
    });
  };

  const updateEntityBounds = (
    field: "x" | "y" | "width" | "height",
    percent: number,
  ) => {
    if (!entityState || !Number.isFinite(percent)) return;
    const bounds = { ...entityState.bounds };
    const value = percent / 100;
    if (field === "x") bounds.x = Math.max(0, Math.min(1 - bounds.width, value));
    if (field === "y") bounds.y = Math.max(0, Math.min(1 - bounds.height, value));
    if (field === "width") bounds.width = Math.max(0.01, Math.min(1 - bounds.x, value));
    if (field === "height") bounds.height = Math.max(0.01, Math.min(1 - bounds.y, value));
    updateEntityState({ bounds });
  };

  const addRelation = () => {
    const sourceState = keyframe?.entity_states.find(
      (state) => state.entity_id === selectedEntityId,
    );
    const targetState = keyframe?.entity_states.find(
      (state) => state.entity_id === relationTarget,
    );
    if (
      !shot || !keyframe || !selectedEntityId || !relationTarget ||
      !sourceState?.visible || !targetState?.visible ||
      relationTarget === selectedEntityId ||
      (relationKind === "custom" && !relationLabel.trim())
    ) return;
    commit((next) => {
      const frame = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot
        .keyframes.find((item) => item.id === keyframe.id);
      const source = frame?.entity_states.find(
        (state) => state.entity_id === selectedEntityId,
      );
      const target = frame?.entity_states.find(
        (state) => state.entity_id === relationTarget,
      );
      if (!frame || !source?.visible || !target?.visible) return;
      frame.relations.push({
        id: id("relation"),
        kind: relationKind,
        source_entity_id: selectedEntityId,
        target_entity_id: relationTarget,
        label: relationKind === "custom" ? relationLabel.trim() : null,
      });
    });
    setRelationLabel("");
  };

  const removeRelation = (relationId: string) => {
    if (!shot || !keyframe) return;
    commit((next) => {
      const frame = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot
        .keyframes.find((item) => item.id === keyframe.id);
      if (frame) frame.relations = frame.relations.filter((item) => item.id !== relationId);
    });
  };

  const removeTimedEvent = (eventId: string) => {
    if (!shot) return;
    commit((next) => {
      removeVisualPlanTimedEvent(next, shot.id, eventId);
    });
  };

  const updateTimedEventRange = (
    eventId: string,
    boundary: "start" | "end",
    seconds: number,
  ) => {
    if (!shot || !Number.isFinite(seconds)) return;
    commit((next) => {
      updateVisualPlanTimedEventRange(next, shot.id, eventId, boundary, seconds * 1000);
    });
  };

  const addEntityEvent = async (kind: VisualEntityEventKindV1) => {
    const runtime = runtimeRef.current;
    if (!runtime || !shot || !selectedEntityId || !acquireEditorLock(runtime)) return;
    try {
      const kindLabel = t(ENTITY_EVENT_LABELS[kind]);
      const description = await promptDialog({
        title: t("visualPlan.addEvent", { kind: kindLabel }),
        placeholder: kind === "dialogue"
          ? t("visualPlan.eventDialoguePlaceholder")
          : t("visualPlan.eventChangePlaceholder", { kind: kindLabel }),
        required: true,
      });
      if (!description?.trim() || runtimeRef.current !== runtime) return;
      const additionId = id("addition");
      const [startMs, endMs] = eventRange(shot, playheadMs);
      commitWhileLocked(runtime, (next) => {
        const target = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot;
        if (!target || !next.entities.some((entity) => entity.id === selectedEntityId)) return;
        next.user_additions.push({
          id: additionId,
          kind: kind === "dialogue" ? "dialogue" : "action",
          description: description.trim(),
          rationale: "用户在导演编排中添加",
          introduced_at_ms: startMs,
        });
        target.entity_events.push({
          id: id("event"),
          kind,
          start_ms: startMs,
          end_ms: endMs,
          entity_id: selectedEntityId,
          target_entity_id: null,
          description: description.trim(),
          source_span_ids: [],
          user_addition_ids: [additionId],
          parameters: {},
        });
      });
    } finally {
      releaseEditorLock(runtime);
    }
  };

  const addSoundEvent = async (kind: VisualSoundEventKindV1) => {
    const runtime = runtimeRef.current;
    if (!runtime || !shot || !acquireEditorLock(runtime)) return;
    try {
      const content = await promptDialog({
        title: t("visualPlan.addEvent", { kind: t(SOUND_EVENT_LABELS[kind]) }),
        placeholder: kind === "silence"
          ? t("visualPlan.silencePlaceholder")
          : t("visualPlan.soundPlaceholder"),
        required: true,
      });
      if (!content?.trim() || runtimeRef.current !== runtime) return;
      const additionId = id("addition");
      const [startMs, endMs] = eventRange(shot, playheadMs);
      commitWhileLocked(runtime, (next) => {
        const target = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot;
        if (!target) return;
        next.user_additions.push({
          id: additionId,
          kind: kind === "dialogue" ? "dialogue" : "sound",
          description: content.trim(),
          rationale: "用户在导演编排中添加",
          introduced_at_ms: startMs,
        });
        target.sound_events.push({
          id: id("sound"),
          kind,
          start_ms: startMs,
          end_ms: endMs,
          speaker_entity_id: ["dialogue", "voice_over", "narration"].includes(kind)
            ? selectedEntityId
            : null,
          content: content.trim(),
          source_span_ids: [],
          user_addition_ids: [additionId],
          parameters: {},
        });
      });
    } finally {
      releaseEditorLock(runtime);
    }
  };

  const addCameraEvent = async (kind: VisualCameraEventKindV1) => {
    const runtime = runtimeRef.current;
    if (!runtime || !shot || !acquireEditorLock(runtime)) return;
    const targetEntityId = selectedEntityId;
    try {
      const description = await promptDialog({
        title: t("visualPlan.addEvent", { kind: t(CAMERA_EVENT_LABELS[kind]) }),
        placeholder: t("visualPlan.cameraEventPlaceholder"),
        required: true,
      });
      if (!description?.trim() || runtimeRef.current !== runtime) return;
      const [startMs, endMs] = eventRange(shot, playheadMs);
      const additionId = id("addition");
      commitWhileLocked(runtime, (next) => {
        const target = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot;
        if (!target) return;
        next.user_additions.push({
          id: additionId,
          kind: "other",
          description: description.trim(),
          rationale: "用户在导演编排中添加",
          introduced_at_ms: startMs,
        });
        const validTargetEntityId = targetEntityId && next.entities.some(
          (entity) => entity.id === targetEntityId,
        ) ? targetEntityId : null;
        target.camera_events.push({
          id: id("camera"),
          kind,
          start_ms: startMs,
          end_ms: endMs,
          target_entity_id: validTargetEntityId,
          description: description.trim(),
          source_span_ids: [],
          user_addition_ids: [additionId],
          parameters: {},
        });
      });
    } finally {
      releaseEditorLock(runtime);
    }
  };

  const addTechniqueEvent = async (kind: VisualTechniqueKindV1) => {
    const runtime = runtimeRef.current;
    if (!runtime || !shot || !acquireEditorLock(runtime)) return;
    try {
      const description = await promptDialog({
        title: t("visualPlan.addEvent", { kind: t(TECHNIQUE_LABELS[kind]) }),
        placeholder: t("visualPlan.techniquePlaceholder"),
        required: true,
      });
      if (!description?.trim() || runtimeRef.current !== runtime) return;
      const [startMs, endMs] = eventRange(shot, playheadMs);
      const additionId = id("addition");
      commitWhileLocked(runtime, (next) => {
        const target = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot;
        if (!target) return;
        next.user_additions.push({
          id: additionId,
          kind: "visual_motif",
          description: description.trim(),
          rationale: "用户在导演编排中添加",
          introduced_at_ms: startMs,
        });
        target.technique_events.push({
          id: id("technique"),
          kind,
          start_ms: startMs,
          end_ms: endMs,
          description: description.trim(),
          source_span_ids: [],
          user_addition_ids: [additionId],
          parameters: {},
        });
      });
    } finally {
      releaseEditorLock(runtime);
    }
  };

  const assignSpan = (spanId: string) => {
    if (!shotRef) return;
    commit((next) => {
      const disposition = next.source_dispositions.find((item) => item.source_span_id === spanId);
      const target = visualShots(next).find(({ shot: item }) => item.id === shotRef.shot.id);
      if (!disposition || !target) return;
      if (
        disposition.disposition !== "adapt" ||
        disposition.narrative_unit_id !== target.unit.id
      ) {
        detachVisualPlanSourceSpanReferences(next, spanId);
        disposition.shot_ids = [];
      }
      disposition.disposition = "adapt";
      disposition.reason = null;
      disposition.narrative_unit_id = target.unit.id;
      if (!disposition.shot_ids.includes(target.shot.id)) disposition.shot_ids.push(target.shot.id);
      if (!target.shot.source_span_ids.includes(spanId)) target.shot.source_span_ids.push(spanId);
      if (!target.unit.source_span_ids.includes(spanId)) target.unit.source_span_ids.push(spanId);
    });
  };

  const omitSpan = async (spanId: string) => {
    const runtime = runtimeRef.current;
    if (!runtime || !acquireEditorLock(runtime)) return;
    try {
      const reason = await promptDialog({
        title: t("visualPlan.omitSource"),
        message: t("visualPlan.omitSourceMessage"),
        placeholder: t("visualPlan.omitSourcePlaceholder"),
        required: true,
      });
      if (!reason?.trim() || runtimeRef.current !== runtime) return;
      commitWhileLocked(runtime, (next) => {
        const disposition = next.source_dispositions.find((item) => item.source_span_id === spanId);
        if (!disposition) return;
        detachVisualPlanSourceSpanReferences(next, spanId);
        disposition.disposition = "omit";
        disposition.reason = reason.trim();
        disposition.narrative_unit_id = null;
        disposition.shot_ids = [];
      });
    } finally {
      releaseEditorLock(runtime);
    }
  };

  const setTransition = (kind: ShootingTransitionTypeV1) => {
    if (!plan || !shot) return;
    const shots = visualShots(plan);
    const index = shots.findIndex(({ shot: item }) => item.id === shot.id);
    const nextShot = shots[index + 1]?.shot;
    if (!nextShot) return;
    commit((next) => {
      const existing = next.transitions.find((item) => item.from_shot_id === shot.id);
      if (existing) {
        existing.kind = kind;
        existing.to_shot_id = nextShot.id;
        existing.at_ms = shot.end_ms;
        existing.duration_ms = kind === "continuous" ? 0 : Math.min(250, nextShot.end_ms - nextShot.start_ms);
      } else {
        next.transitions.push({
          id: id("transition"),
          from_shot_id: shot.id,
          to_shot_id: nextShot.id,
          at_ms: shot.end_ms,
          duration_ms: kind === "continuous" ? 0 : Math.min(250, nextShot.end_ms - nextShot.start_ms),
          kind,
          preserved_action: null,
          preserved_sound: null,
          notes: null,
        });
      }
    });
  };

  const renderSourcePanelContent = () => (
    <div className="space-y-3">
      <details open className="group">
        <summary className="cursor-pointer px-1 py-1 text-small font-semibold text-book-text-main">{t("visualPlan.sourceProcessing")}</summary>
        <div className="mt-1 space-y-1.5">
          {planSet!.source.source_spans.map((span) => {
            const disposition = sourceDispositionBySpanId.get(span.id);
            const assigned = Boolean(shot && disposition?.shot_ids.includes(shot.id));
            return (
              <article key={span.id} className="rounded-lg border border-book-border/60 bg-book-bg-paper p-2">
                <p className="line-clamp-4 text-small leading-relaxed text-book-text-sub">{span.text}</p>
                <div className="mt-1.5 flex flex-wrap gap-1">
                  <button type="button" className={`rounded px-1.5 py-0.5 text-caption ${assigned ? "bg-book-primary text-white" : "bg-book-bg-tertiary text-book-text-muted"}`} onClick={() => assignSpan(span.id)} disabled={!editable || !shot}>
                    {assigned ? t("visualPlan.assignedToShot") : t("visualPlan.assignToShot")}
                  </button>
                  <button type="button" className="rounded bg-book-bg-tertiary px-1.5 py-0.5 text-caption text-book-text-muted" onClick={() => void omitSpan(span.id)} disabled={!editable}>
                    {disposition?.disposition === "omit" ? t("visualPlan.omitted") : t("visualPlan.omit")}
                  </button>
                </div>
                {disposition?.reason ? <p className="mt-1 text-caption text-warning">{disposition.reason}</p> : null}
              </article>
            );
          })}
        </div>
      </details>

      <details open className="border-t border-book-border/60 pt-2">
        <summary className="cursor-pointer px-1 py-1 text-small font-semibold text-book-text-main">{t("visualPlan.sceneEntities")}</summary>
        <div className="mt-1 flex gap-1">
          <ThemedSelect aria-label={t("visualPlan.newEntityType")} className="pyqt-input min-w-0 flex-1 !min-h-8 !py-1 text-small" value={entityKind} onChange={(event) => setEntityKind(event.target.value as VisualEntityKindV1)} disabled={!editable}>
            {ENTITY_KINDS.map(([kind, label]) => <option key={kind} value={kind}>{t(label)}</option>)}
          </ThemedSelect>
          <BookIconButton label={t("visualPlan.addEntityButton")} size="sm" onClick={() => void addEntity()} disabled={!editable}><Plus size={14} /></BookIconButton>
        </div>
        <div className="mt-2 space-y-1">
          {plan!.entities.map((item) => {
            const inFrame = keyframe?.entity_states.some((state) => state.entity_id === item.id);
            return (
              <button key={item.id} type="button" onClick={() => inFrame ? setSelectedEntityId(item.id) : addExistingEntityToFrame(item.id)} disabled={!editable && !inFrame} className={`flex min-h-9 w-full items-center gap-2 rounded-lg border px-2 text-left text-small ${selectedEntityId === item.id ? "border-book-primary bg-book-primary/8" : "border-book-border/60 bg-book-bg-paper"}`}>
                {item.kind === "character" ? <UserRound size={13} /> : <Box size={13} />}
                <span className="min-w-0 flex-1 truncate">{item.label}</span>
                <span className="text-caption text-book-text-muted">{inFrame ? t("visualPlan.inFrame") : t("visualPlan.addToFrame")}</span>
              </button>
            );
          })}
          {plan!.entities.length === 0 ? <p className="px-1 py-2 text-caption leading-relaxed text-book-text-muted">{t("visualPlan.noEntitiesHint")}</p> : null}
        </div>
      </details>

      <details className="border-t border-book-border/60 pt-2">
        <summary className="cursor-pointer px-1 py-1 text-small font-semibold text-book-text-main">{t("visualPlan.accessibilityTree")}</summary>
        <ol className="mt-1 space-y-1 text-small text-book-text-sub">
          {visualShots(plan!).map((item) => (
            <li key={item.shot.id}>
              <button type="button" className="w-full rounded px-2 py-1 text-left hover:bg-book-bg-tertiary focus-visible:outline-2 focus-visible:outline-book-primary" onClick={() => selectShot(item)}>
                {formatPlanTime(item.shot.start_ms)}–{formatPlanTime(item.shot.end_ms)} · {item.shot.title}
              </button>
              {item.shot.id === shot?.id ? (
                <ul className="ml-3 border-l border-book-border pl-2 text-caption text-book-text-muted">
                  {item.shot.keyframes.map((frame) => <li key={frame.id}><button type="button" className="py-1 hover:text-book-primary" onClick={() => { setPlayheadMs(frame.at_ms); setSelectedKeyframeId(frame.id); }}>{formatPlanTime(frame.at_ms)} · {frame.label}</button></li>)}
                </ul>
              ) : null}
            </li>
          ))}
        </ol>
      </details>
    </div>
  );

  const renderInspectorPanelContent = () => (
    shotRef && shot && keyframe ? (
      <div className="space-y-4">
        <section>
          <div className="flex items-center justify-between gap-2">
            <h4 className="text-small font-semibold text-book-text-main">{t("visualPlan.shot")}</h4>
            <div className="flex gap-1">
              <BookIconButton label={t("visualPlan.moveShotEarlier")} size="sm" disabled={!editable || shotRef.shotIndex === 0} onClick={() => commit((next) => Object.assign(next, moveVisualShot(next, shot.id, -1)))}><ArrowLeft size={13} /></BookIconButton>
              <BookIconButton label={t("visualPlan.moveShotLater")} size="sm" disabled={!editable || shotRef.shotIndex === shotRef.unit.shots.length - 1} onClick={() => commit((next) => Object.assign(next, moveVisualShot(next, shot.id, 1)))}><ArrowRight size={13} /></BookIconButton>
            </div>
          </div>
          <label className="mt-2 block text-caption font-semibold text-book-text-muted">{t("visualPlan.titleLabel")}
            <input key={`${shot.id}-title-${shot.title}`} defaultValue={shot.title} disabled={!editable} onBlur={(event) => commit((next) => { const target = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot; if (target) target.title = event.currentTarget.value; })} className="pyqt-input mt-1 w-full !min-h-8 text-small" />
          </label>
          <label className="mt-2 block text-caption font-semibold text-book-text-muted">{t("visualPlan.narrativePurpose")}
            <textarea key={`${shot.id}-purpose-${shot.narrative_purpose}`} defaultValue={shot.narrative_purpose} disabled={!editable} onBlur={(event) => commit((next) => { const target = visualShots(next).find(({ shot: item }) => item.id === shot.id)?.shot; if (target) target.narrative_purpose = event.currentTarget.value; })} className="pyqt-input mt-1 min-h-16 w-full resize-y text-small" />
          </label>
          <label className="mt-2 block text-caption font-semibold text-book-text-muted">{t("visualPlan.durationSeconds")}
            <input key={`${shot.id}-${shot.end_ms}`} type="number" min="0.25" step="0.1" defaultValue={(shot.end_ms - shot.start_ms) / 1000} disabled={!editable} onBlur={(event) => { const seconds = Number(event.currentTarget.value); if (Number.isFinite(seconds)) commit((next) => Object.assign(next, resizeVisualShot(next, shot.id, seconds * 1000))); }} className="pyqt-input mt-1 w-full !min-h-8 text-small" />
          </label>
          <label className="mt-2 block text-caption font-semibold text-book-text-muted">{t("visualPlan.transitionToNext")}
            <ThemedSelect value={plan!.transitions.find((item) => item.from_shot_id === shot.id)?.kind ?? "continuous"} disabled={!editable || visualShots(plan!).at(-1)?.shot.id === shot.id} onChange={(event) => setTransition(event.target.value as ShootingTransitionTypeV1)} className="pyqt-input mt-1 w-full !min-h-8 text-small">
              {TRANSITION_KINDS.map(([kind, label]) => <option key={kind} value={kind}>{t(label)}</option>)}
            </ThemedSelect>
          </label>
        </section>

        <section className="border-t border-book-border/60 pt-3">
          <div className="flex items-center justify-between gap-2">
            <h4 className="text-small font-semibold text-book-text-main">{t("visualPlan.keyframe")} · {formatPlanTime(keyframe.at_ms)}</h4>
            <BookIconButton label={t("visualPlan.deleteKeyframe")} size="sm" disabled={!editable || keyframe.at_ms === shot.start_ms || keyframe.at_ms === shot.end_ms} onClick={removeKeyframe}><Trash2 size={13} /></BookIconButton>
          </div>
          <label className="mt-2 block text-caption font-semibold text-book-text-muted">{t("visualPlan.frameDescription")}
            <input key={`${keyframe.id}-label-${keyframe.label}`} defaultValue={keyframe.label} disabled={!editable} onBlur={(event) => commit((next) => { const frame = visualShots(next).flatMap(({ shot: item }) => item.keyframes).find((item) => item.id === keyframe.id); if (frame) frame.label = event.currentTarget.value; })} className="pyqt-input mt-1 w-full !min-h-8 text-small" />
          </label>
          <label className="mt-2 block text-caption font-semibold text-book-text-muted">{t("visualPlan.cameraZoomLabel")}
            <input type="range" min="0.25" max="4" step="0.05" value={keyframe.camera.zoom} disabled={!editable} onChange={(event) => commit((next) => { const frame = visualShots(next).flatMap(({ shot: item }) => item.keyframes).find((item) => item.id === keyframe.id); if (frame) frame.camera.zoom = Number(event.currentTarget.value); })} className="mt-1 w-full" />
          </label>
        </section>

        <section className="border-t border-book-border/60 pt-3">
          <h4 className="text-small font-semibold text-book-text-main">{t("visualPlan.entityState")}</h4>
          {entity && entityState ? (
            <div className="mt-2 space-y-2">
              <p className="rounded-lg bg-book-bg-tertiary px-2 py-1.5 text-small font-semibold text-book-text-main">{entity.label}</p>
              {(["action", "expression", "pose", "dialogue"] as const).map((field) => (
                <label key={field} className="block text-caption font-semibold text-book-text-muted">{t(({ action: "visualPlan.action", expression: "visualPlan.expression", pose: "visualPlan.pose", dialogue: "visualPlan.dialogue" } as const)[field])}
                  <input value={entityState[field] ?? ""} disabled={!editable} onChange={(event) => updateEntityState({ [field]: event.currentTarget.value || null })} className="pyqt-input mt-1 w-full !min-h-8 text-small" />
                </label>
              ))}
              <fieldset>
                <legend className="text-caption font-semibold text-book-text-muted">{t("visualPlan.positionAndSize")}</legend>
                <div className="mt-1 grid grid-cols-2 gap-1.5">
                  {([
                    ["x", t("visualPlan.left"), (1 - entityState.bounds.width) * 100],
                    ["y", t("visualPlan.top"), (1 - entityState.bounds.height) * 100],
                    ["width", t("visualPlan.width"), (1 - entityState.bounds.x) * 100],
                    ["height", t("visualPlan.height"), (1 - entityState.bounds.y) * 100],
                  ] as const).map(([field, label, max]) => (
                    <label key={field} className="text-caption font-semibold text-book-text-muted">
                      {label}
                      <input
                        key={`${keyframe.id}-${entityState.entity_id}-${field}-${entityState.bounds[field]}`}
                        type="number"
                        min={field === "width" || field === "height" ? 1 : 0}
                        max={Math.round(max)}
                        step="1"
                        defaultValue={Math.round(entityState.bounds[field] * 100)}
                        disabled={!editable}
                        onBlur={(event) => updateEntityBounds(field, Number(event.currentTarget.value))}
                        className="pyqt-input mt-1 w-full !min-h-8 text-small"
                      />
                    </label>
                  ))}
                </div>
                <p className="mt-1 text-caption leading-relaxed text-book-text-muted">{t("visualPlan.boundsHint")}</p>
              </fieldset>
              <label className="flex items-center gap-2 text-small text-book-text-sub"><input type="checkbox" checked={entityState.visible} disabled={!editable} onChange={(event) => updateEntityState({ visible: event.currentTarget.checked })} />{t("visualPlan.visibleInFrame")}</label>
              <p className="text-caption leading-relaxed text-book-text-muted">{t("visualPlan.hiddenEntityHint")}</p>
            </div>
          ) : <p className="mt-2 text-caption leading-relaxed text-book-text-muted">{t("visualPlan.selectEntityHint")}</p>}
        </section>

        <section className="border-t border-book-border/60 pt-3">
          <h4 className="text-small font-semibold text-book-text-main">{t("visualPlan.positionRelations")}</h4>
          <div className="mt-2 grid grid-cols-1 gap-1.5">
            <ThemedSelect aria-label={t("visualPlan.relationType")} value={relationKind} onChange={(event) => setRelationKind(event.currentTarget.value as VisualRelationKindV1)} disabled={!editable || !entityState?.visible} className="pyqt-input !min-h-8 text-small">{RELATION_KINDS.map(([kind, label]) => <option key={kind} value={kind}>{t(label)}</option>)}</ThemedSelect>
            <ThemedSelect aria-label={t("visualPlan.relationTarget")} value={relationTarget} onChange={(event) => setRelationTarget(event.currentTarget.value)} disabled={!editable || !entityState?.visible} className="pyqt-input !min-h-8 text-small"><option value="">{t("visualPlan.selectTarget")}</option>{keyframe.entity_states.filter((state) => state.visible && state.entity_id !== selectedEntityId).map((state) => <option key={state.entity_id} value={state.entity_id}>{entityById.get(state.entity_id)?.label ?? state.entity_id}</option>)}</ThemedSelect>
            {relationKind === "custom" ? (
              <label className="text-caption font-semibold text-book-text-muted">
                {t("visualPlan.relationDescription")}
                <input value={relationLabel} onChange={(event) => setRelationLabel(event.currentTarget.value)} disabled={!editable || !entityState?.visible} placeholder={t("visualPlan.relationLabelPlaceholder")} className="pyqt-input mt-1 w-full !min-h-8 text-small" />
              </label>
            ) : null}
            <BookButton size="sm" variant="secondary" disabled={!editable || !entityState?.visible || !relationTarget || (relationKind === "custom" && !relationLabel.trim())} onClick={addRelation}>{t("visualPlan.addRelation")}</BookButton>
          </div>
          {!entityState?.visible && selectedEntityId ? <p className="mt-1.5 text-caption leading-relaxed text-book-text-muted">{t("visualPlan.relationVisibilityHint")}</p> : null}
          <RelationList entityById={entityById} keyframe={keyframe} editable={editable} onRemove={removeRelation} />
        </section>

        <section className="border-t border-book-border/60 pt-3">
          <h4 className="text-small font-semibold text-book-text-main">{t("visualPlan.timelineSymbols")}</h4>
          <div className="mt-2 space-y-2">
            <EventKindControl label="visualPlan.entityEvents" value={entityEventKind} options={ENTITY_EVENT_KINDS} disabled={!editable || !selectedEntityId} onChange={setEntityEventKind} onAdd={() => void addEntityEvent(entityEventKind)} />
            <EventKindControl label="visualPlan.cameraEvents" value={cameraEventKind} options={CAMERA_EVENT_KINDS} disabled={!editable} onChange={setCameraEventKind} onAdd={() => void addCameraEvent(cameraEventKind)} />
            <EventKindControl label="visualPlan.soundEvents" value={soundEventKind} options={SOUND_EVENT_KINDS} disabled={!editable} icon={<Volume2 size={12} aria-hidden="true" />} onChange={setSoundEventKind} onAdd={() => void addSoundEvent(soundEventKind)} />
            <EventKindControl label="visualPlan.techniques" value={techniqueKind} options={TECHNIQUE_KINDS} disabled={!editable} icon={<Sparkles size={12} aria-hidden="true" />} onChange={setTechniqueKind} onAdd={() => void addTechniqueEvent(techniqueKind)} />
          </div>
          <div className="mt-3 space-y-2">
            <TimedEventList label={t("visualPlan.entityEvents")} events={shot.entity_events} editable={editable} kindLabel={(event) => t(ENTITY_EVENT_LABELS[event.kind])} onRemove={removeTimedEvent} onRangeChange={updateTimedEventRange} />
            <TimedEventList label={t("visualPlan.cameraEvents")} events={shot.camera_events} editable={editable} kindLabel={(event) => t(CAMERA_EVENT_LABELS[event.kind])} onRemove={removeTimedEvent} onRangeChange={updateTimedEventRange} />
            <TimedEventList label={t("visualPlan.soundEvents")} events={shot.sound_events} editable={editable} kindLabel={(event) => t(SOUND_EVENT_LABELS[event.kind])} onRemove={removeTimedEvent} onRangeChange={updateTimedEventRange} />
            <TimedEventList label={t("visualPlan.techniques")} events={shot.technique_events} editable={editable} kindLabel={(event) => t(TECHNIQUE_LABELS[event.kind])} onRemove={removeTimedEvent} onRangeChange={updateTimedEventRange} />
          </div>
        </section>

        {issues.length > 0 ? (
          <section role="alert" className="rounded-lg border border-warning/30 bg-warning-bg p-2">
            <h4 className="text-small font-semibold text-warning">{t("visualPlan.issues", { count: issues.length })}</h4>
            <ul className="mt-1 list-disc space-y-1 pl-4 text-caption leading-relaxed text-book-text-sub">{issues.slice(0, 8).map((issue, index) => <li key={`${issue.key}-${index}`}>{translateVisualPlanIssue(issue, t)}</li>)}</ul>
          </section>
        ) : <p className="rounded-lg bg-success-bg p-2 text-caption text-success">{t("visualPlan.validationPassed")}</p>}
      </div>
    ) : <EmptyHint text={t("visualPlan.selectShot")} />
  );

  if (!chapter) return <EmptyHint text={t("visualPlan.noChapter")} />;
  if (loading || !plan || !planSet) {
    if (loadError && !loading) {
      return (
        <div role="alert" className="a4n-flat-surface flex flex-wrap items-center gap-3 p-4">
          <p className="min-w-0 flex-1 text-small leading-relaxed text-error">
            {t("visualPlan.loadFailed")}: {loadError}
          </p>
          <BookButton size="sm" variant="secondary" onClick={() => void loadPlan()}>
            {t("visualPlan.retryLoad")}
          </BookButton>
        </div>
      );
    }
    return <div className="a4n-flat-surface py-14"><LoadingSpinner size="md" text={t("visualPlan.loadingPlan")} inline={false} /></div>;
  }

  const statusText = record
    ? t("visualPlan.draftRevision", {
        version: record.record.plan_version,
        revision: record.record.revision,
      })
    : planSet.confirmed
      ? t("visualPlan.confirmedVersion", {
          version: planSet.confirmed.record.plan_version,
        })
      : t("visualPlan.draftNotSaved");
  const pendingInitialMode = !record && modeDirty && !planDirty;
  const mobilePrimaryAction = !editing
    ? {
        label: t("visualPlan.draftCreated"),
        icon: <Plus size={13} aria-hidden="true" />,
        disabled: !canEdit || saving || locking,
        onClick: () => void startRevision(),
      }
    : record
      ? {
          label: t("visualPlan.confirmPlan"),
          icon: <CheckCircle2 size={13} aria-hidden="true" />,
          disabled: !canEdit || saving || generating || locking || issues.length > 0,
          onClick: () => void confirmPlan(),
        }
      : {
          label: t("visualPlan.generateDraft"),
          icon: <Bot size={13} aria-hidden="true" />,
          disabled: !canEdit || generating || saving || locking || planDirty,
          onClick: () => void generatePlan(),
        };
  const mobileActionItems = [
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
    ...(editing && record
      ? [{
          label: t("visualPlan.proposeChanges"),
          icon: <Sparkles size={14} aria-hidden="true" />,
          onClick: () => void askAiToRevise(),
          disabled: generating || saving || locking,
        }]
      : []),
    ...(editing
      ? [{
          label: t("common.save"),
          icon: <Save size={14} aria-hidden="true" />,
          onClick: () => void persistDraft(true),
          disabled: saving || locking || !shouldAutoSave,
        }]
      : []),
    ...(editing && record
      ? [{
          label: t("visualPlan.discardDraftButton"),
          icon: <Trash2 size={14} aria-hidden="true" />,
          onClick: () => void discardDraft(),
          disabled: saving || locking,
          danger: true,
        }]
      : []),
  ];

  return (
    <section className="a4n-visual-plan flex min-h-[680px] min-w-0 flex-col gap-2 pb-16 md:pb-0 xl:h-full xl:min-h-0">
      <header className="a4n-flat-surface flex shrink-0 flex-wrap items-center gap-2 px-3 py-2">
        <div className="min-w-0 basis-full xl:flex-1 xl:basis-auto">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="font-serif text-body-md font-semibold text-book-text-main">{t("visualPlan.title")}</h3>
            <span className={`max-w-full truncate rounded-full px-2 py-0.5 text-caption font-semibold ${record ? "bg-warning-bg text-warning" : planSet.confirmed ? "bg-success-bg text-success" : "bg-book-bg-tertiary text-book-text-muted"}`}>
              {statusText}
            </span>
            {saving
              ? <span role="status" className="text-caption text-book-text-muted">{t("common.loading")}</span>
              : shouldAutoSave
                ? <span className="text-caption text-warning">{t("writing.unsavedChanges")}</span>
                : pendingInitialMode
                  ? <span className="text-caption text-book-text-muted">{t("visualPlan.selectedModePending")}</span>
              : null}
          </div>
          <p className="mt-0.5 hidden text-small text-book-text-muted xl:block">{t("visualPlan.stageDescription")}</p>
        </div>
        <SegmentedControl
          className="!hidden max-w-full xl:!flex"
          ariaLabel={t("visualPlan.mode")}
          options={MODE_OPTIONS.map((option) => ({
            ...option,
            label: t(option.label),
            disabled: !editable,
          }))}
          value={mode}
          onChange={changeMode}
        />
        <div className="hidden ml-auto flex-wrap items-center justify-end gap-1.5 xl:flex">
          <BookIconButton label={t("common.undo")} size="sm" onClick={undo} disabled={!editable || undoStack.length === 0}><Undo2 size={14} /></BookIconButton>
          <BookIconButton label={t("common.redo")} size="sm" onClick={redo} disabled={!editable || redoStack.length === 0}><Redo2 size={14} /></BookIconButton>
          {!editing && canEdit ? (
              <BookButton size="sm" variant="secondary" onClick={() => void startRevision()} disabled={saving || locking}><Plus size={13} />{t("visualPlan.draftCreated")}</BookButton>
          ) : null}
          {editing && canEdit ? (
            <>
              <BookButton
                size="sm"
                variant="secondary"
                onClick={() => void generatePlan()}
                disabled={generating || saving || locking || Boolean(record) || planDirty}
                title={record
                  ? t("visualPlan.existingDraftUseAi")
                  : planDirty
                    ? t("visualPlan.waitForSaveBeforeAi")
                    : undefined}
              ><Bot size={13} />{t("visualPlan.generateDraft")}</BookButton>
              <BookButton size="sm" variant="secondary" onClick={() => void askAiToRevise()} disabled={generating || saving || locking || !record}><Sparkles size={13} />{t("visualPlan.proposeChanges")}</BookButton>
              <BookButton size="sm" variant="secondary" onClick={() => void persistDraft(true)} disabled={saving || locking || !shouldAutoSave}><Save size={13} />{t("common.save")}</BookButton>
              <BookButton size="sm" onClick={() => void confirmPlan()} disabled={saving || generating || locking || issues.length > 0}><CheckCircle2 size={13} />{t("visualPlan.confirmPlan")}</BookButton>
              {record ? <BookIconButton label={t("visualPlan.discardDraftButton")} size="sm" onClick={() => void discardDraft()} disabled={saving || locking}><Trash2 size={14} /></BookIconButton> : null}
            </>
          ) : null}
        </div>
        <div className="flex w-full min-w-0 items-center gap-2 xl:hidden">
          <label className="min-w-0 flex-1">
            <span className="sr-only">{t("visualPlan.mode")}</span>
            <ThemedSelect
              aria-label={t("visualPlan.mode")}
              value={mode}
              onChange={(event) => changeMode(event.currentTarget.value as ChapterVisualPlanAuthoringMode)}
              disabled={!editable}
              className="a4n-compact-select pyqt-input w-full text-small"
            >
              {MODE_OPTIONS.map((option) => <option key={option.id} value={option.id}>{t(option.label)}</option>)}
            </ThemedSelect>
          </label>
          <BookButton
            size="sm"
            onClick={mobilePrimaryAction.onClick}
            disabled={mobilePrimaryAction.disabled}
            className="shrink-0 !px-2.5"
          >
            {mobilePrimaryAction.icon}
            {mobilePrimaryAction.label}
          </BookButton>
          <Dropdown
            items={mobileActionItems}
            ariaLabel={t("common.more")}
          />
        </div>
      </header>

      {proposal ? (
        <div role="status" className="flex shrink-0 flex-wrap items-center gap-2 rounded-xl border border-info/30 bg-info-bg px-3 py-2 text-small text-book-text-sub">
          <Sparkles size={14} className="text-info" aria-hidden="true" />
          <span className="min-w-0 flex-1">{t("visualPlan.proposalGenerated")}。{t("visualPlan.proposeChangesMessage")}</span>
          <BookButton size="sm" variant="ghost" onClick={() => setProposal(null)} disabled={saving || locking}>{t("common.cancel")}</BookButton>
          <BookButton size="sm" onClick={() => void applyProposal()} disabled={saving || locking}>{t("visualPlan.proposalApplied")}</BookButton>
        </div>
      ) : null}

      <nav className="a4n-visual-plan-panel-nav grid grid-cols-2 xl:hidden" aria-label={t("visualPlan.panelNavigation")}>
        <BookButton size="sm" variant="secondary" onClick={() => setMobilePanel("source")} aria-haspopup="dialog">
          <Box size={14} aria-hidden="true" />
          {t("visualPlan.sourceAndSymbols")}
        </BookButton>
        <BookButton size="sm" variant="secondary" onClick={() => setMobilePanel("inspector")} aria-haspopup="dialog">
          <SlidersHorizontal size={14} aria-hidden="true" />
          {t("visualPlan.inspector")}
        </BookButton>
      </nav>

      <div className="grid min-h-0 min-w-0 flex-1 grid-cols-1 gap-2 xl:grid-cols-[14rem_minmax(0,1fr)_17rem]">
        <aside className="a4n-flat-surface order-2 hidden max-h-[28rem] min-h-0 overflow-y-auto p-2 custom-scrollbar xl:order-none xl:block xl:max-h-none" aria-label={t("visualPlan.sourceAndSymbols")}>
          {renderSourcePanelContent()}
        </aside>

        <main className="order-1 flex min-h-0 min-w-0 flex-col gap-2 xl:order-none">
          <div className="a4n-flat-surface relative min-h-[240px] min-w-0 flex-1 overflow-hidden bg-book-bg md:min-h-[340px] xl:min-h-0">
            <div className="absolute left-3 top-3 z-10 flex items-center gap-2 rounded-lg border border-book-border/70 bg-book-bg-paper/90 px-2 py-1 text-caption text-book-text-muted shadow-sm backdrop-blur">
              <Film size={12} aria-hidden="true" />
              {shot ? `${shot.title} · ${formatPlanTime(playheadMs)}` : t("visualPlan.noShot")}
            </div>
            <ReactFlow<StageNode, Edge>
              nodes={nodes}
              edges={edges}
              onNodesChange={handleNodeChanges}
              onNodeClick={(_event, node) => setSelectedEntityId(node.id)}
              onNodeDragStop={persistNodePosition}
              onPaneClick={() => setSelectedEntityId(null)}
              nodesConnectable={false}
              edgesReconnectable={false}
              elementsSelectable
              deleteKeyCode={null}
              minZoom={0.5}
              maxZoom={2.5}
              fitView
              fitViewOptions={{ padding: 0.1, maxZoom: 1 }}
              nodeExtent={[[0, 0], [STAGE_WIDTH, STAGE_HEIGHT]]}
              className="a4n-visual-stage"
              aria-label={t("visualPlan.viewerFrame")}
            >
              <Controls showInteractive={false} className="a4n-flow-controls" />
              <Background color="rgb(var(--color-text-tertiary) / 0.18)" gap={32} size={1} />
            </ReactFlow>
            {nodes.length === 0 ? (
              <div className="pointer-events-none absolute inset-0 grid place-items-center px-8 text-center text-small leading-relaxed text-book-text-muted">
                <div><Box size={24} className="mx-auto mb-2 opacity-50" /><p>{t("visualPlan.noEntitiesInKeyframe")}<br />{t("visualPlan.noEntitiesHint")}</p></div>
              </div>
            ) : null}
          </div>

          <Timeline
            plan={plan}
            shots={shotRefs}
            shot={shot}
            playheadMs={playheadMs}
            playing={playing}
            editable={editable}
            onPlay={() => { if (playheadMs >= plan.frame.duration_ms) setPlayheadMs(0); setPlaying((value) => !value); }}
            onSeek={setPlayheadMs}
            onSelectShot={selectShot}
            onSelectKeyframe={(frame) => { setPlayheadMs(frame.at_ms); setSelectedKeyframeId(frame.id); }}
            onAddKeyframe={addKeyframe}
          />
        </main>

        <aside
          className="a4n-flat-surface order-3 hidden max-h-[28rem] min-h-0 overflow-y-auto p-3 custom-scrollbar xl:order-none xl:block xl:max-h-none"
          aria-label={t("visualPlan.inspector")}
        >
          {renderInspectorPanelContent()}
        </aside>
      </div>

      <Modal
        isOpen={mobilePanel === "source"}
        onClose={() => setMobilePanel(null)}
        title={t("visualPlan.sourceAndSymbols")}
        icon={<Box size={15} />}
        maxWidthClassName="max-w-xl"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-3 py-3"
      >
        {renderSourcePanelContent()}
      </Modal>

      <Modal
        isOpen={mobilePanel === "inspector"}
        onClose={() => setMobilePanel(null)}
        title={t("visualPlan.inspector")}
        subtitle={shot ? shot.title : undefined}
        icon={<SlidersHorizontal size={15} />}
        maxWidthClassName="max-w-xl"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-3 py-3"
      >
        {renderInspectorPanelContent()}
      </Modal>
    </section>
  );
};

const defaultEntityState = (entityId: string, offset: number): VisualEntityStateV1 => ({
  entity_id: entityId,
  visible: true,
  bounds: {
    x: Math.min(0.72, 0.12 + (offset % 4) * 0.2),
    y: Math.min(0.72, 0.2 + Math.floor(offset / 4) * 0.22),
    width: 0.16,
    height: 0.2,
  },
  rotation_degrees: 0,
  opacity: 1,
  z_index: offset,
  facing: "unspecified",
  expression: null,
  pose: null,
  action: null,
  gaze_target_entity_id: null,
  dialogue: null,
  notes: null,
});

const eventRange = (shot: VisualShotPlanV1, playheadMs: number): [number, number] => {
  const start = Math.max(shot.start_ms, Math.min(playheadMs, shot.end_ms - 1));
  return [start, Math.min(shot.end_ms, start + 500)];
};

const sameStageNodes = (current: StageNode[], next: StageNode[]) =>
  current.length === next.length && current.every((node, index) => {
    const candidate = next[index];
    return candidate != null &&
      node.id === candidate.id &&
      node.position.x === candidate.position.x &&
      node.position.y === candidate.position.y &&
      node.data.label === candidate.data.label &&
      node.selected === candidate.selected &&
      node.draggable === candidate.draggable &&
      JSON.stringify(node.style) === JSON.stringify(candidate.style);
  });

interface EventKindControlProps<T extends string> {
  label: MessageKey;
  value: T;
  options: ReadonlyArray<readonly [T, MessageKey]>;
  disabled: boolean;
  icon?: React.ReactNode;
  onChange: (value: T) => void;
  onAdd: () => void;
}

const EventKindControl = <T extends string,>({
  label,
  value,
  options,
  disabled,
  icon,
  onChange,
  onAdd,
}: EventKindControlProps<T>) => (
  <EventKindControlView
    label={label}
    value={value}
    options={options}
    disabled={disabled}
    icon={icon}
    onChange={onChange}
    onAdd={onAdd}
  />
);

const EventKindControlView = <T extends string,>({
  label,
  value,
  options,
  disabled,
  icon,
  onChange,
  onAdd,
}: EventKindControlProps<T>) => {
  const { t } = useI18n();
  return (
  <div className="grid grid-cols-[minmax(0,1fr)_auto] items-end gap-1.5">
    <label className="min-w-0 text-caption font-semibold text-book-text-muted">
      {t(label)}
      <ThemedSelect
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.currentTarget.value as T)}
        className="pyqt-input mt-1 w-full !min-h-8 text-small"
      >
        {options.map(([kind, optionLabel]) => (
          <option key={kind} value={kind}>{t(optionLabel)}</option>
        ))}
      </ThemedSelect>
    </label>
    <BookButton
      size="sm"
      variant="secondary"
      disabled={disabled}
      onClick={onAdd}
      aria-label={t("visualPlan.addEvent", { kind: t(label) })}
    >
      {icon}<Plus size={12} aria-hidden="true" />{t("visualPlan.add")}
    </BookButton>
  </div>
  );
};

type VisualEntity = ChapterVisualPlanV1["entities"][number];

const entityLabel = (entityById: ReadonlyMap<string, VisualEntity>, entityId: string) =>
  entityById.get(entityId)?.label ?? entityId;

const RelationList: React.FC<{
  entityById: ReadonlyMap<string, VisualEntity>;
  keyframe: VisualKeyframeV1;
  editable: boolean;
  onRemove: (relationId: string) => void;
}> = ({ entityById, keyframe, editable, onRemove }) => {
  const { t } = useI18n();
  return (
  <div className="mt-2 space-y-1.5">
    {keyframe.relations.map((relation) => (
      <article
        key={relation.id}
        className="flex items-start gap-2 rounded-lg border border-book-border/60 bg-book-bg-paper px-2 py-1.5"
      >
        <div className="min-w-0 flex-1">
          <p className="truncate text-caption font-semibold text-book-text-main">
            {entityLabel(entityById, relation.source_entity_id)} → {entityLabel(entityById, relation.target_entity_id)}
          </p>
          <p className="truncate text-caption text-book-text-muted">
            {relation.label ?? t(RELATION_LABELS[relation.kind])}
          </p>
        </div>
        <BookIconButton
          label={t("visualPlan.deleteRelation", { source: entityLabel(entityById, relation.source_entity_id), target: entityLabel(entityById, relation.target_entity_id) })}
          size="sm"
          disabled={!editable}
          onClick={() => onRemove(relation.id)}
        >
          <Trash2 size={12} aria-hidden="true" />
        </BookIconButton>
      </article>
    ))}
    {keyframe.relations.length === 0 ? (
      <p className="text-caption leading-relaxed text-book-text-muted">{t("visualPlan.noRelations")}</p>
    ) : null}
  </div>
  );
};

const timedEventText = (event: VisualPlanTimedEventV1) =>
  "content" in event ? event.content : event.description;

interface TimedEventListProps<T extends VisualPlanTimedEventV1> {
  label: string;
  events: T[];
  editable: boolean;
  kindLabel: (event: T) => string;
  onRemove: (eventId: string) => void;
  onRangeChange: (
    eventId: string,
    boundary: "start" | "end",
    seconds: number,
  ) => void;
}

const TimedEventList = <T extends VisualPlanTimedEventV1,>({
  label,
  events,
  editable,
  kindLabel,
  onRemove,
  onRangeChange,
}: TimedEventListProps<T>) => (
  <TimedEventListView
    label={label}
    events={events}
    editable={editable}
    kindLabel={kindLabel}
    onRemove={onRemove}
    onRangeChange={onRangeChange}
  />
);

const TimedEventListView = <T extends VisualPlanTimedEventV1,>({
  label,
  events,
  editable,
  kindLabel,
  onRemove,
  onRangeChange,
}: TimedEventListProps<T>) => {
  const { t } = useI18n();
  return (
  <details open={events.length > 0} className="rounded-lg border border-book-border/60 bg-book-bg-paper">
    <summary className="cursor-pointer px-2 py-1.5 text-caption font-semibold text-book-text-main">
      {label} · {events.length}
    </summary>
    {events.length > 0 ? (
      <ul className="space-y-1.5 border-t border-book-border/50 p-2">
        {events.map((event) => (
          <li key={event.id} className="rounded-md bg-book-bg-tertiary/60 p-1.5">
            <div className="flex items-start gap-1.5">
              <div className="min-w-0 flex-1">
                <p className="text-caption font-semibold text-book-text-main">{kindLabel(event)}</p>
                <p className="line-clamp-2 text-caption leading-relaxed text-book-text-muted">
                  {timedEventText(event)}
                </p>
              </div>
              <BookIconButton
                label={`${t("common.delete")} ${label}: ${kindLabel(event)}`}
                size="sm"
                disabled={!editable}
                onClick={() => onRemove(event.id)}
              >
                <Trash2 size={12} aria-hidden="true" />
              </BookIconButton>
            </div>
            <div className="mt-1.5 grid grid-cols-2 gap-1.5">
              <label className="text-[10px] font-semibold text-book-text-muted">
                {t("visualPlan.startSeconds")}
                <input
                  key={`${event.id}-start-${event.start_ms}`}
                  type="number"
                  min="0"
                  step="0.05"
                  defaultValue={event.start_ms / 1000}
                  disabled={!editable}
                  onBlur={(inputEvent) => onRangeChange(
                    event.id,
                    "start",
                    Number(inputEvent.currentTarget.value),
                  )}
                  className="pyqt-input mt-0.5 w-full !min-h-7 !px-1.5 text-caption"
                />
              </label>
              <label className="text-[10px] font-semibold text-book-text-muted">
                {t("visualPlan.endSeconds")}
                <input
                  key={`${event.id}-end-${event.end_ms}`}
                  type="number"
                  min="0.001"
                  step="0.05"
                  defaultValue={event.end_ms / 1000}
                  disabled={!editable}
                  onBlur={(inputEvent) => onRangeChange(
                    event.id,
                    "end",
                    Number(inputEvent.currentTarget.value),
                  )}
                  className="pyqt-input mt-0.5 w-full !min-h-7 !px-1.5 text-caption"
                />
              </label>
            </div>
          </li>
        ))}
      </ul>
    ) : null}
  </details>
  );
};

const Timeline: React.FC<{
  plan: ChapterVisualPlanV1;
  shots: readonly VisualShotRef[];
  shot: VisualShotPlanV1 | null;
  playheadMs: number;
  playing: boolean;
  editable: boolean;
  onPlay: () => void;
  onSeek: (value: number) => void;
  onSelectShot: (shot: VisualShotRef) => void;
  onSelectKeyframe: (keyframe: VisualKeyframeV1) => void;
  onAddKeyframe: () => void;
}> = ({ plan, shots, shot, playheadMs, playing, editable, onPlay, onSeek, onSelectShot, onSelectKeyframe, onAddKeyframe }) => {
  const { t } = useI18n();
  const percent = (value: number) => `${(value / plan.frame.duration_ms) * 100}%`;
  const eventRows = shot ? [
    ["visualPlan.entityEvents", shot.entity_events],
    ["visualPlan.cameraEvents", shot.camera_events],
    ["visualPlan.soundEvents", shot.sound_events],
    ["visualPlan.techniques", shot.technique_events],
  ] as const : [];
  return (
    <section className="a4n-flat-surface shrink-0 overflow-hidden" aria-label={t("visualPlan.chapterTimeline")}>
      <div className="flex flex-wrap items-center gap-2 border-b border-book-border/60 px-2 py-1.5">
        <BookIconButton label={playing ? t("visualPlan.pause") : t("visualPlan.play")} size="sm" onClick={onPlay}>{playing ? <Pause size={14} /> : <Play size={14} />}</BookIconButton>
        <span className="w-12 font-mono text-caption text-book-text-sub">{formatPlanTime(playheadMs)}</span>
        <input aria-label={t("visualPlan.playhead")} type="range" min="0" max={plan.frame.duration_ms} step="50" value={playheadMs} onChange={(event) => onSeek(Number(event.currentTarget.value))} className="min-w-32 flex-1" />
        <span className="font-mono text-caption text-book-text-muted">/ {formatPlanTime(plan.frame.duration_ms)}</span>
        <BookIconButton
          label={t("visualPlan.copyPreviousKeyframe")}
          size="sm"
          onClick={onAddKeyframe}
          disabled={!editable || !shot || playheadMs <= shot.start_ms || playheadMs >= shot.end_ms}
          className="shrink-0 md:hidden"
        >
          <Copy size={13} aria-hidden="true" />
        </BookIconButton>
        <BookButton
          size="sm"
          variant="secondary"
          onClick={onAddKeyframe}
          disabled={!editable || !shot || playheadMs <= shot.start_ms || playheadMs >= shot.end_ms}
          className="hidden shrink-0 md:inline-flex"
        >
          <Plus size={12} />{t("visualPlan.copyPreviousKeyframe")}
        </BookButton>
      </div>
      <div className="overflow-x-auto custom-scrollbar">
        <div className="relative min-w-[max(100%,22rem)] px-2 py-2 sm:min-w-[720px]">
          <div className="flex h-11 overflow-hidden rounded-lg border border-book-border bg-book-bg">
            {shots.map((item) => (
              <button key={item.shot.id} type="button" onClick={() => onSelectShot(item)} className={`min-w-16 overflow-hidden border-r border-book-border/70 px-2 text-left text-caption transition-colors ${shot?.id === item.shot.id ? "bg-book-primary/15 text-book-primary" : "bg-book-bg-paper text-book-text-sub hover:bg-book-bg-tertiary"}`} style={{ width: percent(item.shot.end_ms - item.shot.start_ms) }}>
                <span className="block truncate font-semibold">{item.shot.title}</span><span className="font-mono text-[9px] opacity-70">{formatPlanTime(item.shot.end_ms - item.shot.start_ms)}</span>
              </button>
            ))}
          </div>
          <div className="relative mt-1 h-5 rounded bg-book-bg-tertiary/60">
            {shots.flatMap((item) => item.shot.keyframes).map((frame) => (
              <button key={frame.id} type="button" aria-label={`${formatPlanTime(frame.at_ms)} ${frame.label}`} title={`${formatPlanTime(frame.at_ms)} · ${frame.label}`} onClick={() => onSelectKeyframe(frame)} className="absolute top-1 h-3 w-3 -translate-x-1/2 rotate-45 border border-book-primary bg-book-bg-paper focus-visible:outline-2 focus-visible:outline-book-primary" style={{ left: percent(frame.at_ms) }} />
            ))}
            <div aria-hidden="true" className="pointer-events-none absolute inset-y-0 z-10 w-px bg-error shadow-[0_0_0_1px_rgb(var(--status-error)/0.18)]" style={{ left: percent(playheadMs) }} />
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-1.5 md:hidden">
            <span className="text-caption font-semibold text-book-text-muted">{t("visualPlan.timelineSymbols")}</span>
            {eventRows.map(([label, events]) => events.length > 0 ? (
              <span key={label} className="rounded-md bg-book-bg-tertiary/70 px-1.5 py-0.5 text-[10px] text-book-text-muted">
                {t(label)} · {events.length}
              </span>
            ) : null)}
          </div>
          {eventRows.map(([label, events]) => (
            <div key={label} className="mt-1 hidden grid-cols-[3rem_minmax(0,1fr)] items-center gap-1 md:grid">
              <span className="text-right text-[9px] text-book-text-muted">{t(label)}</span>
              <div className="relative h-4 rounded bg-book-bg-tertiary/50">
                {events.map((event) => <div key={event.id} title={("description" in event ? event.description : "content" in event ? event.content : t(label))} className="absolute top-0.5 h-3 min-w-1 rounded bg-book-primary/55" style={{ left: percent(event.start_ms), width: percent(Math.max(1, event.end_ms - event.start_ms)) }} />)}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
};
