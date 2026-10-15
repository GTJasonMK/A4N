import { lazy, Suspense, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { useStore } from "zustand";
import { BookOpen, Check, Film, ImagePlus, Images, Maximize, Mountain, Pause, Play, Plus, Redo2, RefreshCw, Save, Settings2, SkipBack, SkipForward, Sparkles, Trash2, Undo2, Upload, X } from "lucide-react";
import { extractErrorMessage } from "../../api";
import type { ChapterSummary } from "../../api/chapter";
import type { ChapterVisualPlanV3, ChapterVisualPlanView, SceneBindingV3, VisualPointV3, VisualSegmentV3, VisualPlanPromptCandidate, VisualPlanEditOperation } from "../../api/chapterVisualPlan";
import { getChapterVisualPlan } from "../../api/chapterVisualPlan";
import { IMAGE_TYPE_DIRECTOR_STORYBOARD_FRAME, listImagesByProjects, uploadImage, type GeneratedImage, type ImageSize } from "../../api/image";
import { createSceneProfile, type SceneProfile } from "../../api/sceneProfiles";
import { respondAgentEditor, type AgentEditorRequest, type AgentEditorResponse, type AgentEditorSource, type AgentExecutionMode } from "../../api/agent";
import { useDirectorAgentBridge } from "./visual-plan/useDirectorAgentBridge";
import { DomainAgentConsole } from "../../components/agent/DomainAgentConsole";
import {
  domainAgentComposerOwnerKey,
  domainAgentOperationInFlightForOwner,
  hasDomainAgentComposerDraftForOwner,
  clearDomainAgentComposerDraftForOwner,
} from "../../components/agent/domainAgentDrafts";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { isSafeInteger } from "../../utils/numberRange";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { ZoomableImage } from "../../components/ui/ImageLightbox";
import { Modal } from "../../components/ui/Modal";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { ImageSizeSelect, useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { readFileAsBase64 } from "../../utils/fileUpload";
import { supportedImageAccept } from "../../utils/imageUpload";
import { toMediaSrc } from "../../utils/mediaSrc";
import { DirectorGeneration } from "./visual-plan/DirectorGeneration";
import { DirectorScrubber, DirectorTimeline, type DirectorTimelineView } from "./visual-plan/DirectorTimeline";
import { useDirectorPlan } from "./visual-plan/useDirectorPlan";
import { useDirectorPlayback, type DirectorPlayback } from "./visual-plan/playback";
import { useNovelDetailDirty } from "./novelDetailDirty";
import { useI18n } from "../../i18n";
import { durationMs, formatTime, pointAt, pointsInsideSegment, segmentAt, segmentTimes, type DirectorSelection } from "./visual-plan/timeline";
import "./visual-plan/director.css";

const SceneLibrary = lazy(() => import("./SceneProfilesTab").then((module) => ({ default: module.SceneProfilesTab })));
type ModalPanel = "source" | "scenes" | "media" | "settings" | "agent" | "candidates";
type SplitDraft = { base: ChapterVisualPlanView; segmentId: string; pointId: string; leftPrompt: string; rightPrompt: string; leftScene: SceneBindingV3 | null; rightScene: SceneBindingV3 | null; dirty: boolean };
type MergeDraft = { base: ChapterVisualPlanView; leftId: string; rightId: string; prompt: string; scene: SceneBindingV3 | null; dirty: boolean };

function directorCandidatePointId(image: GeneratedImage): string | null {
  if (image.image_type !== IMAGE_TYPE_DIRECTOR_STORYBOARD_FRAME || typeof image.metadata_json !== "object" || image.metadata_json === null || Array.isArray(image.metadata_json)) return null;
  const candidate = (image.metadata_json as Record<string, unknown>).director_candidate;
  if (typeof candidate !== "object" || candidate === null || Array.isArray(candidate)) return null;
  const pointId = (candidate as Record<string, unknown>).point_id;
  return typeof pointId === "string" ? pointId : null;
}

export function ChapterVisualPlanTab({ projectId, canEdit, chapter, onDirtyChange, onBusyChange }: { projectId: number; canEdit: boolean; chapter: Pick<ChapterSummary, "id" | "project_id" | "chapter_number" | "title"> | null; onDirtyChange?: (dirty: boolean) => void; onBusyChange?: (busy: boolean) => void }) {
  const { t } = useI18n();
  const { addToast } = useToast();
  const agentComposerDirtyKey = `domain-agent-composer:director:${projectId}`;
  const agentComposerOwnerKey = domainAgentComposerOwnerKey(agentComposerDirtyKey);
  const chapterId = chapter?.id ?? null;
  const model = useDirectorPlan(projectId, chapterId, canEdit);
  const plan = model.plan;
  const total = plan ? durationMs(plan) : 0;
  const clock = useDirectorPlayback(total, `${projectId}:${chapterId}`);
  const [selection, setSelection] = useState<DirectorSelection | null>(null);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [modalPanel, setModalPanel] = useState<ModalPanel | null>(null);
  const [showConflictDialog, setShowConflictDialog] = useState(false);
  const [agentExecutionMode, setAgentExecutionMode] = useState<AgentExecutionMode>("autonomous");
  useEffect(() => { if (model.conflict) setShowConflictDialog(true); }, [model.conflict]);
  const [narrow, setNarrow] = useState(true);
  const workspaceRef = useRef<HTMLElement>(null);
  const timelineView = useRef<DirectorTimelineView>(null);
  useLayoutEffect(() => {
    const workspace = workspaceRef.current;
    if (!workspace) return;
    // The chapter sidebar changes the space available without resizing the window.
    let wasNarrow: boolean | undefined;
    const update = () => {
      const next = workspace.clientWidth < 760;
      if (next === wasNarrow) return;
      const previous = wasNarrow;
      wasNarrow = next;
      setNarrow(next);
      setInspectorOpen(next ? false : previous == null || previous);
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(workspace);
    return () => observer.disconnect();
  }, [model.loading, plan != null]);
  const [images, setImages] = useState<GeneratedImage[]>([]);
  const [scenes, setScenes] = useState<Array<Pick<SceneProfile, "id" | "project_id" | "name">>>([]);
  const imageSizeState = useImageGenerationSize();
  const [selectedImageSize, setSelectedImageSize] = useState<ImageSize | null>(null);
  const [split, setSplit] = useState<SplitDraft | null>(null);
  const [merge, setMerge] = useState<MergeDraft | null>(null);
  const [sceneBindingDirtyKeys, setSceneBindingDirtyKeys] = useState<ReadonlySet<string>>(() => new Set());
  const [sceneCreationKeys, setSceneCreationKeys] = useState<ReadonlySet<string>>(() => new Set());
  const [uploading, setUploading] = useState(false);
  const [assetLoadError, setAssetLoadError] = useState<string | null>(null);
  const frameRef = useRef<HTMLDivElement>(null);
  const assetScope = useRef(0);
  const names = useMemo(() => new Map(scenes.map((scene) => [scene.id, scene.name])), [scenes]);
  const imageById = useMemo(() => new Map(images.map((image) => [image.id, image])), [images]);
  const point = selection?.kind === "point" ? plan?.points.find((point) => point.id === selection.id) : undefined;
  const segment = selection?.kind === "segment" ? plan?.segments.find((segment) => segment.id === selection.id) : undefined;
  const pointCandidates = useMemo(() => point && chapterId != null
    ? images
      .filter((image) => image.project_id === projectId && image.chapter_id === chapterId && directorCandidatePointId(image) === point.id)
      .sort((left, right) => String(right.created_at ?? "").localeCompare(String(left.created_at ?? "")))
    : [], [chapterId, images, point?.id, projectId]);
  const pointImageSizes = useMemo(() => plan
    ? imageSizeState.sizes.filter((size) => size.width * plan.frame.height === size.height * plan.frame.width)
    : [], [imageSizeState.sizes, plan?.frame.height, plan?.frame.width]);
  const effectiveImageSize = selectedImageSize && pointImageSizes.some((size) => size.width === selectedImageSize.width && size.height === selectedImageSize.height)
    ? selectedImageSize
    : pointImageSizes[0] ?? null;
  useEffect(() => {
    setSelectedImageSize((current) => current && pointImageSizes.some((size) => size.width === current.width && size.height === current.height) ? current : null);
  }, [pointImageSizes]);
  const disabled = !model.editable;
  const agentSidebarOpen = modalPanel === "agent" && !narrow && inspectorOpen;
  const hasModal = Boolean(split || merge || (modalPanel && !agentSidebarOpen) || (narrow && inspectorOpen));
  const dirty = Boolean(model.dirty || split?.dirty || merge?.dirty);
  const visualPlanDirty = dirty || sceneBindingDirtyKeys.size > 0;
  const setSceneBindingDraftDirty = useCallback((key: string, value: boolean) => {
    setSceneBindingDirtyKeys((current) => {
      if (current.has(key) === value) return current;
      const next = new Set(current);
      if (value) next.add(key);
      else next.delete(key);
      return next;
    });
  }, []);
  const inspectorSceneBindingKey = `inspector:${selection?.kind ?? "none"}:${selection?.id ?? "none"}`;
  const inspectorSceneBindingDirty = sceneBindingDirtyKeys.has(inspectorSceneBindingKey);
  const inspectorCandidateKey = `candidate:${selection?.kind ?? "none"}:${selection?.id ?? "none"}`;
  const inspectorCandidateDirty = sceneBindingDirtyKeys.has(inspectorCandidateKey);
  const inspectorDraftDirty = inspectorSceneBindingDirty || inspectorCandidateDirty;
  const sceneCreationBusy = sceneCreationKeys.size > 0;
  const setCandidateDirty = useCallback((dirty: boolean) => {
    setSceneBindingDraftDirty(inspectorCandidateKey, dirty);
  }, [inspectorCandidateKey, setSceneBindingDraftDirty]);
  const clearInspectorDraft = useCallback(() => {
    setSceneBindingDirtyKeys((current) => new Set([...current].filter((key) => key !== inspectorSceneBindingKey && key !== inspectorCandidateKey)));
  }, [inspectorCandidateKey, inspectorSceneBindingKey]);
  const discardAgentComposer = async (action: string) => {
    if (domainAgentOperationInFlightForOwner(agentComposerOwnerKey)) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    if (!hasDomainAgentComposerDraftForOwner(agentComposerOwnerKey)) return true;
    if (!(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: t("agent.domain.consoleLabel"),
        action,
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return false;
    clearDomainAgentComposerDraftForOwner(agentComposerOwnerKey);
    return true;
  };
  useEffect(() => {
    setSceneBindingDirtyKeys(new Set());
    setSceneCreationKeys(new Set());
  }, [chapterId]);
  useEffect(() => { onDirtyChange?.(visualPlanDirty); }, [onDirtyChange, visualPlanDirty]);
  useEffect(() => { onBusyChange?.(model.busy || model.saving || uploading || sceneCreationBusy); }, [model.busy, model.saving, onBusyChange, sceneCreationBusy, uploading]);
  useEffect(() => () => onBusyChange?.(false), [onBusyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  useNovelDetailDirty(
    `visual-plan:${projectId}:${chapterId ?? "none"}`,
    visualPlanDirty,
    undefined,
    () => !model.busy && !model.saving && !uploading && !sceneCreationBusy,
  );
  const setSceneCreationBusy = useCallback((key: string, busy: boolean) => {
    setSceneCreationKeys((current) => {
      if (current.has(key) === busy) return current;
      const next = new Set(current);
      if (busy) next.add(key);
      else next.delete(key);
      return next;
    });
  }, []);
  const markSceneCreationDirty = useCallback((key: string) => setSceneBindingDraftDirty(key, true), [setSceneBindingDraftDirty]);
  const reportSceneCreationBusy = useCallback(() => {
    model.report(new Error(t("visualPlan.workspace.sceneCreationBusy")));
  }, [model, t]);
  const openModal = async (next: ModalPanel) => {
    if (sceneCreationBusy) { reportSceneCreationBusy(); return; }
    const closesInspector = (next === "agent" && !narrow) || (narrow && inspectorOpen);
    const closesAgentConsole = (agentSidebarOpen || (narrow && modalPanel === "agent")) && next !== "agent";
    if (closesInspector && inspectorDraftDirty && !(await confirmDialog({
      title: t("visualPlan.workspace.discardInspectorTitle"),
      message: t("visualPlan.workspace.discardInspectorOpenMessage"),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    if (closesAgentConsole && !(await discardAgentComposer(t("common.close")))) return;
    if (closesInspector) clearInspectorDraft();
    setModalPanel(next);
    if (next === "agent" && !narrow) {
      setInspectorOpen(true);
    } else if (narrow) {
      setInspectorOpen(false);
    }
  };
  const closeModal = () => setModalPanel(null);
  const closeAgentSidebar = async () => {
    if (sceneCreationBusy) { reportSceneCreationBusy(); return; }
    if (inspectorDraftDirty && !(await confirmDialog({
      title: t("visualPlan.workspace.discardInspectorTitle"),
      message: t("visualPlan.workspace.discardInspectorCloseAgentMessage"),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    if (!(await discardAgentComposer(t("common.close")))) return;
    clearInspectorDraft();
    setModalPanel(null);
    setInspectorOpen(false);
  };
  const closeInspector = async () => {
    if (sceneCreationBusy) { reportSceneCreationBusy(); return; }
    if (inspectorDraftDirty && !(await confirmDialog({
      title: t("visualPlan.workspace.discardInspectorTitle"),
      message: t("visualPlan.workspace.discardInspectorCloseMessage"),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    clearInspectorDraft();
    setInspectorOpen(false);
  };
  const closeSplit = async () => {
    if (sceneCreationBusy) { reportSceneCreationBusy(); return; }
    const discard = split?.dirty || sceneBindingDirtyKeys.has("split:left") || sceneBindingDirtyKeys.has("split:right");
    if (discard && !(await confirmDialog({
      title: t("visualPlan.workspace.discardSplitTitle"),
      message: t("visualPlan.workspace.discardSplitMessage"),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    setSceneBindingDirtyKeys((current) => new Set([...current].filter((key) => !key.startsWith("split:"))));
    setSplit(null);
  };
  const closeMerge = async () => {
    if (sceneCreationBusy) { reportSceneCreationBusy(); return; }
    const discard = merge?.dirty || sceneBindingDirtyKeys.has("merge");
    if (discard && !(await confirmDialog({
      title: t("visualPlan.workspace.discardMergeTitle"),
      message: t("visualPlan.workspace.discardMergeMessage"),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    setSceneBindingDirtyKeys((current) => new Set([...current].filter((key) => key !== "merge")));
    setMerge(null);
  };
  const closeCandidateModal = () => {
    setModalPanel(null);
    if (narrow) setInspectorOpen(true);
  };
  const closeSceneLibrary = () => {
    closeModal();
    void loadAssets();
    if (narrow) setInspectorOpen(true);
  };

  const loadAssets = useCallback(async () => {
    const scope = ++assetScope.current;
    setAssetLoadError(null);
    if (chapterId == null) { setImages([]); setScenes([]); return; }
    const results = await Promise.allSettled([
      listImagesByProjects([...new Set([projectId, chapter?.project_id ?? projectId])]),
      getChapterVisualPlan(projectId, chapterId),
    ]);
    if (scope !== assetScope.current) return;
    const failures: string[] = [];
    if (results[0].status === "fulfilled") setImages(results[0].value);
    else {
      setImages([]);
      failures.push(extractErrorMessage(results[0].reason, t("visualPlan.workspace.imageLibraryLoadFailed")));
    }
    if (results[1].status === "fulfilled") setScenes(results[1].value.scene_choices);
    else {
      setScenes([]);
      failures.push(extractErrorMessage(results[1].reason, t("visualPlan.workspace.sceneLibraryLoadFailed")));
    }
    setAssetLoadError(failures.length > 0 ? failures.join("；") : null);
  }, [chapter?.project_id, chapterId, projectId, t]);

  useEffect(() => {
    setSelection(null);
    setInspectorOpen(!narrow);
    setModalPanel(null);
    setSplit(null);
    setMerge(null);
    setSelectedImageSize(null);
    setImages([]);
    setScenes([]);
    void loadAssets();
    return () => { assetScope.current += 1; };
  }, [chapterId, loadAssets]);

  useEffect(() => {
    if (model.loading || !plan) return;
    if (!selection || (selection.kind === "point" ? !plan.points.some((point) => point.id === selection.id) : !plan.segments.some((segment) => segment.id === selection.id))) {
      setSelection({ kind: "point", id: plan.points[0].id });
    }
  }, [model.loading, plan, selection]);

  const select = async (next: DirectorSelection, time: number) => {
    if (sceneCreationBusy) { reportSceneCreationBusy(); return; }
    const currentKey = `inspector:${selection?.kind ?? "none"}:${selection?.id ?? "none"}`;
    const currentCandidateKey = `candidate:${selection?.kind ?? "none"}:${selection?.id ?? "none"}`;
    const nextKey = `inspector:${next.kind}:${next.id}`;
    if ((sceneBindingDirtyKeys.has(currentKey) || sceneBindingDirtyKeys.has(currentCandidateKey)) && currentKey !== nextKey) {
      const confirmed = await confirmDialog({
        title: t("visualPlan.workspace.discardInspectorTitle"),
        message: t("visualPlan.workspace.discardInspectorChangeMessage"),
        confirmText: t("common.discardDraft"),
        dialogType: "warning",
      });
      if (!confirmed) return;
      setSceneBindingDirtyKeys((current) => new Set([...current].filter((key) => key !== currentKey && key !== currentCandidateKey)));
    }
    setSelection(next);
    setInspectorOpen(true);
    clock.getState().seek(time);
  };
  const updatePoint = (id: string, fields: Partial<VisualPointV3>) => {
    const current = model.planRef.current;
    if (!current || !current.points.some((point) => point.id === id)) throw new Error(t("visualPlan.workspace.pointMissing"));
    model.updateFields({ ...current, points: current.points.map((point) => point.id === id ? { ...point, ...fields } : point) });
  };
  const updateSegment = (id: string, fields: Partial<VisualSegmentV3>) => {
    const current = model.planRef.current;
    if (!current || !current.segments.some((segment) => segment.id === id)) throw new Error(t("visualPlan.workspace.segmentMissing"));
    model.updateFields({ ...current, segments: current.segments.map((segment) => segment.id === id ? { ...segment, ...fields } : segment) });
  };
  const rememberImage = useCallback((image: GeneratedImage) => {
    setImages((current) => current.some((item) => item.id === image.id) ? current : [image, ...current]);
    if (image.image_type === IMAGE_TYPE_DIRECTOR_STORYBOARD_FRAME) {
      setSelectedImageSize({ width: image.width, height: image.height });
    }
  }, []);
  const rememberScene = useCallback((scene: SceneProfile) => {
    setScenes((current) => current.some((item) => item.id === scene.id)
      ? current.map((item) => item.id === scene.id ? scene : item)
      : [...current, scene]);
  }, []);
  const edit = async (operation: VisualPlanEditOperation, base?: ChapterVisualPlanView) => {
    clock.getState().pause();
    await model.mutate(operation, base);
  };

  const openSplit = async () => {
    if (sceneCreationBusy) { reportSceneCreationBusy(); return; }
    if (!plan) return;
    const viewId = model.viewInstanceId.current;
    let base: ChapterVisualPlanView;
    try { base = await model.save(); } catch (failure) { model.report(failure); return; }
    if (model.viewInstanceId.current !== viewId) return;
    const target = base.plan.segments.find((item) => item.id === segment?.id) ?? segmentAt(base.plan, point?.at_ms ?? clock.getState().time);
    if (!target) return;
    const [start, end] = segmentTimes(base.plan, target);
    const candidates = base.plan.points.filter((point) => point.at_ms > start && point.at_ms < end);
    const splitPoint = candidates.find((candidate) => candidate.id === point?.id) ?? candidates[0];
    if (!splitPoint) { model.report(new Error(t("visualPlan.workspace.splitPointRequired"))); return; }
    setSplit({ base, segmentId: target.id, pointId: splitPoint.id, leftPrompt: target.prompt, rightPrompt: "",
      leftScene: target.scene_binding?.kind === "transition" ? { kind: "scene", scene_profile_id: target.scene_binding.from_scene_profile_id } : target.scene_binding,
      rightScene: target.scene_binding, dirty: false });
  };
  const openMerge = async () => {
    if (sceneCreationBusy) { reportSceneCreationBusy(); return; }
    if (!plan || !segment) return;
    const viewId = model.viewInstanceId.current;
    let base: ChapterVisualPlanView;
    try { base = await model.save(); } catch (failure) { model.report(failure); return; }
    if (model.viewInstanceId.current !== viewId) return;
    const index = base.plan.segments.findIndex((item) => item.id === segment.id);
    if (index < 0) return;
    const right = base.plan.segments[index + 1];
    if (!right) return;
    setMerge({ base, leftId: segment.id, rightId: right.id, prompt: "", scene: base.plan.segments[index].scene_binding, dirty: false });
  };
  const adoptImage = async (pointId: string, image: GeneratedImage) => {
    if (!model.editable) throw new Error(t("visualPlan.workspace.draftEditingRequired"));
    updatePoint(pointId, { image_id: image.id });
    await model.save();
    rememberImage(image);
  };
  const adoptPrompt = async (candidate: VisualPlanPromptCandidate): Promise<boolean> => {
    if (!model.editable || candidate.source_chapter_id !== chapterId) throw new Error(t("visualPlan.workspace.candidateWrongChapter"));
    const current = model.planRef.current?.segments.find((segment) => segment.id === candidate.segment_id);
    if (!current) throw new Error(t("visualPlan.workspace.candidateSegmentMissing"));
    if (model.recordRef.current?.record.plan_sha256 !== candidate.plan_sha256) {
      const accepted = await confirmDialog({
        title: t("visualPlan.workspace.candidatePlanChangedTitle"),
        message: `${t("visualPlan.workspace.currentDescription", { description: current.prompt || t("visualPlan.workspace.notFilled") })}\n\n${t("visualPlan.workspace.candidateDescription", { description: candidate.prompt })}\n\n${t("visualPlan.workspace.reviewCandidate")}`,
        confirmText: t("visualPlan.generation.action.adopt"),
      });
      if (!accepted) return false;
    }
    updateSegment(candidate.segment_id, { prompt: candidate.prompt });
    await model.save();
    return true;
  };

  const upload = async (file: File) => {
    if (!point || chapterId == null || disabled) return;
    const pointId = point.id;
    const scope = assetScope.current;
    setUploading(true);
    try {
      const image = await uploadImage(projectId, { file_name: file.name, mime_type: file.type, data_base64: await readFileAsBase64(file),
        image_type: "chapter_reference", prompt: point.description, chapter_id: chapterId, panel_id: null, metadata_json: null });
      if (scope !== assetScope.current) return;
      await adoptImage(pointId, image);
    } catch (failure) { if (scope === assetScope.current) model.report(failure); }
    finally { if (scope === assetScope.current) setUploading(false); }
  };

  const getEditorSource = (): AgentEditorSource | null => {
    if (!plan || chapterId == null) return null;
    const owner = point ? segmentAt(plan, point.at_ms) : segment;
    return { protocol_version: 2, view_instance_id: model.viewInstanceId.current, project_id: projectId, chapter_id: chapterId,
      chapter_number: chapter?.chapter_number ?? null,
      editor_revision: model.editorRevision.current, scene_id: null, shot_id: owner?.id ?? null,
      keyframe_id: point?.id ?? null, entity_id: null };
  };
  const editorRequest = async (request: AgentEditorRequest, runId: string) => {
    const viewInstanceId = model.viewInstanceId.current;
    let submitted = false;
    const respond = async (response: AgentEditorResponse) => {
      submitted = true;
      const reply = await respondAgentEditor({ run_id: runId, execution_id: request.execution_id, view_instance_id: viewInstanceId, response });
      if (reply.status !== "succeeded") throw new Error(reply.error_message || t("visualPlan.workspace.editorRequestFailed"));
      return reply;
    };
    try {
      const currentSource = getEditorSource();
      if (viewInstanceId !== request.source.view_instance_id) return;
      if (!currentSource || currentSource.project_id !== request.source.project_id || currentSource.chapter_id !== request.source.chapter_id) {
        await respond({ kind: "error", message: t("visualPlan.workspace.timelineChanged") });
        return;
      }
      if (!model.planRef.current) throw new Error(t("visualPlan.workspace.editorNotReady"));
      const operation = request.operation;
      const expected = operation.kind === "prepare_write" ? operation.expected_editor_revision : operation.kind === "read" || operation.kind === "save" ? operation.input.expected_editor_revision : null;
      if (expected && expected !== model.editorRevision.current) throw new Error(t("visualPlan.workspace.editorChanged"));
      const view = { editable: model.editable, status: model.recordRef.current?.record.status ?? "unsaved",
        selected_ids: selection ? [selection.id] : [], playhead_ms: Math.round(clock.getState().time), playing: clock.getState().playing,
        visible_range_ms: timelineView.current?.visibleRange() ?? [0, total] as [number, number],
        available_actions: ["select", "locate", "seek", "play", "pause", "fit"],
        blockers: [model.conflict, model.saving ? t("visualPlan.workspace.saving") : null, split || merge ? t("visualPlan.workspace.dialogPending") : null].filter((item): item is string => item != null) };
      const state = { view, editor_revision: model.editorRevision.current, record_id: model.recordRef.current?.record.id ?? null,
        record_revision: model.recordRef.current?.record.revision ?? null, dirty: model.dirty, has_uncommitted_dialog: split != null || merge != null };
      if (operation.kind === "inspect") await respond({ kind: "state", state });
      else if (operation.kind === "control") {
        const action = operation.input.action;
        if ("at_ms" in action && (!isSafeInteger(action.at_ms) || action.at_ms < 0 || action.at_ms > total)) throw new Error(t("visualPlan.workspace.playbackOutOfRange"));
        if (action.kind === "select") {
          if (action.object_kind !== "point" && action.object_kind !== "segment") throw new Error(t("visualPlan.workspace.timelineObjectTypeUnknown"));
          const items = action.object_kind === "point" ? model.planRef.current.points : model.planRef.current.segments;
          if (!items.some((item) => item.id === action.object_id)) throw new Error(t("visualPlan.workspace.timelineObjectDeleted"));
          setSelection({ kind: action.object_kind, id: action.object_id });
          view.selected_ids = [action.object_id];
        } else if (action.kind === "locate") {
          if (!timelineView.current) throw new Error(t("visualPlan.timelineUi.viewNotReady"));
          timelineView.current.locate(action.at_ms);
        } else if (action.kind === "seek") clock.getState().seek(action.at_ms);
        else if (action.kind === "play") { if (!clock.getState().playing) clock.getState().toggle(); }
        else if (action.kind === "pause") clock.getState().pause();
        else {
          if (!timelineView.current) throw new Error(t("visualPlan.timelineUi.viewNotReady"));
          timelineView.current.fit();
        }
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        if (viewInstanceId !== model.viewInstanceId.current) throw new Error(t("visualPlan.workspace.editorClosed"));
        view.playhead_ms = Math.round(clock.getState().time);
        view.playing = clock.getState().playing;
        view.visible_range_ms = timelineView.current?.visibleRange() ?? view.visible_range_ms;
        await respond({ kind: "state", state });
      }
      else if (operation.kind === "read") await respond({ kind: "draft", editor_revision: state.editor_revision, plan: model.planRef.current });
      else if (operation.kind === "save") {
        if (state.has_uncommitted_dialog) throw new Error(t("visualPlan.workspace.finishSplitMerge"));
        await model.saveFromAgent(async (snapshot) => {
          if (viewInstanceId !== model.viewInstanceId.current || state.editor_revision !== model.editorRevision.current) {
            throw new Error(t("visualPlan.workspace.editorChanged"));
          }
          const reply = await respond({ kind: "save_draft", state, plan: snapshot });
          if (!reply.saved_draft) throw new Error(t("visualPlan.workspace.agentSaveNoDraft"));
          return reply.saved_draft;
        });
      } else {
        if (state.dirty || state.has_uncommitted_dialog) throw new Error(t("visualPlan.workspace.unsavedEdits"));
        await respond({ kind: "ready_to_write", state });
      }
    } catch (failure) {
      if (!submitted) await respond({ kind: "error", message: String(failure) });
      throw failure;
    }
  };

  useDirectorAgentBridge(projectId, chapterId, !model.loading && plan != null, model.conflict != null || model.invalidated, getEditorSource, editorRequest, model.report);

  useEffect(() => {
    const keyboard = (event: KeyboardEvent) => {
      if (hasModal || event.defaultPrevented || (event.target instanceof Element && event.target.closest('input,textarea,select,[contenteditable=true],[role="combobox"],[role="dialog"],.director-conflict-details'))) return;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") { event.preventDefault(); event.shiftKey ? model.redo() : model.undo(); }
      if (event.key === " " && !(event.target instanceof HTMLButtonElement)) { event.preventDefault(); clock.getState().toggle(); }
      if (event.key === "Home") { event.preventDefault(); clock.getState().seek(0); }
      if (event.key === "End") { event.preventDefault(); clock.getState().seek(total); }
    };
    window.addEventListener("keydown", keyboard);
    return () => window.removeEventListener("keydown", keyboard);
  }, [clock, model.undo, model.redo, total, hasModal]);

  if (!chapter) return <p className="director-empty">{t("visualPlan.noChapter")}</p>;
  if (model.loading) return <LoadingSpinner inline={false} text={t("visualPlan.loadingPlan")} />;
  if (!plan) return <div className="director-empty"><p>{model.error ?? t("visualPlan.loadFailed")}</p><BookButton onClick={() => void model.reload(true)}>{t("visualPlan.retryLoad")}</BookButton></div>;

  const selectionSummary = point
    ? `${formatTime(point.at_ms)} · ${point.id === plan.points[0].id || point.id === plan.points.at(-1)?.id ? t("visualPlan.timelineUi.fixedEndpoint") : t("visualPlan.workspace.storyboardKeyframe")}`
    : segment ? segmentTimes(plan, segment).map(formatTime).join(" → ") : "";
  const isFixedPoint = point != null && (point.id === plan.points[0].id || point.id === plan.points.at(-1)?.id);
  const pointActions = point && <section className="director-property-actions">
    <h4>{t("visualPlan.workspace.pointActions")}</h4>
    {isFixedPoint
      ? <p className="director-context-line">{t("visualPlan.workspace.fixedPointCannotChange")}</p>
      : <div className="director-inline-actions"><BookButton variant="ghost" size="sm" disabled={disabled} onClick={openSplit}>{t("visualPlan.workspace.splitFromPoint")}</BookButton><BookButton className="director-delete-point" variant="ghost" size="sm" disabled={disabled} onClick={() => void edit({ kind: "remove_point", point_id: point.id })}><Trash2 size={15} />{t("visualPlan.workspace.deletePoint")}</BookButton></div>}
  </section>;
  const conflictContent = model.conflict && <div className="director-error" role="alert">
    <p>{model.conflict}</p>
    <p>{t("visualPlan.workspace.conflictExplanation")}</p>
    <div className="director-conflict-details" role="group" tabIndex={0} aria-label={t("visualPlan.workspace.conflictDetails")}>
      {model.conflictDetails.map((detail, index) => <div key={`${detail.name}:${detail.field}:${index}`}>
        <strong>{detail.name} · {detail.field}</strong>
        <p>{t("visualPlan.workspace.myEdit")}: {detail.local}</p><p>{t("visualPlan.workspace.savedVersion")}: {detail.remote}</p>
      </div>)}
    </div>
    <div className="director-inline-actions">
      <BookButton onClick={() => model.resolveConflict("local")}>{t("visualPlan.workspace.keepMyConflictingEdits")}</BookButton>
      <BookButton onClick={() => model.resolveConflict("remote")}>{t("visualPlan.workspace.useSavedConflictingFields")}</BookButton>
    </div>
  </div>;
  const inspector = <>
        {!narrow && <div className="director-inspector-heading"><h3>{point ? t("visualPlan.workspace.pointDetails") : t("visualPlan.workspace.segmentDetails")}</h3><p className="director-object-time">{selectionSummary}</p><BookIconButton label={t("visualPlan.workspace.collapseInspector")} onClick={() => void closeInspector()}><X size={15} /></BookIconButton></div>}
        {point && <>
          <fieldset disabled={disabled} className="director-fields">
            {narrow && <p className="director-object-time">{selectionSummary}</p>}
            <section className="director-property-group">
              <h4>{t("visualPlan.workspace.pointInformation")}</h4>
              <label>{t("visualPlan.workspace.name")}<input value={point.label} onChange={(event) => updatePoint(point.id, { label: event.target.value })} /></label>
              <label>{t("visualPlan.workspace.imageDescription")}<textarea rows={narrow ? 2 : 4} value={point.description} placeholder={t("visualPlan.workspace.imageDescriptionPlaceholder")} onChange={(event) => updatePoint(point.id, { description: event.target.value })} /></label>
            </section>
            <section className="director-property-group">
              <h4>{t("visualPlan.workspace.imageAssets")}</h4>
              <p className="director-binding-status" role="status">{point.image_id ? t("visualPlan.workspace.boundStoryboardImage") : t("visualPlan.workspace.noStoryboardImageBound")} · {t("visualPlan.workspace.referenceImageCount", { count: point.reference_image_ids.length })}</p>
              <div className="director-point-media-actions">
                <BookButton variant="secondary" size="sm" onClick={() => openModal("media")}><ImagePlus size={15} />{t("visualPlan.workspace.selectFromLibrary")}</BookButton>
                <label className="director-upload-button" aria-busy={uploading}>
                  {uploading ? <span role="status" className="inline-flex items-center gap-1.5"><LoadingSpinner size="sm" />{t("video.chapter.director.uploading")}</span> : <><Upload size={15} />{t("visualPlan.workspace.uploadLocalImage")}</>}
                  <input type="file" accept={supportedImageAccept()} disabled={disabled || uploading} onChange={(event) => { const file = event.target.files?.[0]; if (file) void upload(file); event.target.value = ""; }} />
                </label>
              </div>
              <ImageSizeSelect
                label={t("visualPlan.workspace.imageSize")}
                disabled={disabled}
                error={imageSizeState.error}
                loading={imageSizeState.loading}
                onChange={setSelectedImageSize}
                sizes={pointImageSizes}
                value={effectiveImageSize}
              />
              {point && !imageSizeState.loading && !imageSizeState.error && pointImageSizes.length === 0 && <p role="alert" className="director-context-line">{t("visualPlan.workspace.noCompatibleSize")}</p>}
              <BookButton variant="secondary" size="sm" disabled={pointCandidates.length === 0} onClick={() => openModal("candidates")}><Images size={15} />{t("visualPlan.workspace.storyboardCandidates")}{pointCandidates.length > 0 ? ` (${pointCandidates.length})` : ""}</BookButton>
              <DirectorGeneration
                key={`point:${point.id}`}
                projectId={projectId}
                chapterId={chapter.id}
                plan={plan}
                target={{ kind: "point", id: point.id }}
                save={model.save}
                selectedSize={effectiveImageSize}
                availableSizes={pointImageSizes}
                hasCandidates={pointCandidates.length > 0}
                onImageGenerated={rememberImage}
                onAdoptPrompt={adoptPrompt}
                onCandidateDirty={setCandidateDirty}
              />
              {point.image_id && <BookButton variant="ghost" size="sm" onClick={() => updatePoint(point.id, { image_id: null })}>{t("visualPlan.workspace.unlinkImage")}</BookButton>}
            </section>
          </fieldset>
          {!narrow && pointActions}
        </>}
        {segment && <fieldset disabled={disabled} className="director-fields">
          {narrow && <p className="director-object-time">{selectionSummary}</p>}
          <label>{t("visualPlan.workspace.shotName")}<input value={segment.title} onChange={(event) => updateSegment(segment.id, { title: event.target.value })} /></label>
          <label>{t("visualPlan.workspace.changeDescription")}<textarea rows={4} value={segment.prompt} placeholder={t("visualPlan.workspace.shotPromptPlaceholder")} onChange={(event) => updateSegment(segment.id, { prompt: event.target.value })} /></label>
          <DirectorGeneration
            key={`segment:${segment.id}`}
            projectId={projectId}
            chapterId={chapter.id}
            plan={plan}
            target={{ kind: "segment", id: segment.id }}
            save={model.save}
            selectedSize={effectiveImageSize}
            availableSizes={pointImageSizes}
            onImageGenerated={rememberImage}
            onAdoptPrompt={adoptPrompt}
            onCandidateDirty={setCandidateDirty}
          />
          <SceneBindingEditor key={`segment-scene:${segment.id}`} draftKey={inspectorSceneBindingKey} setDraftDirty={markSceneCreationDirty} setCreationBusy={setSceneCreationBusy} projectId={projectId} chapterNumber={chapter.chapter_number} binding={segment.scene_binding} scenes={scenes} disabled={disabled} onSceneCreated={rememberScene} onChange={(binding) => { setSceneBindingDraftDirty(inspectorSceneBindingKey, false); updateSegment(segment.id, { scene_binding: binding }); }} />
          <label className="director-checkbox" title={t("visualPlan.workspace.endPointImageTooltip")}><input type="checkbox" checked={segment.use_end_point_image} onChange={(event) => updateSegment(segment.id, { use_end_point_image: event.target.checked })} />{t("visualPlan.workspace.useEndPointImage")}</label>
          <div className="director-inline-actions"><BookButton variant="ghost" size="sm" disabled={disabled || sceneCreationBusy} onClick={openSplit}>{t("visualPlan.workspace.splitAction")}</BookButton><BookButton variant="ghost" size="sm" onClick={openMerge} disabled={disabled || sceneCreationBusy || segment.id === plan.segments.at(-1)?.id}>{t("visualPlan.workspace.mergeWithNext")}</BookButton></div>
        </fieldset>}
  </>;

  return <section ref={workspaceRef} className="director-workspace" aria-label={t("visualPlan.workspace.workspace")} style={{ "--director-ratio": plan.frame.width / plan.frame.height } as CSSProperties}>
    {!narrow && conflictContent}
    {narrow && model.conflict && <div className="director-error" role="status">{t("visualPlan.workspace.unresolvedConflict")}<BookButton onClick={() => setShowConflictDialog(true)}>{t("visualPlan.workspace.conflictReview")}</BookButton></div>}
    {model.syncError && <div className="director-error" role="status">{model.syncError}
      {model.invalidated && <BookButton onClick={() => void model.reloadWithBackup()}>{t("visualPlan.workspace.keepLocalBackup")}</BookButton>}
    </div>}
    <header className="director-header"><div><p className="director-eyebrow">{t("visualPlan.title")} · {t("writing.chapterLabel", { number: chapter.chapter_number })}</p><h2 title={chapter.title || t("visualPlan.chapterTimeline")}>{chapter.title || t("visualPlan.chapterTimeline")}</h2></div>
      <div className="director-inline-actions"><span className="director-save-status" role="status">{model.saving ? t("visualPlan.saving") : model.dirty ? t("visualPlan.draftNotSaved") : model.record?.record.status === "confirmed" ? t("visualPlan.workspace.statusConfirmed") : model.record ? t("visualPlan.draftSaved") : t("visualPlan.workspace.statusNew")}</span>
        <BookIconButton label={t("common.undo")} onClick={model.undo} disabled={!model.canUndo || model.busy}><Undo2 size={17} /></BookIconButton>
        <BookIconButton label={t("common.redo")} onClick={model.redo} disabled={!model.canRedo || model.busy}><Redo2 size={17} /></BookIconButton>
        {model.editable ? <><BookIconButton label={t("visualPlan.saveDraft")} onClick={() => void model.save().catch(model.report)} disabled={model.busy || model.saving}><Save size={17} /></BookIconButton><BookButton size="sm" onClick={() => void model.confirm()} disabled={model.busy || model.saving}><Check size={15} />{t("visualPlan.confirmPlan")}</BookButton></>
          : canEdit && <BookButton size="sm" onClick={() => void model.startEditing()} disabled={model.busy}>{t("visualPlan.workspace.editNewVersion")}</BookButton>}
      </div>
    </header>
    {assetLoadError ? <div role="alert" className="director-error flex flex-wrap items-center gap-2"><span className="min-w-0 flex-1">{assetLoadError}</span><BookButton variant="secondary" size="sm" onClick={() => void loadAssets()}><RefreshCw size={13} />{t("visualPlan.workspace.retryAssets")}</BookButton></div> : null}
    <div className="director-alerts">
    {model.recovery && <div className="director-notice">{t("visualPlan.workspace.restoreLocalNotice")}<BookButton variant="secondary" size="sm" onClick={() => { void confirmDialog({ title: t("visualPlan.workspace.restoreLocalTitle"), message: t("visualPlan.workspace.restoreLocalMessage"), confirmText: t("visualPlan.workspace.restoreLocalAction") }).then((accepted) => { if (accepted) void model.restore(); }); }}>{t("visualPlan.workspace.restoreLocalAction")}</BookButton></div>}
    {model.issues.length > 0 && <div className="director-error" role="alert">{model.issues.map((issue) => <button key={issue.path} type="button" onClick={() => {
      const match = /^(points|segments)\[(\d+)\]/.exec(issue.path);
      if (match) { const index = Number(match[2]); const item = match[1] === "points" ? plan.points[index] : plan.segments[index]; if (item) select({ kind: match[1] === "points" ? "point" : "segment", id: item.id }, match[1] === "points" ? plan.points[index].at_ms : segmentTimes(plan, plan.segments[index])[0]); }
    }}>{issue.message}</button>)}</div>}
    </div>

    <div className="director-tool-rail" role="toolbar" aria-label={t("visualPlan.workspace.toolbar")}>
        <Tool icon={Settings2} label={t("visualPlan.workspace.properties")} pressed={inspectorOpen && !agentSidebarOpen} opensDialog={narrow} expanded={inspectorOpen && !agentSidebarOpen} onClick={() => {
          if (agentSidebarOpen) {
            setModalPanel(null);
            setInspectorOpen(true);
          } else {
            setInspectorOpen((current) => !current);
          }
        }} />
        <span className="director-tool-divider" aria-hidden="true" />
        <Tool icon={Images} label={t("visualPlan.workspace.mediaLibrary")} opensDialog expanded={modalPanel === "media"} onClick={() => openModal("media")} />
        <Tool icon={Mountain} label={t("visualPlan.workspace.sceneLibrary")} opensDialog expanded={modalPanel === "scenes"} onClick={() => openModal("scenes")} />
        <Tool icon={BookOpen} label={t("visualPlan.workspace.sourceText")} opensDialog expanded={modalPanel === "source"} onClick={() => openModal("source")} />
        <Tool icon={Film} label={t("visualPlan.workspace.frame")} opensDialog expanded={modalPanel === "settings"} onClick={() => openModal("settings")} />
        <Tool icon={Sparkles} label={t("visualPlan.workspace.directorAgent")} pressed={modalPanel === "agent"} opensDialog={narrow} expanded={modalPanel === "agent"} onClick={() => {
          if (agentSidebarOpen) closeAgentSidebar();
          else openModal("agent");
        }} />
    </div>
    <div className={`director-stage ${inspectorOpen && !narrow ? "has-inspector" : ""}`}>
      <div className="director-editing-column">
        <div className="director-player-column">
          <div className="director-preview-surface">
            <div className="director-viewer" ref={frameRef} style={{ aspectRatio: `${plan.frame.width} / ${plan.frame.height}` }}>
              <DirectorFrame plan={plan} clock={clock} imageById={imageById} />
              <button className="director-fullscreen" type="button" aria-label={t("visualPlan.workspace.fullscreen")} onClick={() => void (document.fullscreenElement ? document.exitFullscreen() : frameRef.current?.requestFullscreen())?.catch(model.report)}><Maximize size={17} /></button>
            </div>
          </div>
          <Transport clock={clock} plan={plan} />
        </div>
        <DirectorTimeline viewRef={timelineView} plan={plan} record={model.record} selection={selection} clock={clock} editable={!disabled} sceneNames={names} onSelect={select} onEdit={edit} onSplit={openSplit} onError={model.report} />
      </div>
      {inspectorOpen && !narrow && <aside className={`director-inspector ${agentSidebarOpen ? "director-agent-inspector" : ""}`} aria-label={agentSidebarOpen ? t("visualPlan.workspace.directorAgent") : t("visualPlan.workspace.properties")}>
        {agentSidebarOpen ? <DomainAgentConsole
          executionMode={agentExecutionMode} onExecutionModeChange={setAgentExecutionMode}
          capability="director"
          projectId={projectId}
          composerDirtyKey={agentComposerDirtyKey}
          chapterId={chapter.id}
          resourceLabel={chapter.title}
          fill
          className="director-agent-console min-h-0 flex-1 rounded-none border-0 shadow-none"
          onClose={closeAgentSidebar}
          getEditorSource={getEditorSource}
        /> : inspector}
      </aside>}
    </div>

    <Modal isOpen={inspectorOpen && narrow} title={point ? t("visualPlan.workspace.pointProperties") : t("visualPlan.workspace.segmentProperties")} onClose={() => void closeInspector()} className="director-mobile-inspector" footer={narrow ? pointActions : undefined}>{inspector}</Modal>
    <Modal isOpen={modalPanel === "source"} title={t("visualPlan.workspace.chapterSource")} onClose={closeModal} maxWidthClassName="max-w-3xl"><div className="director-source">{model.set?.source.content}</div></Modal>
    <Modal isOpen={modalPanel === "scenes"} title={t("visualPlan.workspace.projectSceneLibrary")} onClose={closeSceneLibrary} maxWidthClassName="max-w-6xl"><Suspense fallback={<LoadingSpinner text={t("visualPlan.workspace.loadingSceneLibrary")} />}><SceneLibrary projectId={projectId} canEditProfiles={canEdit} /></Suspense></Modal>
    <Modal
      isOpen={modalPanel === "agent" && narrow}
      title={t("visualPlan.workspace.directorAgent")}
      onClose={closeModal}
      maxWidthClassName="max-w-4xl"
      closeOnBackdrop={false}
      closeOnEscape={false}
      showCloseButton={false}
    >
      <DomainAgentConsole capability="director" projectId={projectId} composerDirtyKey={agentComposerDirtyKey} chapterId={chapter.id} resourceLabel={chapter.title} className="director-agent-console director-agent-modal-console" onClose={closeModal} getEditorSource={getEditorSource} executionMode={agentExecutionMode} onExecutionModeChange={setAgentExecutionMode} />
    </Modal>
    <Modal isOpen={modalPanel === "settings"} title={t("visualPlan.workspace.frame")} onClose={closeModal}>
      <div className="director-inline-actions">{[[1920, 1080, t("visualPlan.workspace.aspect16x9")], [1080, 1920, t("visualPlan.workspace.aspect9x16")], [1080, 1080, t("visualPlan.workspace.aspect1x1")], [1440, 1080, t("visualPlan.workspace.aspect4x3")]].map(([width, height, label]) => {
        const selected = plan.frame.width * Number(height) === plan.frame.height * Number(width);
        return <BookButton key={label} variant={selected ? "primary" : "secondary"} aria-pressed={selected} disabled={disabled} onClick={() => { void edit({ kind: "set_frame", frame: { width: Number(width), height: Number(height) } }); closeModal(); }}>{label}</BookButton>;
      })}</div>
      <p className="director-context-line">{t("visualPlan.workspace.frameImageHint")}</p>
    </Modal>
    <Modal isOpen={modalPanel === "media"} title={point ? t("visualPlan.workspace.pointMediaTitle", { name: point.label || t("visualPlan.timelineUi.imagePoint") }) : t("visualPlan.workspace.mediaLibraryTitle")} onClose={closeModal} maxWidthClassName="max-w-5xl">
      {!point && <p className="director-context-line">{t("visualPlan.workspace.selectPointForBinding")}</p>}
      <div className="director-media-grid">{images.map((image) => <article key={image.id}>
        <ZoomableImage src={toMediaSrc(image.file_path)} alt={image.prompt || t("visualPlan.workspace.imageAsset")} loading="lazy" />
        <p className="mb-2 line-clamp-2 min-h-[2.5rem] text-small leading-relaxed text-book-text-sub">{image.prompt.trim() || t("visualPlan.workspace.imageAsset")}</p>
        <div className="director-inline-actions">
        <BookButton variant="secondary" size="sm" disabled={!point || disabled} onClick={() => { if (point) void adoptImage(point.id, image).then(() => { closeModal(); setInspectorOpen(true); }).catch(model.report); }}>{point?.image_id === image.id ? t("visualPlan.workspace.boundStoryboardImage") : t("visualPlan.workspace.chooseStoryboard")}</BookButton>
        <BookButton variant="ghost" size="sm" disabled={!point || disabled} onClick={() => { if (point) updatePoint(point.id, { reference_image_ids: point.reference_image_ids.includes(image.id) ? point.reference_image_ids.filter((id) => id !== image.id) : [...point.reference_image_ids, image.id] }); }}>{point?.reference_image_ids.includes(image.id) ? t("visualPlan.workspace.cancelReference") : t("visualPlan.workspace.chooseReference")}</BookButton>
      </div></article>)}</div>{images.length === 0 && <p className="director-empty">{t("visualPlan.workspace.noImagesHint")}</p>}
    </Modal>
    <Modal isOpen={modalPanel === "candidates"} title={point ? t("visualPlan.workspace.pointCandidatesTitle", { name: point.label || t("visualPlan.timelineUi.imagePoint") }) : t("visualPlan.workspace.candidatesTitle")} onClose={closeCandidateModal} maxWidthClassName="max-w-4xl">
      <div className="director-candidate-modal">
        <p className="director-context-line">{t("visualPlan.workspace.candidatesCountHint", { count: pointCandidates.length })}</p>
        <div className="director-point-candidate-grid" role="list" aria-label={t("visualPlan.workspace.candidateList")}>
          {pointCandidates.map((image, index) => {
            const bound = point?.image_id === image.id;
            return <article key={image.id} className={`director-point-candidate ${bound ? "is-bound" : ""}`} role="listitem">
              <ZoomableImage src={toMediaSrc(image.file_path)} alt={image.prompt || t("visualPlan.workspace.candidateImageAlt", { number: index + 1 })} loading="lazy" />
              <span className="director-point-candidate-label">{t("visualPlan.workspace.candidateNumber", { number: index + 1 })}</span>
              <BookButton variant={bound ? "primary" : "secondary"} size="sm" disabled={disabled || bound} aria-label={bound ? t("visualPlan.workspace.currentBoundCandidate") : t("visualPlan.workspace.bindCandidate")} aria-pressed={bound} onClick={() => { if (point) void adoptImage(point.id, image).catch(model.report); }}>{bound ? <><Check size={13} />{t("visualPlan.workspace.currentBoundCandidate")}</> : t("visualPlan.workspace.bindCandidate")}</BookButton>
            </article>;
          })}
        </div>
      </div>
    </Modal>
    {split && <Modal isOpen title={t("visualPlan.workspace.splitTitle")} onClose={() => void closeSplit()} footer={<BookButton disabled={model.busy} onClick={() => { void model.mutate({ kind: "split_segment", segment_id: split.segmentId, point_id: split.pointId, right_segment_id: crypto.randomUUID(), left_prompt: split.leftPrompt, right_prompt: split.rightPrompt, left_scene_binding: split.leftScene, right_scene_binding: split.rightScene }, split.base).then((succeeded) => { if (succeeded) setSplit(null); }); }}>{t("visualPlan.workspace.splitAction")}</BookButton>}>
        <div className="director-fields"><label>{t("visualPlan.workspace.splitPointPosition")}<ThemedSelect className="director-select" aria-label={t("visualPlan.workspace.splitPointPosition")} value={split.pointId} onChange={(selectedValue) => setSplit({ ...split, pointId: selectedValue, dirty: true })}>{pointsInsideSegment(split.base.plan, split.segmentId).map((point) => <option key={point.id} value={point.id}>{formatTime(point.at_ms)} · {point.label}</option>)}</ThemedSelect></label>
        <label>{t("visualPlan.workspace.firstPartDescription")}<textarea rows={4} value={split.leftPrompt} onChange={(event) => setSplit({ ...split, leftPrompt: event.target.value, dirty: true })} /></label><SceneBindingEditor draftKey="split:left" setDraftDirty={markSceneCreationDirty} setCreationBusy={setSceneCreationBusy} projectId={projectId} chapterNumber={chapter.chapter_number} binding={split.leftScene} scenes={scenes} disabled={model.busy} onSceneCreated={rememberScene} onChange={(leftScene) => setSplit({ ...split, leftScene, dirty: true })} />
        <label>{t("visualPlan.workspace.secondPartDescription")}<textarea rows={4} value={split.rightPrompt} onChange={(event) => setSplit({ ...split, rightPrompt: event.target.value, dirty: true })} placeholder={t("visualPlan.workspace.secondPartPlaceholder")} /></label><SceneBindingEditor draftKey="split:right" setDraftDirty={markSceneCreationDirty} setCreationBusy={setSceneCreationBusy} projectId={projectId} chapterNumber={chapter.chapter_number} binding={split.rightScene} scenes={scenes} disabled={model.busy} onSceneCreated={rememberScene} onChange={(rightScene) => setSplit({ ...split, rightScene, dirty: true })} /></div>
    </Modal>}
    {merge && <Modal isOpen title={t("visualPlan.workspace.mergeTitle")} onClose={() => void closeMerge()} footer={<BookButton disabled={model.busy} onClick={() => { void model.mutate({ kind: "merge_segments", left_segment_id: merge.leftId, right_segment_id: merge.rightId, prompt: merge.prompt, scene_binding: merge.scene }, merge.base).then((succeeded) => { if (succeeded) setMerge(null); }); }}>{merge.prompt.trim() ? t("visualPlan.workspace.mergeAction") : t("visualPlan.workspace.mergeIncompleteAction")}</BookButton>}>
      <div className="director-fields"><details open><summary>{t("visualPlan.workspace.originalShotDescription")}</summary>{[merge.leftId, merge.rightId].map((id) => <p key={id}>{plan.segments.find((segment) => segment.id === id)?.prompt || t("visualPlan.workspace.notFilled")}</p>)}</details><label>{t("visualPlan.workspace.mergedChangeDescription")}<textarea rows={6} value={merge.prompt} onChange={(event) => setMerge({ ...merge, prompt: event.target.value, dirty: true })} /></label><SceneBindingEditor draftKey="merge" setDraftDirty={markSceneCreationDirty} setCreationBusy={setSceneCreationBusy} projectId={projectId} chapterNumber={chapter.chapter_number} binding={merge.scene} scenes={scenes} disabled={model.busy} onSceneCreated={rememberScene} onChange={(scene) => setMerge({ ...merge, scene, dirty: true })} /><p className="director-context-line">{t("visualPlan.workspace.mergePreservesPoints")}</p></div>
    </Modal>}
    <Modal isOpen={narrow && showConflictDialog && model.conflict != null} title={t("visualPlan.workspace.conflictTitle")} onClose={() => setShowConflictDialog(false)}>{conflictContent}</Modal>
  </section>;
}

function Tool({ icon: Icon, label, pressed, opensDialog = false, expanded = false, onClick }: { icon: typeof Film; label: string; pressed?: boolean; opensDialog?: boolean; expanded?: boolean; onClick: () => void }) {
  return <button type="button" title={label} aria-label={label} aria-pressed={pressed} aria-haspopup={opensDialog ? "dialog" : undefined} aria-expanded={opensDialog ? expanded : undefined} className={pressed ? "is-active" : ""} onClick={onClick}><Icon size={19} /><span>{label}</span></button>;
}

const NEW_SCENE_VALUE = "__new_scene__";
type SceneSelectTarget = "entry" | "exit";

function SceneBindingEditor({ draftKey, setDraftDirty, setCreationBusy, projectId, chapterNumber, binding, scenes, disabled = false, onSceneCreated, onChange }: {
  draftKey: string;
  setDraftDirty: (key: string, dirty: boolean) => void;
  setCreationBusy: (key: string, busy: boolean) => void;
  projectId: number;
  chapterNumber: number;
  binding: SceneBindingV3 | null;
  scenes: Array<Pick<SceneProfile, "id" | "project_id" | "name">>;
  disabled?: boolean;
  onSceneCreated: (scene: SceneProfile) => void;
  onChange: (binding: SceneBindingV3 | null) => void;
}) {
  const { t } = useI18n();
  const { addToast } = useToast();
  const entry = binding?.kind === "scene" ? binding.scene_profile_id : binding?.from_scene_profile_id;
  const exit = binding?.kind === "scene" ? binding.scene_profile_id : binding?.to_scene_profile_id;
  const [newSceneTarget, setNewSceneTarget] = useState<SceneSelectTarget | null>(null);
  const [newSceneName, setNewSceneName] = useState("");
  const [newSceneDescription, setNewSceneDescription] = useState("");
  const [creating, setCreating] = useState(false);
  const scopeRef = useRef(`${projectId}:${chapterNumber}:${draftKey}`);
  scopeRef.current = `${projectId}:${chapterNumber}:${draftKey}`;
  const clearNewScene = () => {
    setDraftDirty(draftKey, false);
    setNewSceneTarget(null);
    setNewSceneName("");
    setNewSceneDescription("");
  };
  const resetNewScene = async (): Promise<boolean> => {
    if (newSceneTarget && (newSceneName.trim() || newSceneDescription.trim()) && !(await confirmDialog({
      title: t("visualPlan.workspace.discardSceneTitle"),
      message: t("visualPlan.workspace.discardSceneMessage"),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return false;
    clearNewScene();
    return true;
  };
  const applyScene = (target: SceneSelectTarget, id: number) => {
    if (target === "entry") {
      onChange(binding?.kind === "transition"
        ? { ...binding, from_scene_profile_id: id }
        : { kind: "scene", scene_profile_id: id });
    } else if (binding?.kind === "transition") {
      onChange({ ...binding, to_scene_profile_id: id });
    }
  };
  const chooseScene = async (target: SceneSelectTarget, value: string) => {
    if (value === NEW_SCENE_VALUE) {
      if (!(await resetNewScene())) return;
      setDraftDirty(draftKey, true);
      setNewSceneTarget(target);
      setNewSceneName("");
      setNewSceneDescription("");
      return;
    }
    if (!(await resetNewScene())) return;
    const id = Number(value);
    if (!id) {
      onChange(null);
      return;
    }
    applyScene(target, id);
  };
  const changeBindingKind = async (value: string) => {
    if (!(await resetNewScene())) return;
    const first = entry ?? scenes[0]?.id;
    if (!first) {
      setDraftDirty(draftKey, true);
      setNewSceneTarget("entry");
      return;
    }
    onChange(value === "scene" ? { kind: "scene", scene_profile_id: first } : { kind: "transition", from_scene_profile_id: first, to_scene_profile_id: exit ?? scenes.find((scene) => scene.id !== first)?.id ?? first });
  };
  const createCustomScene = async () => {
    const name = newSceneName.trim();
    if (!name || !newSceneTarget || creating) return;
    const description = newSceneDescription.trim();
    const requestScope = scopeRef.current;
    setCreating(true);
    setCreationBusy(draftKey, true);
    try {
      const result = await createSceneProfile(projectId, {
        name,
        aliases: [],
        scene_type: "",
        importance: "",
        parent_scene_id: null,
        base_description: description,
        visual_description: description,
        spatial_layout: "",
        atmosphere: "",
        entry_points_json: [],
        rules_json: [],
        first_seen_chapter: Math.max(1, chapterNumber),
        last_seen_chapter: Math.max(1, chapterNumber),
        evidence: [],
        image_prompt_seed: description || name,
      });
      if (scopeRef.current !== requestScope || result.profile.project_id !== projectId || result.profile.first_seen_chapter > chapterNumber || result.profile.last_seen_chapter < chapterNumber) {
        throw new Error(t("visualPlan.workspace.customSceneInvalid"));
      }
      onSceneCreated(result.profile);
      applyScene(newSceneTarget, result.profile.id);
      clearNewScene();
    } catch (error) {
      addToast(extractErrorMessage(error, t("visualPlan.workspace.customSceneCreateFailed")), "error");
    } finally {
      setCreating(false);
      setCreationBusy(draftKey, false);
    }
  };
  const options = <><option value="">{t("visualPlan.workspace.sceneSelect")}</option>{scenes.map((scene) => <option key={scene.id} value={scene.id}>{scene.name}</option>)}<option value={NEW_SCENE_VALUE}>{t("visualPlan.workspace.createSceneOption")}</option></>;
  const entryLabel = binding?.kind === "transition" ? t("visualPlan.workspace.startScene") : t("visualPlan.scene");
  return <div className="director-scene-binding"><label>{t("visualPlan.workspace.sceneBindingMode")}<ThemedSelect className="director-select" aria-label={t("visualPlan.workspace.sceneBindingMode")} value={binding?.kind ?? "scene"} disabled={disabled || creating} onChange={(selectedValue) => void changeBindingKind(selectedValue)}><option value="scene">{t("visualPlan.workspace.singleScene")}</option><option value="transition">{t("visualPlan.workspace.sceneTransition")}</option></ThemedSelect></label>
    <label>{entryLabel}<ThemedSelect className="director-select" aria-label={entryLabel} value={newSceneTarget === "entry" ? NEW_SCENE_VALUE : entry ?? ""} disabled={disabled || creating} onChange={(selectedValue) => void chooseScene("entry", selectedValue)}>{options}</ThemedSelect></label>
    {binding?.kind === "transition" && <label>{t("visualPlan.workspace.targetScene")}<ThemedSelect className="director-select" aria-label={t("visualPlan.workspace.targetScene")} value={newSceneTarget === "exit" ? NEW_SCENE_VALUE : binding.to_scene_profile_id} disabled={disabled || creating} onChange={(selectedValue) => void chooseScene("exit", selectedValue)}>{options}</ThemedSelect></label>}
    {newSceneTarget && <div className="director-custom-scene"><label>{t("visualPlan.workspace.customSceneName")}<input autoFocus value={newSceneName} disabled={disabled || creating} onChange={(event) => { setDraftDirty(draftKey, true); setNewSceneName(event.target.value); }} placeholder={t("visualPlan.workspace.customSceneNamePlaceholder")} /></label><label>{t("visualPlan.workspace.optionalSceneDescription")}<textarea rows={3} value={newSceneDescription} disabled={disabled || creating} onChange={(event) => { setDraftDirty(draftKey, true); setNewSceneDescription(event.target.value); }} placeholder={t("visualPlan.workspace.customSceneDescriptionPlaceholder")} /></label><div className="director-inline-actions"><BookButton variant="secondary" size="sm" disabled={disabled || creating || !newSceneName.trim()} onClick={() => void createCustomScene()}><Plus size={14} />{t("visualPlan.workspace.saveAndBind")}</BookButton><BookButton variant="ghost" size="sm" disabled={creating} onClick={() => void resetNewScene()}>{t("visualPlan.workspace.cancel")}</BookButton></div></div>}
  </div>;
}

function DirectorFrame({ plan, clock, imageById }: { plan: ChapterVisualPlanV3; clock: DirectorPlayback; imageById: ReadonlyMap<number, GeneratedImage> }) {
  const { t } = useI18n();
  const pointId = useStore(clock, (state) => {
    const point = pointAt(plan, state.time);
    return point.at_ms === durationMs(plan) && point.image_id == null ? plan.points[plan.points.length - 2].id : point.id;
  });
  const point = plan.points.find((point) => point.id === pointId)!;
  const image = point.image_id == null ? null : imageById.get(point.image_id);
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [image?.file_path]);
  useEffect(() => {
    const index = plan.points.findIndex((point) => point.id === pointId);
    const next = plan.points[index + 1];
    const image = next?.image_id ? imageById.get(next.image_id) : null;
    if (image) { const preload = new Image(); preload.src = toMediaSrc(image.file_path); }
  }, [imageById, plan.points, pointId]);
  return <>{image && !failed ? <ZoomableImage className="director-frame-image" src={toMediaSrc(image.file_path)} alt={point.description || point.label || t("visualPlan.workspace.currentFrame")} onError={() => setFailed(true)} />
    : <div className="director-frame-empty"><ImagePlus size={34} strokeWidth={1.2} /><strong>{point.image_id ? t("visualPlan.workspace.imageLoadFailed") : t("visualPlan.workspace.pointImageUnbound")}</strong><span>{formatTime(point.at_ms)} · {point.label || t("visualPlan.timelineUi.imagePoint")}</span></div>}
    <div className="director-frame-caption">{point.label || t("visualPlan.workspace.currentFrame")} <span>{formatTime(point.at_ms)}</span></div></>;
}

function Transport({ clock, plan }: { clock: DirectorPlayback; plan: ChapterVisualPlanV3 }) {
  const { t } = useI18n();
  const time = useStore(clock, (state) => state.time);
  const playing = useStore(clock, (state) => state.playing);
  const previous = [...plan.points].reverse().find((point) => point.at_ms < time);
  const next = plan.points.find((point) => point.at_ms > time);
  return <div className="director-transport"><div className="director-inline-actions"><BookIconButton label={t("visualPlan.workspace.previousImagePoint")} disabled={!previous} onClick={() => { if (previous) clock.getState().seek(previous.at_ms); }}><SkipBack size={16} /></BookIconButton><BookIconButton label={playing ? t("visualPlan.workspace.pausePlayback") : t("visualPlan.workspace.playStoryboard")} onClick={() => clock.getState().toggle()}>{playing ? <Pause size={19} /> : <Play size={19} />}</BookIconButton><BookIconButton label={t("visualPlan.workspace.nextImagePoint")} disabled={!next} onClick={() => { if (next) clock.getState().seek(next.at_ms); }}><SkipForward size={16} /></BookIconButton></div><DirectorScrubber clock={clock} total={durationMs(plan)} /><div className="director-timecode"><span>{formatTime(time)}</span><span>/ {formatTime(durationMs(plan))}</span></div><span className="director-preview-label">{t("visualPlan.workspace.storyboardPreview")}</span></div>;
}
