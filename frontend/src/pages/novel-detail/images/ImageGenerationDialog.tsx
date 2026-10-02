import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { useEffect, useRef, useState } from "react";
import { Boxes,
  Camera,
  ChevronRight,
  Clipboard,
  Eye,
  RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ImageSizeSelect } from "../../../components/image/ImageSizeSelect";
import type { ImageSize } from "../../../api/image";
import type { ObjectProfile } from "../../../api/objectProfiles";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { StyleTemplatePicker } from "../StyleTemplatePicker";
import type { ChapterRow } from "../shared";
import { DEFAULT_IMAGE_STYLE, VISUAL_STYLE_TEMPLATES } from "../styleTemplates";
import { useNovelDetailDirty } from "../novelDetailDirty";
import { formatNumber, useI18n } from "../../../i18n";

interface ImageGenerationDialogProps {
  open: boolean;
  chapter: ChapterRow | null;
  genPrompt: string;
  genBusy: boolean;
  objectProfiles: ObjectProfile[];
  objectProfilesError: string | null;
  selectedObjectProfileId: number | null;
  objectImageStyle: string;
  objectImageBusy: boolean;
  objectPromptCopying: boolean;
  imageSize: ImageSize | null;
  imageSizes: readonly ImageSize[];
  imageSizeLoading: boolean;
  imageSizeError: Error | null;
  chapterGenerateLocked: boolean;
  onClose: () => void;
  onGenPromptChange: (value: string) => void;
  onSelectedObjectProfileIdChange: (value: number | null) => void;
  onObjectImageStyleChange: (value: string) => void;
  onImageSizeChange: (size: ImageSize) => void;
  onCopyObjectImagePrompt: () => void;
  onPreviewGenerate: () => void;
  onGenerate: () => void;
  onPreviewObjectImage: () => void;
  onGenerateObjectImage: () => void;
  onRetryObjectProfiles: () => void;
}

/** 图像生成对话框。 */
export function ImageGenerationDialog({
  open,
  chapter,
  genPrompt,
  genBusy,
  objectProfiles,
  objectProfilesError,
  selectedObjectProfileId,
  objectImageStyle,
  objectImageBusy,
  objectPromptCopying,
  imageSize,
  imageSizes,
  imageSizeLoading,
  imageSizeError,
  chapterGenerateLocked,
  onClose,
  onGenPromptChange,
  onSelectedObjectProfileIdChange,
  onObjectImageStyleChange,
  onImageSizeChange,
  onCopyObjectImagePrompt,
  onPreviewGenerate,
  onGenerate,
  onPreviewObjectImage,
  onGenerateObjectImage,
  onRetryObjectProfiles,
}: ImageGenerationDialogProps) {
  const { locale, t } = useI18n();
  const [objectToolsOpen, setObjectToolsOpen] = useState(false);
  const baselineRef = useRef({ prompt: "", profileId: null as number | null, style: DEFAULT_IMAGE_STYLE });
  const hasObjectProfiles = objectProfiles.length > 0;
  const busy = genBusy || objectImageBusy || objectPromptCopying;
  const promptReady = genPrompt.trim().length > 0;

  useEffect(() => {
    if (!open) return;
    baselineRef.current = {
      prompt: genPrompt,
      profileId: selectedObjectProfileId,
      style: objectImageStyle,
    };
    setObjectToolsOpen(chapterGenerateLocked && hasObjectProfiles);
  }, [chapter?.id, open]);

  const draftDirty = open && (
    genPrompt !== baselineRef.current.prompt ||
    selectedObjectProfileId !== baselineRef.current.profileId ||
    objectImageStyle !== baselineRef.current.style
  );
  useNovelDetailDirty("image-generation-editor", draftDirty);
  const close = async () => {
    if (busy) return;
    if (draftDirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("novel.imageLibrary.prompt"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  return (
    <Modal
      isOpen={open}
      onClose={() => void close()}
      title={chapter ? t("novel.imageLibrary.chapterTitle") : t("novel.imageLibrary.projectTitle")}
      subtitle={
        chapter
          ? t("novel.imageLibrary.chapterDescription")
          : t("novel.imageLibrary.projectDescription")
      }
      icon={<Camera size={15} />}
      maxWidthClassName="max-w-2xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
    >
      <div className="space-y-4">
        <ImageSizeSelect
          label={t("settingsForm.selectImageSize")}
          value={imageSize}
          sizes={imageSizes}
          loading={imageSizeLoading}
          error={imageSizeError}
          disabled={busy}
          onChange={onImageSizeChange}
        />
        <section className="space-y-3 text-xs">
          <label className="flex flex-col gap-1">
            <span className="text-small text-book-text-muted">{t("novel.imageLibrary.prompt")}</span>
            <textarea
              value={genPrompt}
              onChange={(event) => onGenPromptChange(event.target.value)}
              placeholder={t("novel.imageLibrary.scenePromptPlaceholder")}
              rows={3}
              className="pyqt-input min-h-[72px] resize-y"
              disabled={chapterGenerateLocked || busy}
            />
          </label>
          {chapterGenerateLocked ? (
            <p className="text-small leading-relaxed text-warning">
              {t("novel.imageLibrary.chapterLocked")}
            </p>
          ) : chapter ? (
            <p className="text-small text-book-text-muted">
              {t("novel.imageLibrary.generatedSceneHint")}
            </p>
          ) : null}
          <div className="flex items-center justify-end gap-2 border-t border-book-border/45 pt-3">
            {!busy && !chapterGenerateLocked && promptReady ? (
              <Dropdown
                items={[
                  {
                    label: t("novel.imageLibrary.viewRequest"),
                    icon: <Eye size={12} />,
                    onClick: onPreviewGenerate,
                  },
                ]}
              />
            ) : null}
            <BookButton
              variant="primary"
              size="sm"
              onClick={onGenerate}
              disabled={busy || chapterGenerateLocked || !promptReady || imageSize == null}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {genBusy ? (
                <span className="inline-flex items-center gap-1">
                  <LoadingSpinner size="sm" /> {t("novel.imageLibrary.generating")}
                </span>
              ) : (
                t("novel.imageLibrary.startGenerate")
              )}
            </BookButton>
          </div>
        </section>

        {hasObjectProfiles ? (
          <details
            open={objectToolsOpen}
            onToggle={(event) => setObjectToolsOpen(event.currentTarget.open)}
            className="a4n-flat-surface group overflow-hidden"
          >
            <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-3 px-3 py-2.5 [&::-webkit-details-marker]:hidden">
              <span className="flex min-w-0 items-start gap-2.5">
                <Boxes size={14} className="mt-0.5 shrink-0 text-book-primary" />
                <span className="min-w-0">
                  <span className="block text-xs font-medium text-book-text-main">
                    {t("novel.imageLibrary.objectTitle")}
                  </span>
                  <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
                    {t("novel.imageLibrary.objectDescription")}
                  </span>
                </span>
              </span>
              <ChevronRight
                size={14}
                className="text-book-text-muted transition-transform group-open:rotate-90"
              />
            </summary>
            <section className="space-y-3 border-t border-book-border/45 p-3 text-xs">
              <div className="flex items-baseline justify-between gap-3">
                <label
                  htmlFor="object-image-profile"
                  className="text-small text-book-text-muted"
                >
                  {t("novel.imageLibrary.objectProfiles")}
                </label>
                <span className="text-caption text-book-text-muted">
                  {t("novel.imageLibrary.availableProfiles", { count: formatNumber(objectProfiles.length, locale) })}
                </span>
              </div>
              {objectProfilesError ? (
                <div role="alert" className="flex flex-wrap items-center gap-2 text-error">
                  <span className="min-w-0 flex-1">{objectProfilesError}</span>
                  <BookButton variant="secondary" size="sm" onClick={onRetryObjectProfiles}>
                    <RefreshCw size={12} /> {t("common.retry")}
                  </BookButton>
                </div>
              ) : null}
              <div className="grid grid-cols-1 gap-2">
                <ThemedSelect
                  id="object-image-profile"
                  value={selectedObjectProfileId ?? ""}
                  onChange={(selectedValue) => {
                    const value = Number(selectedValue);
                    onSelectedObjectProfileIdChange(
                      Number.isFinite(value) && value > 0 ? value : null,
                    );
                  }}
                  className="pyqt-input"
                  disabled={busy || objectProfilesError != null}
                >
                  <option value="">{t("novel.imageLibrary.selectObject")}</option>
                  {objectProfiles.map((profile) => (
                    <option key={profile.id} value={profile.id}>
                      {profile.name}
                    </option>
                  ))}
                </ThemedSelect>
                <StyleTemplatePicker
                  label={t("novel.imageLibrary.style")}
                  value={objectImageStyle}
                  templates={VISUAL_STYLE_TEMPLATES}
                  disabled={busy}
                  rows={4}
                  onChange={onObjectImageStyleChange}
                />
              </div>
              <div className="flex items-center justify-end gap-2 border-t border-book-border/45 pt-3">
                {objectPromptCopying ? (
                  <span className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
                    <LoadingSpinner size="sm" /> {t("novel.imageLibrary.copyingPrompt")}
                  </span>
                ) : !busy && objectProfilesError == null && selectedObjectProfileId != null ? (
                  <Dropdown
                    items={[
                      {
                        label: t("novel.imageLibrary.copyPrompt"),
                        icon: <Clipboard size={12} />,
                        onClick: onCopyObjectImagePrompt,
                      },
                      {
                        label: t("novel.imageLibrary.viewRequest"),
                        icon: <Eye size={12} />,
                        onClick: onPreviewObjectImage,
                      },
                    ]}
                  />
                ) : null}
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={onGenerateObjectImage}
                  disabled={busy || objectProfilesError != null || selectedObjectProfileId == null || imageSize == null}
                  className="min-w-0 flex-1 !text-xs sm:flex-none"
                >
                  {objectImageBusy ? (
                    <span className="inline-flex items-center gap-1">
                      <LoadingSpinner size="sm" /> {t("novel.imageLibrary.generating")}
                    </span>
                  ) : (
                    t("novel.imageLibrary.generateMaterial")
                  )}
                </BookButton>
              </div>
            </section>
          </details>
        ) : null}
      </div>
    </Modal>
  );
}
