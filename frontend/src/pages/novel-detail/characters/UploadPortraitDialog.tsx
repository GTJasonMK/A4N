import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import type { CharacterProfile } from "../../../api/character";
import { uploadPortrait, type PortraitType } from "../../../api/portrait";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput, BookTextarea } from "../../../components/ui/BookInput";
import { Modal } from "../../../components/ui/Modal";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import {
  isSupportedImageMime,
  MAX_UPLOAD_IMAGE_BYTES,
  supportedImageAccept,
} from "../../../utils/imageUpload";
import { useI18n, type MessageTranslator } from "../../../i18n";

interface UploadPortraitDialogProps {
  projectId: number;
  profiles: CharacterProfile[];
  onClose: () => void;
  onUploaded: () => void;
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
    if (!prompt.trim()) {
      const name = selectedProfile?.name ?? "角色";
      setPrompt(`用户上传角色立绘：${name}，来源文件：${nextFile.name}`);
    }
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
      await uploadPortrait({
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
      addToast(t("characterDetail.portrait.uploaded"), "success");
      onUploaded();
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

  return (
    <Modal
      isOpen
      onClose={onClose}
      title={t("characterDetail.portrait.uploadTitle")}
      subtitle={t("characterDetail.portrait.uploadSubtitle")}
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
            value={profileId}
            onChange={(selectedValue) => setProfileId(Number(selectedValue))}
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

        <label
          className={[
            "flex min-h-[168px] cursor-pointer flex-col items-center justify-center gap-2 rounded-[14px] border border-dashed border-book-border/70 bg-book-bg-paper/35 p-3 text-center text-xs text-book-text-muted transition-[border-color,background-color,transform] duration-150 ease-out hover:border-book-primary/55 hover:bg-book-bg-paper/60 active:scale-[0.995]",
            busy ? "pointer-events-none opacity-60" : "",
          ].join(" ")}
        >
          {previewUrl ? (
            <img
              src={previewUrl}
              alt=""
              className="max-h-56 max-w-full rounded-[12px] border border-book-border/50 object-contain"
            />
          ) : (
            <>
              <Upload size={18} />
              <span>{t("characterDetail.portrait.chooseImage")}</span>
            </>
          )}
          {file ? (
            <span className="max-w-full truncate text-small text-book-text-sub">
              {t("characterDetail.portrait.selectedFile", { name: file.name })}
            </span>
          ) : null}
          <input
            type="file"
            accept={supportedImageAccept()}
            className="hidden"
            disabled={busy}
            onChange={(event) => {
              const nextFile = event.currentTarget.files?.[0] ?? null;
              event.currentTarget.value = "";
              void handleFileChange(nextFile);
            }}
          />
        </label>

        <BookTextarea
          label={t("characterDetail.portrait.promptLabel")}
          value={prompt}
          rows={3}
          disabled={busy}
          onChange={(event) => setPrompt(event.target.value)}
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
