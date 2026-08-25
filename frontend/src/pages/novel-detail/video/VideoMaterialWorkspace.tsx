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
  type ProfileMaterialFilter,
  type ProfileMaterialGroup,
  type ProfileMaterialKind,
  type ProfileMaterialKindFilter,
  type ProfileMaterialSort,
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
import { readFileAsBase64 } from "../../../utils/fileUpload";
import { toMediaSrc } from "../../../utils/mediaSrc";
import {
  isSupportedImageMime,
  supportedImageAccept,
} from "../../../utils/imageUpload";
import { ImageActionDialog } from "../images/ImageActionDialog";
import { chapterRangeText } from "../shared";
import { useI18n, type MessageKey, type MessageTranslator } from "../../../i18n";
import {
  VISUAL_STYLE_TEMPLATES,
  renderVisualStyleTemplate,
} from "../styleTemplates";

type LoadMaterialsOptions = { force?: boolean };

const CUSTOM_STYLE_VALUE = "custom";
const DEFAULT_STYLE_TEMPLATE_ID = VISUAL_STYLE_TEMPLATES[0].id;
const STYLE_TEMPLATE_VERSION = "1";
const MAX_UPLOAD_IMAGE_BYTES = 20 * 1024 * 1024;

const kindOptions = (t: MessageTranslator): Array<{ value: ProfileMaterialKindFilter; label: string }> => [
  { value: "all", label: t("video.material.all") },
  { value: "scene", label: t("video.material.scene") },
  { value: "object", label: t("video.material.object") },
];

const filterOptions = (t: MessageTranslator): Array<{ value: ProfileMaterialFilter; label: string }> => [
  { value: "all", label: t("video.material.allStates") },
  { value: "missing", label: t("video.material.missing") },
  { value: "stale", label: t("video.material.stale") },
  { value: "unknown", label: t("video.material.old") },
  { value: "ready", label: t("video.material.ready") },
  { value: "favorite", label: t("video.material.favorite") },
];

const sortOptions = (t: MessageTranslator): Array<{ value: ProfileMaterialSort; label: string }> => [
  { value: "missing_first", label: t("video.material.missingFirst") },
  { value: "stale_first", label: t("video.material.staleFirst") },
  { value: "newest", label: t("video.material.newest") },
  { value: "importance", label: t("video.material.importance") },
];

const materialStateLabel = (value: ProfileMaterialState, t: MessageTranslator): string =>
  t(`video.material.state${value[0].toUpperCase()}${value.slice(1)}` as MessageKey);

const materialKindLabel = (value: ProfileMaterialKind, t: MessageTranslator): string =>
  t(`video.material.${value}` as MessageKey);

const IMPORTANCE_KEYS: Record<string, MessageKey> = {
  primary: "video.material.importancePrimary",
  secondary: "video.material.importanceSecondary",
  minor: "video.material.importanceMinor",
  major: "video.material.importanceMajor",
  supporting: "video.material.importanceSupporting",
  background: "video.material.importanceBackground",
  high: "video.material.importanceHigh",
  medium: "video.material.importanceMedium",
  low: "video.material.importanceLow",
  normal: "video.material.importanceNormal",
  critical: "video.material.importanceCritical",
};

const CATEGORY_KEYS: Record<string, MessageKey> = {
  room: "video.material.categoryRoom",
  building: "video.material.categoryBuilding",
  street: "video.material.categoryStreet",
  city: "video.material.categoryCity",
  wilderness: "video.material.categoryWilderness",
  route: "video.material.categoryRoute",
  landmark: "video.material.categoryLandmark",
  location: "video.material.categoryLocation",
  interior: "video.material.categoryInterior",
  exterior: "video.material.categoryExterior",
  region: "video.material.categoryRegion",
  world: "video.material.categoryWorld",
  organization: "video.material.categoryOrganization",
  weapon: "video.material.categoryWeapon",
  prop: "video.material.categoryProp",
  tool: "video.material.categoryTool",
  token: "video.material.categoryToken",
  document: "video.material.categoryDocument",
  vehicle: "video.material.categoryVehicle",
  clothing: "video.material.categoryClothing",
  location_object: "video.material.categoryLocationObject",
  environment_object: "video.material.categoryEnvironmentObject",
  artifact: "video.material.categoryArtifact",
  consumable: "video.material.categoryConsumable",
  other: "video.material.categoryOther",
};

const categoryLabel = (value: string, t: MessageTranslator): string => {
  const key = CATEGORY_KEYS[value];
  return key ? t(key) : t("video.material.uncategorized");
};

const importanceLabel = (value: string, t: MessageTranslator): string => {
  const key = IMPORTANCE_KEYS[value];
  return key ? t(key) : t("video.material.unmarked");
};

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
  const { t } = useI18n();
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
  const loadInFlightRef = useRef<{ key: string; seq: number; promise: Promise<void> } | null>(
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
        onToast(extractErrorMessage(err, t("video.material.loadFailed")), "error");
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
          title: t("video.material.generationTitle", { name: group.profile.name }),
          startMessage: t("video.material.preparingGeneration"),
          successMessage: t("video.material.generationCompleted"),
          errorMessage: t("video.material.generationFailed"),
          total: 3,
          cancelLabel: t("video.material.stopGeneration"),
        },
        async (task) => {
          if (!isCurrentProject(requestProjectId)) throw new Error(t("video.material.taskStopped"));
          await verifyImageGenerationResult(task, requestProjectId);
        },
      );
      if (!isCurrentProject(requestProjectId)) return;
      onToast(t("video.material.generatedProjectMaterial", { name: group.profile.name }), "success");
      if (options.refresh !== false) {
        await refreshAfterMaterialChange();
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
      await refreshAfterMaterialChange();
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
        await generateMaterialCore(group, requestProjectId, { refresh: false });
      }
      if (!isCurrentProject(requestProjectId)) return;
      await refreshAfterMaterialChange();
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
      await loadMaterials({ force: true });
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
      await loadMaterials({ force: true });
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
      await loadMaterials({ force: true });
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
      await refreshAfterMaterialChange();
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
      await refreshAfterMaterialChange();
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
        <div className="border-t border-book-border/35 px-3 py-2 text-small leading-relaxed text-book-text-muted">
          {t("video.material.statsProfiles", { count: groups.length })} · {t("video.material.statsImages", { count: stats.imageCount })}
          {stats.missingCount > 0 ? ` · ${t("video.material.statsMissing", { count: stats.missingCount })}` : ""}
          {stats.staleCount > 0 ? ` · ${t("video.material.statsStale", { count: stats.staleCount })}` : ""}
        </div>
        <details className="group border-t border-book-border/35">
          <summary className="flex min-h-10 cursor-pointer list-none items-center gap-3 px-3 py-2 text-xs [&::-webkit-details-marker]:hidden">
            <span className="font-medium text-book-text-main">{t("video.material.generationSettings")}</span>
            <span className="min-w-0 flex-1 truncate text-right text-caption text-book-text-muted">
              {selectedStyleTemplate?.name ?? t("video.material.customStyle")}
            </span>
          </summary>
          <div className="grid min-w-0 grid-cols-1 gap-2 border-t border-book-border/25 px-3 pb-3 pt-2 sm:grid-cols-2">
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
              wide
            />
            <label className="flex min-h-9 min-w-0 items-center gap-2 rounded-xl border border-book-border/60 bg-book-bg-paper/60 px-3 text-xs transition-colors focus-within:border-book-primary/70 focus-within:bg-book-bg-paper">
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
            {(stats.missingCount > 0 || stats.staleCount > 0) && (
              <div className="flex min-w-0 flex-wrap items-center justify-end gap-2 sm:col-span-2">
                {stats.missingCount > 0 && (
                  <button
                    type="button"
                    onClick={() => void batchGenerate("missing")}
                    disabled={loading || busyKey != null || batchBusy}
                    className="pyqt-btn-primary inline-flex min-h-9 flex-1 items-center justify-center gap-1.5 px-3 text-xs sm:flex-none"
                    title={t("video.material.generateMissingTitle")}
                  >
                    {batchBusy ? <LoadingSpinner size="sm" /> : <Sparkles size={13} />}
                    {t("video.material.fillMissing", { count: stats.missingCount })}
                  </button>
                )}
                {stats.staleCount > 0 && (
                  <button
                    type="button"
                    onClick={() => void batchGenerate("stale")}
                    disabled={loading || busyKey != null || batchBusy}
                    className="pyqt-btn-secondary inline-flex min-h-9 flex-1 items-center justify-center gap-1.5 px-3 text-xs sm:flex-none"
                    title={t("video.material.generateStaleTitle")}
                  >
                    <RefreshCw size={13} />
                    {t("video.material.updateStale", { count: stats.staleCount })}
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
  const { t } = useI18n();
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
            label: t("video.material.regenerate"),
            icon: <Sparkles size={13} />,
            disabled,
            onClick: onGenerate,
          },
          { type: "divider" as const },
        ]
      : []),
    {
      label: t("video.material.copyPrompt"),
      icon: <Clipboard size={13} />,
      disabled,
      onClick: onCopyPrompt,
    },
    {
      label: t("video.material.upload"),
      icon: <Upload size={13} />,
      disabled,
      onClick: onUpload,
    },
    ...(group.primary_image_id != null
      ? [
          { type: "divider" as const },
          {
            label: t("video.material.clearPrimary"),
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
            {materialKindLabel(group.kind, t)} · {categoryLabel(group.profile.category, t)} · {importanceLabel(group.profile.importance, t)} · {chapterRangeText(group.profile.first_seen_chapter, group.profile.last_seen_chapter, t)}
          </p>
        </div>
        <div className="shrink-0 text-right text-caption leading-relaxed">
          <div className={stateTone}>{materialStateLabel(group.material_state, t)}</div>
          <div className="text-book-text-muted">{t("video.material.imageCount", { count: group.image_count })}</div>
        </div>
      </div>

      <button
        type="button"
        disabled={!hero}
        onClick={() => hero && onOpenImage(hero.image)}
        className="relative flex aspect-[16/9] w-full items-center justify-center overflow-hidden rounded-2xl border border-book-border/45 bg-book-bg-tertiary text-xs text-book-text-muted disabled:cursor-default"
        title={hero ? t("video.material.openImage") : t("video.material.noImage")}
      >
        {heroSrc ? (
          <img
            src={heroSrc}
            alt={`${group.profile.name} ${t("video.material.primaryMaterial")}`}
            className="h-full w-full object-cover"
          />
        ) : (
          t("video.material.noImage")
        )}
        {hero ? (
          <div className="absolute left-2 top-2 flex gap-1.5 text-white">
            {hero.is_primary ? (
              <span
                className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-white/30 bg-black/35 backdrop-blur-md"
                title={t("video.material.primaryMaterial")}
                aria-label={t("video.material.primaryMaterial")}
              >
                <BadgeCheck size={13} />
              </span>
            ) : null}
            {hero.image.is_favorite ? (
              <span
                className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-white/30 bg-black/35 backdrop-blur-md"
                title={t("video.material.favoritedLabel")}
                aria-label={t("video.material.favoritedLabel")}
              >
                <Star size={13} fill="currentColor" />
              </span>
            ) : null}
          </div>
        ) : null}
      </button>

      <p className="mt-2 line-clamp-2 text-xs leading-relaxed text-book-text-sub">
        {group.profile.primary_text || t("video.material.noVisualDescription")}
      </p>
      <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-muted">
        {group.profile.secondary_text || t("video.material.noSupplementalInfo")}
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
                  title={t("video.material.materialImage")}
                >
                  {src ? (
                    <img
                      src={src}
                      alt={`${group.profile.name} ${t("video.material.candidateImage")}`}
                      className="h-full w-full object-cover"
                    />
                  ) : (
                    t("video.material.noImageShort")
                  )}
                  {item.staleness === "stale" ? (
                    <span
                      className="absolute right-1 top-1 h-2 w-2 rounded-full bg-warning ring-2 ring-black/25"
                      title={t("video.material.staleImage")}
                      aria-label={t("video.material.staleImage")}
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
                    title={t("video.material.setPrimary")}
                    aria-label={t("video.material.setPrimary")}
                  >
                    <BadgeCheck size={12} />
                  </button>
                ) : null}
              </div>
            );
          })
        ) : hero ? (
          <div className="flex h-14 w-full items-center justify-center rounded-xl border border-dashed border-book-border/50 bg-book-bg-paper/30 text-small text-book-text-muted">
            {t("video.material.noOtherCandidates")}
          </div>
        ) : (
          <div className="flex h-14 w-full items-center justify-center rounded-xl border border-dashed border-book-border/50 bg-book-bg-paper/30 text-small text-book-text-muted">
            {t("video.material.missingImage")}
          </div>
        )}
      </div>

      <div className="mt-auto flex items-center gap-2 pt-3">
        {busy ? (
          <span
            role="status"
            className="inline-flex min-h-9 min-w-0 flex-1 items-center justify-center gap-1.5 text-xs text-book-text-muted"
          >
            <LoadingSpinner size="sm" /> {t("video.material.generating")}
          </span>
        ) : directGenerate ? (
          <button
            type="button"
            onClick={onGenerate}
            disabled={disabled}
            className={`${missingMaterial ? "pyqt-btn-primary" : "pyqt-btn-secondary"} inline-flex min-h-9 min-w-0 flex-1 items-center justify-center gap-1.5 px-3 text-xs`}
            title={missingMaterial ? t("video.material.generateReusableTitle") : t("video.material.updateStaleTitle")}
          >
            {staleMaterial ? <RefreshCw size={13} /> : <Sparkles size={13} />}
            {staleMaterial ? t("video.material.updateImage") : t("video.material.generateImage")}
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
  const { t } = useI18n();
  return (
    <Modal
      isOpen
      onClose={() => {
        if (!busy) onClose();
      }}
      title={t("video.material.uploadTitle")}
      subtitle={`${materialKindLabel(group.kind, t)} · ${group.profile.name}`}
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
              {t("video.material.uploadAfterPrimary")}
            </span>
            <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
              {t("video.material.uploadAfterPrimaryHelp")}
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
            {t("video.material.chooseImage")}
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
            {t("video.material.supportedFormats")}
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
            {t("video.material.copyCurrentPrompt")}
          </button>
          {busy ? (
            <span role="status" className="inline-flex items-center gap-1.5 text-xs text-book-text-muted">
              <LoadingSpinner size="sm" />
              {t("video.material.uploading")}
            </span>
          ) : null}
        </div>
      </div>
    </Modal>
  );
}
