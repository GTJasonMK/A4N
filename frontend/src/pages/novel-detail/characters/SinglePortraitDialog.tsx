import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { Check, Eye, Images, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ImageSizeSelect, useImageGenerationSize } from "../../../components/image/ImageSizeSelect";
import { extractErrorMessage } from "../../../api";
import type { CharacterProfile } from "../../../api/character";
import { listImagesByProject, type GeneratedImage, uploadImage } from "../../../api/image";
import {
  previewGeneratePortraitRequest,
  startPortraitGeneration,
  verifyPortraitGenerationResult,
} from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import { useProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { nonEmptyText } from "../../../utils/text";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import { StyleTemplatePicker } from "../StyleTemplatePicker";
import { imageTypeLabel } from "../images/imageTabUtils";
import { DEFAULT_IMAGE_STYLE, VISUAL_STYLE_TEMPLATES } from "../styleTemplates";
import { firstProfileName } from "./characterUtils";
import {
  isSupportedImageMime,
  MAX_UPLOAD_IMAGE_BYTES,
  supportedImageAccept,
} from "../../../utils/imageUpload";
import { useI18n } from "../../../i18n";

const MAX_REFERENCE_IMAGES = 4;

interface SinglePortraitDialogProps {
  projectId: number;
  profiles: CharacterProfile[];
  onClose: () => void;
  onCreated: () => void;
}

export const SinglePortraitDialog: React.FC<SinglePortraitDialogProps> = ({
  projectId,
  profiles,
  onClose,
  onCreated,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const { runProgressTask } = useProgressTask();
  const {
    error: imageSizeError,
    loading: imageSizeLoading,
    size: imageSize,
    sizes: imageSizes,
    setSize: setImageSize,
  } = useImageGenerationSize();
  const [name, setName] = useState(firstProfileName(profiles));
  const [style, setStyle] = useState(DEFAULT_IMAGE_STYLE);
  const [expressionInput, setExpressionInput] = useState("");
  const [referenceImages, setReferenceImages] = useState<GeneratedImage[]>([]);
  const [selectedReferenceImageIds, setSelectedReferenceImageIds] = useState<number[]>([]);
  const [referenceLoading, setReferenceLoading] = useState(false);
  const [uploadingReference, setUploadingReference] = useState(false);
  const [busy, setBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const activeProjectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  const referenceLoadSeqRef = useRef(0);
  const requestPreviewSeqRef = useRef(0);
  const uploadReferenceBusyRef = useRef(false);
  const submitBusyRef = useRef(false);
  const previewBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      uploadReferenceBusyRef.current = false;
      submitBusyRef.current = false;
      previewBusyRef.current = false;
      requestPreviewSeqRef.current += 1;
    };
  }, []);

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  useEffect(() => {
    const requestProjectId = projectId;
    const requestSeq = referenceLoadSeqRef.current + 1;
    referenceLoadSeqRef.current = requestSeq;
    setReferenceImages([]);
    setSelectedReferenceImageIds([]);
    setReferenceLoading(true);
    listImagesByProject(requestProjectId)
      .then((images) => {
        if (
          isCurrentProject(requestProjectId) &&
          referenceLoadSeqRef.current === requestSeq
        ) {
          setReferenceImages(images);
        }
      })
      .catch((err) => {
        if (
          isCurrentProject(requestProjectId) &&
          referenceLoadSeqRef.current === requestSeq
        ) {
          addToast(extractErrorMessage(err, t("characterDetail.portrait.referenceLoadFailed")), "error");
        }
      })
      .finally(() => {
        if (
          isCurrentProject(requestProjectId) &&
          referenceLoadSeqRef.current === requestSeq
        ) {
          setReferenceLoading(false);
        }
      });
  }, [addToast, isCurrentProject, projectId, t]);

  useEffect(() => {
    if (profiles.length === 0) return;
    setName((current) =>
      profiles.some((profile) => profile.name === current)
        ? current
        : firstProfileName(profiles),
    );
  }, [profiles]);

  const toggleReferenceImage = (imageId: number) => {
    setSelectedReferenceImageIds((current) => {
      if (current.includes(imageId)) {
        return current.filter((id) => id !== imageId);
      }
      if (current.length >= MAX_REFERENCE_IMAGES) {
        addToast(t("characterDetail.portrait.maxReferences", { count: MAX_REFERENCE_IMAGES }), "error");
        return current;
      }
      return [...current, imageId];
    });
  };

  const addUploadedReferenceImage = (image: GeneratedImage) => {
    setReferenceImages((current) => [image, ...current.filter((item) => item.id !== image.id)]);
    setSelectedReferenceImageIds((current) => {
      if (current.includes(image.id)) return current;
      const next = [...current, image.id];
      return next.slice(Math.max(0, next.length - MAX_REFERENCE_IMAGES));
    });
  };

  const handleUploadReferenceImage = async (file: File) => {
    if (uploadReferenceBusyRef.current) return;
    const requestProjectId = projectId;
    if (!isSupportedImageMime(file.type)) {
      addToast(t("characterDetail.portrait.invalidImage"), "error");
      return;
    }
    if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
      addToast(t("characterDetail.portrait.fileTooLarge", {
        size: MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024,
      }), "error");
      return;
    }
    uploadReferenceBusyRef.current = true;
    setUploadingReference(true);
    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentProject(requestProjectId)) return;
      const image = await uploadImage(requestProjectId, {
        file_name: file.name,
        mime_type: file.type,
        data_base64: dataBase64,
        image_type: "portrait_reference",
        prompt: `人物三视图拆解参考图：${file.name}`,
        metadata_json: {
          source: "portrait_reference_upload",
          manual_upload: true,
        },
      });
      if (!isCurrentProject(requestProjectId)) return;
      addUploadedReferenceImage(image);
      addToast(t("characterDetail.portrait.referenceUploaded"), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      addToast(extractErrorMessage(err, t("characterDetail.portrait.uploadFailed")), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        uploadReferenceBusyRef.current = false;
        setUploadingReference(false);
      }
    }
  };

  const handleSubmit = async () => {
    if (submitBusyRef.current) return;
    if (!name.trim()) {
      addToast(t("characterDetail.portrait.nameRequired"), "error");
      return;
    }
    const requestProjectId = projectId;
    const requestName = name.trim();
    const requestStyle = nonEmptyText(style);
    const requestExpression = nonEmptyText(expressionInput);
    const requestReferenceImageIds = selectedReferenceImageIds;
    const requestSize = imageSize;
    if (!requestSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    submitBusyRef.current = true;
    setBusy(true);
    try {
      const handle = await startPortraitGeneration({
        project_id: requestProjectId,
        operation: {
          kind: "generate",
          character_name: requestName,
          style: requestStyle,
          expression: requestExpression,
          pose: null,
          reference_image_ids: requestReferenceImageIds,
          width: requestSize.width,
          height: requestSize.height,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("characterDetail.portrait.generationTitle"),
          startMessage: t("characterDetail.portrait.generationStart"),
          successMessage: t("characterDetail.portrait.generationSuccess"),
          errorMessage: t("characterDetail.portrait.generationFailed"),
          cancelLabel: t("characterDetail.portrait.stopGeneration"),
        },
        async (task) => {
          await verifyPortraitGenerationResult(task, requestProjectId);
        },
      );
      if (!isCurrentProject(requestProjectId)) return;
      addToast(t("characterDetail.portrait.generated", { name: requestName }), "success");
      onCreated();
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      const message = extractErrorMessage(err, t("characterDetail.portrait.generationFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        submitBusyRef.current = false;
        setBusy(false);
      }
    }
  };

  const handlePreview = async () => {
    if (previewBusyRef.current) return;
    if (!name.trim()) {
      addToast(t("characterDetail.portrait.nameRequired"), "error");
      return;
    }
    const requestProjectId = projectId;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    const requestName = name.trim();
    const requestStyle = nonEmptyText(style);
    const requestExpression = nonEmptyText(expressionInput);
    const requestReferenceImageIds = selectedReferenceImageIds;
    const requestSize = imageSize;
    if (!requestSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    previewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGeneratePortraitRequest({
        project_id: requestProjectId,
        character_name: requestName,
        style: requestStyle,
        expression: requestExpression,
        pose: null,
        reference_image_ids: requestReferenceImageIds,
        width: requestSize.width,
        height: requestSize.height,
      });
      if (
        !isCurrentProject(requestProjectId) ||
        requestPreviewSeqRef.current !== requestSeq
      ) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (
        !isCurrentProject(requestProjectId) ||
        requestPreviewSeqRef.current !== requestSeq
      ) {
        return;
      }
      addToast(extractErrorMessage(err, t("characterDetail.portrait.generationPreviewFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (
        isCurrentProject(requestProjectId) &&
        requestPreviewSeqRef.current === requestSeq
      ) {
        previewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  };

  const canSubmit = Boolean(name.trim());

  return (
    <>
      <Modal
        isOpen
        onClose={onClose}
        title={t("characterDetail.portrait.generationTitle")}
        subtitle={t("characterDetail.portrait.generationSubtitle")}
        icon={<Images size={15} />}
        maxWidthClassName="max-w-3xl"
        closeOnBackdrop={!busy}
        closeOnEscape={!busy}
        showCloseButton={!busy}
        footer={
          <div className="flex w-full items-center gap-2">
            <Dropdown
              items={[
                {
                  label: t("characterDetail.portrait.viewRequest"),
                  icon: <Eye size={12} />,
                  onClick: handlePreview,
                  disabled: busy || requestPreviewLoading || !canSubmit,
                },
              ]}
            />
            <span className="min-w-0 flex-1" />
            <BookButton
              variant="primary"
              size="sm"
              onClick={handleSubmit}
              disabled={busy || !canSubmit || imageSize == null}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {busy ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> {t("characterDetail.portrait.generating")}
                </span>
              ) : (
                t("characterDetail.portrait.generateAction")
              )}
            </BookButton>
          </div>
        }
      >
        <div className="space-y-3 text-sm">
          <div>
            <label className="mb-1 block text-xs text-book-text-muted">{t("characterDetail.name")}</label>
            {profiles.length > 0 ? (
              <ThemedSelect
                value={name}
                onChange={setName}
                className="pyqt-input w-full"
              >
                {profiles.map((profile) => (
                  <option key={profile.id} value={profile.name}>
                    {profile.name}
                  </option>
                ))}
              </ThemedSelect>
            ) : (
              <BookInput value={name} onChange={(event) => setName(event.target.value)} />
            )}
          </div>
          <div>
            <ImageSizeSelect
              label={t("settingsForm.selectImageSize")}
              value={imageSize}
              sizes={imageSizes}
              loading={imageSizeLoading}
              error={imageSizeError}
              disabled={busy}
              onChange={setImageSize}
            />
          </div>
          <div>
            <StyleTemplatePicker
              label={t("characterDetail.portrait.style")}
              value={style}
              templates={VISUAL_STYLE_TEMPLATES}
              disabled={busy}
              rows={4}
              onChange={setStyle}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-book-text-muted">
              {t("characterDetail.portrait.expressionOptional")}
            </label>
            <BookInput
              value={expressionInput}
              onChange={(event) => setExpressionInput(event.target.value)}
              placeholder={t("characterDetail.portrait.expressionExample")}
            />
          </div>
          <section className="border-t border-book-border/45 pt-4">
            <header className="mb-3 flex items-center justify-between gap-3">
              <div className="min-w-0">
                <h3 className="text-sm font-semibold text-book-text-main">{t("characterDetail.portrait.references")}</h3>
                <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
                  {selectedReferenceImageIds.length > 0
                    ? t("characterDetail.portrait.selectedReferences", {
                        selected: selectedReferenceImageIds.length,
                        max: MAX_REFERENCE_IMAGES,
                      })
                    : t("characterDetail.portrait.optionalReferences", {
                        max: MAX_REFERENCE_IMAGES,
                      })}
                </p>
              </div>
              <label
                className={[
                  "pyqt-btn-secondary inline-flex min-h-8 shrink-0 cursor-pointer items-center gap-1.5 px-2.5 text-xs",
                  busy || uploadingReference ? "pointer-events-none opacity-60" : "",
                ].join(" ")}
              >
                {uploadingReference ? (
                  <LoadingSpinner size="sm" />
                ) : (
                  <Upload size={12} />
                )}
                {uploadingReference ? t("characterDetail.portrait.uploading") : t("characterDetail.portrait.upload")}
                <input
                  type="file"
                  accept={supportedImageAccept()}
                  className="hidden"
                  disabled={busy || uploadingReference}
                  onChange={(event) => {
                    const file = event.currentTarget.files?.[0] ?? null;
                    event.currentTarget.value = "";
                    if (file) void handleUploadReferenceImage(file);
                  }}
                />
              </label>
            </header>
            {referenceLoading ? (
              <div className="flex min-h-[96px] items-center justify-center text-xs text-book-text-muted">
                <LoadingSpinner size="sm" className="mr-1.5" />
                {t("characterDetail.portrait.loadingImages")}
              </div>
            ) : referenceImages.length === 0 ? (
              <div className="rounded-[12px] border border-dashed border-book-border/55 bg-book-bg-paper/35 px-3 py-3 text-xs leading-relaxed text-book-text-muted">
                {t("characterDetail.portrait.noImages")}
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 md:grid-cols-5">
                {referenceImages.map((image) => {
                  const selected = selectedReferenceImageIds.includes(image.id);
                  const src = toMediaSrc(image.file_path);
                  return (
                    <button
                      key={image.id}
                      type="button"
                      onClick={() => toggleReferenceImage(image.id)}
                      disabled={busy || uploadingReference}
                      data-active={selected ? "1" : undefined}
                      aria-pressed={selected}
                      className={[
                        "group min-w-0 overflow-hidden rounded-[12px] border bg-book-bg-paper/45 text-left transition-[border-color,background-color,box-shadow,transform] duration-150 ease-out active:scale-[0.98] focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary/40 disabled:opacity-60",
                        selected
                          ? "border-book-primary/75 bg-book-primary/5 shadow-[0_0_0_1px_rgb(var(--color-primary)/0.18)]"
                          : "border-book-border/50 hover:border-book-primary/45 hover:bg-book-bg-paper/75",
                      ].join(" ")}
                      title={selected
                        ? t("characterDetail.portrait.deselectReference")
                        : t("characterDetail.portrait.selectReference")}
                    >
                      <span className="relative flex aspect-[4/3] items-center justify-center overflow-hidden bg-book-bg-tertiary text-caption text-book-text-muted">
                        {src ? (
                          <img src={src} alt="" className="h-full w-full object-cover" />
                        ) : (
                          <span>{t("characterDetail.portrait.noImage")}</span>
                        )}
                        {selected && (
                          <span className="absolute right-1.5 top-1.5 inline-flex h-5 w-5 items-center justify-center rounded-full border border-white/65 bg-book-primary text-white shadow-sm">
                            <Check size={11} strokeWidth={2.5} />
                          </span>
                        )}
                      </span>
                      <span className={`block truncate px-2 py-1.5 text-caption ${selected ? "text-book-primary" : "text-book-text-muted"}`}>
                        {imageTypeLabel(image.image_type, t)}
                      </span>
                    </button>
                  );
                })}
              </div>
            )}
          </section>
        </div>
      </Modal>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("characterDetail.portrait.requestPreviewTitle")}
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          previewBusyRef.current = false;
          setRequestPreviewOpen(false);
          setRequestPreviewLoading(false);
        }}
      />
    </>
  );
};
