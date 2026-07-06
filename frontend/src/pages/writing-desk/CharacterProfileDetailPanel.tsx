import React from "react";
import { Sparkles, Trash2, User, UserPlus } from "lucide-react";
import { type CharacterProfile } from "../../api/character";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
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
      <main className="min-h-0 p-4">
        <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
          <User size={28} className="text-book-text-muted" />
          <div className="text-sm font-semibold text-book-text-main">尚无角色档案</div>
          <BookButton variant="primary" onClick={onCreate}>
            <span className="inline-flex items-center gap-1.5">
              <UserPlus size={14} /> 新建角色
            </span>
          </BookButton>
        </div>
      </main>
    );
  }

  const description = describeProfile(profile);

  return (
    <main className="min-h-0 p-4">
      <div className="flex h-full min-h-0 flex-col">
        <div className="mb-3 flex flex-col gap-3 border-b border-book-border/50 pb-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-1.5">
              <h3 className="truncate font-serif text-lg font-semibold text-book-text-main">
                {profile.name}
              </h3>
              {profile.gender && (
                <span className="pyqt-pill !text-[10px]">{profile.gender}</span>
              )}
              {profile.age && <span className="pyqt-pill !text-[10px]">{profile.age}</span>}
              <span className="pyqt-pill !text-[10px]">
                已同步第 {profile.last_synced_chapter} 章
              </span>
            </div>
            {description && (
              <p className="mt-1 max-w-2xl text-xs leading-relaxed text-book-text-muted">
                {description}
              </p>
            )}
          </div>
          <div className="flex shrink-0 flex-wrap items-center gap-1.5">
            <BookButton variant="secondary" size="sm" onClick={() => onOpenDetail(profile)}>
              <span className="inline-flex items-center gap-1.5">
                <Sparkles size={13} /> 深度详情
              </span>
            </BookButton>
            <BookIconButton
              label="删除角色档案"
              onClick={() => void onDelete(profile)}
              variant="secondary"
              size="sm"
              className="!border-error/50 !text-error hover:!bg-error-bg dark:hover:!bg-error-bg"
            >
              <Trash2 size={12} />
            </BookIconButton>
          </div>
        </div>

        <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 overflow-y-auto pr-1 text-xs md:grid-cols-3 custom-scrollbar">
          <ProfileField label="性格" value={profile.personality} />
          <ProfileField label="背景" value={profile.background} />
          <ProfileField label="外观" value={profile.appearance} />
          <ProfileField
            label="档案统计"
            value={`显性 ${profile.attribute_counts.explicit} / 隐性 ${profile.attribute_counts.implicit} / 社会关系 ${profile.attribute_counts.social}`}
          />
          <ProfileField label="同步状态" value={`第 ${profile.last_synced_chapter} 章`} />
          <ProfileField
            label="详情入口"
            value="事件溯源、行为证据、隐性复审、删除标记、快照和回滚集中在深度详情中。"
          />
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
    </main>
  );
};
