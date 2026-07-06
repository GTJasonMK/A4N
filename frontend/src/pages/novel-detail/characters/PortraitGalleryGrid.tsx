import React, { useMemo } from "react";
import { MoreHorizontal, Star } from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import type { Portrait } from "../../../api/portrait";
import { BookCard } from "../../../components/ui/BookCard";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { EmptyHint } from "../shared";

interface PortraitGalleryGridProps {
  portraits: Portrait[];
  profiles: CharacterProfile[];
  busyId: number | null;
  onOpenActions: (portrait: Portrait) => void;
}

export const PortraitGalleryGrid: React.FC<PortraitGalleryGridProps> = ({
  portraits,
  profiles,
  busyId,
  onOpenActions,
}) => {
  const profilesById = useMemo(
    () => new Map(profiles.map((profile) => [profile.id, profile])),
    [profiles],
  );

  if (portraits.length === 0) {
    return (
      <BookCard className="p-8">
        <EmptyHint text="尚未生成任何人物三视图拆解。建立角色档案后，点击右上按钮可批量生成。" />
      </BookCard>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
      {portraits.map((portrait) => {
        const profile = profilesById.get(portrait.profile_id);
        const busy = busyId === portrait.id;
        const imageSrc = toMediaSrc(portrait.image_path);

        return (
          <div
            key={portrait.id}
            className="a4n-stat-tile flex flex-col p-2"
          >
            <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden rounded bg-book-bg-tertiary text-xs text-book-text-muted">
              {imageSrc ? (
                <img
                  src={imageSrc}
                  alt={`人物三视图拆解 ${portrait.id}`}
                  className="h-full w-full object-cover"
                />
              ) : (
                "（无图）"
              )}
              {portrait.is_primary ? (
                <span className="absolute right-1 top-1 inline-flex items-center gap-0.5 rounded bg-book-primary px-1.5 py-0.5 text-[10px] text-white">
                  <Star size={9} fill="currentColor" /> 主图
                </span>
              ) : null}
            </div>
            {profile ? (
              <div className="mt-1.5 truncate text-xs text-book-text-main">{profile.name}</div>
            ) : null}
            <div className="mt-1 flex flex-wrap items-center gap-1 text-xs text-book-text-muted">
              {portrait.portrait_type ? (
                <span className="pyqt-pill">{portrait.portrait_type}</span>
              ) : null}
              {portrait.expression ? (
                <span className="pyqt-pill">{portrait.expression}</span>
              ) : null}
            </div>
            <div className="mt-1.5 flex flex-wrap items-center gap-1">
              <span className="flex-1" />
              <button
                type="button"
                className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
                onClick={() => onOpenActions(portrait)}
                disabled={busy}
                title={`管理人物三视图拆解 #${portrait.id}`}
              >
                <MoreHorizontal size={13} />
              </button>
            </div>
          </div>
        );
      })}
    </div>
  );
};
