import React from "react";
import { Sparkles, Trash2, User, UserPlus } from "lucide-react";
import { type CharacterProfile } from "../../api/character";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import {
  characterAttributeEntries,
  characterAttributeText,
  type CharacterAttributeCategory,
} from "../../utils/characterAttributes";
import { AttributePreview } from "./CharacterProfileDisplay";
import { useI18n } from "../../i18n";

export const CharacterProfileDetailPanel: React.FC<{
  profile: CharacterProfile | null;
  onCreate: () => void;
  onOpenDetail: (profile: CharacterProfile) => void;
  onOpenHistory: (
    profile: CharacterProfile,
    category: CharacterAttributeCategory,
    key: string,
  ) => void;
  onDelete: (profile: CharacterProfile) => void | Promise<void>;
}> = ({ profile, onCreate, onOpenDetail, onOpenHistory, onDelete }) => {
  const { t } = useI18n();
  if (!profile) {
    return (
      <section className="min-h-0 min-w-0 p-4" aria-label={t("characterDetail.profileDetailAria")}>
        <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
          <User size={28} className="text-book-text-muted" />
          <div className="text-sm font-semibold text-book-text-main">{t("characterDetail.noProfile")}</div>
          <BookButton variant="primary" onClick={onCreate}>
            <span className="inline-flex items-center gap-1.5">
              <UserPlus size={14} /> {t("characterDetail.createProfile")}
            </span>
          </BookButton>
        </div>
      </section>
    );
  }

  const gender = characterAttributeText(profile, "explicit", "gender");
  const age = characterAttributeText(profile, "explicit", "age");
  const profileMeta = [
    gender,
    age,
    profile.last_synced_chapter > 0
      ? t("characterDetail.syncedChapterStatus", { chapter: profile.last_synced_chapter })
      : t("characterDetail.notSyncedChapterStatus"),
  ].filter((value): value is string => Boolean(value));

  return (
    <section className="min-h-0 min-w-0 p-4" aria-label={t("characterDetail.profileDetailAria")}>
      <div className="flex h-full min-h-0 flex-col">
        <div className="mb-3 flex flex-col gap-3 border-b border-book-border/50 pb-3 sm:flex-row sm:items-center">
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-sm font-semibold text-book-text-main">{profile.name}</h2>
            <p className="mt-1 text-small text-book-text-muted">{profileMeta.join(" · ")}</p>
          </div>
          <div className="flex min-w-0 items-center gap-2">
            <Dropdown
              items={[
                {
                  label: t("characterDetail.deleteProfile"),
                  icon: <Trash2 size={12} />,
                  danger: true,
                  onClick: () => void onDelete(profile),
                },
              ]}
            />
            <BookButton
              variant="primary"
              size="sm"
              className="min-w-0 flex-1 sm:flex-none"
              onClick={() => onOpenDetail(profile)}
            >
              <Sparkles size={13} /> {t("characterDetail.deepDetails")}
            </BookButton>
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto pr-1 text-xs custom-scrollbar">
          <div className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
            <AttributePreview
              label={t("characterDetail.labels.explicit")}
              category="explicit"
              entries={characterAttributeEntries(profile, "explicit")}
              onOpenHistory={(category, key) => onOpenHistory(profile, category, key)}
            />
            <AttributePreview
              label={t("characterDetail.labels.implicit")}
              category="implicit"
              entries={characterAttributeEntries(profile, "implicit")}
              onOpenHistory={(category, key) => onOpenHistory(profile, category, key)}
            />
            <AttributePreview
              label={t("characterDetail.labels.social")}
              category="social"
              entries={characterAttributeEntries(profile, "social")}
              onOpenHistory={(category, key) => onOpenHistory(profile, category, key)}
            />
          </div>
        </div>
      </div>
    </section>
  );
};
