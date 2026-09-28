import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ImageSizeSelect, useImageGenerationSize } from "../../../components/image/ImageSizeSelect";
import { extractErrorMessage } from "../../../api";
import {
  previewGenerateAgeVariantRequest,
  startPortraitGeneration,
  verifyPortraitGenerationResult,
  type Portrait,
} from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import { useProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { useI18n } from "../../../i18n";
import { useNovelDetailDirty } from "../novelDetailDirty";

interface AgeVariantDialogProps {
  portrait: Portrait;
  profileName?: string;
  onClose: () => void;
  onCreated: () => boolean | void | Promise<boolean | void>;
}

export const AgeVariantDialog: React.FC<AgeVariantDialogProps> = ({
  portrait,
  profileName,
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
  const [age, setAge] = useState("");
  const [busy, setBusy] = useState(false);
  const draftDirty = age.trim().length > 0;

  useNovelDetailDirty(
    `portrait-age:${portrait.project_id}:${portrait.id}`,
    draftDirty || busy,
    onClose,
    () => !busy,
  );
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const activePortraitScopeRef = useRef(`${portrait.project_id}:${portrait.id}`);
  const mountedRef = useRef(true);
  const requestPreviewSeqRef = useRef(0);
  const submitBusyRef = useRef(false);
  const previewBusyRef = useRef(false);

  activePortraitScopeRef.current = `${portrait.project_id}:${portrait.id}`;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      submitBusyRef.current = false;
      previewBusyRef.current = false;
      requestPreviewSeqRef.current += 1;
    };
  }, []);

  useEffect(() => {
    submitBusyRef.current = false;
    previewBusyRef.current = false;
    requestPreviewSeqRef.current += 1;
    setBusy(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [portrait.id, portrait.project_id]);

  const isCurrentPortrait = useCallback((scope: string) => {
    return mountedRef.current && activePortraitScopeRef.current === scope;
  }, []);

  const requestClose = async () => {
    if (busy) return;
    if (draftDirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: t("characterDetail.portrait.ageTitle"),
        action: t("common.close"),
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  const handleSubmit = async () => {
    if (submitBusyRef.current) return;
    const trimmedAge = age.trim();
    if (!trimmedAge) {
      addToast(t("characterDetail.portrait.ageRequired"), "error");
      return;
    }
    const requestScope = `${portrait.project_id}:${portrait.id}`;
    const requestPortraitId = portrait.id;
    const requestProjectId = portrait.project_id;
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
          kind: "generate_age_variant",
          portrait_id: requestPortraitId,
          age: trimmedAge,
          reference_image_ids: [],
          width: requestSize.width,
          height: requestSize.height,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("characterDetail.portrait.ageTitle"),
          startMessage: t("characterDetail.portrait.ageStart"),
          successMessage: t("characterDetail.portrait.ageSuccess"),
          errorMessage: t("characterDetail.portrait.generationFailed"),
          cancelLabel: t("characterDetail.portrait.stopGeneration"),
        },
        async (task) => {
          await verifyPortraitGenerationResult(task, requestProjectId);
          if (!isCurrentPortrait(requestScope)) {
            throw new Error(t("characterDetail.taskStopped"));
          }
          if (await onCreated() === false) {
            throw new Error(t("characterDetail.portrait.generationFailed"));
          }
        },
      );
      if (!isCurrentPortrait(requestScope)) return;
      addToast(t("characterDetail.portrait.ageGenerated", { age: trimmedAge }), "success");
    } catch (err) {
      if (!isCurrentPortrait(requestScope)) return;
      const message = extractErrorMessage(err, t("characterDetail.portrait.generationFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentPortrait(requestScope)) {
        submitBusyRef.current = false;
        setBusy(false);
      }
    }
  };

  const handlePreview = async () => {
    if (previewBusyRef.current) return;
    const trimmedAge = age.trim();
    if (!trimmedAge) {
      addToast(t("characterDetail.portrait.ageRequired"), "error");
      return;
    }
    const requestScope = `${portrait.project_id}:${portrait.id}`;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    const requestPortraitId = portrait.id;
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
      const preview = await previewGenerateAgeVariantRequest({
        portrait_id: requestPortraitId,
        age: trimmedAge,
        reference_image_ids: [],
        width: requestSize.width,
        height: requestSize.height,
      });
      if (!isCurrentPortrait(requestScope) || requestPreviewSeqRef.current !== requestSeq) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentPortrait(requestScope) || requestPreviewSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("characterDetail.portrait.agePreviewFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentPortrait(requestScope) && requestPreviewSeqRef.current === requestSeq) {
        previewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  };

  return (
    <>
      <Modal
        isOpen
        onClose={() => void requestClose()}
        title={t("characterDetail.portrait.ageTitle")}
        subtitle={profileName}
        maxWidthClassName="max-w-sm"
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
                  disabled: busy || !age.trim() || imageSize == null,
                },
              ]}
            />
            <span className="min-w-0 flex-1" />
            <BookButton
              variant="primary"
              onClick={handleSubmit}
              disabled={busy || !age.trim() || imageSize == null}
            >
              {busy ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> {t("characterDetail.portrait.generating")}
                </span>
              ) : (
                t("characterDetail.portrait.ageGenerate")
              )}
            </BookButton>
          </div>
        }
      >
        <div className="space-y-3 text-sm">
          <div className="text-xs text-book-text-muted">
            {t("characterDetail.portrait.ageDescription")}
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
          <BookInput
            label={t("characterDetail.portrait.ageLabel")}
            value={age}
            onChange={(event) => setAge(event.target.value)}
            placeholder={t("characterDetail.portrait.agePlaceholder")}
            autoFocus
          />
        </div>
      </Modal>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("characterDetail.portrait.ageTitle")}
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
