import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import type { CharacterProfile } from "../../../api/character";
import { uploadPortrait, type Portrait, type PortraitType } from "../../../api/portrait";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput, BookTextarea } from "../../../components/ui/BookInput";
import { ImageZoomButton } from "../../../components/ui/ImageLightbox";
import { Modal } from "../../../components/ui/Modal";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import { MAX_UPLOAD_IMAGE_BYTES } from "../../../api/generated/config";
import {
  isSupportedImageMime,
  supportedImageAccept,
} from "../../../utils/imageUpload";
import { useI18n, type MessageTranslator } from "../../../i18n";
import { useNovelDetailDirty } from "../novelDetailDirty";

interface UploadPortraitDialogProps {
  projectId: number;
  profiles: CharacterProfile[];
  onClose: () => void;
  onUploaded: (portrait: Portrait) => void | Promise<void>;
}

const PORTRAIT_TYPE_OPTIONS: ReadonlyArray<{ value: PortraitType }> = [
  { value: "full_body" },
  { value: "half_body" },
  { value: "face" },
  { value: "chibi" },
];

export const UploadPortraitDialog: React.FC<UploadPortraitDialogProps> = ({
  projectId,
  profiles,
  onClose,
  onUploaded,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const [profileId, setProfileId] = useState(profiles[0]?.id ?? 0);
  const [portraitType, setPortraitType] = useState<PortraitType>("full_body");
  const [prompt, setPrompt] = useState("");
  const [expression, setExpression] = useState("");
  const [pose, setPose] = useState("");
  const [setPrimary, setSetPrimary] = useState(true);
  const [file, setFile] = useState<File | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const activeProjectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  const submitBusyRef = useRef(false);
  const promptEditedRef = useRef(false);

  activeProjectIdRef.current = projectId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      submitBusyRef.current = false;
    };
  }, []);

  const isCurrentProject = useCallback((requestProjectId: number) => {
    return mountedRef.current && activeProjectIdRef.current === requestProjectId;
  }, []);

  const selectedProfile = useMemo(
    () => profiles.find((profile) => profile.id === profileId) ?? null,
    [profileId, profiles],
  );
  const draftDirty = profileId !== (profiles[0]?.id ?? 0) ||
    portraitType !== "full_body" ||
    prompt.trim().length > 0 ||
    expression.trim().length > 0 ||
    pose.trim().length > 0 ||
    setPrimary !== true ||
    file != null;

  useNovelDetailDirty(
    `portrait-upload:${projectId}`,
    draftDirty || busy,
    onClose,
    () => !busy,
  );

  const canSubmit =
    selectedProfile != null && file != null && prompt.trim().length > 0;

  useEffect(() => {
    setProfileId((current) => {
      if (profiles.some((profile) => profile.id === current)) return current;
      return profiles[0]?.id ?? 0;
    });
  }, [profiles]);

  useEffect(() => {
    return () => {
      if (previewUrl) URL.revokeObjectURL(previewUrl);
    };
  }, [previewUrl]);

  const updateSuggestedPrompt = (profileName: string, fileName: string) => {
    if (promptEditedRef.current) return;
    setPrompt(t("characterDetail.portrait.uploadPromptDefault", { name: profileName, fileName }));
  };

  const handleFileChange = async (nextFile: File | null) => {
    if (!nextFile) return;
    if (!isSupportedImageMime(nextFile.type)) {
      addToast(t("characterDetail.portrait.invalidImage"), "error");
      return;
    }
    if (nextFile.size > MAX_UPLOAD_IMAGE_BYTES) {
      addToast(t("characterDetail.portrait.fileTooLarge", {
        size: MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024,
      }), "error");
      return;
    }
    setFile(nextFile);
    setPreviewUrl((current) => {
      if (current) URL.revokeObjectURL(current);
      return URL.createObjectURL(nextFile);
    });
    updateSuggestedPrompt(selectedProfile?.name ?? "角色", nextFile.name);
  };

  const handleSubmit = async () => {
    if (submitBusyRef.current) return;
    if (!selectedProfile) {
      addToast(t("characterDetail.portrait.profileRequired"), "error");
      return;
    }
    if (!file) {
      addToast(t("characterDetail.portrait.imageRequired"), "error");
      return;
    }
    if (!prompt.trim()) {
      addToast(t("characterDetail.portrait.descriptionRequired"), "error");
      return;
    }
    const requestProjectId = projectId;
    const requestProfile = selectedProfile;
    const requestFile = file;
    submitBusyRef.current = true;
    setBusy(true);
    try {
      const dataBase64 = await readFileAsBase64(requestFile);
      if (!isCurrentProject(requestProjectId)) return;
      const portrait = await uploadPortrait({
        project_id: requestProjectId,
        profile_id: requestProfile.id,
        file_name: requestFile.name,
        mime_type: requestFile.type,
        data_base64: dataBase64,
        portrait_type: portraitType,
        prompt: prompt.trim(),
        expression: expression.trim() || null,
        pose: pose.trim() || null,
        set_primary: setPrimary,
      });
      if (!isCurrentProject(requestProjectId)) return;
      await onUploaded(portrait);
      addToast(t("characterDetail.portrait.uploaded"), "success");
    } catch (err) {
      if (!isCurrentProject(requestProjectId)) return;
      addToast(extractErrorMessage(err, t("characterDetail.portrait.uploadFailed")), "error");
    } finally {
      if (isCurrentProject(requestProjectId)) {
        submitBusyRef.current = false;
        setBusy(false);
      }
    }
  };

  const requestClose = async () => {
    if (busy) return;
    if (draftDirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: t("characterDetail.portrait.uploadTitle"),
        action: t("common.close"),
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  return (
    <Modal
      isOpen
      onClose={() => void requestClose()}
      title={t("characterDetail.portrait.uploadTitle")}
      icon={<Upload size={15} />}
      maxWidthClassName="max-w-2xl"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
      footer={
        <BookButton
          variant="primary"
          size="sm"
          onClick={handleSubmit}
          disabled={busy || !canSubmit}
          className="w-full !text-xs sm:ml-auto sm:w-auto"
        >
          {busy ? (
            <span className="inline-flex items-center gap-1.5">
              <LoadingSpinner size="sm" />
              {t("characterDetail.portrait.uploading")}
            </span>
          ) : (
            t("characterDetail.portrait.uploadAction")
          )}
        </BookButton>
      }
    >
      <div className="space-y-3 text-sm">
        {profiles.length === 0 ? (
          <div className="rounded-[12px] border border-book-border/50 bg-book-bg-paper/45 px-3 py-3 text-xs leading-relaxed text-book-text-muted">
            {t("characterDetail.portrait.noProfilesHint")}
          </div>
        ) : null}

        <div>
          <label className="mb-1 block text-xs text-book-text-muted">{t("characterDetail.portrait.profile")}</label>
          <ThemedSelect
            aria-label={t("characterDetail.portrait.profile")}
            value={profileId}
            onChange={(selectedValue) => {
              const nextProfileId = Number(selectedValue);
              setProfileId(nextProfileId);
              const nextProfile = profiles.find((profile) => profile.id === nextProfileId);
              if (file) updateSuggestedPrompt(nextProfile?.name ?? "角色", file.name);
            }}
            className="pyqt-input w-full"
            disabled={busy || profiles.length === 0}
          >
            {profiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name}
              </option>
            ))}
          </ThemedSelect>
        </div>

        <div>
          <label className="mb-1 block text-xs text-book-text-muted">{t("characterDetail.portrait.type")}</label>
          <ThemedSelect
            aria-label={t("characterDetail.portrait.type")}
            value={portraitType}
            onChange={(selectedValue) => setPortraitType(selectedValue as PortraitType)}
            className="pyqt-input w-full"
            disabled={busy}
          >
            {PORTRAIT_TYPE_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {portraitTypeLabel(option.value, t)}
              </option>
            ))}
          </ThemedSelect>
        </div>

        <div className="relative">
          <label
            className={[
              "flex min-h-[168px] cursor-pointer flex-col items-center justify-center gap-2 rounded-[14px] border border-dashed border-book-border/70 bg-book-bg-paper/35 p-3 text-center text-xs text-book-text-muted transition-[border-color,background-color,transform] duration-150 ease-out hover:border-book-primary/55 hover:bg-book-bg-paper/60 focus-within:outline focus-within:outline-2 focus-within:outline-book-primary active:scale-[0.995]",
              busy ? "pointer-events-none opacity-60" : "",
            ].join(" ")}
          >
            {previewUrl ? (
              <img
                src={previewUrl}
                alt=""
                className="max-h-56 max-w-full rounded-[12px] border border-book-border/50 object-contain"
              />
            ) : null}
            <span className="inline-flex items-center gap-1.5 text-book-primary">
              <Upload size={16} />
              {t(file ? "characterDetail.portrait.changeImage" : "characterDetail.portrait.chooseImage")}
            </span>
            {file ? (
              <span className="max-w-full truncate text-small text-book-text-sub">
                {t("characterDetail.portrait.selectedFile", { name: file.name })}
              </span>
            ) : null}
            <input
              type="file"
              aria-label={t(file ? "characterDetail.portrait.changeImage" : "characterDetail.portrait.chooseImage")}
              accept={supportedImageAccept()}
              className="sr-only"
              disabled={busy}
              onChange={(event) => {
                const nextFile = event.currentTarget.files?.[0] ?? null;
                event.currentTarget.value = "";
                void handleFileChange(nextFile);
              }}
            />
          </label>
          {previewUrl ? <ImageZoomButton src={previewUrl} alt={selectedProfile?.name ?? ""} className="absolute right-3 top-3" /> : null}
        </div>

        <BookTextarea
          label={t("characterDetail.portrait.promptLabel")}
          value={prompt}
          rows={3}
          disabled={busy}
          onChange={(event) => {
            promptEditedRef.current = true;
            setPrompt(event.target.value);
          }}
          placeholder={t("characterDetail.portrait.promptPlaceholder")}
        />

        <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
          <BookInput
            label={t("characterDetail.portrait.expressionLabel")}
            value={expression}
            disabled={busy}
            onChange={(event) => setExpression(event.target.value)}
            placeholder={t("characterDetail.portrait.expressionPlaceholder")}
          />
          <BookInput
            label={t("characterDetail.portrait.poseLabel")}
            value={pose}
            disabled={busy}
            onChange={(event) => setPose(event.target.value)}
            placeholder={t("characterDetail.portrait.posePlaceholder")}
          />
        </div>

        <button
          type="button"
          role="switch"
          aria-checked={setPrimary}
          disabled={busy}
          onClick={() => setSetPrimary((current) => !current)}
          className="flex w-full items-center justify-between gap-3 rounded-[12px] border border-book-border/50 bg-book-bg-paper/35 px-3 py-2.5 text-left transition-colors hover:bg-book-bg-paper/60 focus-visible:outline-2 focus-visible:outline-book-primary/35 disabled:cursor-not-allowed disabled:opacity-60"
        >
          <span className="min-w-0">
            <span className="block text-xs font-medium text-book-text-main">{t("characterDetail.portrait.primaryTitle")}</span>
            <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
              {t("characterDetail.portrait.primaryDescription")}
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
      </div>
    </Modal>
  );
};

function portraitTypeLabel(type: PortraitType, t: MessageTranslator): string {
  switch (type) {
    case "full_body":
      return t("characterDetail.portrait.typeFullBody");
    case "half_body":
      return t("characterDetail.portrait.typeHalfBody");
    case "face":
      return t("characterDetail.portrait.typeFace");
    case "chibi":
      return t("characterDetail.portrait.typeChibi");
  }
}
