import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { extractErrorMessage } from "../../../api";
import {
  deleteImage,
  updateImage,
  type GeneratedImage,
  type UpdateImageInput,
} from "../../../api/image";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import type { ToastType } from "../../../components/feedback/Toast";

type AddToast = (message: unknown, type: ToastType) => void;

interface UseVideoImageActionsParams {
  addToast: AddToast;
  images: GeneratedImage[];
  actionScopeKey: string | null;
  setImages: Dispatch<SetStateAction<GeneratedImage[]>>;
  loadAll: () => Promise<void>;
}

export const useVideoImageActions = ({
  addToast,
  images,
  actionScopeKey,
  setImages,
  loadAll,
}: UseVideoImageActionsParams) => {
  const [imageDetails, setImageDetails] = useState<GeneratedImage | null>(null);
  const [imageBusyId, setImageBusyId] = useState<number | null>(null);
  const [imageScoringId, setImageScoringId] = useState<number | null>(null);
  const [imageScoreInput, setImageScoreInput] = useState("0.8");
  const actionScopeRef = useRef(actionScopeKey);

  useEffect(() => {
    actionScopeRef.current = actionScopeKey;
    setImageDetails(null);
    setImageBusyId(null);
    setImageScoringId(null);
  }, [actionScopeKey]);

  useEffect(() => {
    setImageDetails((current) =>
      current ? images.find((image) => image.id === current.id) ?? null : null,
    );
  }, [images]);

  const isCurrentActionScope = useCallback(
    (scope: string | null) => actionScopeRef.current === scope,
    [],
  );

  const handleToggleImageFavorite = useCallback(
    async (image: GeneratedImage) => {
      const requestScope = actionScopeKey;
      setImageBusyId(image.id);
      try {
        const updated = await updateImage(image.id, { is_favorite: !image.is_favorite });
        if (!isCurrentActionScope(requestScope)) return;
        setImages((prev) => prev.map((item) => (item.id === updated.id ? updated : item)));
        setImageDetails((current) => (current?.id === updated.id ? updated : current));
        addToast(updated.is_favorite ? "已收藏" : "已取消收藏", "success");
      } catch (err) {
        if (!isCurrentActionScope(requestScope)) return;
        addToast(extractErrorMessage(err, "切换素材收藏失败"), "error");
      } finally {
        if (isCurrentActionScope(requestScope)) setImageBusyId(null);
      }
    },
    [actionScopeKey, addToast, isCurrentActionScope, setImages],
  );

  const handleOpenImageDetails = useCallback((image: GeneratedImage) => {
    setImageScoreInput(image.ai_score == null ? "0.8" : String(image.ai_score));
    setImageDetails(image);
  }, []);

  const handleCloseImageDetails = useCallback(() => {
    if (imageBusyId == null && imageScoringId == null) {
      setImageDetails(null);
    }
  }, [imageBusyId, imageScoringId]);

  const handleScoreImage = useCallback(
    async (image: GeneratedImage) => {
      const requestScope = actionScopeKey;
      const score = Number(imageScoreInput);
      if (!Number.isFinite(score)) {
        addToast("评分必须是数字", "error");
        return;
      }
      setImageScoringId(image.id);
      try {
        const updated = await updateImage(image.id, { ai_score: score });
        if (!isCurrentActionScope(requestScope)) return;
        setImages((prev) => prev.map((item) => (item.id === updated.id ? updated : item)));
        setImageDetails((current) => (current?.id === updated.id ? updated : current));
        addToast(`已打分 ${score}`, "success");
      } catch (err) {
        if (!isCurrentActionScope(requestScope)) return;
        addToast(extractErrorMessage(err, "打分失败"), "error");
      } finally {
        if (isCurrentActionScope(requestScope)) setImageScoringId(null);
      }
    },
    [actionScopeKey, addToast, imageScoreInput, isCurrentActionScope, setImages],
  );

  const handleUpdateImage = useCallback(
    async (image: GeneratedImage, input: UpdateImageInput) => {
      const requestScope = actionScopeKey;
      setImageBusyId(image.id);
      try {
        const updated = await updateImage(image.id, input);
        if (!isCurrentActionScope(requestScope)) return;
        setImages((prev) => prev.map((item) => (item.id === updated.id ? updated : item)));
        setImageDetails((current) => (current?.id === updated.id ? updated : current));
        addToast("图片记录已更新", "success");
      } catch (err) {
        if (!isCurrentActionScope(requestScope)) return;
        addToast(extractErrorMessage(err, "更新图片记录失败"), "error");
        throw err;
      } finally {
        if (isCurrentActionScope(requestScope)) setImageBusyId(null);
      }
    },
    [actionScopeKey, addToast, isCurrentActionScope, setImages],
  );

  const handleDeleteImage = useCallback(
    async (image: GeneratedImage) => {
      const requestScope = actionScopeKey;
      const ok = await confirmDialog({
        title: "删除图片",
        message: `删除图片 #${image.id}？相关视频参考图会同步解除关联。`,
        confirmText: "删除",
        dialogType: "danger",
      });
      if (!ok) return;
      if (!isCurrentActionScope(requestScope)) return;
      setImageBusyId(image.id);
      try {
        await deleteImage(image.id);
        if (!isCurrentActionScope(requestScope)) return;
        await loadAll();
        if (!isCurrentActionScope(requestScope)) return;
        setImageDetails(null);
        addToast("已删除", "success");
      } catch (err) {
        if (!isCurrentActionScope(requestScope)) return;
        addToast(extractErrorMessage(err, "删除失败"), "error");
      } finally {
        if (isCurrentActionScope(requestScope)) setImageBusyId(null);
      }
    },
    [
      actionScopeKey,
      addToast,
      isCurrentActionScope,
      loadAll,
    ],
  );

  return {
    imageBusyId,
    imageDetails,
    imageScoreInput,
    imageScoringId,
    setImageScoreInput,
    handleCloseImageDetails,
    handleDeleteImage,
    handleOpenImageDetails,
    handleScoreImage,
    handleToggleImageFavorite,
    handleUpdateImage,
  };
};
