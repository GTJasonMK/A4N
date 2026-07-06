import React, { useCallback, useEffect, useRef, useState, type ChangeEvent } from "react";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../components/feedback/PromptDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../../api";
import {
  deleteImage,
  generateSceneImage,
  listImagesByChapter,
  listImagesByProject,
  previewSceneImageRequest,
  scoreImage,
  toggleImageFavorite,
  updateImage,
  uploadImage,
  type GeneratedImage,
  type UpdateImageInput,
} from "../../api/image";
import {
  generateObjectProfileImage,
  generateObjectProfileImagePrompt,
  listObjectProfiles,
  previewObjectProfileImageRequest,
  type ObjectProfile,
} from "../../api/objectProfiles";
import type { ModelRequestPreview } from "../../api/requestPreview";
import {
  chapterGeneratingMessage,
  EmptyHint,
  isChapterGenerating,
  type ChapterRow,
} from "./shared";
import { createProgressTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { copyTextToClipboard } from "../../utils/clipboard";
import { sha256Hex } from "../../utils/contentHash";
import {
  isSupportedImageMime,
  MAX_UPLOAD_IMAGE_BYTES,
  readFileAsBase64,
  supportedImageAccept,
} from "../../utils/imageUpload";
import { ImageActionDialog } from "./images/ImageActionDialog";
import { ImageGalleryPanel } from "./images/ImageGalleryPanel";
import { ImageGenerationDialog } from "./images/ImageGenerationDialog";
import { type ImageScope } from "./images/imageTabUtils";
import { DEFAULT_IMAGE_STYLE } from "./styleTemplates";

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
  const { runProgressTask } = useProgressTask();
  const hasChapterContext = fixedChapter !== undefined;
  const chapter = fixedChapter ?? null;
  const [allImages, setAllImages] = useState<GeneratedImage[]>([]);
  const [loading, setLoading] = useState(false);
  const [scopeMode, setScopeMode] = useState<ImageScope>(
    hasChapterContext ? "chapter" : "project",
  );
  const [favOnly, setFavOnly] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [importBusy, setImportBusy] = useState(false);
  const [genPrompt, setGenPrompt] = useState("");
  const [genBusy, setGenBusy] = useState(false);
  const [objectProfiles, setObjectProfiles] = useState<ObjectProfile[]>([]);
  const [selectedObjectProfileId, setSelectedObjectProfileId] = useState<number | null>(null);
  const [objectImageStyle, setObjectImageStyle] = useState(DEFAULT_IMAGE_STYLE);
  const [objectImageBusy, setObjectImageBusy] = useState(false);
  const [objectPromptCopying, setObjectPromptCopying] = useState(false);
  const [scoringId, setScoringId] = useState<number | null>(null);
  const [scoreInput, setScoreInput] = useState("0.8");
  const [currentContentHash, setCurrentContentHash] = useState<string | null>(null);
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
  const objectProfilesRef = useRef<ObjectProfile[]>([]);
  const loadSeqRef = useRef(0);
  const loadInFlightRef = useRef<{
    scope: string;
    token: symbol;
    promise: Promise<void>;
  } | null>(null);
  const objectProfilesLoadSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const actionBusyRef = useRef(false);
  const previewSeqRef = useRef(0);
  const previewBusyRef = useRef(false);
  const importFileInputRef = useRef<HTMLInputElement | null>(null);

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
        addToast("已有图片操作进行中，请等待当前操作完成", "error");
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
    addToast("当前账号没有编辑此项目图像的权限", "error");
    return false;
  }, [addToast, canEditImages]);

  const finishAction = useCallback((requestSeq: number) => {
    if (actionSeqRef.current === requestSeq) {
      actionBusyRef.current = false;
    }
  }, []);

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
    setBusyId(null);
    setImportBusy(false);
    setGenBusy(false);
    setObjectProfiles([]);
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

  useEffect(() => {
    let cancelled = false;
    if (!chapter?.content?.trim()) {
      setCurrentContentHash(null);
      return;
    }
    void sha256Hex(chapter.content)
      .then((hash) => {
        if (!cancelled) setCurrentContentHash(hash);
      })
      .catch(() => {
        if (!cancelled) setCurrentContentHash(null);
      });
    return () => {
      cancelled = true;
    };
  }, [chapter?.content]);

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
      if (isCurrentLoad(requestScope, requestSeq)) setLoading(true);
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
          addToast(extractErrorMessage(err, "加载图像失败"), "error");
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

  const loadObjectProfiles = useCallback(async () => {
    if (!canEditImages) {
      objectProfilesRef.current = [];
      setObjectProfiles([]);
      setSelectedObjectProfileId(null);
      return;
    }
    const requestScope = scopeKey;
    const requestProjectId = projectId;
    const requestSeq = objectProfilesLoadSeqRef.current + 1;
    objectProfilesLoadSeqRef.current = requestSeq;
    try {
      const rows = await listObjectProfiles(requestProjectId);
      if (!isCurrentObjectProfilesLoad(requestScope, requestSeq)) return;
      objectProfilesRef.current = rows;
      setObjectProfiles(rows);
      setSelectedObjectProfileId((current) =>
        current != null && rows.some((profile) => profile.id === current)
          ? current
          : rows[0]?.id ?? null,
      );
    } catch (err) {
      if (isCurrentObjectProfilesLoad(requestScope, requestSeq)) {
        addToast(extractErrorMessage(err, "加载物体档案失败"), "error");
      }
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
      addToast(chapterGeneratingMessage(chapter, "生成章节场景图"), "error");
      return;
    }
    if (!requestPrompt) {
      addToast("请填写 prompt", "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    const taskId = createProgressTaskId("image-scene", requestProjectId);
    setGenBusy(true);
    try {
      await runProgressTask(
        {
          title: requestChapter ? "生成章节场景图" : "生成项目场景图",
          startMessage: "正在准备图片生成…",
          successMessage: requestChapter ? "章节场景图已生成" : "项目场景图已生成",
          errorMessage: "图片生成失败",
          total: 3,
          taskId,
          cancelLabel: "停止生成",
          runningHint: "图片生成过程中请保持当前窗口打开",
        },
        async ({ setProgress }) => {
          ensureCurrentAction(requestScope, actionSeq);
          setProgress("正在提交图片生成任务…", 0, 3);
          await generateSceneImage(
            requestProjectId,
            requestPrompt,
            null,
            null,
            requestChapter?.id ?? null,
            taskId,
          );
          ensureCurrentAction(requestScope, actionSeq);
          setProgress("正在刷新图片列表…", 3, 3);
          await load({ force: true });
          ensureCurrentAction(requestScope, actionSeq);
        },
      );
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast(requestChapter ? "已生成章节场景图" : "已生成项目场景图", "success");
      setGenPrompt("");
    } catch (err) {
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const message = extractErrorMessage(err, "生成失败");
      if (isCancelledOperation(message)) {
        addToast("图片生成已停止", "info");
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
      addToast(chapterGeneratingMessage(chapter, "预览章节场景图请求"), "error");
      return;
    }
    if (!requestPrompt) {
      addToast("请填写 prompt", "error");
      return;
    }
    const requestSeq = beginPreview(requestScope);
    if (requestSeq == null) return;
    try {
      const preview = await previewSceneImageRequest(
        requestProjectId,
        requestPrompt,
        null,
        requestChapter?.id ?? null,
      );
      if (!isCurrentPreview(requestScope, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentPreview(requestScope, requestSeq)) {
        addToast(extractErrorMessage(err, "预览图片请求失败"), "error");
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
      addToast("请先选择物体档案", "error");
      return;
    }
    if (!objectProfilesRef.current.some((profile) => profile.id === requestProfileId)) {
      addToast("物体档案已不在当前项目中", "error");
      return;
    }
    const requestSeq = beginPreview(requestScope);
    if (requestSeq == null) return;
    try {
      const preview = await previewObjectProfileImageRequest({
        object_profile_id: requestProfileId,
        chapter_id: requestChapterId,
        style: requestStyle,
      });
      if (!isCurrentPreview(requestScope, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentPreview(requestScope, requestSeq)) {
        addToast(extractErrorMessage(err, "预览物体素材图请求失败"), "error");
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
      addToast("请先选择物体档案", "error");
      return;
    }
    if (!objectProfilesRef.current.some((profile) => profile.id === requestProfileId)) {
      addToast("物体档案已不在当前项目中", "error");
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
      addToast(`已复制 ${result.object_name} 的素材提示词`, "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "复制物体素材提示词失败"), "error");
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
      addToast("请先选择物体档案", "error");
      return;
    }
    if (!objectProfilesRef.current.some((profile) => profile.id === requestProfileId)) {
      addToast("物体档案已不在当前项目中", "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    const taskId = createProgressTaskId("object-profile-image", requestProjectId);
    setObjectImageBusy(true);
    try {
      await runProgressTask(
        {
          title: "生成物体素材图",
          startMessage: "正在准备物体素材图",
          successMessage: "物体素材图已生成",
          errorMessage: "物体素材图生成失败",
          total: 3,
          taskId,
          cancelLabel: "停止生成",
          runningHint: "图片生成过程中请保持当前窗口打开",
        },
        async () => {
          ensureCurrentAction(requestScope, actionSeq);
          await generateObjectProfileImage(
            {
              object_profile_id: requestProfileId,
              chapter_id: requestChapterId,
              style: requestStyle,
            },
            taskId,
          );
          ensureCurrentAction(requestScope, actionSeq);
          await load({ force: true });
          ensureCurrentAction(requestScope, actionSeq);
          await loadObjectProfiles();
          ensureCurrentAction(requestScope, actionSeq);
        },
      );
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast("物体素材图已生成", "success");
    } catch (err) {
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const message = extractErrorMessage(err, "物体素材图生成失败");
      if (isCancelledOperation(message)) {
        addToast("图片生成已停止", "info");
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
      addToast("图片已不在当前列表中", "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setBusyId(requestImage.id);
    try {
      await toggleImageFavorite(requestImage.id);
      if (!isCurrentAction(requestScope, actionSeq)) return;
      await load({ force: true });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      setActionImage(null);
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "切换收藏失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setBusyId(null);
      finishAction(actionSeq);
    }
  };

  const handlePickImportImage = () => {
    if (!ensureCanEditImages()) return;
    if (importBusy || actionBusyRef.current) {
      addToast("已有图片操作进行中，请等待当前操作完成", "error");
      return;
    }
    importFileInputRef.current?.click();
  };

  const handleImportImageFile = async (event: ChangeEvent<HTMLInputElement>) => {
    if (!ensureCanEditImages()) {
      event.currentTarget.value = "";
      return;
    }
    const requestScope = scopeKey;
    const requestProjectId = projectId;
    const requestChapterScoped = chapterScoped;
    const requestChapter = chapter;
    const file = event.currentTarget.files?.[0] ?? null;
    event.currentTarget.value = "";
    if (!file) return;
    if (!isSupportedImageMime(file.type)) {
      addToast("只支持 PNG/JPEG/WebP 图片", "error");
      return;
    }
    if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
      addToast(`上传图片不能超过 ${MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024}MB`, "error");
      return;
    }
    if (!isCurrentScope(requestScope)) return;
    const prompt = await promptDialog({
      title: "导入图片",
      message: "填写这张图片的用途或画面描述。",
      defaultValue: "外部导入图片",
      placeholder: "图片描述",
      confirmText: "导入",
      multiline: true,
      required: true,
    });
    if (!prompt?.trim()) return;
    if (!isCurrentScope(requestScope)) return;
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setImportBusy(true);
    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentAction(requestScope, actionSeq)) return;
      await uploadImage(requestProjectId, {
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
      addToast("图片已导入", "success");
      await load({ force: true });
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "导入图片失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setImportBusy(false);
      finishAction(actionSeq);
    }
  };

  const handleUpdateImage = async (img: GeneratedImage, input: UpdateImageInput) => {
    if (!ensureCanEditImages()) throw new Error("当前账号没有编辑此项目图像的权限");
    const requestScope = scopeKey;
    const requestImage = allImagesRef.current.find((image) => image.id === img.id) ?? null;
    if (!requestImage) {
      addToast("图片已不在当前列表中", "error");
      throw new Error("图片已不在当前列表中");
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) throw new Error("任务已停止");
    setBusyId(requestImage.id);
    try {
      const updated = await updateImage(requestImage.id, input);
      if (!isCurrentAction(requestScope, actionSeq)) throw new Error("任务已停止");
      setActionImage((current) => (current?.id === updated.id ? updated : current));
      addToast("图片记录已更新", "success");
      await load({ force: true });
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "更新图片记录失败"), "error");
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
      addToast("图片已不在当前列表中", "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    const ok = await confirmDialog({
      title: "删除图片",
      message: `删除图片 #${requestImage.id}？相关漫画分镜或视频参考图会同步解除关联。`,
      confirmText: "删除",
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
      addToast("图片已不在当前列表中", "error");
      finishAction(actionSeq);
      return;
    }
    setBusyId(requestImage.id);
    try {
      await deleteImage(requestImage.id);
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast("已删除", "success");
      await load({ force: true });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      setActionImage(null);
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "删除失败"), "error");
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
      addToast("图片已不在当前列表中", "error");
      return;
    }
    const score = Number(scoreInput);
    if (!Number.isFinite(score)) {
      addToast("评分必须是数字", "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setScoringId(requestImage.id);
    try {
      const updated = await scoreImage(requestImage.id, score);
      if (!isCurrentAction(requestScope, actionSeq)) return;
      setActionImage((current) => (current?.id === updated.id ? updated : current));
      addToast(`已打分 ${score}`, "success");
      await load({ force: true });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      setActionImage(null);
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "打分失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setScoringId(null);
      finishAction(actionSeq);
    }
  };

  const handleCloseImageActions = () => {
    if (busyId == null && scoringId == null) setActionImage(null);
  };

  if (hasChapterContext && !chapter) {
    return <EmptyHint text="请先选择已生成的章节，再管理本章图像素材。" />;
  }

  return (
    <div className="content-wide flex flex-col gap-4">
      <ImageGalleryPanel
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
        onImportImage={handlePickImportImage}
        onToggleFavorite={(image) => void handleToggleFavorite(image)}
        onOpenActions={setActionImage}
      />
      <input
        ref={importFileInputRef}
        type="file"
        accept={supportedImageAccept()}
        className="hidden"
        onChange={(event) => void handleImportImageFile(event)}
      />

      <ImageGenerationDialog
        open={canEditImages && toolsOpen}
        chapter={chapter}
        genPrompt={genPrompt}
        genBusy={genBusy}
        objectProfiles={objectProfiles}
        selectedObjectProfileId={selectedObjectProfileId}
        objectImageStyle={objectImageStyle}
        objectImageBusy={objectImageBusy}
        objectPromptCopying={objectPromptCopying}
        chapterGenerateLocked={chapterGenerateLocked}
        onClose={() => setToolsOpen(false)}
        onGenPromptChange={setGenPrompt}
        onSelectedObjectProfileIdChange={setSelectedObjectProfileId}
        onObjectImageStyleChange={setObjectImageStyle}
        onCopyObjectImagePrompt={() => void handleCopyObjectImagePrompt()}
        onPreviewGenerate={() => void handlePreviewGenerate()}
        onGenerate={() => void handleGenerate()}
        onPreviewObjectImage={() => void handlePreviewObjectImage()}
        onGenerateObjectImage={() => void handleGenerateObjectImage()}
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
        fallbackTitle="图片请求预览"
        onClose={closeRequestPreview}
      />
    </div>
  );
};
