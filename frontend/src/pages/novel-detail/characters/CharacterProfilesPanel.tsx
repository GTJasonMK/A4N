import React, { useEffect, useState } from "react";
import { ChevronRight } from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import { CharacterDetailDialog } from "../../CharacterDetailDialog";
import {
  characterAttributeEntries,
  characterAttributeValueText,
} from "../../../utils/characterAttributes";
import { attributeKeyLabel } from "../../character-detail-dialog/manageViewUtils";
import { EmptyHint } from "../shared";

const profileDescription = (profile: CharacterProfile): string =>
  [profile.personality, profile.background, profile.appearance]
    .map((value) => value?.trim())
    .find((value): value is string => !!value) ?? "暂无角色摘要。";

const syncedChapterText = (chapter: number): string =>
  chapter > 0 ? `同步至第 ${chapter} 章` : "未同步";

const visibleAttributeEntries = (profile: CharacterProfile): Array<[string, string]> =>
  [
    ...characterAttributeEntries(profile, "explicit"),
    ...characterAttributeEntries(profile, "implicit"),
    ...characterAttributeEntries(profile, "social"),
  ]
    .map(([key, value]) => [attributeKeyLabel(key), characterAttributeValueText(value)] as [string, string])
    .filter(([, value]) => value && !value.trim().startsWith("{") && !value.trim().startsWith("["))
    .slice(0, 3);

/**
 * 角色档案：每个角色保持独立边界，点击卡片进入详情。
 */
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

  if (profiles.length === 0) {
    return <EmptyHint text="尚未从正文分析出角色档案。" />;
  }

  const latestSyncedChapter = profiles.reduce(
    (latest, profile) => Math.max(latest, profile.last_synced_chapter),
    0,
  );

  return (
    <>
      <section>
        <header className="mb-3 px-1">
          <h2 className="text-sm font-semibold text-book-text-main">角色档案</h2>
          <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
            共 {profiles.length} 位角色
            {latestSyncedChapter > 0 ? `，最新同步至第 ${latestSyncedChapter} 章` : ""}
          </p>
        </header>
        <ul className="grid grid-cols-1 items-start gap-2 sm:grid-cols-2">
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
                className="a4n-flat-surface flex min-h-20 cursor-pointer items-start gap-3 p-3 transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/35 focus-visible:outline-2 focus-visible:outline-book-primary/35"
                title="查看角色详情"
              >
                <span className="mt-0.5 inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-book-primary/10 text-xs font-semibold text-book-primary">
                  {profile.name.trim().charAt(0).toUpperCase() || "?"}
                </span>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-semibold text-book-text-main">{profile.name}</div>
                  <div className="mt-0.5 text-caption text-book-text-muted">
                    {[profile.gender, profile.age, syncedChapterText(profile.last_synced_chapter)]
                      .filter(Boolean)
                      .join(" · ")}
                  </div>
                  <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-sub">
                    {profileDescription(profile)}
                  </p>
                  {attributes.length > 0 ? (
                    <p className="mt-1 truncate text-caption text-book-text-muted">
                      {attributes.map(([key, value]) => `${key}：${value}`).join(" · ")}
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
      />
    </>
  );
};
