import React from "react";
import { Eye, RefreshCw, Search, UserPlus } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { type CharacterProfile } from "../../api/character";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { describeProfile } from "./CharacterProfileDisplay";
import { useI18n } from "../../i18n";

export const CharacterProfileList: React.FC<{
  profiles: CharacterProfile[];
  selectedId: number | null;
  loading: boolean;
  discovering: boolean;
  syncingAll: boolean;
  canSyncAll: boolean;
  aiActionBlockedReason: string | null;
  onRefresh: () => void | Promise<void>;
  onCreate: () => void;
  onPreviewDiscover: () => void | Promise<void>;
  onDiscover: () => void | Promise<void>;
  onPreviewSyncAll: () => void | Promise<void>;
  onSyncAll: () => void | Promise<void>;
  onSelect: (profileId: number) => void;
}> = ({
  profiles,
  selectedId,
  loading,
  discovering,
  syncingAll,
  canSyncAll,
  aiActionBlockedReason,
  onRefresh,
  onCreate,
  onPreviewDiscover,
  onDiscover,
  onPreviewSyncAll,
  onSyncAll,
  onSelect,
}) => {
  const { t } = useI18n();
  return (
  <aside className="flex min-h-0 min-w-0 flex-col border-b border-book-border bg-book-bg-paper/45 p-3 md:border-b-0 md:border-r">
    <div className="mb-3 flex items-center gap-2">
      <p className="min-w-0 flex-1 text-small text-book-text-muted">
        {profiles.length > 0 ? t("characterDetail.profileCount", { count: profiles.length }) : t("characterDetail.noProfiles")}
      </p>
      <Dropdown
        items={[
          {
            label: t("characterDetail.refreshProfiles"),
            icon: <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />,
            disabled: loading || discovering || syncingAll,
            onClick: () => void onRefresh(),
          },
          {
            label: t("characterDetail.createProfile"),
            icon: <UserPlus size={12} />,
            disabled: discovering || syncingAll,
            onClick: onCreate,
          },
          { type: "divider" },
          {
            label: t("characterDetail.previewDiscoverProfiles"),
            icon: <Eye size={12} />,
            disabled: discovering || syncingAll || loading || aiActionBlockedReason != null,
            onClick: () => void onPreviewDiscover(),
          },
          {
            label: t("characterDetail.previewSyncAllProfiles"),
            icon: <Eye size={12} />,
            disabled:
              syncingAll ||
              discovering ||
              loading ||
              profiles.length === 0 ||
              !canSyncAll ||
              aiActionBlockedReason != null,
            onClick: () => void onPreviewSyncAll(),
          },
          {
            label: t("characterDetail.syncAllProfiles"),
            icon: <RefreshCw size={12} />,
            disabled:
              syncingAll ||
              discovering ||
              loading ||
              profiles.length === 0 ||
              !canSyncAll ||
              aiActionBlockedReason != null,
            onClick: () => void onSyncAll(),
          },
        ]}
      />
      <BookButton
        variant="primary"
        size="sm"
        onClick={() => void onDiscover()}
        disabled={discovering || syncingAll || loading || aiActionBlockedReason != null}
        className="min-w-0 flex-1 md:flex-none"
      >
        {syncingAll ? (
          <span className="inline-flex items-center gap-1.5">
            <LoadingSpinner size="sm" /> {t("characterDetail.syncingProfiles")}
          </span>
        ) : discovering ? (
          <span className="inline-flex items-center gap-1.5">
            <LoadingSpinner size="sm" /> {t("characterDetail.discoveringProfiles")}
          </span>
        ) : (
          <span className="inline-flex items-center gap-1.5">
            <Search size={12} /> {t("characterDetail.discoverProfiles")}
          </span>
        )}
      </BookButton>
    </div>

    {aiActionBlockedReason ? (
      <p className="mb-3 text-caption leading-relaxed text-error">{aiActionBlockedReason}</p>
    ) : null}

    {loading && profiles.length === 0 ? (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" /> {t("characterDetail.loadingProfiles")}
      </div>
    ) : profiles.length === 0 ? (
      <div className="flex flex-col items-center gap-3 py-6 text-center">
        <p className="text-xs text-book-text-muted">{t("characterDetail.createFirstProfileHint")}</p>
        <BookButton variant="secondary" size="sm" onClick={onCreate}>
          <UserPlus size={12} /> {t("characterDetail.createProfile")}
        </BookButton>
      </div>
    ) : (
      <ul
        aria-label={t("characterDetail.profileListAria")}
        className="a4n-mobile-scroll-rail custom-scrollbar flex max-w-full gap-2 overflow-x-auto pb-1 md:min-h-0 md:flex-1 md:flex-col md:overflow-x-hidden md:overflow-y-auto md:pb-0 md:pr-1"
      >
        {profiles.map((profile) => {
          const active = selectedId === profile.id;
          const initial = profile.name.trim().charAt(0).toUpperCase();
          return (
            <li key={profile.id}>
              <button
                type="button"
                onClick={() => onSelect(profile.id)}
                aria-pressed={active}
                data-active={active ? "1" : undefined}
                className="a4n-flat-surface group flex min-w-[11rem] items-start gap-2 px-2.5 py-2 text-left transition-colors hover:border-book-primary/55 hover:bg-book-bg-tertiary/45 data-[active=1]:border-book-primary/70 data-[active=1]:bg-book-primary/10 md:w-full md:min-w-0 md:px-3 md:py-2.5"
              >
                <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-book-primary/10 text-xs font-semibold text-book-primary">
                  {initial}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="flex items-center gap-1.5">
                    <span className="truncate text-xs font-semibold text-book-text-main">
                      {profile.name}
                    </span>
                  </span>
                  {describeProfile(profile) && (
                    <span className="mt-0.5 block truncate text-caption text-book-text-muted">
                      {describeProfile(profile)}
                    </span>
                  )}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    )}
  </aside>
  );
};
