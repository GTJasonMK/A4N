import React, { useEffect, useState } from "react";
import { ChevronRight, Sparkles, User, Users } from "lucide-react";
import { extractErrorMessage } from "../../api";
import { getBlueprint, type Blueprint } from "../../api/blueprint";
import { listProfiles, type CharacterProfile } from "../../api/character";
import { getPrimaryPortrait, type Portrait } from "../../api/portrait";
import { useToast } from "../../components/feedback/Toast";
import { Modal } from "../../components/ui/Modal";
import { characterAttributeText } from "../../utils/characterAttributes";
import { toMediaSrc } from "../../utils/mediaSrc";
import { nonEmptyText } from "../../utils/text";
import { formatNumber, useI18n, type Locale } from "../../i18n";

/** 写作侧栏的蓝图与角色直接入口。 */

interface WritingContextCardsProps {
  projectId: number;
  chapterCount?: number;
  totalChapters?: number;
  refreshKey?: number;
  onOpenProfiles?: () => void;
}

const blueprintField = (label: string, value: string | number | null | undefined, locale: Locale) => {
  const text = typeof value === "number" ? formatNumber(value, locale) : nonEmptyText(value);
  if (!text) return null;
  return (
    <section className="border-b border-book-border/30 pb-3 last:border-b-0 last:pb-0">
      <div className="mb-1 text-small font-semibold text-book-text-muted">{label}</div>
      <div className="whitespace-pre-wrap text-xs leading-relaxed text-book-text-main">{text}</div>
    </section>
  );
};

export const WritingContextCards: React.FC<WritingContextCardsProps> = ({
  projectId,
  chapterCount = 0,
  totalChapters = 0,
  refreshKey = 0,
  onOpenProfiles,
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [blueprintError, setBlueprintError] = useState<string | null>(null);
  const [primaryProfile, setPrimaryProfile] = useState<CharacterProfile | null>(null);
  const [profileCount, setProfileCount] = useState(0);
  const [portrait, setPortrait] = useState<Portrait | null>(null);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [blueprintDetailOpen, setBlueprintDetailOpen] = useState(false);
  const primaryPortraitSrc = toMediaSrc(portrait?.image_path);
  const primaryPersonality = primaryProfile
    ? characterAttributeText(primaryProfile, "implicit", "personality")
    : null;

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const bp = await getBlueprint(projectId);
        if (!cancelled) {
          setBlueprint(bp);
          setBlueprintError(null);
        }
      } catch (err) {
        if (!cancelled) {
          const message = extractErrorMessage(err, t("writing.blueprintLoadFailed"));
          setBlueprint(null);
          setBlueprintError(message);
          addToast(message, "error");
        }
      }
      try {
        const profiles = await listProfiles(projectId);
        const main = profiles[0] ?? null;
        if (cancelled) return;
        setPrimaryProfile(main);
        setProfileCount(profiles.length);
        setPortrait(null);
        setProfileError(null);
        if (main) {
          const p = await getPrimaryPortrait(main.id);
          if (!cancelled) setPortrait(p);
        }
      } catch (err) {
        if (!cancelled) {
          setPrimaryProfile(null);
          setProfileCount(0);
          setPortrait(null);
          const message = extractErrorMessage(err, t("writing.profileLoadFailedContext"));
          setProfileError(message);
          addToast(message, "error");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [projectId, refreshKey, addToast, t]);

  const progress =
    totalChapters > 0 ? Math.round((chapterCount / totalChapters) * 100) : null;

  return (
    <>
      <div className="min-w-0 space-y-2">
        <button
          type="button"
          onClick={() => setBlueprintDetailOpen(true)}
          className="a4n-flat-surface block w-full px-3 py-2.5 text-left text-small transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/45 focus-visible:outline-2 focus-visible:outline-book-primary/35 disabled:cursor-default disabled:hover:border-book-border disabled:hover:bg-transparent"
          disabled={!blueprint || !!blueprintError}
          title={t("writing.viewBlueprint")}
        >
          <div className="flex items-center gap-2">
            <Sparkles size={12} className="shrink-0 text-book-primary" />
            <span className="font-semibold text-book-text-main">{t("writing.storyBlueprint")}</span>
            {progress != null ? (
              <span className="ml-auto text-caption text-book-text-muted">
                {t("writing.chapterProgress", { count: formatNumber(chapterCount, locale), total: formatNumber(totalChapters, locale) })}
              </span>
            ) : null}
            <ChevronRight size={13} className="shrink-0 text-book-text-muted" />
          </div>
          {blueprintError ? (
            <p className="mt-1 text-error">{t("writing.blueprintLoadUnavailable")}</p>
          ) : blueprint ? (
            <div className="mt-1 min-w-0 leading-relaxed text-book-text-sub">
              {nonEmptyText(blueprint.style) ? (
                <p className="truncate text-book-text-muted">
                  {t("writing.blueprintStyle")}<span className="text-book-text-main">{nonEmptyText(blueprint.style)}</span>
                </p>
              ) : null}
              {nonEmptyText(blueprint.full_synopsis) ? (
                <p className="mt-0.5 line-clamp-2">{nonEmptyText(blueprint.full_synopsis)}</p>
              ) : null}
              {progress != null ? (
                <div className="mt-2 h-1 overflow-hidden rounded-full bg-book-bg-tertiary">
                  <div
                    className="h-full rounded-full bg-book-primary"
                    style={{ width: `${progress}%` }}
                  />
                </div>
              ) : null}
            </div>
          ) : (
            <p className="mt-1 text-book-text-muted">{t("writing.blueprintNotLoaded")}</p>
          )}
        </button>

        <button
          type="button"
          onClick={onOpenProfiles}
          className="a4n-flat-surface flex w-full items-center gap-2.5 px-3 py-2.5 text-left text-small text-book-text-sub transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/45 focus-visible:outline-2 focus-visible:outline-book-primary/35 disabled:cursor-default disabled:hover:border-book-border disabled:hover:bg-transparent"
          disabled={!onOpenProfiles}
          title={t("writing.openProfilesWorkspace")}
        >
          <div className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-book-bg-tertiary">
            {primaryPortraitSrc && primaryProfile?.name.trim() ? (
              <img
                src={primaryPortraitSrc}
                alt={primaryProfile.name.trim()}
                className="h-full w-full object-cover"
              />
            ) : primaryProfile?.name.trim() ? (
              <span className="font-semibold text-book-primary">
                {primaryProfile.name.trim().charAt(0)}
              </span>
            ) : (
              <User size={14} className="text-book-text-muted" />
            )}
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <Users size={12} className="shrink-0 text-book-primary" />
              <span className="font-semibold text-book-text-main">{t("characterDetail.profilesTitle")}</span>
              <span className="ml-auto text-caption text-book-text-muted">
                {profileCount > 0 ? t("writing.profileCountUnit", { count: formatNumber(profileCount, locale) }) : t("writing.noProfilesShort")}
              </span>
            </div>
            {profileError ? (
              <p className="mt-1 text-error">{t("writing.profileLoadUnavailable")}</p>
            ) : primaryProfile ? (
              <p className="mt-1 truncate text-book-text-muted">
                {primaryProfile.name}
                {primaryPersonality ? ` · ${primaryPersonality}` : ""}
              </p>
            ) : (
              <p className="mt-1 text-book-text-muted">{t("writing.openWorkspaceToCreateProfile")}</p>
            )}
          </div>
          <ChevronRight size={13} className="shrink-0 text-book-text-muted" />
        </button>
      </div>

      <Modal
        isOpen={blueprintDetailOpen && !!blueprint}
        onClose={() => setBlueprintDetailOpen(false)}
        title={t("writing.storyBlueprint")}
        icon={<Sparkles size={14} />}
        maxWidthClassName="max-w-2xl"
      >
        {blueprint && (
          <div className="space-y-3">
            {blueprintField(t("writing.blueprintFieldTitle"), blueprint.title, locale)}
            {blueprintField(t("writing.blueprintFieldGenre"), blueprint.genre, locale)}
            {blueprintField(t("writing.blueprintFieldOneSentence"), blueprint.one_sentence_summary, locale)}
            {blueprintField(t("writing.blueprintFieldSynopsis"), blueprint.full_synopsis, locale)}
            {blueprintField(t("writing.blueprintFieldStyle"), blueprint.style, locale)}
            {blueprintField(t("writing.blueprintFieldTone"), blueprint.tone, locale)}
            {blueprintField(t("writing.blueprintFieldAudience"), blueprint.target_audience, locale)}
            {blueprintField(t("writing.blueprintFieldTotalChapters"), blueprint.total_chapters, locale)}
            {blueprintField(t("writing.blueprintFieldChaptersPerPart"), blueprint.chapters_per_part, locale)}
          </div>
        )}
      </Modal>
    </>
  );
};
