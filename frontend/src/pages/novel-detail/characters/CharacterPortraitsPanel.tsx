import React, { useCallback, useEffect, useRef, useState } from "react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ImageSizeSelect, useImageGenerationSize } from "../../../components/image/ImageSizeSelect";
import { extractErrorMessage } from "../../../api";
import type { ImageSize } from "../../../api/image";
import { listProfiles, type CharacterProfile } from "../../../api/character";
import {
  listPortraitsByProject,
  previewGenerateMissingPortraitsRequest,
  previewRegeneratePortraitRequest,
  setPrimaryPortrait,
  startPortraitGeneration,
  verifyPortraitGenerationResult,
  type Portrait,
} from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import { useProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { useI18n } from "../../../i18n";
import { AgeVariantDialog } from "./AgeVariantDialog";
import { PortraitActionDialog } from "./PortraitActionDialog";
import { PortraitGalleryGrid } from "./PortraitGalleryGrid";
import { PortraitPromptDialog } from "./PortraitPromptDialog";
import { PortraitToolbar } from "./PortraitToolbar";
import { SinglePortraitDialog } from "./SinglePortraitDialog";
import { UploadPortraitDialog } from "./UploadPortraitDialog";

type PortraitGenerationAction = "generate" | "preview";

export const CharacterPortraitsPanel: React.FC<{ projectId: number }> = ({
  projectId,
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
  const [portraits, setPortraits] = useState<Portrait[]>([]);
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [showSingle, setShowSingle] = useState(false);
  const [showUpload, setShowUpload] = useState(false);
  const [showAge, setShowAge] = useState<Portrait | null>(null);
  const [showPrompt, setShowPrompt] = useState<Portrait | null>(null);
  const [actionPortrait, setActionPortrait] = useState<Portrait | null>(null);
  const [generationSizeAction, setGenerationSizeAction] =
    useState<PortraitGenerationAction | null>(null);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const activeProjectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  const loadSeqRef = useRef(0);
  const requestPreviewSeqRef = useRef(0);
  const generatingRef = useRef(false);
  const portraitActionBusyRef = useRef<number | null>(null);
  const requestPreviewBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      generatingRef.current = false;
      portraitActionBusyRef.current = null;
      requestPreviewBusyRef.current = false;
      requestPreviewSeqRef.current += 1;
    };
  }, []);

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  const isCurrentLoad = useCallback((requestProjectId: number, seq: number) => {
    return isCurrentProject(requestProjectId) && loadSeqRef.current === seq;
  }, [isCurrentProject]);

  const load = useCallback(async (): Promise<boolean> => {
    const requestProjectId = projectId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const [portraitsResult, profilesResult] = await Promise.allSettled([
        listPortraitsByProject(requestProjectId),
        listProfiles(requestProjectId),
      ]);
      if (!isCurrentLoad(requestProjectId, requestSeq)) return false;
      const failures: string[] = [];
      if (portraitsResult.status === "fulfilled") {
        setPortraits(portraitsResult.value);
      } else {
        failures.push(extractErrorMessage(
          portraitsResult.reason,
          t("characterDetail.portrait.missingFailed"),
        ));
      }
      if (profilesResult.status === "fulfilled") {
        setProfiles(profilesResult.value);
        setSelectedProfileId((current) =>
          current != null && profilesResult.value.some((profile) => profile.id === current)
            ? current
            : null,
        );
      } else {
        failures.push(extractErrorMessage(
          profilesResult.reason,
          t("characterDetail.portrait.missingFailed"),
        ));
      }
      if (failures.length > 0) addToast(failures.join("；"), "error");
      return failures.length === 0;
    } finally {
      if (isCurrentLoad(requestProjectId, requestSeq)) {
        setLoadedProjectId(requestProjectId);
        setLoading(false);
      }
    }
  }, [addToast, isCurrentLoad, projectId, t]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    generatingRef.current = false;
    portraitActionBusyRef.current = null;
    requestPreviewBusyRef.current = false;
    setGenerating(false);
    setBusyId(null);
    requestPreviewSeqRef.current += 1;
    setShowSingle(false);
    setShowUpload(false);
    setShowAge(null);
    setShowPrompt(null);
    setActionPortrait(null);
    setGenerationSizeAction(null);
    setSelectedProfileId(null);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [projectId]);

  const beginPortraitAction = (portraitId: number) => {
    if (generatingRef.current || portraitActionBusyRef.current != null) return false;
    portraitActionBusyRef.current = portraitId;
    setBusyId(portraitId);
    return true;
  };

  const finishPortraitAction = (portraitId: number) => {
    if (portraitActionBusyRef.current === portraitId) {
      portraitActionBusyRef.current = null;
    }
    setBusyId((current) => (current === portraitId ? null : current));
  };

  useEffect(() => {
    setActionPortrait((current) =>
      current ? portraits.find((portrait) => portrait.id === current.id) ?? null : null,
    );
    setShowAge((current) =>
      current ? portraits.find((portrait) => portrait.id === current.id) ?? null : null,
    );
    setShowPrompt((current) =>
      current ? portraits.find((portrait) => portrait.id === current.id) ?? null : null,
    );
  }, [portraits]);

  const handleGenerateMissing = async (requestSize: ImageSize) => {
    if (generatingRef.current || portraitActionBusyRef.current != null) return;
    const requestProjectId = projectId;
    generatingRef.current = true;
    setGenerating(true);
    let refreshSucceeded = true;
    try {
      const handle = await startPortraitGeneration({
        project_id: requestProjectId,
        operation: {
          kind: "generate_missing",
          width: requestSize.width,
          height: requestSize.height,
        },
      });
      const createdCount = await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("characterDetail.portrait.batchMissing"),
          startMessage: t("characterDetail.portrait.missingStart"),
          successMessage: t("characterDetail.portrait.missingSuccess"),
          errorMessage: t("characterDetail.portrait.missingFailed"),
          cancelLabel: t("characterDetail.portrait.stopGeneration"),
        },
        async (task) => {
          const generated = await verifyPortraitGenerationResult(task, requestProjectId);
          if (!isCurrentProject(requestProjectId)) throw new Error(t("characterDetail.taskStopped"));
          refreshSucceeded = await load();
          if (!isCurrentProject(requestProjectId)) throw new Error(t("characterDetail.taskStopped"));
          return generated.length;
        },
      );
      if (!isCurrentProject(requestProjectId)) return;
      if (!refreshSucceeded) return;
      addToast(t("characterDetail.portrait.batchGenerated", { count: createdCount }), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      const message = extractErrorMessage(err, t("characterDetail.portrait.missingFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        generatingRef.current = false;
        setGenerating(false);
      }
    }
  };

  const handlePreviewGenerateMissing = async (requestSize: ImageSize) => {
    if (requestPreviewBusyRef.current) return;
    const requestProjectId = projectId;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    requestPreviewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGenerateMissingPortraitsRequest({
        project_id: requestProjectId,
        width: requestSize.width,
        height: requestSize.height,
      });
      if (!isCurrentProject(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentProject(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      addToast(extractErrorMessage(err, t("characterDetail.portrait.missingPreviewFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) {
        requestPreviewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  };

  const handleRegenerate = async (portrait: Portrait, requestSize: ImageSize) => {
    const requestProjectId = projectId;
    if (portrait.project_id !== requestProjectId) return;
    if (!beginPortraitAction(portrait.id)) return;
    try {
      const handle = await startPortraitGeneration({
        project_id: requestProjectId,
        operation: {
          kind: "regenerate",
          portrait_id: portrait.id,
          reference_image_ids: [],
          width: requestSize.width,
          height: requestSize.height,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("characterDetail.portrait.redrawTitle"),
          startMessage: t("characterDetail.portrait.redrawStart"),
          successMessage: t("characterDetail.portrait.redrawSuccess"),
          errorMessage: t("characterDetail.portrait.redrawFailed"),
          cancelLabel: t("characterDetail.portrait.stopGeneration"),
        },
        async (task) => {
          await verifyPortraitGenerationResult(task, requestProjectId);
        },
      );
      if (!isCurrentProject(requestProjectId)) return;
      if (!await load()) return;
      addToast(t("characterDetail.portrait.redrawSuccess"), "success");
      if (!isCurrentProject(requestProjectId)) return;
      setActionPortrait(null);
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      const message = extractErrorMessage(err, t("characterDetail.portrait.redrawFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentProject(requestProjectId)) finishPortraitAction(portrait.id);
    }
  };

  const handlePreviewRegenerate = async (portrait: Portrait, requestSize: ImageSize) => {
    if (requestPreviewBusyRef.current) return;
    const requestProjectId = projectId;
    if (portrait.project_id !== requestProjectId) return;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    requestPreviewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewRegeneratePortraitRequest({
        portrait_id: portrait.id,
        reference_image_ids: [],
        width: requestSize.width,
        height: requestSize.height,
      });
      if (!isCurrentProject(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentProject(requestProjectId) || requestPreviewSeqRef.current !== requestSeq) {
        return;
      }
      addToast(extractErrorMessage(err, t("characterDetail.portrait.redrawPreviewFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentProject(requestProjectId) && requestPreviewSeqRef.current === requestSeq) {
        requestPreviewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  };

  const handleSetPrimary = async (portrait: Portrait) => {
    if (portrait.is_primary) return;
    const requestProjectId = projectId;
    if (portrait.project_id !== requestProjectId) return;
    if (!beginPortraitAction(portrait.id)) return;
    try {
      await setPrimaryPortrait(portrait.id);
      if (!isCurrentProject(requestProjectId)) return;
      if (!await load()) return;
      addToast(t("characterDetail.portrait.primarySet"), "success");
      if (!isCurrentProject(requestProjectId)) return;
      setActionPortrait(null);
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      addToast(extractErrorMessage(err, t("characterDetail.portrait.primarySetFailed")), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) finishPortraitAction(portrait.id);
    }
  };

  const handleCloseActions = () => {
    if (busyId == null) setActionPortrait(null);
  };

  const openGenerationSizeDialog = (action: PortraitGenerationAction) => {
    if (action === "generate" && (generatingRef.current || portraitActionBusyRef.current != null)) return;
    if (action === "preview" && requestPreviewBusyRef.current) return;
    setGenerationSizeAction(action);
  };

  const handleGenerationSizeConfirm = (size: ImageSize) => {
    const action = generationSizeAction;
    setGenerationSizeAction(null);
    if (action === "generate") {
      void handleGenerateMissing(size);
    } else if (action === "preview") {
      void handlePreviewGenerateMissing(size);
    }
  };

  const displayPortraits = loadedProjectId === projectId ? portraits : [];
  const displayProfiles = loadedProjectId === projectId ? profiles : [];
  const displayLoading = loading || loadedProjectId !== projectId;
  const visiblePortraits =
    selectedProfileId == null
      ? displayPortraits
      : displayPortraits.filter((portrait) => portrait.profile_id === selectedProfileId);
  const selectedProfile =
    selectedProfileId == null
      ? null
      : displayProfiles.find((profile) => profile.id === selectedProfileId) ?? null;

  if (displayLoading) {
    return (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" /> {t("characterDetail.portrait.loading")}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <PortraitToolbar
        visiblePortraitCount={visiblePortraits.length}
        totalPortraitCount={displayPortraits.length}
        profiles={displayProfiles}
        selectedProfileId={selectedProfileId}
        generating={generating}
        onProfileChange={setSelectedProfileId}
        onOpenSingle={() => setShowSingle(true)}
        onOpenUpload={() => setShowUpload(true)}
        onPreviewGenerateMissing={() => openGenerationSizeDialog("preview")}
        onGenerateMissing={() => openGenerationSizeDialog("generate")}
      />

      <PortraitGalleryGrid
        portraits={visiblePortraits}
        profiles={displayProfiles}
        busyId={busyId}
        onOpenActions={setActionPortrait}
        emptyText={
          selectedProfile
            ? t("characterDetail.portrait.emptySelected", { name: selectedProfile.name })
            : undefined
        }
      />

      {showSingle ? (
        <SinglePortraitDialog
          projectId={projectId}
          profiles={displayProfiles}
          onClose={() => setShowSingle(false)}
          onCreated={() => {
            if (!isCurrentProject(projectId)) return;
            setShowSingle(false);
            void load();
          }}
        />
      ) : null}

      {showUpload ? (
        <UploadPortraitDialog
          projectId={projectId}
          profiles={displayProfiles}
          onClose={() => setShowUpload(false)}
          onUploaded={() => {
            if (!isCurrentProject(projectId)) return;
            setShowUpload(false);
            void load();
          }}
        />
      ) : null}

      {showAge ? (
        <AgeVariantDialog
          portrait={showAge}
          profileName={displayProfiles.find((profile) => profile.id === showAge.profile_id)?.name}
          onClose={() => setShowAge(null)}
          onCreated={() => {
            if (!isCurrentProject(projectId)) return;
            setShowAge(null);
            void load();
          }}
        />
      ) : null}

      {showPrompt ? (
        <PortraitPromptDialog portrait={showPrompt} profileName={displayProfiles.find((profile) => profile.id === showPrompt.profile_id)?.name} onClose={() => setShowPrompt(null)} />
      ) : null}

      {actionPortrait ? (
        <PortraitActionDialog
          portrait={actionPortrait}
          profileName={displayProfiles.find((profile) => profile.id === actionPortrait.profile_id)?.name}
          busyId={busyId}
          imageSize={imageSize}
          imageSizes={imageSizes}
          imageSizeLoading={imageSizeLoading}
          imageSizeError={imageSizeError}
          onImageSizeChange={setImageSize}
          onClose={handleCloseActions}
          onPreviewRegenerate={handlePreviewRegenerate}
          onRegenerate={handleRegenerate}
          onSetPrimary={handleSetPrimary}
          onAgeVariant={(portrait) => {
            setActionPortrait(null);
            setShowAge(portrait);
          }}
          onPrompt={(portrait) => {
            setActionPortrait(null);
            setShowPrompt(portrait);
          }}
        />
      ) : null}

      {generationSizeAction ? (
        <PortraitGenerationSizeDialog
          action={generationSizeAction}
          imageSize={imageSize}
          imageSizes={imageSizes}
          imageSizeLoading={imageSizeLoading}
          imageSizeError={imageSizeError}
          onImageSizeChange={setImageSize}
          onClose={() => setGenerationSizeAction(null)}
          onConfirm={handleGenerationSizeConfirm}
        />
      ) : null}

      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("characterDetail.portrait.requestPreviewTitle")}
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          requestPreviewBusyRef.current = false;
          setRequestPreviewOpen(false);
          setRequestPreviewLoading(false);
        }}
      />
    </div>
  );
};

interface PortraitGenerationSizeDialogProps {
  action: PortraitGenerationAction;
  imageSize: ImageSize | null;
  imageSizes: readonly ImageSize[];
  imageSizeLoading: boolean;
  imageSizeError: Error | null;
  onImageSizeChange: (size: ImageSize) => void;
  onClose: () => void;
  onConfirm: (size: ImageSize) => void;
}

const PortraitGenerationSizeDialog: React.FC<PortraitGenerationSizeDialogProps> = ({
  action,
  imageSize,
  imageSizes,
  imageSizeLoading,
  imageSizeError,
  onImageSizeChange,
  onClose,
  onConfirm,
}) => {
  const { t } = useI18n();
  const canConfirm = imageSize != null && !imageSizeLoading && imageSizeError == null;
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={action === "preview" ? t("characterDetail.portrait.previewMissing") : t("characterDetail.portrait.batchMissing")}
      subtitle={t("settingsForm.selectImageSize")}
      maxWidthClassName="max-w-sm"
      bodyClassName="px-4 py-4"
      footer={
        <div className="flex w-full justify-end gap-2">
          <BookButton variant="secondary" size="sm" onClick={onClose}>
            {t("common.cancel")}
          </BookButton>
          <BookButton
            variant="primary"
            size="sm"
            disabled={!canConfirm}
            onClick={() => {
              if (imageSize) onConfirm(imageSize);
            }}
          >
            {action === "preview" ? t("characterDetail.portrait.viewRequest") : t("characterDetail.portrait.batchMissing")}
          </BookButton>
        </div>
      }
    >
      <ImageSizeSelect
        label={t("settingsForm.selectImageSize")}
        value={imageSize}
        sizes={imageSizes}
        loading={imageSizeLoading}
        error={imageSizeError}
        onChange={onImageSizeChange}
      />
    </Modal>
  );
};
