import { lazy, Suspense, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { useStore } from "zustand";
import { BookOpen, Check, Film, ImagePlus, Images, Maximize, Mountain, Pause, Play, Redo2, Save, Settings2, SkipBack, SkipForward, Sparkles, Trash2, Undo2, Upload, X } from "lucide-react";
import type { ChapterSummary } from "../../api/chapter";
import type { ChapterVisualPlanV3, SceneBindingV3, VisualPointV3, VisualSegmentV3, VisualPlanPromptCandidate, VisualPlanEditOperation } from "../../api/chapterVisualPlan";
import { listImagesByProjects, uploadImage, type GeneratedImage } from "../../api/image";
import { listSceneProfiles, type SceneProfile } from "../../api/sceneProfiles";
import { respondAgentEditor, type AgentEditorRequest, type AgentEditorResponse, type AgentEditorSource } from "../../api/agent";
import { DomainAgentConsole } from "../../components/agent/DomainAgentConsole";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { Modal } from "../../components/ui/Modal";
import { readFileAsBase64 } from "../../utils/fileUpload";
import { toMediaSrc } from "../../utils/mediaSrc";
import { DirectorGeneration } from "./visual-plan/DirectorGeneration";
import { DirectorScrubber, DirectorTimeline } from "./visual-plan/DirectorTimeline";
import { useDirectorPlan } from "./visual-plan/useDirectorPlan";
import { useDirectorPlayback, type DirectorPlayback } from "./visual-plan/playback";
import { durationMs, formatTime, pointAt, segmentAt, segmentTimes, type DirectorSelection } from "./visual-plan/timeline";
import "./visual-plan/director.css";

const SceneLibrary = lazy(() => import("./SceneProfilesTab").then((module) => ({ default: module.SceneProfilesTab })));
type Panel = "properties" | "source" | "scenes" | "media" | "settings" | "agent" | null;
type SplitDraft = { segmentId: string; pointId: string; leftPrompt: string; rightPrompt: string; leftScene: SceneBindingV3 | null; rightScene: SceneBindingV3 | null };
type MergeDraft = { leftId: string; rightId: string; prompt: string; scene: SceneBindingV3 | null };

export function ChapterVisualPlanTab({ projectId, canEdit, chapter }: { projectId: number; canEdit: boolean; chapter: ChapterSummary | null }) {
  const chapterId = chapter?.id ?? null;
  const model = useDirectorPlan(projectId, chapterId, canEdit);
  const plan = model.plan;
  const total = plan ? durationMs(plan) : 0;
  const clock = useDirectorPlayback(total, `${projectId}:${chapterId}`);
  const [selection, setSelection] = useState<DirectorSelection | null>(null);
  const [panel, setPanel] = useState<Panel>(null);
  const [narrow, setNarrow] = useState(true);
  const workspaceRef = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    const workspace = workspaceRef.current;
    if (!workspace) return;
    // The chapter sidebar changes the space available without resizing the window.
    let wasNarrow: boolean | undefined;
    const update = () => {
      const next = workspace.clientWidth < 760;
      if (next === wasNarrow) return;
      wasNarrow = next;
      setNarrow(next);
      setPanel((current) => current == null || current === "properties" ? (next ? null : "properties") : current);
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(workspace);
    return () => observer.disconnect();
  }, [model.loading, plan != null]);
  const [generation, setGeneration] = useState<DirectorSelection | null>(null);
  const [images, setImages] = useState<GeneratedImage[]>([]);
  const [scenes, setScenes] = useState<SceneProfile[]>([]);
  const [assetError, setAssetError] = useState<string | null>(null);
  const [split, setSplit] = useState<SplitDraft | null>(null);
  const [merge, setMerge] = useState<MergeDraft | null>(null);
  const [uploading, setUploading] = useState(false);
  const frameRef = useRef<HTMLDivElement>(null);
  const assetScope = useRef(0);
  const names = useMemo(() => new Map(scenes.map((scene) => [scene.id, scene.name])), [scenes]);
  const imageById = useMemo(() => new Map(images.map((image) => [image.id, image])), [images]);
  const point = selection?.kind === "point" ? plan?.points.find((point) => point.id === selection.id) : undefined;
  const segment = selection?.kind === "segment" ? plan?.segments.find((segment) => segment.id === selection.id) : undefined;
  const disabled = !model.editable || model.busy;
  const hasModal = Boolean(generation || split || merge || (panel && (panel !== "properties" || narrow)));
  const closePanel = () => setPanel(narrow ? null : "properties");

  const loadAssets = useCallback(async () => {
    const scope = ++assetScope.current;
    const results = await Promise.allSettled([
      listImagesByProjects([...new Set([projectId, chapter?.project_id ?? projectId])]),
      listSceneProfiles(chapter?.project_id ?? projectId),
    ]);
    if (scope !== assetScope.current) return;
    const failures: string[] = [];
    if (results[0].status === "fulfilled") setImages(results[0].value);
    else failures.push(`图片库加载失败：${String(results[0].reason)}`);
    if (results[1].status === "fulfilled") setScenes(results[1].value);
    else failures.push(`场景库加载失败：${String(results[1].reason)}`);
    setAssetError(failures.length ? failures.join("；") : null);
  }, [projectId, chapter?.project_id]);

  useEffect(() => {
    setSelection(null);
    setPanel(null);
    setGeneration(null);
    setSplit(null);
    setMerge(null);
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

  useEffect(() => { if (point) clock.getState().seek(point.at_ms); }, [clock, point?.id, point?.at_ms]);

  const select = (next: DirectorSelection, time: number) => {
    setSelection(next);
    setPanel("properties");
    clock.getState().seek(time);
  };
  const updatePoint = (id: string, fields: Partial<VisualPointV3>) => {
    const current = model.planRef.current;
    if (!current || !current.points.some((point) => point.id === id)) throw new Error("图片点已不存在");
    model.updateFields({ ...current, points: current.points.map((point) => point.id === id ? { ...point, ...fields } : point) });
  };
  const updateSegment = (id: string, fields: Partial<VisualSegmentV3>) => {
    const current = model.planRef.current;
    if (!current || !current.segments.some((segment) => segment.id === id)) throw new Error("镜头段已不存在");
    model.updateFields({ ...current, segments: current.segments.map((segment) => segment.id === id ? { ...segment, ...fields } : segment) });
  };
  const edit = async (operation: VisualPlanEditOperation) => {
    clock.getState().pause();
    await model.mutate(operation).catch(model.report);
  };

  const openSplit = () => {
    if (!plan) return;
    const target = segment ?? segmentAt(plan, point?.at_ms ?? clock.getState().time);
    if (!target) return;
    const [start, end] = segmentTimes(plan, target);
    const candidates = plan.points.filter((point) => point.at_ms > start && point.at_ms < end);
    const splitPoint = candidates.find((candidate) => candidate.id === point?.id) ?? candidates[0];
    if (!splitPoint) { model.report(new Error("请先在镜头内部添加一个图片点，再从该点拆分镜头。")); return; }
    setSplit({ segmentId: target.id, pointId: splitPoint.id, leftPrompt: target.prompt, rightPrompt: "",
      leftScene: target.scene_binding?.kind === "transition" ? { kind: "scene", scene_profile_id: target.scene_binding.from_scene_profile_id } : target.scene_binding,
      rightScene: target.scene_binding });
  };
  const openMerge = () => {
    if (!plan || !segment) return;
    const right = plan.segments[plan.segments.findIndex((item) => item.id === segment.id) + 1];
    if (!right) return;
    setMerge({ leftId: segment.id, rightId: right.id, prompt: "", scene: segment.scene_binding });
  };
  const adoptImage = async (pointId: string, image: GeneratedImage) => {
    if (!model.editable) throw new Error("请先进入草稿编辑模式");
    updatePoint(pointId, { image_id: image.id });
    await model.save();
    setImages((current) => current.some((item) => item.id === image.id) ? current : [...current, image]);
  };
  const adoptPrompt = async (candidate: VisualPlanPromptCandidate) => {
    if (!model.editable || candidate.source_chapter_id !== chapterId) throw new Error("候选不属于当前可编辑章节");
    const current = model.planRef.current?.segments.find((segment) => segment.id === candidate.segment_id);
    if (!current) throw new Error("候选对应的镜头段已删除");
    if (model.recordRef.current?.record.plan_sha256 !== candidate.plan_sha256) {
      const accepted = await confirmDialog({ title: "编排已修改", message: `当前描述：${current.prompt || "尚未填写"}\n\n候选描述：${candidate.prompt}\n\n请核对后采用。`, confirmText: "采用候选" });
      if (!accepted) throw new Error("已保留候选，尚未采用");
    }
    updateSegment(candidate.segment_id, { prompt: candidate.prompt });
    await model.save();
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
    return { view_instance_id: model.viewInstanceId.current, project_id: projectId, chapter_id: chapterId,
      editor_revision: model.editorRevision.current, scene_id: null, shot_id: owner?.id ?? null,
      keyframe_id: point?.id ?? null, entity_id: null };
  };
  const editorRequest = async (request: AgentEditorRequest, runId: string) => {
    if (request.source.view_instance_id !== model.viewInstanceId.current) return;
    let submitted = false;
    const respond = async (response: AgentEditorResponse) => {
      submitted = true;
      const reply = await respondAgentEditor({ run_id: runId, execution_id: request.execution_id, view_instance_id: model.viewInstanceId.current, response });
      if (reply.status !== "succeeded") throw new Error(reply.error_message || "编辑器请求失败");
      return reply;
    };
    try {
      if (model.busy || model.saving || !model.planRef.current) throw new Error("编辑器正在保存，请稍后重试");
      const operation = request.operation;
      const expected = operation.kind === "prepare_write" ? operation.expected_editor_revision : operation.kind === "read" || operation.kind === "save" ? operation.input.expected_editor_revision : null;
      if (expected && expected !== model.editorRevision.current) throw new Error("编辑器内容已改变，请重新读取");
      const state = { editor_revision: model.editorRevision.current, record_id: model.recordRef.current?.record.id ?? null,
        record_revision: model.recordRef.current?.record.revision ?? null, dirty: model.dirty, has_uncommitted_dialog: split != null || merge != null };
      if (operation.kind === "inspect") await respond({ kind: "state", state });
      else if (operation.kind === "read") await respond({ kind: "draft", editor_revision: state.editor_revision, plan: model.planRef.current });
      else if (operation.kind === "save") {
        if (state.has_uncommitted_dialog) throw new Error("请先完成拆分或合并编辑");
        const reply = await respond({ kind: "save_draft", state, plan: model.planRef.current });
        if (!reply.saved_draft) throw new Error("Agent 保存未返回草稿");
        model.acceptRecord(reply.saved_draft);
        model.replace(reply.saved_draft.plan);
      } else {
        if (state.dirty || state.has_uncommitted_dialog) throw new Error("当前仍有未保存编辑，请先保存");
        await respond({ kind: "ready_to_write", state });
        await model.reload();
      }
    } catch (failure) {
      if (!submitted) await respond({ kind: "error", message: String(failure) });
      throw failure;
    }
  };

  useEffect(() => {
    const keyboard = (event: KeyboardEvent) => {
      if (hasModal || event.defaultPrevented || (event.target instanceof Element && event.target.closest('input,textarea,select,[contenteditable=true],[role="dialog"]'))) return;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") { event.preventDefault(); event.shiftKey ? model.redo() : model.undo(); }
      if (event.key === " " && !(event.target instanceof HTMLButtonElement)) { event.preventDefault(); clock.getState().toggle(); }
      if (event.key === "Home") { event.preventDefault(); clock.getState().seek(0); }
      if (event.key === "End") { event.preventDefault(); clock.getState().seek(total); }
    };
    window.addEventListener("keydown", keyboard);
    return () => window.removeEventListener("keydown", keyboard);
  }, [clock, model.undo, model.redo, total, hasModal]);

  if (!chapter) return <p className="director-empty">选择一个章节，开始编排分镜。</p>;
  if (model.loading) return <LoadingSpinner inline={false} text="正在加载导演编排…" />;
  if (!plan) return <div className="director-error" role="alert">{model.error}<BookButton onClick={() => void model.reload(true)}>重新加载</BookButton></div>;

  const selectionSummary = point
    ? `${formatTime(point.at_ms)} · ${point.id === plan.points[0].id || point.id === plan.points.at(-1)?.id ? "固定端点" : "分镜关键帧"}`
    : segment ? segmentTimes(plan, segment).map(formatTime).join(" → ") : "";
  const inspector = <>
        {!narrow && <div className="director-inspector-heading"><h3>{point ? "图片点" : "镜头段"}</h3><p className="director-object-time">{selectionSummary}</p><BookIconButton label="收起属性" onClick={() => setPanel(null)}><X size={15} /></BookIconButton></div>}
        {point && <fieldset disabled={disabled} className="director-fields">
          {narrow && <p className="director-object-time">{selectionSummary}</p>}
          <label>名称<input value={point.label} onChange={(event) => updatePoint(point.id, { label: event.target.value })} /></label>
          <label>画面描述<textarea rows={4} value={point.description} placeholder="这个时刻的主体、构图和画面状态…" onChange={(event) => updatePoint(point.id, { description: event.target.value })} /></label>
          <div className="director-inline-actions"><BookButton variant="secondary" size="sm" onClick={() => setPanel("media")}><ImagePlus size={15} />选择图片</BookButton><label className="director-upload-button"><Upload size={15} />上传<input type="file" accept="image/png,image/jpeg,image/webp" disabled={disabled || uploading} onChange={(event) => { const file = event.target.files?.[0]; if (file) void upload(file); event.target.value = ""; }} /></label></div>
          <BookButton variant="secondary" onClick={() => setGeneration({ kind: "point", id: point.id })}><Sparkles size={15} />生成分镜图片</BookButton>
          <p className="director-context-line">{point.image_id ? "已绑定分镜图片" : "尚未绑定图片"} · {point.reference_image_ids.length} 张参考图</p>
          {point.image_id && <BookButton variant="ghost" size="sm" onClick={() => updatePoint(point.id, { image_id: null })}>解除图片绑定</BookButton>}
          <div className="director-inline-actions"><BookButton variant="ghost" size="sm" onClick={openSplit}>从此处分镜</BookButton><BookIconButton label="删除图片点" disabled={disabled || point.id === plan.points[0].id || point.id === plan.points.at(-1)?.id} onClick={() => void edit({ kind: "remove_point", point_id: point.id })}><Trash2 size={15} /></BookIconButton></div>
        </fieldset>}
        {segment && <fieldset disabled={disabled} className="director-fields">
          {narrow && <p className="director-object-time">{selectionSummary}</p>}
          <label>镜头名称<input value={segment.title} onChange={(event) => updateSegment(segment.id, { title: event.target.value })} /></label>
          <label>变化描述<textarea rows={4} value={segment.prompt} placeholder="从起点画面开始，动作、镜头语言、风格和剧情如何变化…" onChange={(event) => updateSegment(segment.id, { prompt: event.target.value })} /></label>
          <BookButton variant="secondary" onClick={() => setGeneration({ kind: "segment", id: segment.id })}><Sparkles size={15} />生成描述候选</BookButton>
          <SceneBindingEditor binding={segment.scene_binding} scenes={scenes} onChange={(binding) => updateSegment(segment.id, { scene_binding: binding })} />
          <label className="director-checkbox" title="默认只使用起点和段内关键帧，结束点可以是下一镜头。"><input type="checkbox" checked={segment.use_end_point_image} onChange={(event) => updateSegment(segment.id, { use_end_point_image: event.target.checked })} />使用结束点图片作为目标尾帧</label>
          <div className="director-inline-actions"><BookButton variant="ghost" size="sm" onClick={openSplit}>拆分镜头</BookButton><BookButton variant="ghost" size="sm" onClick={openMerge} disabled={disabled || segment.id === plan.segments.at(-1)?.id}>与后一段合并</BookButton></div>
        </fieldset>}
  </>;

  return <section ref={workspaceRef} className="director-workspace" aria-label="导演编排" style={{ "--director-ratio": plan.frame.width / plan.frame.height } as CSSProperties}>
    <header className="director-header"><div><p className="director-eyebrow">导演编排 · 第 {chapter.chapter_number} 章</p><h2 title={chapter.title || "分镜时间线"}>{chapter.title || "分镜时间线"}</h2></div>
      <div className="director-inline-actions"><span className="director-save-status" role="status">{model.saving ? "保存中…" : model.dirty ? "未保存" : model.record?.record.status === "confirmed" ? "方案已确认" : model.record ? "草稿已保存" : "新编排"}</span>
        <BookIconButton label="撤销" onClick={model.undo} disabled={!model.canUndo || model.busy}><Undo2 size={17} /></BookIconButton>
        <BookIconButton label="重做" onClick={model.redo} disabled={!model.canRedo || model.busy}><Redo2 size={17} /></BookIconButton>
        {model.editable ? <><BookIconButton label="保存草稿" onClick={() => void model.save().catch(model.report)} disabled={model.busy || model.saving}><Save size={17} /></BookIconButton><BookButton size="sm" onClick={() => void model.confirm()} disabled={model.busy || model.saving}><Check size={15} />确认方案</BookButton></>
          : canEdit && <BookButton size="sm" onClick={() => void model.startEditing()} disabled={model.busy}>编辑新版本</BookButton>}
      </div>
    </header>
    <div className="director-alerts">
    {model.error && <div className="director-error" role="alert">{model.error}<BookButton variant="ghost" size="sm" onClick={() => void model.save().catch(model.report)} disabled={!model.editable || model.busy}>重试保存</BookButton></div>}
    {assetError && <div className="director-error" role="alert">{assetError}<BookButton variant="ghost" size="sm" onClick={() => void loadAssets()}>重试加载素材</BookButton></div>}
    {model.recovery && <div className="director-notice">此页面有未同步的本地编辑。<BookButton variant="secondary" size="sm" onClick={() => { void confirmDialog({ title: "恢复未同步编辑", message: "将用本地保留的内容更新当前草稿，已确认版本不受影响。", confirmText: "恢复草稿" }).then((accepted) => { if (accepted) void model.restore(); }); }}>恢复草稿</BookButton></div>}
    {model.issues.length > 0 && <div className="director-error" role="alert">{model.issues.map((issue) => <button key={issue.path} type="button" onClick={() => {
      const match = /^(points|segments)\[(\d+)\]/.exec(issue.path);
      if (match) { const index = Number(match[2]); const item = match[1] === "points" ? plan.points[index] : plan.segments[index]; if (item) select({ kind: match[1] === "points" ? "point" : "segment", id: item.id }, match[1] === "points" ? plan.points[index].at_ms : segmentTimes(plan, plan.segments[index])[0]); }
    }}>{issue.message}</button>)}</div>}
    </div>

    <nav className="director-tool-rail" aria-label="导演工具">
        <Tool icon={Settings2} label="属性" active={panel === "properties"} onClick={() => setPanel(panel === "properties" ? null : "properties")} />
        <Tool icon={Images} label="素材库" active={panel === "media"} onClick={() => setPanel("media")} />
        <Tool icon={Mountain} label="场景库" active={panel === "scenes"} onClick={() => setPanel("scenes")} />
        <Tool icon={BookOpen} label="原文" active={panel === "source"} onClick={() => setPanel("source")} />
        <Tool icon={Film} label="画幅" active={panel === "settings"} onClick={() => setPanel("settings")} />
        <Tool icon={Sparkles} label="导演 Agent" active={panel === "agent"} onClick={() => setPanel("agent")} />
    </nav>
    <div className={`director-stage ${panel === "properties" && !narrow ? "has-inspector" : ""}`}>
      <div className="director-editing-column">
        <div className="director-player-column">
          <div className="director-preview-surface">
            <div className="director-viewer" ref={frameRef} style={{ aspectRatio: `${plan.frame.width} / ${plan.frame.height}` }}>
              <DirectorFrame plan={plan} clock={clock} imageById={imageById} />
              <button className="director-fullscreen" type="button" aria-label="全屏查看分镜" onClick={() => void (document.fullscreenElement ? document.exitFullscreen() : frameRef.current?.requestFullscreen())?.catch(model.report)}><Maximize size={17} /></button>
            </div>
          </div>
          <Transport clock={clock} plan={plan} />
        </div>
        <DirectorTimeline plan={plan} selection={selection} clock={clock} editable={!disabled} error={model.error} sceneNames={names} onSelect={select} onEdit={edit} onSplit={openSplit} />
      </div>
      {panel === "properties" && !narrow && <aside className="director-inspector" aria-label="所选对象属性">{inspector}</aside>}
    </div>

    <Modal isOpen={panel === "properties" && narrow} title={point ? "图片点属性" : "镜头段属性"} onClose={() => setPanel(null)} className="director-mobile-inspector">{inspector}</Modal>
    <Modal isOpen={panel === "source"} title="章节原文" onClose={closePanel} maxWidthClassName="max-w-3xl"><div className="director-source">{model.set?.source.content}</div></Modal>
    <Modal isOpen={panel === "scenes"} title="场景库" onClose={() => { closePanel(); void loadAssets(); }} maxWidthClassName="max-w-6xl"><Suspense fallback={<LoadingSpinner text="加载场景库…" />}><SceneLibrary projectId={chapter.project_id} canEditProfiles={canEdit} /></Suspense></Modal>
    <Modal isOpen={panel === "agent"} title="导演 Agent" onClose={closePanel} maxWidthClassName="max-w-4xl" keepMounted>
      <DomainAgentConsole capability="director" projectId={projectId} chapterId={chapter.id} resourceLabel={chapter.title} onResourceChanged={() => model.reload()} getEditorSource={getEditorSource} onEditorRequest={editorRequest} />
    </Modal>
    <Modal isOpen={panel === "settings"} title="画幅" onClose={closePanel}>
      <div className="director-inline-actions">{[[1920, 1080, "16:9 横屏"], [1080, 1920, "9:16 竖屏"], [1080, 1080, "1:1 方形"], [1440, 1080, "4:3"]].map(([width, height, label]) => <BookButton key={label} variant="secondary" disabled={disabled} onClick={() => { void edit({ kind: "set_frame", frame: { width: Number(width), height: Number(height) } }); closePanel(); }}>{label}</BookButton>)}</div>
      <p className="director-context-line">图片完整显示在画框中；总时长在时间条上等比调整。</p>
    </Modal>
    <Modal isOpen={panel === "media"} title={point ? `${point.label || "图片点"} · 选择分镜与参考图` : "图片素材库"} onClose={closePanel} maxWidthClassName="max-w-5xl">
      {!point && <p className="director-context-line">在时间线上选中图片点后，可以绑定分镜与参考图。</p>}
      <div className="director-media-grid">{images.map((image) => <article key={image.id}><img src={toMediaSrc(image.file_path)} alt={image.prompt || "图片素材"} loading="lazy" /><div className="director-inline-actions">
        <BookButton variant="secondary" size="sm" disabled={!point || disabled} onClick={() => { if (point) void adoptImage(point.id, image).then(() => setPanel("properties")).catch(model.report); }}>{point?.image_id === image.id ? "已绑定" : "选作分镜"}</BookButton>
        <BookButton variant="ghost" size="sm" disabled={!point || disabled} onClick={() => { if (point) updatePoint(point.id, { reference_image_ids: point.reference_image_ids.includes(image.id) ? point.reference_image_ids.filter((id) => id !== image.id) : [...point.reference_image_ids, image.id] }); }}>{point?.reference_image_ids.includes(image.id) ? "取消参考" : "选作参考"}</BookButton>
      </div></article>)}</div>{images.length === 0 && <p className="director-empty">暂无图片。可以在图片点属性中上传或生成分镜。</p>}
    </Modal>
    {split && <Modal isOpen title="拆分镜头" onClose={() => setSplit(null)} footer={<BookButton disabled={model.busy} onClick={() => { void model.mutate({ kind: "split_segment", segment_id: split.segmentId, point_id: split.pointId, right_segment_id: crypto.randomUUID(), left_prompt: split.leftPrompt, right_prompt: split.rightPrompt, left_scene_binding: split.leftScene, right_scene_binding: split.rightScene }).then(() => setSplit(null)).catch(model.report); }}>拆分镜头</BookButton>}>
      <div className="director-fields"><label>分镜位置<select value={split.pointId} onChange={(event) => setSplit({ ...split, pointId: event.target.value })}>{plan.points.filter((point) => { const target = plan.segments.find((segment) => segment.id === split.segmentId)!; const [start, end] = segmentTimes(plan, target); return point.at_ms > start && point.at_ms < end; }).map((point) => <option key={point.id} value={point.id}>{formatTime(point.at_ms)} · {point.label}</option>)}</select></label>
        <label>前段描述<textarea rows={4} value={split.leftPrompt} onChange={(event) => setSplit({ ...split, leftPrompt: event.target.value })} /></label><SceneBindingEditor binding={split.leftScene} scenes={scenes} onChange={(leftScene) => setSplit({ ...split, leftScene })} />
        <label>后段描述<textarea rows={4} value={split.rightPrompt} onChange={(event) => setSplit({ ...split, rightPrompt: event.target.value })} placeholder="为新镜头填写描述，也可以拆分后生成。" /></label><SceneBindingEditor binding={split.rightScene} scenes={scenes} onChange={(rightScene) => setSplit({ ...split, rightScene })} /></div>
    </Modal>}
    {merge && <Modal isOpen title="合并镜头" onClose={() => setMerge(null)} footer={<BookButton disabled={model.busy} onClick={() => { void model.mutate({ kind: "merge_segments", left_segment_id: merge.leftId, right_segment_id: merge.rightId, prompt: merge.prompt, scene_binding: merge.scene }).then(() => setMerge(null)).catch(model.report); }}>{merge.prompt.trim() ? "合并镜头" : "合并为待完善镜头"}</BookButton>}>
      <div className="director-fields"><details open><summary>原镜头描述</summary>{[merge.leftId, merge.rightId].map((id) => <p key={id}>{plan.segments.find((segment) => segment.id === id)?.prompt || "尚未填写"}</p>)}</details><label>合并后的变化描述<textarea rows={6} value={merge.prompt} onChange={(event) => setMerge({ ...merge, prompt: event.target.value })} /></label><SceneBindingEditor binding={merge.scene} scenes={scenes} onChange={(scene) => setMerge({ ...merge, scene })} /><p className="director-context-line">原有图片点都会保留。可在合并后继续填写或生成描述。</p></div>
    </Modal>}
    {generation && <DirectorGeneration key={`${generation.kind}:${generation.id}`} projectId={projectId} chapterId={chapter.id} plan={plan} target={generation} save={model.save} onAdoptImage={adoptImage} onAdoptPrompt={adoptPrompt} onClose={() => setGeneration(null)} />}
  </section>;
}

function Tool({ icon: Icon, label, active = false, onClick }: { icon: typeof Film; label: string; active?: boolean; onClick: () => void }) {
  return <button type="button" title={label} aria-label={label} aria-pressed={active} className={active ? "is-active" : ""} onClick={onClick}><Icon size={19} /><span>{label}</span></button>;
}

function SceneBindingEditor({ binding, scenes, onChange }: { binding: SceneBindingV3 | null; scenes: SceneProfile[]; onChange: (binding: SceneBindingV3 | null) => void }) {
  const entry = binding?.kind === "scene" ? binding.scene_profile_id : binding?.from_scene_profile_id;
  const exit = binding?.kind === "scene" ? binding.scene_profile_id : binding?.to_scene_profile_id;
  const options = <><option value="">选择场景</option>{scenes.map((scene) => <option key={scene.id} value={scene.id}>{scene.name}</option>)}</>;
  return <div className="director-scene-binding"><label>场景方式<select value={binding?.kind ?? "scene"} disabled={scenes.length === 0} onChange={(event) => {
    const first = entry ?? scenes[0]?.id;
    if (!first) return;
    onChange(event.target.value === "scene" ? { kind: "scene", scene_profile_id: first } : { kind: "transition", from_scene_profile_id: first, to_scene_profile_id: exit ?? scenes.find((scene) => scene.id !== first)?.id ?? first });
  }}><option value="scene">单一场景</option><option value="transition">场景转场</option></select></label>
    <label>{binding?.kind === "transition" ? "起始场景" : "场景"}<select value={entry ?? ""} onChange={(event) => { const id = Number(event.target.value); onChange(!id ? null : binding?.kind === "transition" ? { ...binding, from_scene_profile_id: id } : { kind: "scene", scene_profile_id: id }); }}>{options}</select></label>
    {binding?.kind === "transition" && <label>目标场景<select value={binding.to_scene_profile_id} onChange={(event) => { const id = Number(event.target.value); onChange(id ? { ...binding, to_scene_profile_id: id } : null); }}>{options}</select></label>}
  </div>;
}

function DirectorFrame({ plan, clock, imageById }: { plan: ChapterVisualPlanV3; clock: DirectorPlayback; imageById: ReadonlyMap<number, GeneratedImage> }) {
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
  return <>{image && !failed ? <img className="director-frame-image" src={toMediaSrc(image.file_path)} alt={point.description || point.label || "当前分镜"} onError={() => setFailed(true)} />
    : <div className="director-frame-empty"><ImagePlus size={34} strokeWidth={1.2} /><strong>{point.image_id ? "分镜图片无法加载" : "此处尚未绑定图片"}</strong><span>{formatTime(point.at_ms)} · {point.label || "图片点"}</span></div>}
    <div className="director-frame-caption">{point.label || "分镜"} <span>{formatTime(point.at_ms)}</span></div></>;
}

function Transport({ clock, plan }: { clock: DirectorPlayback; plan: ChapterVisualPlanV3 }) {
  const time = useStore(clock, (state) => state.time);
  const playing = useStore(clock, (state) => state.playing);
  const previous = [...plan.points].reverse().find((point) => point.at_ms < time);
  const next = plan.points.find((point) => point.at_ms > time);
  return <div className="director-transport"><div className="director-inline-actions"><BookIconButton label="上一个图片点" disabled={!previous} onClick={() => { if (previous) clock.getState().seek(previous.at_ms); }}><SkipBack size={16} /></BookIconButton><BookIconButton label={playing ? "暂停" : "播放分镜"} onClick={() => clock.getState().toggle()}>{playing ? <Pause size={19} /> : <Play size={19} />}</BookIconButton><BookIconButton label="下一个图片点" disabled={!next} onClick={() => { if (next) clock.getState().seek(next.at_ms); }}><SkipForward size={16} /></BookIconButton></div><DirectorScrubber clock={clock} total={durationMs(plan)} /><div className="director-timecode"><span>{formatTime(time)}</span><span>/ {formatTime(durationMs(plan))}</span></div><span className="director-preview-label">分镜预演</span></div>;
}
