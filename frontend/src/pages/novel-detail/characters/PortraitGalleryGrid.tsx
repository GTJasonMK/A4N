import React, { useMemo } from "react";
import { Star } from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import type { Portrait } from "../../../api/portrait";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { EmptyHint } from "../shared";

interface PortraitGalleryGridProps {
  portraits: Portrait[];
  profiles: CharacterProfile[];
  busyId: number | null;
  onOpenActions: (portrait: Portrait) => void;
  emptyText?: string;
}

export const PortraitGalleryGrid: React.FC<PortraitGalleryGridProps> = ({
  portraits,
  profiles,
  busyId,
  onOpenActions,
  emptyText,
}) => {
  const profilesById = useMemo(
    () => new Map(profiles.map((profile) => [profile.id, profile])),
    [profiles],
  );

  if (portraits.length === 0) {
    return (
      <div className="py-8">
        <EmptyHint text={emptyText ?? "尚未生成任何人物三视图拆解。建立角色档案后，点击右上按钮可批量生成。"} />
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
          <button
            key={portrait.id}
            type="button"
            onClick={() => onOpenActions(portrait)}
            disabled={busy}
            className="group min-w-0 overflow-hidden rounded-[14px] border border-book-border/55 bg-book-bg-paper/45 text-left transition-colors hover:border-book-primary/45 disabled:cursor-wait disabled:opacity-60"
            title="管理人物三视图拆解"
          >
            <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden bg-book-bg-tertiary text-xs text-book-text-muted">
              {imageSrc ? (
                <img
                  src={imageSrc}
                  alt={profile?.name ? `${profile.name}人物三视图拆解` : "人物三视图拆解"}
                  className="h-full w-full object-cover transition-transform duration-300 ease-out group-hover:scale-[1.02]"
                />
              ) : (
                "无图"
              )}
              {portrait.is_primary ? (
                <span className="absolute right-1.5 top-1.5 inline-flex items-center gap-1 rounded-md bg-book-primary/90 px-2 py-1 text-caption text-white backdrop-blur-sm">
                  <Star size={9} fill="currentColor" /> 主图
                </span>
              ) : null}
            </div>
            <div className="p-2.5">
              <div className="truncate text-xs font-medium text-book-text-main">
                {profile?.name ?? "未关联角色"}
              </div>
              <div className="mt-1 flex min-w-0 items-center gap-2 text-caption text-book-text-muted">
                {portrait.portrait_type ? <span className="truncate">{portrait.portrait_type}</span> : null}
                {portrait.expression ? <span className="truncate">{portrait.expression}</span> : null}
              </div>
            </div>
          </button>
        );
      })}
    </div>
  );
};
