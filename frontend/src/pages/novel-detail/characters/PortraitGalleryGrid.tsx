import React, { useMemo } from "react";
import { Star } from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import type { Portrait } from "../../../api/portrait";
import { ImageZoomButton } from "../../../components/ui/ImageLightbox";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { EmptyHint } from "../shared";
import { useI18n } from "../../../i18n";
import { portraitTypeLabel } from "./characterUtils";

interface PortraitGalleryGridProps {
  portraits: Portrait[];
  profiles: CharacterProfile[];
  profilesUnavailable?: boolean;
  busyId: number | null;
  onOpenActions: (portrait: Portrait) => void;
  emptyText?: string;
}

export const PortraitGalleryGrid: React.FC<PortraitGalleryGridProps> = ({
  portraits,
  profiles,
  profilesUnavailable = false,
  busyId,
  onOpenActions,
  emptyText,
}) => {
  const { t } = useI18n();
  const profilesById = useMemo(
    () => new Map(profiles.map((profile) => [profile.id, profile])),
    [profiles],
  );

  if (portraits.length === 0) {
    return (
      <div className="py-8">
        <EmptyHint text={emptyText ?? t("characterDetail.portrait.empty")} />
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
      {portraits.map((portrait) => {
        const profile = profilesById.get(portrait.profile_id);
        const busy = busyId === portrait.id;
        const imageSrc = toMediaSrc(portrait.image_path);

        return (
          <div key={portrait.id} className="relative min-w-0">
          <button
            type="button"
            onClick={() => onOpenActions(portrait)}
            disabled={busy}
            className="group w-full min-w-0 overflow-hidden rounded-[14px] border border-book-border/55 bg-book-bg-paper/45 text-left transition-colors hover:border-book-primary/45 disabled:cursor-wait disabled:opacity-60"
            title={t("characterDetail.portrait.manage")}
          >
            <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden bg-book-bg-tertiary text-xs text-book-text-muted">
              {imageSrc ? (
                <img
                  src={imageSrc}
                alt={profile?.name
                    ? t("characterDetail.portrait.alt", { name: profile.name })
                    : t(profilesUnavailable
                      ? "characterDetail.portrait.profileUnavailable"
                      : "characterDetail.portrait.altGeneric")}
                  className="h-full w-full object-cover"
                />
              ) : (
                <span>{t("characterDetail.portrait.noImage")}</span>
              )}
              {portrait.is_primary ? (
                <span className="absolute right-1.5 top-1.5 inline-flex items-center gap-1 rounded-md bg-book-primary/90 px-2 py-1 text-caption text-white backdrop-blur-sm">
                  <Star size={9} fill="currentColor" /> {t("characterDetail.portrait.primary")}
                </span>
              ) : null}
            </div>
            <div className="p-2.5">
              <div className="truncate text-xs font-medium text-book-text-main">
                {profile?.name ?? t(profilesUnavailable
                  ? "characterDetail.portrait.profileUnavailable"
                  : "characterDetail.portrait.unlinked")}
              </div>
              <div className="portrait-card-metadata mt-1 flex min-w-0 flex-col gap-0.5 text-caption text-book-text-muted">
                <span className="break-words leading-snug">{portraitTypeLabel(portrait.portrait_type, t)}</span>
                {portrait.expression ? <span className="break-words leading-snug">{portrait.expression}</span> : null}
              </div>
            </div>
          </button>
          {imageSrc ? <ImageZoomButton src={imageSrc} alt={profile?.name ?? ""} className="absolute left-2 top-2" /> : null}
          </div>
        );
      })}
    </div>
  );
};
