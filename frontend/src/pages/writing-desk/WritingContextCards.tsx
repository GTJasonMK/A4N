import React, { useEffect, useState } from "react";
import { ChevronRight, Sparkles, User, Users } from "lucide-react";
import { extractErrorMessage } from "../../api";
import { getBlueprint, type Blueprint } from "../../api/blueprint";
import { listProfiles, type CharacterProfile } from "../../api/character";
import { getPrimaryPortrait, type Portrait } from "../../api/portrait";
import { useToast } from "../../components/feedback/Toast";
import { Modal } from "../../components/ui/Modal";
import { toMediaSrc } from "../../utils/mediaSrc";
import { nonEmptyText } from "../../utils/text";

/** 写作侧栏的蓝图与角色直接入口。 */

interface WritingContextCardsProps {
  projectId: number;
  chapterCount?: number;
  totalChapters?: number;
  refreshKey?: number;
  onOpenProfiles?: () => void;
}

const blueprintField = (label: string, value: string | number | null | undefined) => {
  const text = typeof value === "number" ? String(value) : nonEmptyText(value);
  if (!text) return null;
  return (
    <section className="border-b border-book-border/30 pb-3 last:border-b-0 last:pb-0">
      <div className="mb-1 text-[11px] font-semibold text-book-text-muted">{label}</div>
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
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [blueprintError, setBlueprintError] = useState<string | null>(null);
  const [primaryProfile, setPrimaryProfile] = useState<CharacterProfile | null>(null);
  const [profileCount, setProfileCount] = useState(0);
  const [portrait, setPortrait] = useState<Portrait | null>(null);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [blueprintDetailOpen, setBlueprintDetailOpen] = useState(false);
  const primaryPortraitSrc = toMediaSrc(portrait?.image_path);

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
          const message = extractErrorMessage(err, "加载故事蓝图失败");
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
          const message = extractErrorMessage(err, "加载角色档案失败");
          setProfileError(message);
          addToast(message, "error");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [projectId, refreshKey, addToast]);

  const progress =
    totalChapters > 0 ? Math.round((chapterCount / totalChapters) * 100) : null;

  return (
    <>
      <div className="min-w-0 space-y-2">
        <button
          type="button"
          onClick={() => setBlueprintDetailOpen(true)}
          className="a4n-flat-surface block w-full px-3 py-2.5 text-left text-[11px] transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/45 focus-visible:outline-2 focus-visible:outline-book-primary/35 disabled:cursor-default disabled:hover:border-book-border disabled:hover:bg-transparent"
          disabled={!blueprint || !!blueprintError}
          title="查看故事蓝图"
        >
          <div className="flex items-center gap-2">
            <Sparkles size={12} className="shrink-0 text-book-primary" />
            <span className="font-semibold text-book-text-main">故事蓝图</span>
            {progress != null ? (
              <span className="ml-auto text-[10px] text-book-text-muted">
                {chapterCount} / {totalChapters} 章
              </span>
            ) : null}
            <ChevronRight size={13} className="shrink-0 text-book-text-muted" />
          </div>
          {blueprintError ? (
            <p className="mt-1 text-error">故事蓝图暂时无法加载。</p>
          ) : blueprint ? (
            <div className="mt-1 min-w-0 leading-relaxed text-book-text-sub">
              {nonEmptyText(blueprint.style) ? (
                <p className="truncate text-book-text-muted">
                  风格：<span className="text-book-text-main">{nonEmptyText(blueprint.style)}</span>
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
            <p className="mt-1 text-book-text-muted">故事蓝图未加载</p>
          )}
        </button>

        <button
          type="button"
          onClick={onOpenProfiles}
          className="a4n-flat-surface flex w-full items-center gap-2.5 px-3 py-2.5 text-left text-[11px] text-book-text-sub transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/45 focus-visible:outline-2 focus-visible:outline-book-primary/35 disabled:cursor-default disabled:hover:border-book-border disabled:hover:bg-transparent"
          disabled={!onOpenProfiles}
          title="打开角色档案工作台"
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
              <span className="font-semibold text-book-text-main">角色档案</span>
              <span className="ml-auto text-[10px] text-book-text-muted">
                {profileCount > 0 ? `${profileCount} 位` : "尚无角色"}
              </span>
            </div>
            {profileError ? (
              <p className="mt-1 text-error">角色档案暂时无法加载。</p>
            ) : primaryProfile ? (
              <p className="mt-1 truncate text-book-text-muted">
                {primaryProfile.name}
                {primaryProfile.personality ? ` · ${primaryProfile.personality}` : ""}
              </p>
            ) : (
              <p className="mt-1 text-book-text-muted">打开工作台建立角色档案</p>
            )}
          </div>
          <ChevronRight size={13} className="shrink-0 text-book-text-muted" />
        </button>
      </div>

      <Modal
        isOpen={blueprintDetailOpen && !!blueprint}
        onClose={() => setBlueprintDetailOpen(false)}
        title="故事蓝图"
        icon={<Sparkles size={14} />}
        maxWidthClassName="max-w-2xl"
      >
        {blueprint && (
          <div className="space-y-3">
            {blueprintField("标题", blueprint.title)}
            {blueprintField("类型", blueprint.genre)}
            {blueprintField("一句话简介", blueprint.one_sentence_summary)}
            {blueprintField("完整梗概", blueprint.full_synopsis)}
            {blueprintField("风格", blueprint.style)}
            {blueprintField("语气", blueprint.tone)}
            {blueprintField("目标读者", blueprint.target_audience)}
            {blueprintField("规划章节数", blueprint.total_chapters)}
            {blueprintField("每部分章节数", blueprint.chapters_per_part)}
          </div>
        )}
      </Modal>
    </>
  );
};
