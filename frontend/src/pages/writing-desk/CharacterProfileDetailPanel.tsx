import React from "react";
import { Sparkles, Trash2, User, UserPlus } from "lucide-react";
import { type CharacterProfile } from "../../api/character";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import {
  CHARACTER_ATTRIBUTE_LABELS,
  characterAttributeEntries,
  type CharacterAttributeCategory,
} from "../../utils/characterAttributes";
import {
  AttributePreview,
  ProfileField,
  describeProfile,
} from "./CharacterProfileDisplay";

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
  if (!profile) {
    return (
      <section className="min-h-0 min-w-0 p-4" aria-label="人物档案详情">
        <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
          <User size={28} className="text-book-text-muted" />
          <div className="text-sm font-semibold text-book-text-main">尚无角色档案</div>
          <BookButton variant="primary" onClick={onCreate}>
            <span className="inline-flex items-center gap-1.5">
              <UserPlus size={14} /> 新建角色
            </span>
          </BookButton>
        </div>
      </section>
    );
  }

  const profileMeta = [
    profile.gender,
    profile.age,
    profile.last_synced_chapter > 0
      ? `已同步至第 ${profile.last_synced_chapter} 章`
      : "尚未同步章节状态",
  ].filter((value): value is string => Boolean(value));

  return (
    <section className="min-h-0 min-w-0 p-4" aria-label="人物档案详情">
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
                  label: "删除角色档案",
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
              <Sparkles size={13} /> 深度详情
            </BookButton>
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto pr-1 text-xs custom-scrollbar">
          <div className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
            {describeProfile(profile) ? (
              <section className="px-3 py-3 sm:px-4">
                <h3 className="mb-2 text-small font-semibold text-book-text-main">角色概况</h3>
                <dl className="space-y-2">
                  <ProfileField label="性格" value={profile.personality} />
                  <ProfileField label="背景" value={profile.background} />
                  <ProfileField label="外观" value={profile.appearance} />
                </dl>
              </section>
            ) : null}
            <AttributePreview
              label={CHARACTER_ATTRIBUTE_LABELS.explicit}
              category="explicit"
              entries={characterAttributeEntries(profile, "explicit")}
              onOpenHistory={(category, key) => onOpenHistory(profile, category, key)}
            />
            <AttributePreview
              label={CHARACTER_ATTRIBUTE_LABELS.implicit}
              category="implicit"
              entries={characterAttributeEntries(profile, "implicit")}
              onOpenHistory={(category, key) => onOpenHistory(profile, category, key)}
            />
            <AttributePreview
              label={CHARACTER_ATTRIBUTE_LABELS.social}
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
