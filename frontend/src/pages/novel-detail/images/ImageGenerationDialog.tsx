import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { useEffect, useState } from "react";
import { Boxes,
  Camera,
  ChevronRight,
  Clipboard,
  Eye } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ImageSizeSelect } from "../../../components/image/ImageSizeSelect";
import type { ImageSize } from "../../../api/image";
import type { ObjectProfile } from "../../../api/objectProfiles";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { StyleTemplatePicker } from "../StyleTemplatePicker";
import type { ChapterRow } from "../shared";
import { VISUAL_STYLE_TEMPLATES } from "../styleTemplates";
import { formatNumber, useI18n } from "../../../i18n";

interface ImageGenerationDialogProps {
  open: boolean;
  chapter: ChapterRow | null;
  genPrompt: string;
  genBusy: boolean;
  objectProfiles: ObjectProfile[];
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
}

/** 图像生成对话框。 */
export function ImageGenerationDialog({
  open,
  chapter,
  genPrompt,
  genBusy,
  objectProfiles,
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
}: ImageGenerationDialogProps) {
  const { locale, t } = useI18n();
  const [objectToolsOpen, setObjectToolsOpen] = useState(false);
  const hasObjectProfiles = objectProfiles.length > 0;
  const busy = genBusy || objectImageBusy || objectPromptCopying;
  const promptReady = genPrompt.trim().length > 0;

  useEffect(() => {
    if (!open) return;
    setObjectToolsOpen(chapterGenerateLocked && hasObjectProfiles);
  }, [chapter?.id, chapterGenerateLocked, hasObjectProfiles, open]);

  return (
    <Modal
      isOpen={open}
      onClose={onClose}
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
                  disabled={busy}
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
                ) : !busy && selectedObjectProfileId != null ? (
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
                  disabled={busy || selectedObjectProfileId == null || imageSize == null}
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
