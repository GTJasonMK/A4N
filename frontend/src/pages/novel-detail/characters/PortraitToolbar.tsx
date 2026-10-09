import React from "react";
import { Camera, Eye, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import type { CharacterProfile } from "../../../api/character";
import { formatNumber, useI18n } from "../../../i18n";

interface PortraitToolbarProps {
  visiblePortraitCount: number;
  totalPortraitCount: number;
  profiles: CharacterProfile[];
  profilesUnavailable: boolean;
  selectedProfileId: number | null;
  generating: boolean;
  onProfileChange: (profileId: number | null) => void;
  onOpenSingle: () => void;
  onOpenUpload: () => void;
  onPreviewGenerateMissing: () => void | Promise<void>;
  onGenerateMissing: () => void | Promise<void>;
}

export const PortraitToolbar: React.FC<PortraitToolbarProps> = ({
  visiblePortraitCount,
  totalPortraitCount,
  profiles,
  profilesUnavailable,
  selectedProfileId,
  generating,
  onProfileChange,
  onOpenSingle,
  onOpenUpload,
  onPreviewGenerateMissing,
  onGenerateMissing,
}) => {
  const { locale, t } = useI18n();
  const selectedProfile =
    selectedProfileId == null
      ? null
      : profiles.find((profile) => profile.id === selectedProfileId) ?? null;

  return (
    <section className="a4n-flat-surface overflow-hidden">
      <header className="flex flex-col gap-2 px-3 pb-3 pt-3 sm:flex-row sm:items-center sm:justify-between sm:px-4">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold text-book-text-main">{t("characterDetail.portrait.title")}</h2>
          <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
            {selectedProfile
              ? t("characterDetail.portrait.summarySelected", {
                  name: selectedProfile.name,
                  visible: formatNumber(visiblePortraitCount, locale),
                  total: formatNumber(totalPortraitCount, locale),
                })
            : profilesUnavailable
              ? t("characterDetail.portrait.profileCountUnavailable", {
                  total: formatNumber(totalPortraitCount, locale),
                })
              : t("characterDetail.portrait.summaryAll", {
                  total: formatNumber(totalPortraitCount, locale),
                  profiles: formatNumber(profiles.length, locale),
                })}
          </p>
        </div>
        <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
          {!generating ? (
            <Dropdown
              items={[
                {
                  label: t("characterDetail.portrait.generateSingle"),
                  icon: <Camera size={12} />,
                  onClick: onOpenSingle,
                },
                {
                  label: t("characterDetail.portrait.upload"),
                  icon: <Upload size={12} />,
                  onClick: onOpenUpload,
                },
                { type: "divider" },
                {
                  label: t("characterDetail.portrait.previewMissing"),
                  icon: <Eye size={12} />,
                  onClick: () => void onPreviewGenerateMissing(),
                },
              ]}
            />
          ) : null}
          <BookButton
            variant="primary"
            size="sm"
            onClick={() => void onGenerateMissing()}
            disabled={generating}
            className="min-w-0 flex-1 !text-xs sm:flex-none"
          >
            {generating ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" /> {t("characterDetail.portrait.generating")}
              </span>
            ) : (
              t("characterDetail.portrait.batchMissing")
            )}
          </BookButton>
        </div>
      </header>
      {profiles.length > 0 ? (
        <div className="border-t border-book-border/40 px-3 py-2.5 sm:px-4">
        <div className="a4n-filter-chips !max-w-full !flex-wrap !overflow-visible" aria-label={t("characterDetail.portrait.filterAria")}>
          <button
            type="button"
            className="a4n-filter-chip"
            data-active={selectedProfileId == null ? "1" : undefined}
            aria-pressed={selectedProfileId == null}
            onClick={() => onProfileChange(null)}
          >
            {t("characterDetail.portrait.all")}
          </button>
          {profiles.map((profile) => (
            <button
              key={profile.id}
              type="button"
              className="a4n-filter-chip"
              data-active={selectedProfileId === profile.id ? "1" : undefined}
              aria-pressed={selectedProfileId === profile.id}
              onClick={() => onProfileChange(profile.id)}
              title={t("characterDetail.portrait.onlyProfile", { name: profile.name })}
            >
              {profile.name}
            </button>
          ))}
        </div>
        </div>
      ) : null}
    </section>
  );
};
