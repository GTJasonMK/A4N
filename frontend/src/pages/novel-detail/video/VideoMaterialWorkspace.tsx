import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { BadgeCheck,
  Box,
  Clipboard,
  MapPinned,
  RefreshCw,
  Search,
  Sparkles,
  Star,
  Upload, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  deleteImage,
  startImageGeneration,
  updateImage,
  verifyImageGenerationResult,
  type GeneratedImage,
  type UpdateImageInput,
} from "../../../api/image";
import {
  listProfileMaterials,
  setProfilePrimaryMaterial,
  uploadProfileMaterialImage,
  type ProfileMaterialGroup,
  type ProfileMaterialKind,
  type ProfileMaterialState,
} from "../../../api/profileMaterials";
import { generateObjectProfileImagePrompt } from "../../../api/objectProfiles";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { generateSceneProfileImagePrompt } from "../../../api/sceneProfiles";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import type { RunProgressTask } from "../../../hooks/useProgressTask";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import { toMediaSrc } from "../../../utils/mediaSrc";
import {
  isSupportedImageMime,
  readFileAsBase64,
  supportedImageAccept,
} from "../../../utils/imageUpload";
import { ImageActionDialog } from "../images/ImageActionDialog";
import { chapterRangeText } from "../shared";
import {
  IMAGE_STYLE_TEMPLATES,
  renderVisualStyleTemplate,
} from "../styleTemplates";

type MaterialKindFilter = "all" | ProfileMaterialKind;
type MaterialFilter = "all" | ProfileMaterialState | "favorite";
type MaterialSort = "missing_first" | "stale_first" | "newest" | "importance";
type LoadMaterialsOptions = { force?: boolean };

const CUSTOM_STYLE_VALUE = "custom";
const DEFAULT_STYLE_TEMPLATE_ID = IMAGE_STYLE_TEMPLATES[0]?.id ?? CUSTOM_STYLE_VALUE;
const STYLE_TEMPLATE_VERSION = "1";
const MAX_UPLOAD_IMAGE_BYTES = 20 * 1024 * 1024;

const KIND_OPTIONS: Array<{ value: MaterialKindFilter; label: string }> = [
  { value: "all", label: "全部" },
  { value: "scene", label: "场景" },
  { value: "object", label: "物品" },
];

const FILTER_OPTIONS: Array<{ value: MaterialFilter; label: string }> = [
  { value: "all", label: "全部状态" },
  { value: "missing", label: "缺失" },
  { value: "stale", label: "已过期" },
  { value: "unknown", label: "旧素材" },
  { value: "ready", label: "可用" },
  { value: "favorite", label: "收藏" },
];

const SORT_OPTIONS: Array<{ value: MaterialSort; label: string }> = [
  { value: "missing_first", label: "缺失优先" },
  { value: "stale_first", label: "过期优先" },
  { value: "newest", label: "最新素材" },
  { value: "importance", label: "重要性" },
];

const MATERIAL_STATE_LABELS: Record<ProfileMaterialState, string> = {
  missing: "缺失",
  ready: "可用",
  stale: "已过期",
  unknown: "旧素材",
};

const MATERIAL_KIND_LABELS: Record<ProfileMaterialKind, string> = {
  scene: "场景",
  object: "物品",
};

const IMPORTANCE_LABELS: Record<string, string> = {
  primary: "关键",
  secondary: "重要",
  minor: "次要",
  major: "主要",
  supporting: "辅助",
  background: "背景",
  high: "高",
  medium: "中",
  low: "低",
  normal: "普通",
  critical: "关键",
};

const CATEGORY_LABELS: Record<string, string> = {
  room: "房间",
  building: "建筑",
  street: "街道",
  city: "城市",
  wilderness: "荒野",
  route: "路线",
  landmark: "地标",
  location: "地点",
  interior: "室内",
  exterior: "室外",
  region: "区域",
  world: "世界",
  organization: "组织空间",
  weapon: "武器",
  prop: "道具",
  tool: "工具",
  token: "信物",
  document: "文书",
  vehicle: "载具",
  clothing: "服装",
  location_object: "设施",
  environment_object: "环境物件",
  artifact: "器物",
  consumable: "消耗品",
  other: "其他",
};

const categoryLabel = (value: string): string => CATEGORY_LABELS[value] ?? "未分类";
const importanceLabel = (value: string): string => IMPORTANCE_LABELS[value] ?? "未标注";

const groupKey = (group: ProfileMaterialGroup): string => `${group.kind}:${group.profile.id}`;

interface VideoMaterialWorkspaceProps {
  projectId: number;
  onToast: (message: string, type: "success" | "error" | "info") => void;
  runProgressTask: RunProgressTask;
  onRefreshVideoData: () => Promise<void> | void;
}

export const VideoMaterialWorkspace: React.FC<VideoMaterialWorkspaceProps> = ({
  projectId,
  onToast,
  runProgressTask,
  onRefreshVideoData,
}) => {
  const [groups, setGroups] = useState<ProfileMaterialGroup[]>([]);
  const [loading, setLoading] = useState(false);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [batchBusy, setBatchBusy] = useState(false);
  const [kindFilter, setKindFilter] = useState<MaterialKindFilter>("all");
  const [stateFilter, setStateFilter] = useState<MaterialFilter>("all");
  const [sort, setSort] = useState<MaterialSort>("missing_first");
  const [query, setQuery] = useState("");
  const [selectedStyleTemplateId, setSelectedStyleTemplateId] =
    useState(DEFAULT_STYLE_TEMPLATE_ID);
  const [customStyle, setCustomStyle] = useState("");
  const [imageDetails, setImageDetails] = useState<GeneratedImage | null>(null);
  const [imageBusyId, setImageBusyId] = useState<number | null>(null);
  const [imageScoringId, setImageScoringId] = useState<number | null>(null);
  const [imageScoreInput, setImageScoreInput] = useState("0.8");
  const [uploadTarget, setUploadTarget] = useState<ProfileMaterialGroup | null>(null);
  const [uploadSetPrimary, setUploadSetPrimary] = useState(true);
  const loadSeqRef = useRef(0);
  const loadInFlightRef = useRef<{ key: string; seq: number; promise: Promise<void> } | null>(
    null,
  );
  const actionBusyRef = useRef(false);
  const activeProjectRef = useRef(projectId);
  const loadRequestKey = `${projectId}:${kindFilter}:${query.trim()}:${stateFilter}:${sort}`;
  const activeLoadKeyRef = useRef(loadRequestKey);

  const selectedStyleTemplate = useMemo(
    () =>
      IMAGE_STYLE_TEMPLATES.find((template) => template.id === selectedStyleTemplateId) ??
      null,
    [selectedStyleTemplateId],
  );
  const materialStyle = selectedStyleTemplate
    ? renderVisualStyleTemplate(selectedStyleTemplate)
    : customStyle;

  useLayoutEffect(() => {
    activeProjectRef.current = projectId;
    setGroups([]);
    setImageDetails(null);
    setUploadTarget(null);
    setBusyKey(null);
    setBatchBusy(false);
    setLoading(false);
    loadInFlightRef.current = null;
    actionBusyRef.current = false;
  }, [projectId]);

  useLayoutEffect(() => {
    activeLoadKeyRef.current = loadRequestKey;
    loadSeqRef.current += 1;
    loadInFlightRef.current = null;
  }, [loadRequestKey]);

  useEffect(
    () => () => {
      loadInFlightRef.current = null;
      actionBusyRef.current = false;
    },
    [],
  );

  const isCurrentProject = (requestProjectId: number) =>
    activeProjectRef.current === requestProjectId;

  const beginAction = () => {
    if (actionBusyRef.current) return false;
    actionBusyRef.current = true;
    return true;
  };

  const finishAction = (requestProjectId: number) => {
    if (isCurrentProject(requestProjectId)) {
      actionBusyRef.current = false;
    }
  };

  const loadMaterials = useCallback(async (options: LoadMaterialsOptions = {}) => {
    const requestSeq = loadSeqRef.current + 1;
    const requestKey = loadRequestKey;
    const currentLoad = loadInFlightRef.current;
    if (!options.force && currentLoad?.key === requestKey) {
      return currentLoad.promise;
    }
    loadSeqRef.current = requestSeq;
    setLoading(true);
    const promise = (async () => {
      try {
        const result = await listProfileMaterials({
          project_id: projectId,
          kind: kindFilter,
          query: query.trim() || null,
          filter: stateFilter,
          sort,
          image_limit_per_profile: 12,
        });
        if (loadSeqRef.current !== requestSeq || activeLoadKeyRef.current !== requestKey) return;
        setGroups(result);
      } catch (err) {
        if (loadSeqRef.current !== requestSeq || activeLoadKeyRef.current !== requestKey) return;
        onToast(extractErrorMessage(err, "加载素材管理数据失败"), "error");
      } finally {
        if (loadSeqRef.current === requestSeq && activeLoadKeyRef.current === requestKey) {
          setLoading(false);
        }
        if (
          loadInFlightRef.current?.key === requestKey &&
          loadInFlightRef.current.seq === requestSeq
        ) {
          loadInFlightRef.current = null;
        }
      }
    })();
    loadInFlightRef.current = { key: requestKey, seq: requestSeq, promise };
    return promise;
  }, [kindFilter, loadRequestKey, onToast, projectId, query, sort, stateFilter]);

  useEffect(() => {
    void loadMaterials();
  }, [loadMaterials]);

  const currentImageById = useMemo(() => {
    const map = new Map<number, GeneratedImage>();
    for (const group of groups) {
      if (group.primary_image) {
        map.set(group.primary_image.image.id, group.primary_image.image);
      }
      for (const item of group.images) map.set(item.image.id, item.image);
    }
    return map;
  }, [groups]);

  useEffect(() => {
    setImageDetails((current) =>
      current ? currentImageById.get(current.id) ?? null : null,
    );
    setUploadTarget((current) => {
      if (!current) return null;
      const currentKey = groupKey(current);
      return groups.find((group) => groupKey(group) === currentKey) ?? null;
    });
  }, [currentImageById, groups]);

  const stats = useMemo(() => {
    let imageCount = 0;
    let missingCount = 0;
    let staleCount = 0;
    for (const group of groups) {
      imageCount += group.image_count;
      if (group.material_state === "missing") missingCount += 1;
      if (group.material_state === "stale") staleCount += 1;
    }
    return { imageCount, missingCount, staleCount };
  }, [groups]);

  const applyStyleTemplate = (templateId: string) => {
    setSelectedStyleTemplateId(
      templateId === CUSTOM_STYLE_VALUE ||
        IMAGE_STYLE_TEMPLATES.some((item) => item.id === templateId)
        ? templateId
        : CUSTOM_STYLE_VALUE,
    );
  };

  const generationStylePayload = () => ({
    style: materialStyle.trim() || null,
    style_template_id: selectedStyleTemplate?.id ?? null,
    style_template_name: selectedStyleTemplate?.name ?? null,
    style_template_version: selectedStyleTemplate ? STYLE_TEMPLATE_VERSION : null,
    custom_style: selectedStyleTemplate ? null : customStyle.trim() || null,
  });

  const refreshAfterMaterialChange = async () => {
    await Promise.all([loadMaterials({ force: true }), Promise.resolve(onRefreshVideoData())]);
  };

  const generateMaterialCore = async (
    group: ProfileMaterialGroup,
    requestProjectId: number,
    options: { refresh?: boolean } = {},
  ) => {
    const key = groupKey(group);
    setBusyKey(key);
    try {
      const stylePayload = generationStylePayload();
      const handle = await startImageGeneration({
        project_id: requestProjectId,
        operation: group.kind === "scene"
          ? {
              kind: "scene_profile",
              scene_profile_id: group.profile.id,
              chapter_id: null,
              config_id: null,
              ...stylePayload,
            }
          : {
              kind: "object_profile",
              object_profile_id: group.profile.id,
              chapter_id: null,
              config_id: null,
              ...stylePayload,
            },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: `${group.profile.name} 素材图生成`,
          startMessage: "正在准备素材图生成",
          successMessage: "素材图生成完成",
          errorMessage: "素材图生成失败",
          total: 3,
          cancelLabel: "停止生成",
        },
        async (task) => {
          if (!isCurrentProject(requestProjectId)) throw new Error("任务已停止");
          await verifyImageGenerationResult(task, requestProjectId);
        },
      );
      if (!isCurrentProject(requestProjectId)) return;
      onToast(`已生成 ${group.profile.name} 的项目级素材图`, "success");
      if (options.refresh !== false) {
        await refreshAfterMaterialChange();
      }
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, "素材图生成失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setBusyKey(null);
    }
  };

  const generateMaterial = async (
    group: ProfileMaterialGroup,
    options: { refresh?: boolean } = {},
  ) => {
    const requestProjectId = projectId;
    if (!beginAction()) return;
    try {
      await generateMaterialCore(group, requestProjectId, options);
    } finally {
      finishAction(requestProjectId);
    }
  };

  const buildMaterialPrompt = async (
    group: ProfileMaterialGroup,
    stylePayload: ReturnType<typeof generationStylePayload> = generationStylePayload(),
  ): Promise<string> => {
    if (group.kind === "scene") {
      const output = await generateSceneProfileImagePrompt({
        scene_profile_id: group.profile.id,
        chapter_id: null,
        config_id: null,
        ...stylePayload,
      });
      return output.prompt;
    }
    const output = await generateObjectProfileImagePrompt({
      object_profile_id: group.profile.id,
      chapter_id: null,
      config_id: null,
      ...stylePayload,
    });
    return output.prompt;
  };

  const copyMaterialPrompt = async (group: ProfileMaterialGroup) => {
    const requestProjectId = projectId;
    if (!beginAction()) return;
    const key = groupKey(group);
    setBusyKey(key);
    try {
      const prompt = await buildMaterialPrompt(group);
      if (!isCurrentProject(requestProjectId)) return;
      await copyTextToClipboard(prompt);
      if (!isCurrentProject(requestProjectId)) return;
      onToast(`已复制 ${group.profile.name} 的图片提示词`, "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, "复制提示词失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setBusyKey(null);
      finishAction(requestProjectId);
    }
  };

  const openUploadDialog = (group: ProfileMaterialGroup) => {
    setUploadTarget(group);
    setUploadSetPrimary(true);
  };

  const uploadMaterialFile = async (file: File) => {
    if (!uploadTarget) return;
    const requestProjectId = projectId;
    if (!beginAction()) return;
    const group = uploadTarget;
    const key = groupKey(group);
    const stylePayload = generationStylePayload();
    try {
      if (!isSupportedImageMime(file.type)) {
        onToast("只支持 PNG/JPEG/WebP 图片", "error");
        return;
      }
      if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
        onToast(`上传图片不能超过 ${MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024}MB`, "error");
        return;
      }
      setBusyKey(key);
      const [prompt, dataBase64] = await Promise.all([
        buildMaterialPrompt(group, stylePayload),
        readFileAsBase64(file),
      ]);
      if (!isCurrentProject(requestProjectId)) return;
      await uploadProfileMaterialImage({
        project_id: projectId,
        profile_kind: group.kind,
        profile_id: group.profile.id,
        file_name: file.name,
        mime_type: file.type,
        data_base64: dataBase64,
        set_primary: uploadSetPrimary,
        prompt_snapshot: prompt,
        style_template_id: stylePayload.style_template_id,
        style_template_name: stylePayload.style_template_name,
        style_template_version: stylePayload.style_template_version,
        custom_style: stylePayload.custom_style,
      });
      if (!isCurrentProject(requestProjectId)) return;
      setUploadTarget(null);
      await refreshAfterMaterialChange();
      if (!isCurrentProject(requestProjectId)) return;
      onToast(uploadSetPrimary ? "已上传并设为主素材" : "已上传素材图", "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, "上传素材图失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setBusyKey(null);
      finishAction(requestProjectId);
    }
  };

  const batchGenerate = async (mode: "missing" | "stale") => {
    const requestProjectId = projectId;
    const targets = groups.filter((group) => group.material_state === mode);
    if (targets.length === 0) {
      onToast(mode === "missing" ? "当前没有缺失素材" : "当前没有过期素材", "info");
      return;
    }
    if (!beginAction()) return;
    setBatchBusy(true);
    try {
      for (const group of targets) {
        if (!isCurrentProject(requestProjectId)) return;
        await generateMaterialCore(group, requestProjectId, { refresh: false });
      }
      if (!isCurrentProject(requestProjectId)) return;
      await refreshAfterMaterialChange();
      if (!isCurrentProject(requestProjectId)) return;
      onToast(`批量生成完成：${targets.length} 个档案`, "success");
    } finally {
      if (isCurrentProject(requestProjectId)) setBatchBusy(false);
      finishAction(requestProjectId);
    }
  };

  const openImageDetails = (image: GeneratedImage) => {
    setImageScoreInput(image.ai_score == null ? "0.8" : String(image.ai_score));
    setImageDetails(image);
  };

  const closeImageDetails = () => {
    if (imageBusyId == null && imageScoringId == null) setImageDetails(null);
  };

  const toggleFavorite = async (image: GeneratedImage) => {
    const requestProjectId = projectId;
    if (!beginAction()) return;
    setImageBusyId(image.id);
    try {
      const updated = await updateImage(image.id, { is_favorite: !image.is_favorite });
      if (!isCurrentProject(requestProjectId)) return;
      setImageDetails((current) => (current?.id === updated.id ? updated : current));
      await loadMaterials({ force: true });
      if (!isCurrentProject(requestProjectId)) return;
      onToast(updated.is_favorite ? "已收藏" : "已取消收藏", "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, "切换素材收藏失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setImageBusyId(null);
      finishAction(requestProjectId);
    }
  };

  const saveImageScore = async (image: GeneratedImage) => {
    const requestProjectId = projectId;
    const score = Number(imageScoreInput);
    if (!Number.isFinite(score)) {
      onToast("评分必须是数字", "error");
      return;
    }
    if (!beginAction()) return;
    setImageScoringId(image.id);
    try {
      const updated = await updateImage(image.id, { ai_score: score });
      if (!isCurrentProject(requestProjectId)) return;
      setImageDetails((current) => (current?.id === updated.id ? updated : current));
      await loadMaterials({ force: true });
      if (!isCurrentProject(requestProjectId)) return;
      onToast(`已打分 ${score}`, "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, "打分失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setImageScoringId(null);
      finishAction(requestProjectId);
    }
  };

  const updateImageRecord = async (image: GeneratedImage, input: UpdateImageInput) => {
    const requestProjectId = projectId;
    if (!beginAction()) return;
    setImageBusyId(image.id);
    try {
      const updated = await updateImage(image.id, input);
      if (!isCurrentProject(requestProjectId)) return;
      setImageDetails((current) => (current?.id === updated.id ? updated : current));
      await loadMaterials({ force: true });
      if (!isCurrentProject(requestProjectId)) return;
      onToast("图片记录已更新", "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, "更新图片记录失败"), "error");
      throw err;
    } finally {
      if (isCurrentProject(requestProjectId)) setImageBusyId(null);
      finishAction(requestProjectId);
    }
  };

  const removeImage = async (image: GeneratedImage) => {
    const requestProjectId = projectId;
    if (!beginAction()) return;
    try {
      const ok = await confirmDialog({
        title: "删除素材图",
        message: "删除这张素材图？相关视频参考图会同步解除关联。",
        confirmText: "删除",
        dialogType: "danger",
      });
      if (!ok) return;
      if (!isCurrentProject(requestProjectId)) return;
      setImageBusyId(image.id);
      await deleteImage(image.id);
      if (!isCurrentProject(requestProjectId)) return;
      setImageDetails(null);
      await refreshAfterMaterialChange();
      if (!isCurrentProject(requestProjectId)) return;
      onToast("已删除素材图", "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, "删除素材图失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setImageBusyId(null);
      finishAction(requestProjectId);
    }
  };

  const setPrimary = async (group: ProfileMaterialGroup, imageId: number | null) => {
    const requestProjectId = projectId;
    if (!beginAction()) return;
    setBusyKey(groupKey(group));
    try {
      await setProfilePrimaryMaterial({
        project_id: projectId,
        profile_kind: group.kind,
        profile_id: group.profile.id,
        image_id: imageId,
      });
      if (!isCurrentProject(requestProjectId)) return;
      await refreshAfterMaterialChange();
      if (!isCurrentProject(requestProjectId)) return;
      onToast(imageId == null ? "已取消主素材" : "已设为主素材", "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, "设置主素材失败"), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setBusyKey(null);
      finishAction(requestProjectId);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <section className="a4n-flat-surface shrink-0 overflow-hidden">
        <div className="flex min-w-0 flex-wrap items-center gap-2 p-3">
          <label className="flex min-h-9 min-w-[180px] flex-1 items-center gap-2 rounded-xl border border-book-border/60 bg-book-bg-paper/60 px-3 text-xs transition-colors focus-within:border-book-primary/70 focus-within:bg-book-bg-paper sm:max-w-sm">
            <Search size={13} className="shrink-0 text-book-text-muted" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              disabled={loading || busyKey != null || batchBusy}
              className="min-w-0 flex-1 bg-transparent text-book-text-main outline-none placeholder:text-book-text-muted"
              placeholder="搜索场景或物品"
            />
          </label>
          <ToolbarSelect
            label="类型"
            value={kindFilter}
            options={KIND_OPTIONS}
            disabled={loading || busyKey != null || batchBusy}
            onChange={(value) => setKindFilter(value as MaterialKindFilter)}
          />
          <ToolbarSelect
            label="状态"
            value={stateFilter}
            options={FILTER_OPTIONS}
            disabled={loading || busyKey != null || batchBusy}
            onChange={(value) => setStateFilter(value as MaterialFilter)}
          />
          <ToolbarSelect
            label="排序"
            value={sort}
            options={SORT_OPTIONS}
            disabled={loading || busyKey != null || batchBusy}
            onChange={(value) => setSort(value as MaterialSort)}
          />
          <button
            type="button"
            onClick={() => void loadMaterials()}
            disabled={loading || busyKey != null || batchBusy}
            className="pyqt-btn-icon !h-9 !w-9"
            title="刷新素材管理数据"
            aria-label="刷新素材管理数据"
          >
            {loading ? <LoadingSpinner size="sm" /> : <RefreshCw size={13} />}
          </button>
        </div>
        <div className="border-t border-book-border/35 px-3 py-2 text-small leading-relaxed text-book-text-muted">
          当前 {groups.length} 个档案 · {stats.imageCount} 张素材图
          {stats.missingCount > 0 ? ` · ${stats.missingCount} 项待补齐` : ""}
          {stats.staleCount > 0 ? ` · ${stats.staleCount} 项需更新` : ""}
        </div>
        <details className="group border-t border-book-border/35">
          <summary className="flex min-h-10 cursor-pointer list-none items-center gap-3 px-3 py-2 text-xs [&::-webkit-details-marker]:hidden">
            <span className="font-medium text-book-text-main">生成设置</span>
            <span className="min-w-0 flex-1 truncate text-right text-caption text-book-text-muted">
              {selectedStyleTemplate?.name ?? "自定义风格"}
            </span>
          </summary>
          <div className="grid min-w-0 grid-cols-1 gap-2 border-t border-book-border/25 px-3 pb-3 pt-2 sm:grid-cols-2">
            <ToolbarSelect
              label="风格模板"
              value={selectedStyleTemplate?.id ?? CUSTOM_STYLE_VALUE}
              options={[
                ...IMAGE_STYLE_TEMPLATES.map((template) => ({
                  value: template.id,
                  label: template.name,
                })),
                { value: CUSTOM_STYLE_VALUE, label: "自定义/后端默认" },
              ]}
              disabled={busyKey != null || batchBusy}
              onChange={applyStyleTemplate}
              wide
            />
            <label className="flex min-h-9 min-w-0 items-center gap-2 rounded-xl border border-book-border/60 bg-book-bg-paper/60 px-3 text-xs transition-colors focus-within:border-book-primary/70 focus-within:bg-book-bg-paper">
              <span className="shrink-0 text-book-text-muted">补充</span>
              <input
                value={selectedStyleTemplate ? "" : customStyle}
                onChange={(event) => setCustomStyle(event.target.value)}
                disabled={busyKey != null || batchBusy || selectedStyleTemplate != null}
                className="min-w-0 flex-1 bg-transparent text-book-text-main outline-none placeholder:text-book-text-muted"
                placeholder={
                  selectedStyleTemplate
                    ? selectedStyleTemplate.summary
                    : "输入自定义风格；清空则使用后端默认"
                }
              />
            </label>
            {(stats.missingCount > 0 || stats.staleCount > 0) && (
              <div className="flex min-w-0 flex-wrap items-center justify-end gap-2 sm:col-span-2">
                {stats.missingCount > 0 && (
                  <button
                    type="button"
                    onClick={() => void batchGenerate("missing")}
                    disabled={loading || busyKey != null || batchBusy}
                    className="pyqt-btn-primary inline-flex min-h-9 flex-1 items-center justify-center gap-1.5 px-3 text-xs sm:flex-none"
                    title="为当前筛选结果中的缺失档案生成素材图"
                  >
                    {batchBusy ? <LoadingSpinner size="sm" /> : <Sparkles size={13} />}
                    补齐 {stats.missingCount} 项缺失
                  </button>
                )}
                {stats.staleCount > 0 && (
                  <button
                    type="button"
                    onClick={() => void batchGenerate("stale")}
                    disabled={loading || busyKey != null || batchBusy}
                    className="pyqt-btn-secondary inline-flex min-h-9 flex-1 items-center justify-center gap-1.5 px-3 text-xs sm:flex-none"
                    title="为当前筛选结果中的过期档案重新生成素材图"
                  >
                    <RefreshCw size={13} />
                    更新 {stats.staleCount} 项过期素材
                  </button>
                )}
              </div>
            )}
          </div>
        </details>
      </section>

      <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
        {groups.length === 0 ? (
          <div className="flex h-full min-h-[180px] items-center justify-center rounded-2xl border border-dashed border-book-border/50 bg-book-bg-paper/30 text-xs text-book-text-muted">
            {loading ? "正在加载素材..." : "当前筛选条件下没有素材档案。"}
          </div>
        ) : (
          <div className="grid grid-cols-1 items-start gap-2 lg:grid-cols-2 xl:grid-cols-3">
            {groups.map((group) => (
              <MaterialCard
                key={groupKey(group)}
                group={group}
                busy={busyKey === groupKey(group)}
                disabled={busyKey != null || batchBusy}
                onCopyPrompt={() => void copyMaterialPrompt(group)}
                onUpload={() => openUploadDialog(group)}
                onGenerate={() => void generateMaterial(group)}
                onOpenImage={openImageDetails}
                onSetPrimary={(imageId) => void setPrimary(group, imageId)}
              />
            ))}
          </div>
        )}
      </div>

      <ImageActionDialog
        image={imageDetails}
        busyId={imageBusyId}
        scoringId={imageScoringId}
        scoreInput={imageScoreInput}
        onClose={closeImageDetails}
        onToggleFavorite={(image) => void toggleFavorite(image)}
        onUpdate={(image, input) => updateImageRecord(image, input)}
        onScoreInputChange={setImageScoreInput}
        onScore={(image) => void saveImageScore(image)}
        onDelete={(image) => void removeImage(image)}
      />
      <ProfileMaterialUploadDialog
        group={uploadTarget}
        busy={uploadTarget != null && busyKey === groupKey(uploadTarget)}
        setPrimary={uploadSetPrimary}
        onSetPrimaryChange={setUploadSetPrimary}
        onClose={() => {
          if (busyKey == null) setUploadTarget(null);
        }}
        onCopyPrompt={() => {
          if (uploadTarget) void copyMaterialPrompt(uploadTarget);
        }}
        onUpload={(file) => void uploadMaterialFile(file)}
      />
    </div>
  );
};

function MaterialCard({
  group,
  busy,
  disabled,
  onCopyPrompt,
  onUpload,
  onGenerate,
  onOpenImage,
  onSetPrimary,
}: {
  group: ProfileMaterialGroup;
  busy: boolean;
  disabled: boolean;
  onCopyPrompt: () => void;
  onUpload: () => void;
  onGenerate: () => void;
  onOpenImage: (image: GeneratedImage) => void;
  onSetPrimary: (imageId: number | null) => void;
}) {
  const icon = group.kind === "scene" ? <MapPinned size={14} /> : <Box size={14} />;
  const hero = group.primary_image ?? group.images[0] ?? null;
  const heroSrc = hero ? toMediaSrc(hero.image.file_path) : null;
  const candidateImages = hero
    ? group.images.filter((item) => item.image.id !== hero.image.id)
    : group.images;
  const stateTone =
    group.material_state === "ready"
      ? "text-success"
      : group.material_state === "missing" || group.material_state === "stale"
        ? "text-warning"
        : "text-book-text-muted";
  const missingMaterial = group.material_state === "missing" || group.image_count === 0;
  const staleMaterial = group.material_state === "stale" && group.image_count > 0;
  const directGenerate = missingMaterial || staleMaterial;
  const menuItems: React.ComponentProps<typeof Dropdown>["items"] = [
    ...(!directGenerate
      ? [
          {
            label: "重新生成素材图",
            icon: <Sparkles size={13} />,
            disabled,
            onClick: onGenerate,
          },
          { type: "divider" as const },
        ]
      : []),
    {
      label: "复制素材提示词",
      icon: <Clipboard size={13} />,
      disabled,
      onClick: onCopyPrompt,
    },
    {
      label: "上传素材图",
      icon: <Upload size={13} />,
      disabled,
      onClick: onUpload,
    },
    ...(group.primary_image_id != null
      ? [
          { type: "divider" as const },
          {
            label: "取消主素材",
            disabled,
            onClick: () => onSetPrimary(null),
          },
        ]
      : []),
  ];
  return (
    <article
      className="a4n-flat-surface flex min-w-0 flex-col p-3"
      data-state={group.material_state}
    >
      <div className="mb-3 flex min-w-0 items-start gap-2.5">
        <span className="mt-0.5 shrink-0 text-book-primary">{icon}</span>
        <div className="min-w-0 flex-1">
          <h3 className="truncate text-sm font-medium text-book-text-main">
            {group.profile.name}
          </h3>
          <p className="mt-1 truncate text-caption text-book-text-muted">
            {MATERIAL_KIND_LABELS[group.kind]} · {categoryLabel(group.profile.category)} · {importanceLabel(group.profile.importance)} · {chapterRangeText(group.profile.first_seen_chapter, group.profile.last_seen_chapter)}
          </p>
        </div>
        <div className="shrink-0 text-right text-caption leading-relaxed">
          <div className={stateTone}>{MATERIAL_STATE_LABELS[group.material_state]}</div>
          <div className="text-book-text-muted">{group.image_count} 张</div>
        </div>
      </div>

      <button
        type="button"
        disabled={!hero}
        onClick={() => hero && onOpenImage(hero.image)}
        className="relative flex aspect-[16/9] w-full items-center justify-center overflow-hidden rounded-2xl border border-book-border/45 bg-book-bg-tertiary text-xs text-book-text-muted disabled:cursor-default"
        title={hero ? "打开素材图" : "暂无素材图"}
      >
        {heroSrc ? (
          <img
            src={heroSrc}
            alt={`${group.profile.name} 主素材`}
            className="h-full w-full object-cover"
          />
        ) : (
          "暂无素材图"
        )}
        {hero ? (
          <div className="absolute left-2 top-2 flex gap-1.5 text-white">
            {hero.is_primary ? (
              <span
                className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-white/30 bg-black/35 backdrop-blur-md"
                title="主素材"
                aria-label="主素材"
              >
                <BadgeCheck size={13} />
              </span>
            ) : null}
            {hero.image.is_favorite ? (
              <span
                className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-white/30 bg-black/35 backdrop-blur-md"
                title="已收藏"
                aria-label="已收藏"
              >
                <Star size={13} fill="currentColor" />
              </span>
            ) : null}
          </div>
        ) : null}
      </button>

      <p className="mt-2 line-clamp-2 text-xs leading-relaxed text-book-text-sub">
        {group.profile.primary_text || "暂无视觉描述。"}
      </p>
      <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-muted">
        {group.profile.secondary_text || "未记录补充信息。"}
      </p>

      <div className="mt-2 flex min-h-14 gap-2 overflow-x-auto pb-1 custom-scrollbar">
        {candidateImages.length > 0 ? (
          candidateImages.slice(0, 12).map((item) => {
            const src = toMediaSrc(item.image.file_path);
            return (
              <div key={item.image.id} className="group/thumb relative h-14 w-14 shrink-0">
                <button
                  type="button"
                  onClick={() => onOpenImage(item.image)}
                  className={`relative flex h-full w-full items-center justify-center overflow-hidden rounded-xl border bg-book-bg-tertiary text-caption text-book-text-muted ${
                    item.is_primary
                      ? "border-book-primary/80"
                      : "border-book-border/45"
                  }`}
                  title="素材图"
                >
                  {src ? (
                    <img
                      src={src}
                      alt={`${group.profile.name} 候选素材图`}
                      className="h-full w-full object-cover"
                    />
                  ) : (
                    "无图"
                  )}
                  {item.staleness === "stale" ? (
                    <span
                      className="absolute right-1 top-1 h-2 w-2 rounded-full bg-warning ring-2 ring-black/25"
                      title="素材已过期"
                      aria-label="素材已过期"
                    />
                  ) : null}
                  {item.is_primary ? (
                    <BadgeCheck
                      size={12}
                      className="absolute bottom-0.5 left-0.5 text-book-primary"
                      fill="currentColor"
                    />
                  ) : null}
                </button>
                {!item.is_primary ? (
                  <button
                    type="button"
                    onClick={() => onSetPrimary(item.image.id)}
                    disabled={disabled}
                    className="absolute bottom-1 right-1 inline-flex h-6 w-6 items-center justify-center rounded-full border border-white/25 bg-black/40 text-white opacity-80 backdrop-blur-sm transition-opacity sm:opacity-0 sm:group-hover/thumb:opacity-100 sm:focus-visible:opacity-100 disabled:opacity-40"
                    title="设为主素材"
                    aria-label="设为主素材"
                  >
                    <BadgeCheck size={12} />
                  </button>
                ) : null}
              </div>
            );
          })
        ) : hero ? (
          <div className="flex h-14 w-full items-center justify-center rounded-xl border border-dashed border-book-border/50 bg-book-bg-paper/30 text-small text-book-text-muted">
            暂无其他候选素材
          </div>
        ) : (
          <div className="flex h-14 w-full items-center justify-center rounded-xl border border-dashed border-book-border/50 bg-book-bg-paper/30 text-small text-book-text-muted">
            缺失素材图
          </div>
        )}
      </div>

      <div className="mt-auto flex items-center gap-2 pt-3">
        {busy ? (
          <span
            role="status"
            className="inline-flex min-h-9 min-w-0 flex-1 items-center justify-center gap-1.5 text-xs text-book-text-muted"
          >
            <LoadingSpinner size="sm" /> 正在生成
          </span>
        ) : directGenerate ? (
          <button
            type="button"
            onClick={onGenerate}
            disabled={disabled}
            className={`${missingMaterial ? "pyqt-btn-primary" : "pyqt-btn-secondary"} inline-flex min-h-9 min-w-0 flex-1 items-center justify-center gap-1.5 px-3 text-xs`}
            title={missingMaterial ? "生成项目级可复用素材图" : "更新已过期的素材图"}
          >
            {staleMaterial ? <RefreshCw size={13} /> : <Sparkles size={13} />}
            {staleMaterial ? "更新素材图" : "生成素材图"}
          </button>
        ) : (
          <span className="min-w-0 flex-1" aria-hidden="true" />
        )}
        <Dropdown items={menuItems} />
      </div>
    </article>
  );
}

function ToolbarSelect({
  label,
  value,
  options,
  disabled,
  onChange,
  wide = false,
}: {
  label: string;
  value: string;
  options: Array<{ value: string; label: string }>;
  disabled?: boolean;
  onChange: (value: string) => void;
  wide?: boolean;
}) {
  return (
    <label
      className={`flex min-h-9 min-w-0 items-center gap-2 rounded-xl border border-book-border/60 bg-book-bg-paper/60 px-3 text-xs transition-colors focus-within:border-book-primary/70 focus-within:bg-book-bg-paper ${
        wide ? "w-full sm:col-span-2 sm:flex-1 sm:min-w-[210px]" : "sm:min-w-[130px]"
      }`}
    >
      <span className="shrink-0 text-book-text-muted">{label}</span>
      <select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        disabled={disabled}
        className="pyqt-input a4n-compact-select h-7 !min-h-7 min-w-0 flex-1 !border-transparent !bg-transparent !pl-1 text-xs text-book-text-main"
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function ProfileMaterialUploadDialog({
  group,
  busy,
  setPrimary,
  onSetPrimaryChange,
  onClose,
  onCopyPrompt,
  onUpload,
}: {
  group: ProfileMaterialGroup | null;
  busy: boolean;
  setPrimary: boolean;
  onSetPrimaryChange: (value: boolean) => void;
  onClose: () => void;
  onCopyPrompt: () => void;
  onUpload: (file: File) => void;
}) {
  if (!group) return null;
  return (
    <Modal
      isOpen
      onClose={() => {
        if (!busy) onClose();
      }}
      title="上传素材图"
      subtitle={`${MATERIAL_KIND_LABELS[group.kind]} · ${group.profile.name}`}
      icon={<Upload size={15} />}
      maxWidthClassName="max-w-md"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
    >
      <div className="space-y-4">
        <button
          type="button"
          role="switch"
          aria-checked={setPrimary}
          disabled={busy}
          onClick={() => onSetPrimaryChange(!setPrimary)}
          className="flex w-full items-center justify-between gap-3 border-b border-book-border/35 pb-3 text-left disabled:cursor-not-allowed disabled:opacity-60"
        >
          <span className="min-w-0">
            <span className="block text-xs font-medium text-book-text-main">
              上传后设为主素材
            </span>
            <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
              在项目素材和后续生成中优先使用这张图片
            </span>
          </span>
          <span
            className={`relative inline-flex h-6 w-11 shrink-0 rounded-full transition-colors ${
              setPrimary ? "bg-book-primary" : "bg-book-text-muted/25"
            }`}
            aria-hidden="true"
          >
            <span
              className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-transform ${
                setPrimary ? "translate-x-[22px]" : "translate-x-0.5"
              }`}
            />
          </span>
        </button>

        <div className="rounded-2xl border border-dashed border-book-border/70 bg-book-bg-tertiary/35 p-4">
          <label
            className={`inline-flex min-h-10 items-center gap-2 rounded-xl bg-book-primary px-3.5 text-xs font-medium text-white transition-colors hover:bg-book-primary/90 ${
              busy ? "pointer-events-none opacity-60" : "cursor-pointer"
            }`}
          >
            <Upload size={14} />
            选择图片
            <input
              type="file"
              accept={supportedImageAccept()}
              disabled={busy}
              className="sr-only"
              onChange={(event) => {
                const file = event.target.files?.[0] ?? null;
                event.currentTarget.value = "";
                if (file) onUpload(file);
              }}
            />
          </label>
          <div className="mt-2 text-small text-book-text-muted">
            支持 PNG、JPEG、WebP，文件不超过 20MB。
          </div>
        </div>

        <div className="flex min-w-0 items-center justify-between gap-3">
          <button
            type="button"
            onClick={onCopyPrompt}
            disabled={busy}
            className="inline-flex min-h-9 items-center gap-1.5 text-xs text-book-text-sub transition-colors hover:text-book-text-main disabled:opacity-50"
          >
            <Clipboard size={13} />
            复制当前提示词
          </button>
          {busy ? (
            <span role="status" className="inline-flex items-center gap-1.5 text-xs text-book-text-muted">
              <LoadingSpinner size="sm" />
              正在上传
            </span>
          ) : null}
        </div>
      </div>
    </Modal>
  );
}
