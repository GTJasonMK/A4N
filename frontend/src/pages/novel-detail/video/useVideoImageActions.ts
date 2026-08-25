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
import type { MessageTranslator } from "../../../i18n";

type AddToast = (message: string, type: ToastType) => void;

interface UseVideoImageActionsParams {
  t: MessageTranslator;
  addToast: AddToast;
  images: GeneratedImage[];
  actionScopeKey: string | null;
  setImages: Dispatch<SetStateAction<GeneratedImage[]>>;
  loadAll: () => Promise<void>;
}

export const useVideoImageActions = ({
  t,
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
        addToast(updated.is_favorite ? t("video.material.favorited") : t("video.material.unfavorited"), "success");
      } catch (err) {
        if (!isCurrentActionScope(requestScope)) return;
        addToast(extractErrorMessage(err, t("video.material.favoriteFailed")), "error");
      } finally {
        if (isCurrentActionScope(requestScope)) setImageBusyId(null);
      }
    },
    [actionScopeKey, addToast, isCurrentActionScope, setImages, t],
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
        addToast(t("video.material.scoreInvalid"), "error");
        return;
      }
      setImageScoringId(image.id);
      try {
        const updated = await updateImage(image.id, { ai_score: score });
        if (!isCurrentActionScope(requestScope)) return;
        setImages((prev) => prev.map((item) => (item.id === updated.id ? updated : item)));
        setImageDetails((current) => (current?.id === updated.id ? updated : current));
        addToast(t("video.material.scored", { score }), "success");
      } catch (err) {
        if (!isCurrentActionScope(requestScope)) return;
        addToast(extractErrorMessage(err, t("video.material.scoreFailed")), "error");
      } finally {
        if (isCurrentActionScope(requestScope)) setImageScoringId(null);
      }
    },
    [actionScopeKey, addToast, imageScoreInput, isCurrentActionScope, setImages, t],
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
        addToast(t("video.material.imageUpdated"), "success");
      } catch (err) {
        if (!isCurrentActionScope(requestScope)) return;
        addToast(extractErrorMessage(err, t("video.material.updateImageFailed")), "error");
        throw err;
      } finally {
        if (isCurrentActionScope(requestScope)) setImageBusyId(null);
      }
    },
    [actionScopeKey, addToast, isCurrentActionScope, setImages, t],
  );

  const handleDeleteImage = useCallback(
    async (image: GeneratedImage) => {
      const requestScope = actionScopeKey;
      const ok = await confirmDialog({
        title: t("video.material.deleteTitle"),
        message: t("video.material.deleteMessage"),
        confirmText: t("common.delete"),
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
        addToast(t("video.material.deleted"), "success");
      } catch (err) {
        if (!isCurrentActionScope(requestScope)) return;
        addToast(extractErrorMessage(err, t("video.material.deleteFailed")), "error");
      } finally {
        if (isCurrentActionScope(requestScope)) setImageBusyId(null);
      }
    },
    [
      actionScopeKey,
      addToast,
      isCurrentActionScope,
      loadAll,
      t,
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
