import React from "react";
import { Camera,
  ChevronRight,
  Clock3,
  Eye,
  FileText,
  RefreshCw,
  Star, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ImageSizeSelect } from "../../../components/image/ImageSizeSelect";
import type { ImageSize } from "../../../api/image";
import type { Portrait } from "../../../api/portrait";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { ZoomableImage } from "../../../components/ui/ImageLightbox";
import { Modal } from "../../../components/ui/Modal";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { useI18n } from "../../../i18n";
import { portraitTypeLabel } from "./characterUtils";

interface PortraitActionDialogProps {
  portrait: Portrait;
  busyId: number | null;
  imageSize: ImageSize | null;
  imageSizes: readonly ImageSize[];
  imageSizeLoading: boolean;
  imageSizeError: Error | null;
  onImageSizeChange: (size: ImageSize) => void;
  onClose: () => void;
  onPreviewRegenerate: (portrait: Portrait, size: ImageSize) => void | Promise<void>;
  onRegenerate: (portrait: Portrait, size: ImageSize) => void | Promise<void>;
  onSetPrimary: (portrait: Portrait) => void | Promise<void>;
  onAgeVariant: (portrait: Portrait) => void;
  onPrompt: (portrait: Portrait) => void;
}

export const PortraitActionDialog: React.FC<PortraitActionDialogProps> = ({
  portrait,
  busyId,
  imageSize,
  imageSizes,
  imageSizeLoading,
  imageSizeError,
  onImageSizeChange,
  onClose,
  onPreviewRegenerate,
  onRegenerate,
  onSetPrimary,
  onAgeVariant,
  onPrompt,
}) => {
  const { t } = useI18n();
  const busy = busyId === portrait.id;
  const sizeUnavailable = imageSize == null || imageSizeLoading || imageSizeError != null;
  const imageSrc = toMediaSrc(portrait.image_path);
  const metadata = [portraitTypeLabel(portrait.portrait_type, t), portrait.expression, portrait.pose]
    .map((item) => item?.trim())
    .filter(Boolean)
    .join(" · ");

  return (
    <Modal
      isOpen
      onClose={() => {
        if (!busy) onClose();
      }}
      title={t("characterDetail.portrait.detailsTitle")}
      subtitle={metadata || t("characterDetail.portrait.material")}
      icon={<Camera size={15} />}
      maxWidthClassName="max-w-2xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
      footer={
        <div className="flex w-full items-center gap-2">
          <Dropdown
            items={[
              {
                label: t("characterDetail.portrait.viewRegenerateRequest"),
                icon: <Eye size={12} />,
                onClick: () => {
                  if (imageSize) void onPreviewRegenerate(portrait, imageSize);
                },
                disabled: busy || sizeUnavailable,
              },
            ]}
          />
          <span className="min-w-0 flex-1" />
          <BookButton
            variant="primary"
            size="sm"
            onClick={() => {
              if (imageSize) void onRegenerate(portrait, imageSize);
            }}
            disabled={busy || sizeUnavailable}
            className="!text-xs"
          >
            {busy ? (
              <LoadingSpinner size="sm" />
            ) : (
              <RefreshCw size={13} />
            )}
            {busy ? t("characterDetail.portrait.redrawBusy") : t("characterDetail.portrait.redraw")}
          </BookButton>
        </div>
      }
    >
      <div className="grid gap-4 sm:grid-cols-[180px_minmax(0,1fr)]">
        <div className="min-w-0">
          <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden rounded-[14px] border border-book-border/55 bg-book-bg-tertiary text-xs text-book-text-muted">
            {imageSrc ? (
              <ZoomableImage
                src={imageSrc}
                alt={t("characterDetail.portrait.previewAlt")}
                className="h-full w-full object-contain"
              />
            ) : (
              <span>{t("characterDetail.portrait.noImage")}</span>
            )}
            {portrait.is_primary ? (
              <span className="absolute right-2 top-2 inline-flex items-center gap-1 rounded-lg bg-book-primary/90 px-2 py-1 text-caption font-medium text-white backdrop-blur-sm">
                <Star size={9} fill="currentColor" /> {t("characterDetail.portrait.primary")}
              </span>
            ) : null}
          </div>
        </div>

        <div className="min-w-0">
          <ImageSizeSelect
            label={t("settingsForm.selectImageSize")}
            value={imageSize}
            sizes={imageSizes}
            loading={imageSizeLoading}
            error={imageSizeError}
            disabled={busy}
            onChange={onImageSizeChange}
          />
          <div className="mt-3 space-y-2">
            {!portrait.is_primary ? (
              <PortraitActionButton
                icon={<Star size={14} />}
                title={t("characterDetail.portrait.setPrimary")}
                description={t("characterDetail.portrait.setPrimaryDescription")}
                disabled={busy}
                onClick={() => void onSetPrimary(portrait)}
              />
            ) : null}
            <PortraitActionButton
              icon={<Clock3 size={14} />}
              title={t("characterDetail.portrait.ageVariant")}
              description={t("characterDetail.portrait.ageVariantDescription")}
              disabled={busy}
              onClick={() => onAgeVariant(portrait)}
            />
            <PortraitActionButton
              icon={<FileText size={14} />}
              title={t("characterDetail.portrait.viewPrompt")}
              description={t("characterDetail.portrait.viewPromptDescription")}
              disabled={busy}
              onClick={() => onPrompt(portrait)}
            />
          </div>
          <p className="mt-2 px-1 text-caption leading-relaxed text-book-text-muted">
            {t("characterDetail.portrait.redrawHint")}
          </p>
        </div>
      </div>
    </Modal>
  );
};

const PortraitActionButton: React.FC<{
  icon: React.ReactNode;
  title: string;
  description: string;
  disabled?: boolean;
  onClick: () => void;
}> = ({ icon, title, description, disabled, onClick }) => (
  <button
    type="button"
    disabled={disabled}
    onClick={onClick}
    className="a4n-flat-surface flex w-full items-start gap-3 px-3 py-3 text-left transition-colors hover:border-book-primary/30 hover:bg-book-bg-tertiary/45 disabled:cursor-not-allowed disabled:opacity-55"
  >
    <span className="mt-0.5 inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-book-primary/10 text-book-primary">
      {icon}
    </span>
    <span className="min-w-0 flex-1">
      <span className="block text-sm font-semibold text-book-text-main">{title}</span>
      <span className="mt-1 block text-small leading-relaxed text-book-text-muted">
        {description}
      </span>
    </span>
    <ChevronRight size={14} className="mt-2 shrink-0 text-book-text-muted" />
  </button>
);
