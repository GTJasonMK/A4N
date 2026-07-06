import React, { useEffect, useState } from "react";
import { RotateCw, Sparkles, User, Users } from "lucide-react";
import { extractErrorMessage } from "../../api";
import { getBlueprint, type Blueprint } from "../../api/blueprint";
import { listProfiles, type CharacterProfile } from "../../api/character";
import { getPrimaryPortrait, type Portrait } from "../../api/portrait";
import { useToast } from "../../components/feedback/Toast";
import { Modal } from "../../components/ui/Modal";
import { toMediaSrc } from "../../utils/mediaSrc";
import { nonEmptyText } from "../../utils/text";

/**
 * 正反两面卡片：
 *   - 正面：故事蓝图（风格 + 摘要 + 进度）
 *   - 背面：角色档案入口（代表角色 + 档案数量）
 * 点击右上角翻转按钮切换。
 */

interface FlippableBlueprintCardProps {
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

export const FlippableBlueprintCard: React.FC<FlippableBlueprintCardProps> = ({
  projectId,
  chapterCount = 0,
  totalChapters = 0,
  refreshKey = 0,
  onOpenProfiles,
}) => {
  const { addToast } = useToast();
  const [side, setSide] = useState<"front" | "back">("front");
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
      <div className="pyqt-card flex h-full min-h-0 min-w-0 flex-col overflow-hidden p-2">
        <div className="mb-1 flex min-w-0 shrink-0 items-center gap-1.5">
          <span className="inline-flex min-w-0 flex-1 items-center gap-1 truncate text-[11px] font-semibold text-book-text-main">
            {side === "front" ? (
              <>
                <Sparkles size={11} className="text-book-primary" /> 故事蓝图
              </>
            ) : (
              <>
                <Users size={11} className="text-book-primary" /> 角色档案
              </>
            )}
          </span>
          <button
            type="button"
            onClick={() => setSide(side === "front" ? "back" : "front")}
            className="pyqt-btn-icon shrink-0 !h-8 !w-8 sm:!h-5 sm:!w-5"
            title="翻转卡片"
          >
            <RotateCw size={10} />
          </button>
        </div>

        {side === "front" ? (
          <button
            type="button"
            onClick={() => setBlueprintDetailOpen(true)}
            className="min-h-0 min-w-0 flex-1 overflow-hidden rounded-sm text-left text-[11px] leading-relaxed text-book-text-sub transition-colors hover:bg-book-bg-tertiary/45 focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary disabled:cursor-default disabled:hover:bg-transparent"
            disabled={!blueprint || !!blueprintError}
            title="查看故事蓝图"
          >
            {blueprintError ? (
              <p className="text-error">{blueprintError}</p>
            ) : blueprint ? (
              <>
                {nonEmptyText(blueprint.style) && (
                  <div className="truncate text-book-text-muted">
                    风格：<span className="text-book-text-main">{nonEmptyText(blueprint.style)}</span>
                  </div>
                )}
                {nonEmptyText(blueprint.full_synopsis) && (
                  <p className="mt-0.5 line-clamp-2">{nonEmptyText(blueprint.full_synopsis)}</p>
                )}
                {progress != null && (
                  <div className="mt-1.5">
                    <div className="flex min-w-0 items-center gap-1.5 text-[10px] text-book-text-muted">
                      <span>进度</span>
                      <span className="min-w-0 flex-1 truncate text-right">
                        {chapterCount} / {totalChapters}
                      </span>
                    </div>
                    <div className="mt-0.5 h-1 overflow-hidden rounded-sm border border-book-border bg-book-bg-tertiary">
                      <div className="h-full bg-book-primary" style={{ width: `${progress}%` }} />
                    </div>
                  </div>
                )}
              </>
            ) : (
              <p className="text-book-text-muted">故事蓝图未加载</p>
            )}
          </button>
        ) : (
          <button
            type="button"
            onClick={onOpenProfiles}
            className="flex min-h-0 min-w-0 flex-1 gap-2 overflow-hidden rounded-sm text-left text-[11px] text-book-text-sub transition-colors hover:bg-book-bg-tertiary/45 disabled:cursor-default disabled:hover:bg-transparent"
            disabled={!onOpenProfiles}
            title="打开角色档案工作台"
          >
            <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-sm border border-book-border bg-book-bg-tertiary">
              {primaryPortraitSrc && primaryProfile?.name.trim() ? (
                <img
                  src={primaryPortraitSrc}
                  alt={primaryProfile.name.trim()}
                  className="h-full w-full object-cover"
                />
              ) : (
                <User size={14} className="text-book-text-muted" />
              )}
            </div>
            <div className="min-w-0 flex-1">
              {profileError ? (
                <p className="text-error">{profileError}</p>
              ) : primaryProfile ? (
                <>
                  <div className="truncate font-semibold text-book-text-main">{primaryProfile.name}</div>
                  {primaryProfile.personality && (
                    <p className="mt-0.5 line-clamp-2 text-[10px]">{primaryProfile.personality}</p>
                  )}
                  <p className="mt-0.5 text-[10px] text-book-text-muted">共 {profileCount} 位角色</p>
                </>
              ) : (
                <p className="text-book-text-muted italic">尚无角色档案</p>
              )}
            </div>
          </button>
        )}
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
