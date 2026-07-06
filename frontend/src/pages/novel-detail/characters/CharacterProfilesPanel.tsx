import React, { useEffect, useState } from "react";
import { Sparkles } from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import { BookCard } from "../../../components/ui/BookCard";
import { CharacterDetailDialog } from "../../CharacterDetailDialog";
import {
  characterAttributeEntries,
  characterAttributeValueText,
} from "../../../utils/characterAttributes";
import { EmptyHint } from "../shared";

const profileDescription = (profile: CharacterProfile): string =>
  [profile.personality, profile.background, profile.appearance]
    .map((value) => value?.trim())
    .find((value): value is string => !!value) ?? "暂无角色摘要。";

const attributeCountText = (profile: CharacterProfile): string =>
  `显性 ${profile.attribute_counts.explicit} / 隐性 ${profile.attribute_counts.implicit} / 社会关系 ${profile.attribute_counts.social}`;

const syncedChapterText = (chapter: number): string =>
  chapter > 0 ? `已同步第 ${chapter} 章` : "未同步";

const visibleAttributeEntries = (profile: CharacterProfile): Array<[string, string]> =>
  [
    ...characterAttributeEntries(profile, "explicit"),
    ...characterAttributeEntries(profile, "implicit"),
    ...characterAttributeEntries(profile, "social"),
  ]
    .map(([key, value]) => [key, characterAttributeValueText(value)] as [string, string])
    .filter(([, value]) => value)
    .slice(0, 4);

export const CharacterProfilesPanel: React.FC<{
  projectId: number;
  profiles: CharacterProfile[];
  onChanged: () => void | Promise<void>;
}> = ({ projectId, profiles, onChanged }) => {
  const [detailFor, setDetailFor] = useState<CharacterProfile | null>(null);

  useEffect(() => {
    setDetailFor((current) => {
      if (!current) return null;
      return profiles.find((profile) => profile.id === current.id) ?? null;
    });
  }, [profiles]);

  return (
    <>
      <BookCard className="p-5">
        <h3 className="mb-3 font-serif text-sm text-book-text-main">
          角色档案 <span className="text-xs text-book-text-muted">({profiles.length})</span>
        </h3>
        {profiles.length === 0 ? (
          <EmptyHint text="尚未从正文分析出角色档案。" />
        ) : (
          <ul className="space-y-1.5">
            {profiles.map((profile) => {
              const attributes = visibleAttributeEntries(profile);
              return (
                <li
                  key={profile.id}
                  role="button"
                  tabIndex={0}
                  onClick={() => setDetailFor(profile)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      setDetailFor(profile);
                    }
                  }}
                  className="flex cursor-pointer items-start gap-3 rounded-sm border border-book-border bg-book-bg-paper px-3 py-2 transition-colors hover:border-book-primary/60 hover:bg-book-bg-tertiary/40"
                  title="查看角色详情"
                >
                  <div className="mt-0.5 flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-sm border border-book-primary/30 bg-book-primary/10 text-xs font-bold text-book-primary">
                    {profile.name.trim().charAt(0).toUpperCase() || "?"}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-1.5">
                      <span className="text-sm font-semibold text-book-text-main">
                        {profile.name}
                      </span>
                      {profile.gender ? (
                        <span className="pyqt-pill !text-[10px]">{profile.gender}</span>
                      ) : null}
                      {profile.age ? (
                        <span className="pyqt-pill !text-[10px]">{profile.age}</span>
                      ) : null}
                      <span className="pyqt-pill !text-[10px]">
                        {syncedChapterText(profile.last_synced_chapter)}
                      </span>
                    </div>
                    <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-book-text-muted">
                      {profileDescription(profile)}
                    </p>
                    <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[10px] text-book-text-muted">
                      <span>{attributeCountText(profile)}</span>
                      {attributes.map(([key, value]) => (
                        <span key={key} className="pyqt-pill !text-[10px]" title={value}>
                          {key}
                        </span>
                      ))}
                    </div>
                  </div>
                  <Sparkles size={13} className="mt-1 shrink-0 text-book-text-muted" />
                </li>
              );
            })}
          </ul>
        )}
      </BookCard>

      <CharacterDetailDialog
        open={detailFor != null}
        onClose={() => setDetailFor(null)}
        projectId={projectId}
        character={detailFor}
        onChanged={onChanged}
      />
    </>
  );
};
