import React, { useCallback, useEffect, useRef, useState } from "react";
import { FileInput } from "lucide-react";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../components/feedback/PromptDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { BookButton } from "../../components/ui/BookButton";
import { BookInput } from "../../components/ui/BookInput";
import { Modal } from "../../components/ui/Modal";
import { extractErrorMessage } from "../../api";
import {
  deleteImage,
  listImagesByChapter,
  listImagesByProject,
  previewSceneImageRequest,
  readLocalImageFile,
  startImageGeneration,
  updateImage,
  uploadImage,
  verifyImageGenerationResult,
  type GeneratedImage,
  type LocalImageFile,
  type UpdateImageInput,
} from "../../api/image";
import {
  generateObjectProfileImagePrompt,
  listObjectProfileIndex,
  previewObjectProfileImageRequest,
  type ObjectProfileIndex,
} from "../../api/objectProfiles";
import type { ModelRequestPreview } from "../../api/requestPreview";
import {
  chapterGeneratingMessage,
  EmptyHint,
  isChapterGenerating,
  type ChapterRow,
} from "./shared";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import { useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { copyTextToClipboard } from "../../utils/clipboard";
import { readFileAsBase64 } from "../../utils/fileUpload";
import { isDesktopTauriRuntime } from "../../utils/runtimeMode";
import {
  isSupportedImageMime,
  MAX_UPLOAD_IMAGE_BYTES,
} from "../../utils/imageUpload";
import { ImageActionDialog } from "./images/ImageActionDialog";
import { ImageGalleryPanel } from "./images/ImageGalleryPanel";
import { ImageGenerationDialog } from "./images/ImageGenerationDialog";
import { useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { type ImageScope } from "./images/imageTabUtils";
import { DEFAULT_IMAGE_STYLE } from "./styleTemplates";
import { formatNumber, useI18n } from "../../i18n";
import { useChapterContentHash } from "./video/useChapterContentHash";

interface ImagesTabProps {
  projectId: number;
  canEditImages?: boolean;
  /**
   * 传入 undefined 时作为项目图库使用；传入 null 时表示写作桌尚未选择章节。
   */
  fixedChapter?: ChapterRow | null;
}

export const ImagesTab: React.FC<ImagesTabProps> = ({
  projectId,
  canEditImages = true,
  fixedChapter,
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const { runProgressTask } = useProgressTask();
  const {
    error: imageSizeError,
    loading: imageSizeLoading,
    size: imageSize,
    sizes: imageSizes,
    setSize: setImageSize,
  } = useImageGenerationSize();
  const hasChapterContext = fixedChapter !== undefined;
  const chapter = fixedChapter ?? null;
  const [allImages, setAllImages] = useState<GeneratedImage[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [scopeMode, setScopeMode] = useState<ImageScope>(
    hasChapterContext ? "chapter" : "project",
  );
  const [favOnly, setFavOnly] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [importBusy, setImportBusy] = useState(false);
  const [localPathDialogOpen, setLocalPathDialogOpen] = useState(false);
  const [localImagePath, setLocalImagePath] = useState("");
  const [localPathBusy, setLocalPathBusy] = useState(false);
  const [genPrompt, setGenPrompt] = useState("");
  const [genBusy, setGenBusy] = useState(false);
  const [objectProfiles, setObjectProfiles] = useState<ObjectProfileIndex[]>([]);
  const [objectProfilesError, setObjectProfilesError] = useState<string | null>(null);
  const [selectedObjectProfileId, setSelectedObjectProfileId] = useState<number | null>(null);
  const [objectImageStyle, setObjectImageStyle] = useState(DEFAULT_IMAGE_STYLE);
  const [objectImageBusy, setObjectImageBusy] = useState(false);
  const [objectPromptCopying, setObjectPromptCopying] = useState(false);
  const [scoringId, setScoringId] = useState<number | null>(null);
  const [scoreInput, setScoreInput] = useState("0.8");
  const { currentContentHash, chapterContentHashError } = useChapterContentHash(chapter);
  const [toolsOpen, setToolsOpen] = useState(false);
  const [actionImage, setActionImage] = useState<GeneratedImage | null>(null);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);

  const chapterScoped = Boolean(hasChapterContext && chapter && scopeMode === "chapter");
  const scopeKey = `${projectId}:${hasChapterContext ? chapter?.id ?? "none" : "project"}:${scopeMode}`;
  const mountedRef = useRef(false);
  const activeScopeRef = useRef(scopeKey);
  const allImagesRef = useRef<GeneratedImage[]>([]);
  const objectProfilesRef = useRef<ObjectProfileIndex[]>([]);
  const loadSeqRef = useRef(0);
  const loadInFlightRef = useRef<{
    scope: string;
    token: symbol;
    promise: Promise<void>;
  } | null>(null);
  const objectProfilesLoadSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const actionBusyRef = useRef(false);
  const importBusyRef = useRef(false);
  const previewSeqRef = useRef(0);
  const previewBusyRef = useRef(false);
  const localPathInputRef = useRef<HTMLInputElement | null>(null);
  activeScopeRef.current = scopeKey;
  allImagesRef.current = allImages;
  objectProfilesRef.current = objectProfiles;

  const isCurrentScope = useCallback(
    (requestScope: string) => mountedRef.current && activeScopeRef.current === requestScope,
    [],
  );

  const isCurrentLoad = useCallback(
    (requestScope: string, requestSeq: number) =>
      isCurrentScope(requestScope) && loadSeqRef.current === requestSeq,
    [isCurrentScope],
  );

  const isCurrentObjectProfilesLoad = useCallback(
    (requestScope: string, requestSeq: number) =>
      isCurrentScope(requestScope) && objectProfilesLoadSeqRef.current === requestSeq,
    [isCurrentScope],
  );

  const isCurrentAction = useCallback(
    (requestScope: string, requestSeq: number) =>
      isCurrentScope(requestScope) && actionSeqRef.current === requestSeq,
    [isCurrentScope],
  );

  const isCurrentPreview = useCallback(
    (requestScope: string, requestSeq: number) =>
      isCurrentScope(requestScope) && previewSeqRef.current === requestSeq,
    [isCurrentScope],
  );

  const ensureCurrentAction = useCallback(
    (requestScope: string, requestSeq: number) => {
      if (!isCurrentAction(requestScope, requestSeq)) {
        throw new Error("任务已停止");
      }
    },
    [isCurrentAction],
  );

  const beginAction = useCallback(
    (requestScope: string): number | null => {
      if (!isCurrentScope(requestScope)) return null;
      if (actionBusyRef.current) {
        addToast(t("novel.imageLibrary.operationInProgress"), "error");
        return null;
      }
      actionBusyRef.current = true;
      actionSeqRef.current += 1;
      return actionSeqRef.current;
    },
    [addToast, isCurrentScope],
  );

  const ensureCanEditImages = useCallback(() => {
    if (canEditImages) return true;
    addToast(t("novel.imageLibrary.noEditPermission"), "error");
    return false;
  }, [addToast, canEditImages]);

  const finishAction = useCallback((requestSeq: number) => {
    if (actionSeqRef.current === requestSeq) {
      actionBusyRef.current = false;
    }
  }, []);

  const commitImages = (images: GeneratedImage[]) => {
    loadSeqRef.current += 1;
    loadInFlightRef.current = null;
    allImagesRef.current = images;
    setAllImages(images);
    setLoading(false);
    setLoadError(null);
  };

  const commitImage = (image: GeneratedImage) => {
    const inScope = !chapterScoped || image.chapter_id === chapter?.id;
    const exists = allImagesRef.current.some((row) => row.id === image.id);
    const next = allImagesRef.current.flatMap((row) =>
      row.id === image.id ? (inScope ? [image] : []) : [row],
    );
    commitImages(inScope && !exists ? [image, ...next] : next);
    setActionImage((current) => current?.id === image.id ? (inScope ? image : null) : current);
  };

  const closeRequestPreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyRef.current = false;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, []);

  const beginPreview = useCallback(
    (requestScope: string): number | null => {
      if (!isCurrentScope(requestScope) || previewBusyRef.current) return null;
      previewBusyRef.current = true;
      const requestSeq = previewSeqRef.current + 1;
      previewSeqRef.current = requestSeq;
      setRequestPreviewOpen(true);
      setRequestPreviewLoading(true);
      setRequestPreviewData(null);
      return requestSeq;
    },
    [isCurrentScope],
  );

  const finishPreview = useCallback((requestScope: string, requestSeq: number) => {
    if (!isCurrentPreview(requestScope, requestSeq)) return;
    setRequestPreviewLoading(false);
    previewBusyRef.current = false;
  }, [isCurrentPreview]);

  const chapterGenerating = isChapterGenerating(chapter);
  const chapterGenerateLocked = Boolean(hasChapterContext && chapterGenerating);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      loadInFlightRef.current = null;
      objectProfilesLoadSeqRef.current += 1;
      actionSeqRef.current += 1;
      previewSeqRef.current += 1;
      actionBusyRef.current = false;
      previewBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    activeScopeRef.current = scopeKey;
    loadSeqRef.current += 1;
    loadInFlightRef.current = null;
    objectProfilesLoadSeqRef.current += 1;
    actionSeqRef.current += 1;
    previewSeqRef.current += 1;
    actionBusyRef.current = false;
    previewBusyRef.current = false;
    allImagesRef.current = [];
    objectProfilesRef.current = [];
    setAllImages([]);
    setLoading(false);
    setLoadError(null);
    setBusyId(null);
    setImportBusy(false);
    setGenBusy(false);
    setObjectProfiles([]);
    setObjectProfilesError(null);
    setSelectedObjectProfileId(null);
    setObjectImageBusy(false);
    setObjectPromptCopying(false);
    setScoringId(null);
    setToolsOpen(false);
    setActionImage(null);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [scopeKey]);

  useEffect(() => {
    if (canEditImages) return;
    actionSeqRef.current += 1;
    previewSeqRef.current += 1;
    actionBusyRef.current = false;
    previewBusyRef.current = false;
    setImportBusy(false);
    setGenBusy(false);
    setObjectImageBusy(false);
    setObjectPromptCopying(false);
    setBusyId(null);
    setScoringId(null);
    setToolsOpen(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [canEditImages]);

  useEffect(() => {
    if (hasChapterContext) setScopeMode("chapter");
  }, [hasChapterContext, chapter?.id]);

  const load = useCallback(async (options: { force?: boolean } = {}) => {
    const requestScope = scopeKey;
    if (!options.force && loadInFlightRef.current?.scope === requestScope) {
      return loadInFlightRef.current.promise;
    }
    const requestProjectId = projectId;
    const requestChapter = chapter;
    const requestChapterScoped = chapterScoped;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    const requestToken = Symbol(requestScope);
    const loadPromise = (async () => {
      if (hasChapterContext && !chapter) {
        if (isCurrentLoad(requestScope, requestSeq)) {
          setAllImages([]);
          setLoading(false);
        }
        return;
      }
      if (isCurrentLoad(requestScope, requestSeq)) {
        setLoading(true);
        setLoadError(null);
      }
      try {
        let data: GeneratedImage[];
        if (requestChapterScoped && requestChapter) {
          data = await listImagesByChapter(requestProjectId, requestChapter.id);
        } else {
          data = await listImagesByProject(requestProjectId);
        }
        if (!isCurrentLoad(requestScope, requestSeq)) return;
        allImagesRef.current = data;
        setAllImages(data);
      } catch (err) {
        if (isCurrentLoad(requestScope, requestSeq)) {
          const message = extractErrorMessage(err, t("novel.imageLibrary.loadFailed"));
          setLoadError(message);
          addToast(message, "error");
        }
      } finally {
        if (isCurrentLoad(requestScope, requestSeq)) setLoading(false);
        if (
          loadInFlightRef.current?.scope === requestScope &&
          loadInFlightRef.current.token === requestToken
        ) {
          loadInFlightRef.current = null;
        }
      }
    })();
    loadInFlightRef.current = { scope: requestScope, token: requestToken, promise: loadPromise };
    return loadPromise;
  }, [
    addToast,
    chapter,
    chapterScoped,
    hasChapterContext,
    isCurrentLoad,
    projectId,
    scopeMode,
    scopeKey,
  ]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setActionImage((current) => {
      if (!current) return null;
      return allImages.find((image) => image.id === current.id) ?? null;
    });
  }, [allImages]);

  const loadObjectProfiles = useCallback(async (notify = true): Promise<boolean> => {
    if (!canEditImages) {
      objectProfilesRef.current = [];
      setObjectProfiles([]);
      setObjectProfilesError(null);
      setSelectedObjectProfileId(null);
      return true;
    }
    const requestScope = scopeKey;
    const requestProjectId = projectId;
    const requestSeq = objectProfilesLoadSeqRef.current + 1;
    objectProfilesLoadSeqRef.current = requestSeq;
    setObjectProfilesError(null);
    try {
      const rows = await listObjectProfileIndex(requestProjectId);
      if (!isCurrentObjectProfilesLoad(requestScope, requestSeq)) return false;
      objectProfilesRef.current = rows;
      setObjectProfiles(rows);
      setObjectProfilesError(null);
      setSelectedObjectProfileId((current) =>
        current != null && rows.some((profile) => profile.id === current)
          ? current
          : rows[0]?.id ?? null,
      );
      return true;
    } catch (err) {
      if (isCurrentObjectProfilesLoad(requestScope, requestSeq)) {
        const message = extractErrorMessage(err, t("novel.imageLibrary.loadProfilesFailed"));
        setObjectProfilesError(message);
        if (notify) {
          addToast(message, "error");
        }
      }
      return false;
    }
  }, [addToast, canEditImages, isCurrentObjectProfilesLoad, projectId, scopeKey]);

  useEffect(() => {
    void loadObjectProfiles();
  }, [loadObjectProfiles]);

  const handleGenerate = async () => {
    if (!ensureCanEditImages()) return;
    const requestScope = scopeKey;
    const requestProjectId = projectId;
    const requestChapter = chapter;
    const requestPrompt = genPrompt.trim();
    if (chapterGenerateLocked) {
      addToast(chapterGeneratingMessage(chapter, t("novel.imageLibrary.chapterTitle"), t), "error");
      return;
    }
    if (!requestPrompt) {
      addToast(t("novel.imageLibrary.promptRequired"), "error");
      return;
    }
    const requestSize = imageSize;
    if (!requestSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setGenBusy(true);
    try {
      const handle = await startImageGeneration({
        project_id: requestProjectId,
        operation: {
          kind: "scene",
          prompt: requestPrompt,
          width: requestSize.width,
          height: requestSize.height,
          config_id: null,
          chapter_id: requestChapter?.id ?? null,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: requestChapter ? t("novel.imageLibrary.chapterTitle") : t("novel.imageLibrary.projectTitle"),
          startMessage: t("novel.imageLibrary.generating"),
          successMessage: requestChapter ? t("novel.imageLibrary.chapterTitle") : t("novel.imageLibrary.projectTitle"),
          errorMessage: t("novel.imageLibrary.generationFailed"),
          cancelLabel: t("workflow.stop"),
        },
        async (task) => {
          ensureCurrentAction(requestScope, actionSeq);
          const images = await verifyImageGenerationResult(task, requestProjectId);
          ensureCurrentAction(requestScope, actionSeq);
          images.forEach(commitImage);
        },
      );
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast(requestChapter ? t("novel.imageLibrary.chapterTitle") : t("novel.imageLibrary.projectTitle"), "success");
      setGenPrompt("");
    } catch (err) {
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const message = extractErrorMessage(err, t("novel.imageLibrary.generationFailed"));
      if (isCancelledOperation(message)) {
        addToast(t("novel.imageLibrary.generationStopped"), "info");
      } else {
        addToast(message, "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setGenBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewGenerate = async () => {
    if (!ensureCanEditImages()) return;
    const requestScope = scopeKey;
    const requestProjectId = projectId;
    const requestChapter = chapter;
    const requestPrompt = genPrompt.trim();
    if (chapterGenerateLocked) {
      addToast(chapterGeneratingMessage(chapter, t("novel.imageLibrary.requestPreview"), t), "error");
      return;
    }
    if (!requestPrompt) {
      addToast(t("novel.imageLibrary.promptRequired"), "error");
      return;
    }
    if (!imageSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const requestSeq = beginPreview(requestScope);
    if (requestSeq == null) return;
    try {
      const preview = await previewSceneImageRequest(
        requestProjectId,
        requestPrompt,
        imageSize.width,
        imageSize.height,
        null,
        requestChapter?.id ?? null,
      );
      if (!isCurrentPreview(requestScope, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentPreview(requestScope, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.imageLibrary.previewFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      finishPreview(requestScope, requestSeq);
    }
  };

  const handlePreviewObjectImage = async () => {
    if (!ensureCanEditImages()) return;
    const requestScope = scopeKey;
    const requestProfileId = selectedObjectProfileId;
    const requestChapterId = chapter?.id ?? null;
    const requestStyle = objectImageStyle.trim() || null;
    if (requestProfileId == null) {
      addToast(t("novel.imageLibrary.objectRequired"), "error");
      return;
    }
    if (!objectProfilesRef.current.some((profile) => profile.id === requestProfileId)) {
      addToast(t("novel.imageLibrary.objectNotFound"), "error");
      return;
    }
    if (!imageSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const requestSeq = beginPreview(requestScope);
    if (requestSeq == null) return;
    try {
      const preview = await previewObjectProfileImageRequest({
        object_profile_id: requestProfileId,
        chapter_id: requestChapterId,
        style: requestStyle,
        width: imageSize.width,
        height: imageSize.height,
      });
      if (!isCurrentPreview(requestScope, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentPreview(requestScope, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.imageLibrary.previewFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      finishPreview(requestScope, requestSeq);
    }
  };

  const handleCopyObjectImagePrompt = async () => {
    if (!ensureCanEditImages()) return;
    const requestScope = scopeKey;
    const requestProfileId = selectedObjectProfileId;
    const requestChapterId = chapter?.id ?? null;
    const requestStyle = objectImageStyle.trim() || null;
    if (requestProfileId == null) {
      addToast(t("novel.imageLibrary.objectRequired"), "error");
      return;
    }
    if (!objectProfilesRef.current.some((profile) => profile.id === requestProfileId)) {
      addToast(t("novel.imageLibrary.objectNotFound"), "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setObjectPromptCopying(true);
    try {
      const result = await generateObjectProfileImagePrompt({
        object_profile_id: requestProfileId,
        chapter_id: requestChapterId,
        style: requestStyle,
      });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      await copyTextToClipboard(result.prompt);
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast(t("novel.imageLibrary.copyPromptSuccess", { name: result.object_name }), "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, t("novel.imageLibrary.copyPromptFailed")), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setObjectPromptCopying(false);
      finishAction(actionSeq);
    }
  };

  const handleGenerateObjectImage = async () => {
    if (!ensureCanEditImages()) return;
    const requestScope = scopeKey;
    const requestProjectId = projectId;
    const requestProfileId = selectedObjectProfileId;
    const requestChapterId = chapter?.id ?? null;
    const requestStyle = objectImageStyle.trim() || null;
    if (requestProfileId == null) {
      addToast(t("novel.imageLibrary.objectRequired"), "error");
      return;
    }
    if (!objectProfilesRef.current.some((profile) => profile.id === requestProfileId)) {
      addToast(t("novel.imageLibrary.objectNotFound"), "error");
      return;
    }
    if (!imageSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setObjectImageBusy(true);
    try {
      const handle = await startImageGeneration({
        project_id: requestProjectId,
        operation: {
          kind: "object_profile",
          object_profile_id: requestProfileId,
          chapter_id: requestChapterId,
          style: requestStyle,
          width: imageSize.width,
          height: imageSize.height,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("novel.profile.generateObjectImage"),
          startMessage: t("novel.imageLibrary.generating"),
          successMessage: t("novel.profile.generateObjectImage"),
          errorMessage: t("novel.imageLibrary.generationFailed"),
          cancelLabel: t("workflow.stop"),
        },
        async (task) => {
          ensureCurrentAction(requestScope, actionSeq);
          const images = await verifyImageGenerationResult(task, requestProjectId);
          ensureCurrentAction(requestScope, actionSeq);
          images.forEach(commitImage);
          if (!await loadObjectProfiles(false)) {
            throw new Error(t("novel.imageLibrary.loadProfilesFailed"));
          }
          ensureCurrentAction(requestScope, actionSeq);
        },
      );
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast(t("novel.profile.generateObjectImage"), "success");
    } catch (err) {
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const message = extractErrorMessage(err, t("novel.imageLibrary.generationFailed"));
      if (isCancelledOperation(message)) {
        addToast(t("novel.imageLibrary.generationStopped"), "info");
      } else {
        addToast(message, "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setObjectImageBusy(false);
      finishAction(actionSeq);
    }
  };

  const handleToggleFavorite = async (img: GeneratedImage) => {
    if (!ensureCanEditImages()) return;
    const requestScope = scopeKey;
    const requestImage = allImagesRef.current.find((image) => image.id === img.id) ?? null;
    if (!requestImage) {
      addToast(t("novel.imageLibrary.imageNotFound"), "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setBusyId(requestImage.id);
    try {
      const updated = await updateImage(requestImage.id, { is_favorite: !requestImage.is_favorite });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      commitImage(updated);
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, t("novel.imageLibrary.favoriteFailed")), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setBusyId(null);
      finishAction(actionSeq);
    }
  };

  const handleImportImageFile = async (file: File | LocalImageFile) => {
    if (!ensureCanEditImages() || importBusyRef.current) return;
    const requestScope = scopeKey;
    const requestProjectId = projectId;
    const requestChapterScoped = chapterScoped;
    const requestChapter = chapter;
    importBusyRef.current = true;
    setImportBusy(true);
    try {
      if (!isSupportedImageMime(file.type)) {
        addToast(t("novel.imageLibrary.unsupportedFormat"), "error");
        return;
      }
      if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
        addToast(t("novel.importReview.fileTooLarge", { size: formatNumber(MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024, locale) }), "error");
        return;
      }
      if (!isCurrentScope(requestScope)) return;
      const prompt = await promptDialog({
        title: t("novel.imageLibrary.importTitle"),
        message: t("novel.imageLibrary.importMessage"),
        defaultValue: t("novel.imageLibrary.externalImport"),
        placeholder: t("novel.imageLibrary.descriptionPlaceholder"),
        confirmText: t("novel.imageLibrary.importConfirm"),
        multiline: true,
        required: true,
      });
      if (!prompt?.trim() || !isCurrentScope(requestScope)) return;
      const actionSeq = beginAction(requestScope);
      if (actionSeq == null) return;
      try {
        const dataBase64 = "dataBase64" in file
          ? file.dataBase64
          : await readFileAsBase64(file);
        if (!isCurrentAction(requestScope, actionSeq)) return;
        const imported = await uploadImage(requestProjectId, {
          file_name: file.name,
          mime_type: file.type,
          data_base64: dataBase64,
          image_type: requestChapterScoped && requestChapter ? "chapter_reference" : "project_reference",
          prompt: prompt.trim(),
          chapter_id: requestChapterScoped && requestChapter ? requestChapter.id : null,
          metadata_json: {
            source: requestChapterScoped && requestChapter ? "chapter_image_upload" : "project_image_upload",
            manual_upload: true,
          },
        });
        if (!isCurrentAction(requestScope, actionSeq)) return;
        commitImage(imported);
        addToast(t("novel.imageLibrary.imported"), "success");
      } catch (err) {
        if (isCurrentAction(requestScope, actionSeq)) {
          addToast(extractErrorMessage(err, t("novel.imageLibrary.importFailed")), "error");
        }
      } finally {
        finishAction(actionSeq);
      }
    } catch (err) {
      if (isCurrentScope(requestScope)) {
        addToast(extractErrorMessage(err, t("novel.imageLibrary.importFailed")), "error");
      }
    } finally {
      importBusyRef.current = false;
      if (isCurrentScope(requestScope)) setImportBusy(false);
    }
  };

  const handleOpenLocalImagePath = () => {
    if (!ensureCanEditImages() || importBusyRef.current || localPathBusy) return;
    setLocalImagePath("");
    setLocalPathDialogOpen(true);
  };

  const handleReadLocalImage = async () => {
    const path = localImagePath.trim();
    if (!path) {
      addToast(t("novel.imageLibrary.localPathRequired"), "error");
      localPathInputRef.current?.focus();
      return;
    }
    setLocalPathBusy(true);
    try {
      const file = await readLocalImageFile(path);
      setLocalPathDialogOpen(false);
      await handleImportImageFile(file);
    } catch (err) {
      addToast(extractErrorMessage(err, t("novel.imageLibrary.importFailed")), "error");
    } finally {
      setLocalPathBusy(false);
    }
  };

  const handleUpdateImage = async (img: GeneratedImage, input: UpdateImageInput) => {
    if (!ensureCanEditImages()) throw new Error(t("novel.imageLibrary.noEditPermission"));
    const requestScope = scopeKey;
    const requestImage = allImagesRef.current.find((image) => image.id === img.id) ?? null;
    if (!requestImage) {
      addToast(t("novel.imageLibrary.imageNotFound"), "error");
      throw new Error(t("novel.imageLibrary.imageNotFound"));
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) throw new Error("任务已停止");
    setBusyId(requestImage.id);
    try {
      const updated = await updateImage(requestImage.id, input);
      if (!isCurrentAction(requestScope, actionSeq)) throw new Error("任务已停止");
      commitImage(updated);
      addToast(t("novel.imageLibrary.updateSuccess"), "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, t("novel.imageLibrary.updateFailed")), "error");
      }
      throw err;
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setBusyId(null);
      finishAction(actionSeq);
    }
  };

  const handleDelete = async (img: GeneratedImage) => {
    if (!ensureCanEditImages()) return;
    const requestScope = scopeKey;
    const requestImage = allImagesRef.current.find((image) => image.id === img.id) ?? null;
    if (!requestImage) {
      addToast(t("novel.imageLibrary.imageNotFound"), "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    const ok = await confirmDialog({
      title: t("novel.imageLibrary.deleteTitle"),
      message: t("novel.imageLibrary.deleteMessage"),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!ok) {
      finishAction(actionSeq);
      return;
    }
    if (!isCurrentAction(requestScope, actionSeq)) {
      finishAction(actionSeq);
      return;
    }
    if (!allImagesRef.current.some((image) => image.id === requestImage.id)) {
      addToast(t("novel.imageLibrary.imageNotFound"), "error");
      finishAction(actionSeq);
      return;
    }
    setBusyId(requestImage.id);
    try {
      await deleteImage(requestImage.id);
      if (!isCurrentAction(requestScope, actionSeq)) return;
      commitImages(allImagesRef.current.filter((image) => image.id !== requestImage.id));
      addToast(t("novel.imageLibrary.deleted"), "success");
      setActionImage(null);
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, t("novel.imageLibrary.deleteFailed")), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setBusyId(null);
      finishAction(actionSeq);
    }
  };

  const handleScore = async (img: GeneratedImage) => {
    if (!ensureCanEditImages()) return;
    const requestScope = scopeKey;
    const requestImage = allImagesRef.current.find((image) => image.id === img.id) ?? null;
    if (!requestImage) {
      addToast(t("novel.imageLibrary.imageNotFound"), "error");
      return;
    }
    const score = Number(scoreInput);
    if (!Number.isFinite(score) || score < 0 || score > 1) {
      addToast(t("novel.imageLibrary.scoreInvalid"), "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setScoringId(requestImage.id);
    try {
      const updated = await updateImage(requestImage.id, { ai_score: score });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      commitImage(updated);
      addToast(t("novel.imageLibrary.scoreSaved", { value: formatNumber(score, locale) }), "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, t("novel.imageLibrary.scoreFailed")), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setScoringId(null);
      finishAction(actionSeq);
    }
  };

  const handleCloseImageActions = () => {
    if (busyId == null && scoringId == null) setActionImage(null);
  };

  const handleOpenImageActions = (image: GeneratedImage) => {
    setScoreInput(image.ai_score == null ? "0.8" : image.ai_score.toFixed(2));
    setActionImage(image);
  };

  if (hasChapterContext && !chapter) {
    return <EmptyHint text={t("novel.imageLibrary.selectChapter")} />;
  }

  return (
    <div className="content-wide flex flex-col gap-4">
      <ImageGalleryPanel
        loadError={loadError}
        allImages={allImages}
        chapter={chapter}
        hasChapterContext={hasChapterContext}
        chapterScoped={chapterScoped}
        scopeMode={scopeMode}
        favOnly={favOnly}
        loading={loading}
        importBusy={importBusy}
        currentContentHash={currentContentHash}
        busyId={busyId}
        scoringId={scoringId}
        canEditImages={canEditImages}
        onScopeModeChange={setScopeMode}
        onFavoriteOnlyChange={setFavOnly}
        onRefresh={() => void load()}
        onOpenTools={() => setToolsOpen(true)}
        onImportImageFile={(file) => void handleImportImageFile(file)}
        onPickLocalImageFile={isDesktopTauriRuntime() ? handleOpenLocalImagePath : undefined}
        onToggleFavorite={(image) => void handleToggleFavorite(image)}
        onOpenActions={handleOpenImageActions}
        contentHashError={chapterContentHashError}
      />
      <ImageGenerationDialog
        open={canEditImages && toolsOpen}
        chapter={chapter}
        genPrompt={genPrompt}
        genBusy={genBusy}
        objectProfiles={objectProfiles}
        objectProfilesError={objectProfilesError}
        selectedObjectProfileId={selectedObjectProfileId}
        objectImageStyle={objectImageStyle}
        objectImageBusy={objectImageBusy}
        objectPromptCopying={objectPromptCopying}
        imageSize={imageSize}
        imageSizes={imageSizes}
        imageSizeLoading={imageSizeLoading}
        imageSizeError={imageSizeError}
        chapterGenerateLocked={chapterGenerateLocked}
        onClose={() => setToolsOpen(false)}
        onGenPromptChange={setGenPrompt}
        onSelectedObjectProfileIdChange={setSelectedObjectProfileId}
        onObjectImageStyleChange={setObjectImageStyle}
        onImageSizeChange={setImageSize}
        onCopyObjectImagePrompt={() => void handleCopyObjectImagePrompt()}
        onPreviewGenerate={() => void handlePreviewGenerate()}
        onGenerate={() => void handleGenerate()}
        onPreviewObjectImage={() => void handlePreviewObjectImage()}
        onGenerateObjectImage={() => void handleGenerateObjectImage()}
        onRetryObjectProfiles={() => void loadObjectProfiles()}
      />

      <ImageActionDialog
        image={actionImage}
        busyId={busyId}
        scoringId={scoringId}
        canEditImages={canEditImages}
        scoreInput={scoreInput}
        onClose={handleCloseImageActions}
        onToggleFavorite={(image) => void handleToggleFavorite(image)}
        onUpdate={(image, input) => handleUpdateImage(image, input)}
        onScoreInputChange={setScoreInput}
        onScore={(image) => void handleScore(image)}
        onDelete={(image) => void handleDelete(image)}
      />
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("novel.imageLibrary.requestPreview")}
        onClose={closeRequestPreview}
      />
      <Modal
        isOpen={localPathDialogOpen}
        onClose={() => {
          if (!localPathBusy) setLocalPathDialogOpen(false);
        }}
        title={t("novel.imageLibrary.localPathTitle")}
        initialFocusRef={localPathInputRef}
        footer={(
          <div className="flex justify-end gap-2">
            <BookButton
              variant="ghost"
              size="sm"
              onClick={() => setLocalPathDialogOpen(false)}
              disabled={localPathBusy}
            >
              {t("common.cancel")}
            </BookButton>
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void handleReadLocalImage()}
              disabled={localPathBusy || !localImagePath.trim()}
            >
              <FileInput size={13} />
              {t("novel.imageLibrary.localPathRead")}
            </BookButton>
          </div>
        )}
      >
        <div className="space-y-3">
          <p className="text-small leading-relaxed text-book-text-muted">
            {t("novel.imageLibrary.localPathMessage")}
          </p>
          <BookInput
            ref={localPathInputRef}
            label={t("novel.imageLibrary.localPathLabel")}
            value={localImagePath}
            onChange={(event) => setLocalImagePath(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !localPathBusy) void handleReadLocalImage();
            }}
            placeholder={t("novel.imageLibrary.localPathPlaceholder")}
            disabled={localPathBusy}
            autoComplete="off"
            spellCheck={false}
          />
        </div>
      </Modal>
    </div>
  );
};
