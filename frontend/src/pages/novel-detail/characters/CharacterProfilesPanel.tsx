import React, { useEffect, useState } from "react";
import { ChevronRight } from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import { CharacterDetailDialog } from "../../CharacterDetailDialog";
import { BookCardAction } from "../../../components/ui/BookCard";
import {
  characterAttributeEntries,
  characterAttributeText,
  characterAttributeValueText,
} from "../../../utils/characterAttributes";
import { attributeKeyLabel } from "../../character-detail-dialog/manageViewUtils";
import { EmptyHint } from "../shared";
import { useI18n, type MessageTranslator } from "../../../i18n";
import { isDistinctText } from "../../../utils/text";

const profileDescription = (profile: CharacterProfile, t: MessageTranslator): string =>
  characterAttributeText(profile, "implicit", "personality") ??
  characterAttributeText(profile, "explicit", "background") ??
  characterAttributeText(profile, "explicit", "appearance") ??
  t("characterDetail.profileSummaryMissing");

const syncedChapterText = (chapter: number, t: MessageTranslator): string =>
  chapter > 0
    ? t("characterDetail.syncThroughChapter", { chapter })
    : t("characterDetail.notSynced");

const visibleAttributeEntries = (
  profile: CharacterProfile,
  t: MessageTranslator,
): Array<[string, string]> =>
  [
    ...characterAttributeEntries(profile, "explicit"),
    ...characterAttributeEntries(profile, "implicit"),
    ...characterAttributeEntries(profile, "social"),
  ]
    .filter(([key]) => !["gender", "age", "personality"].includes(key))
    .map(([key, value]) => [attributeKeyLabel(key, t), characterAttributeValueText(value)] as [string, string])
    .filter(([, value]) => value && !value.trim().startsWith("{") && !value.trim().startsWith("["))
    .slice(0, 3);

/**
 * 角色档案：每个角色保持独立边界，点击卡片进入详情。
 */
export const CharacterProfilesPanel: React.FC<{
  projectId: number;
  profiles: CharacterProfile[];
  loadError: string | null;
  onChanged: () => boolean | void | Promise<boolean | void>;
  onEdit: (profile: CharacterProfile) => void;
}> = ({ projectId, profiles, loadError, onChanged, onEdit }) => {
  const { t } = useI18n();
  const [detailFor, setDetailFor] = useState<CharacterProfile | null>(null);

  useEffect(() => {
    setDetailFor((current) => {
      if (!current) return null;
      return profiles.find((profile) => profile.id === current.id) ?? null;
    });
  }, [profiles]);

  if (profiles.length === 0) {
    return loadError ? null : <EmptyHint text={t("characterDetail.profilesEmpty")} />;
  }

  const latestSyncedChapter = profiles.reduce(
    (latest, profile) => Math.max(latest, profile.last_synced_chapter),
    0,
  );

  return (
    <>
      <section>
        <header className="mb-3 px-1">
          <h2 className="text-sm font-semibold text-book-text-main">{t("characterDetail.profilesTitle")}</h2>
          <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
            {t("characterDetail.profileCount", { count: profiles.length })}
            {latestSyncedChapter > 0 ? ` · ${t("characterDetail.latestSynced", { chapter: latestSyncedChapter })}` : ""}
          </p>
        </header>
        <ul className="grid grid-cols-1 items-start gap-2 sm:grid-cols-2">
          {profiles.map((profile) => {
            const description = profileDescription(profile, t);
            const attributes = visibleAttributeEntries(profile, t)
              .filter(([, value]) => isDistinctText(value, description));
            const gender = characterAttributeText(profile, "explicit", "gender");
            const age = characterAttributeText(profile, "explicit", "age");
            return (
              <li
                key={profile.id}
                className="a4n-flat-surface relative flex min-h-20 items-start gap-3 p-3 transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/35"
              >
                <BookCardAction
                  label={`${profile.name} · ${t("characterDetail.viewDetails")}`}
                  onClick={() => setDetailFor(profile)}
                  disabled={loadError != null}
                />
                <span className="mt-0.5 inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-book-primary/10 text-xs font-semibold text-book-primary">
                  {profile.name.trim().charAt(0).toUpperCase() || "?"}
                </span>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-semibold text-book-text-main">{profile.name}</div>
                  <div className="mt-0.5 text-caption text-book-text-muted">
                    {[gender, age, syncedChapterText(profile.last_synced_chapter, t)]
                      .filter(Boolean)
                      .join(" · ")}
                  </div>
                  <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-sub">
                    {description}
                  </p>
                  {attributes.length > 0 ? (
                    <p className="mt-1 truncate text-caption text-book-text-muted">
                      {attributes.map(([key, value]) => `${key}: ${value}`).join(" · ")}
                    </p>
                  ) : null}
                </div>
                <ChevronRight size={14} className="mt-1 shrink-0 text-book-text-muted" />
              </li>
            );
          })}
        </ul>
      </section>

      <CharacterDetailDialog
        open={detailFor != null}
        onClose={() => setDetailFor(null)}
        projectId={projectId}
        character={detailFor}
        onChanged={onChanged}
        onEditInitial={(profile) => {
          setDetailFor(null);
          onEdit(profile);
        }}
      />
    </>
  );
};
