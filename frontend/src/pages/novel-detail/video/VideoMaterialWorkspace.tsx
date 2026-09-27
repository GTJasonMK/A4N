import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ChevronRight, RefreshCw, Search, Sparkles } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ImageSizeSelect, useImageGenerationSize } from "../../../components/image/ImageSizeSelect";
import type { ImageSize } from "../../../api/image";
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
  type ProfileMaterialFilter,
  type ProfileMaterialGroup,
  type ProfileMaterialKindFilter,
  type ProfileMaterialSort,
} from "../../../api/profileMaterials";
import { generateObjectProfileImagePrompt } from "../../../api/objectProfiles";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { generateSceneProfileImagePrompt } from "../../../api/sceneProfiles";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import type { RunProgressTask } from "../../../hooks/useProgressTask";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import { isSupportedImageMime } from "../../../utils/imageUpload";
import { ImageActionDialog } from "../images/ImageActionDialog";
import { useI18n } from "../../../i18n";
import { useNovelDetailDirty } from "../novelDetailDirty";
import {
  VISUAL_STYLE_TEMPLATES,
  renderVisualStyleTemplate,
} from "../styleTemplates";
import {
  filterOptions,
  groupKey,
  kindOptions,
  MaterialCard,
  ProfileMaterialUploadDialog,
  sortOptions,
  ToolbarSelect,
} from "./VideoMaterialComponents";

type LoadMaterialsOptions = { force?: boolean };

const CUSTOM_STYLE_VALUE = "custom";
const DEFAULT_STYLE_TEMPLATE_ID = VISUAL_STYLE_TEMPLATES[0].id;
const STYLE_TEMPLATE_VERSION = "1";
const MAX_UPLOAD_IMAGE_BYTES = 20 * 1024 * 1024;


interface VideoMaterialWorkspaceProps {
  projectId: number;
  onToast: (message: string, type: "success" | "error" | "info") => void;
  runProgressTask: RunProgressTask;
  onRefreshVideoData: () => Promise<boolean | void> | boolean | void;
}

export const VideoMaterialWorkspace: React.FC<VideoMaterialWorkspaceProps> = ({
  projectId,
  onToast,
  runProgressTask,
  onRefreshVideoData,
}) => {
  const { t } = useI18n();
  const {
    error: imageSizeError,
    loading: imageSizeLoading,
    size: imageSize,
    sizes: imageSizes,
    setSize: setImageSize,
  } = useImageGenerationSize();
  const [groups, setGroups] = useState<ProfileMaterialGroup[]>([]);
  const [loading, setLoading] = useState(false);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [batchBusy, setBatchBusy] = useState(false);
  const [kindFilter, setKindFilter] = useState<ProfileMaterialKindFilter>("all");
  const [stateFilter, setStateFilter] = useState<ProfileMaterialFilter>("all");
  const [sort, setSort] = useState<ProfileMaterialSort>("missing_first");
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
  const loadInFlightRef = useRef<{ key: string; seq: number; promise: Promise<boolean> } | null>(
    null,
  );
  const actionBusyRef = useRef(false);
  const activeProjectRef = useRef(projectId);
  const loadRequestKey = `${projectId}:${kindFilter}:${query.trim()}:${stateFilter}:${sort}`;
  const activeLoadKeyRef = useRef(loadRequestKey);

  const selectedStyleTemplate = useMemo(
    () =>
      VISUAL_STYLE_TEMPLATES.find((template) => template.id === selectedStyleTemplateId) ??
      null,
    [selectedStyleTemplateId],
  );
  const materialStyle = selectedStyleTemplate
    ? renderVisualStyleTemplate(selectedStyleTemplate)
    : customStyle;
  useNovelDetailDirty(
    `video-material-style:${projectId}`,
    selectedStyleTemplateId === CUSTOM_STYLE_VALUE && customStyle.trim().length > 0,
  );

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

  const loadMaterials = useCallback(async (options: LoadMaterialsOptions = {}): Promise<boolean> => {
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
        if (loadSeqRef.current !== requestSeq || activeLoadKeyRef.current !== requestKey) return false;
        setGroups(result);
        return true;
      } catch (err) {
        if (loadSeqRef.current !== requestSeq || activeLoadKeyRef.current !== requestKey) return false;
        onToast(extractErrorMessage(err, t("video.material.loadFailed")), "error");
        return false;
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
  }, [kindFilter, loadRequestKey, onToast, projectId, query, sort, stateFilter, t]);

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
      VISUAL_STYLE_TEMPLATES.some((item) => item.id === templateId)
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

  const refreshAfterMaterialChange = async (): Promise<boolean> => {
    const [materials, parent] = await Promise.all([
      loadMaterials({ force: true }),
      Promise.resolve(onRefreshVideoData()),
    ]);
    return materials && parent !== false;
  };

  const generateMaterialCore = async (
    group: ProfileMaterialGroup,
    requestProjectId: number,
    requestSize: ImageSize,
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
              width: requestSize.width,
              height: requestSize.height,
              ...stylePayload,
            }
          : {
              kind: "object_profile",
              object_profile_id: group.profile.id,
              chapter_id: null,
              config_id: null,
              width: requestSize.width,
              height: requestSize.height,
              ...stylePayload,
            },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("video.material.generationTitle", { name: group.profile.name }),
          startMessage: t("video.material.preparingGeneration"),
          successMessage: t("video.material.generationCompleted"),
          errorMessage: t("video.material.generationFailed"),
          cancelLabel: t("video.material.stopGeneration"),
        },
        async (task) => {
          if (!isCurrentProject(requestProjectId)) throw new Error(t("video.material.taskStopped"));
          await verifyImageGenerationResult(task, requestProjectId);
        },
      );
      if (!isCurrentProject(requestProjectId)) return;
      if (options.refresh !== false) {
        if (!await refreshAfterMaterialChange()) return;
        onToast(t("video.material.generatedProjectMaterial", { name: group.profile.name }), "success");
      }
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, t("video.material.generationFailed")), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setBusyKey(null);
    }
  };

  const generateMaterial = async (
    group: ProfileMaterialGroup,
    options: { refresh?: boolean } = {},
  ) => {
    const requestProjectId = projectId;
    const requestSize = imageSize;
    if (!requestSize) {
      onToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    if (!beginAction()) return;
    try {
      await generateMaterialCore(group, requestProjectId, requestSize, options);
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
      onToast(t("video.material.copiedPrompt", { name: group.profile.name }), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, t("video.material.copyPromptFailed")), "error");
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
        onToast(t("video.material.imageFormats"), "error");
        return;
      }
      if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
        onToast(t("video.material.imageTooLarge", { size: MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024 }), "error");
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
      if (!await refreshAfterMaterialChange()) return;
      if (!isCurrentProject(requestProjectId)) return;
      onToast(uploadSetPrimary ? t("video.material.uploadedPrimary") : t("video.material.uploaded"), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, t("video.material.uploadFailed")), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setBusyKey(null);
      finishAction(requestProjectId);
    }
  };

  const batchGenerate = async (mode: "missing" | "stale") => {
    const requestProjectId = projectId;
    const requestSize = imageSize;
    if (!requestSize) {
      onToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const targets = groups.filter((group) => group.material_state === mode);
    if (targets.length === 0) {
      onToast(mode === "missing" ? t("video.material.noMissing") : t("video.material.noStale"), "info");
      return;
    }
    if (!beginAction()) return;
    setBatchBusy(true);
    try {
      for (const group of targets) {
        if (!isCurrentProject(requestProjectId)) return;
        await generateMaterialCore(group, requestProjectId, requestSize, { refresh: false });
      }
      if (!isCurrentProject(requestProjectId)) return;
      if (!await refreshAfterMaterialChange()) return;
      if (!isCurrentProject(requestProjectId)) return;
      onToast(t("video.material.batchCompleted", { count: targets.length }), "success");
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
      if (!await loadMaterials({ force: true })) return;
      if (!isCurrentProject(requestProjectId)) return;
      onToast(updated.is_favorite ? t("video.material.favorited") : t("video.material.unfavorited"), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, t("video.material.favoriteFailed")), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) setImageBusyId(null);
      finishAction(requestProjectId);
    }
  };

  const saveImageScore = async (image: GeneratedImage) => {
    const requestProjectId = projectId;
    const score = Number(imageScoreInput);
    if (!Number.isFinite(score)) {
      onToast(t("video.material.scoreInvalid"), "error");
      return;
    }
    if (!beginAction()) return;
    setImageScoringId(image.id);
    try {
      const updated = await updateImage(image.id, { ai_score: score });
      if (!isCurrentProject(requestProjectId)) return;
      setImageDetails((current) => (current?.id === updated.id ? updated : current));
      if (!await loadMaterials({ force: true })) return;
      if (!isCurrentProject(requestProjectId)) return;
      onToast(t("video.material.scored", { score }), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, t("video.material.scoreFailed")), "error");
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
      if (!await loadMaterials({ force: true })) return;
      if (!isCurrentProject(requestProjectId)) return;
      onToast(t("video.material.imageUpdated"), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, t("video.material.updateImageFailed")), "error");
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
        title: t("video.material.deleteTitle"),
        message: t("video.material.deleteMessage"),
        confirmText: t("common.delete"),
        dialogType: "danger",
      });
      if (!ok) return;
      if (!isCurrentProject(requestProjectId)) return;
      setImageBusyId(image.id);
      await deleteImage(image.id);
      if (!isCurrentProject(requestProjectId)) return;
      setImageDetails(null);
      if (!await refreshAfterMaterialChange()) return;
      if (!isCurrentProject(requestProjectId)) return;
      onToast(t("video.material.deleted"), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, t("video.material.deleteFailed")), "error");
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
      if (!await refreshAfterMaterialChange()) return;
      if (!isCurrentProject(requestProjectId)) return;
      onToast(imageId == null ? t("video.material.primaryCleared") : t("video.material.primarySet"), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      onToast(extractErrorMessage(err, t("video.material.setPrimaryFailed")), "error");
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
              aria-label={t("video.material.searchPlaceholder")}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              disabled={loading || busyKey != null || batchBusy}
              className="min-w-0 flex-1 bg-transparent text-book-text-main outline-none placeholder:text-book-text-muted"
              placeholder={t("video.material.searchPlaceholder")}
            />
          </label>
          <ToolbarSelect
            label={t("video.material.type")}
            value={kindFilter}
            options={kindOptions(t)}
            disabled={loading || busyKey != null || batchBusy}
            onChange={(value) => setKindFilter(value as ProfileMaterialKindFilter)}
          />
          <ToolbarSelect
            label={t("video.material.status")}
            value={stateFilter}
            options={filterOptions(t)}
            disabled={loading || busyKey != null || batchBusy}
            onChange={(value) => setStateFilter(value as ProfileMaterialFilter)}
          />
          <ToolbarSelect
            label={t("video.material.sort")}
            value={sort}
            options={sortOptions(t)}
            disabled={loading || busyKey != null || batchBusy}
            onChange={(value) => setSort(value as ProfileMaterialSort)}
          />
          <button
            type="button"
            onClick={() => void loadMaterials()}
            disabled={loading || busyKey != null || batchBusy}
            className="pyqt-btn-icon !h-9 !w-9"
            title={t("video.material.refresh")}
            aria-label={t("video.material.refresh")}
          >
            {loading ? <LoadingSpinner size="sm" /> : <RefreshCw size={13} />}
          </button>
        </div>
        <div className="flex flex-wrap items-center gap-3 border-t border-book-border/35 px-3 py-2 text-small leading-relaxed text-book-text-muted">
          <p className="min-w-0 basis-full sm:flex-1">
            {t("video.material.statsProfiles", { count: groups.length })} · {t("video.material.statsImages", { count: stats.imageCount })}
            {stats.missingCount > 0 ? ` · ${t("video.material.statsMissing", { count: stats.missingCount })}` : ""}
            {stats.staleCount > 0 ? ` · ${t("video.material.statsStale", { count: stats.staleCount })}` : ""}
          </p>
          {(stats.missingCount > 0 || stats.staleCount > 0) && (
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              {stats.missingCount > 0 && (
                <button type="button" onClick={() => void batchGenerate("missing")} disabled={loading || busyKey != null || batchBusy}
                  className="pyqt-btn-primary inline-flex min-h-9 items-center justify-center gap-1.5 px-3 text-xs" title={t("video.material.generateMissingTitle")}>
                  {batchBusy ? <LoadingSpinner size="sm" /> : <Sparkles size={13} />}
                  {t("video.material.fillMissing", { count: stats.missingCount })}
                </button>
              )}
              {stats.staleCount > 0 && (
                <button type="button" onClick={() => void batchGenerate("stale")} disabled={loading || busyKey != null || batchBusy}
                  className="pyqt-btn-secondary inline-flex min-h-9 items-center justify-center gap-1.5 px-3 text-xs" title={t("video.material.generateStaleTitle")}>
                  <RefreshCw size={13} />{t("video.material.updateStale", { count: stats.staleCount })}
                </button>
              )}
            </div>
          )}
        </div>
        <details className="group border-t border-book-border/35">
          <summary className="flex min-h-10 cursor-pointer list-none items-center gap-3 px-3 py-2 text-xs [&::-webkit-details-marker]:hidden">
            <span className="font-medium text-book-text-main">{t("video.material.generationSettings")}</span>
            <span className="min-w-0 flex-1 truncate text-right text-caption text-book-text-muted">
              {selectedStyleTemplate?.name ?? t("video.material.customStyle")}
            </span>
            <ChevronRight size={14} className="shrink-0 transition-transform group-open:rotate-90" />
          </summary>
          <div className="grid min-w-0 grid-cols-1 gap-2 border-t border-book-border/25 px-3 pb-3 pt-2 sm:grid-cols-2">
            <ImageSizeSelect
              label={t("settingsForm.selectImageSize")}
              value={imageSize}
              sizes={imageSizes}
              loading={imageSizeLoading}
              error={imageSizeError}
              disabled={busyKey != null || batchBusy}
              onChange={setImageSize}
            />
            <ToolbarSelect
              label={t("video.material.styleTemplate")}
              value={selectedStyleTemplate?.id ?? CUSTOM_STYLE_VALUE}
              options={[
                ...VISUAL_STYLE_TEMPLATES.map((template) => ({
                  value: template.id,
                  label: template.name,
                })),
                { value: CUSTOM_STYLE_VALUE, label: t("video.material.customBackendDefault") },
              ]}
              disabled={busyKey != null || batchBusy}
              onChange={applyStyleTemplate}
            />
            <label className="flex min-h-9 min-w-0 items-center gap-2 rounded-xl border border-book-border/60 bg-book-bg-paper/60 px-3 text-xs transition-colors focus-within:border-book-primary/70 focus-within:bg-book-bg-paper sm:col-span-2">
              <span className="shrink-0 text-book-text-muted">{t("video.material.supplement")}</span>
              <input
                value={selectedStyleTemplate ? "" : customStyle}
                onChange={(event) => setCustomStyle(event.target.value)}
                disabled={busyKey != null || batchBusy || selectedStyleTemplate != null}
                className="min-w-0 flex-1 bg-transparent text-book-text-main outline-none placeholder:text-book-text-muted"
                placeholder={
                  selectedStyleTemplate
                    ? selectedStyleTemplate.summary
                    : t("video.material.customStylePlaceholder")
                }
              />
            </label>
          </div>
        </details>
      </section>

      <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
        {groups.length === 0 ? (
          <div className="flex h-full min-h-[180px] items-center justify-center rounded-2xl border border-dashed border-book-border/50 bg-book-bg-paper/30 text-xs text-book-text-muted">
            {loading ? t("video.material.loading") : t("video.material.empty")}
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
