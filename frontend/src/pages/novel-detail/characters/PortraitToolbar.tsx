import React from "react";
import { Camera, Eye, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import type { CharacterProfile } from "../../../api/character";

interface PortraitToolbarProps {
  visiblePortraitCount: number;
  totalPortraitCount: number;
  profiles: CharacterProfile[];
  selectedProfileId: number | null;
  generating: boolean;
  onProfileChange: (profileId: number | null) => void;
  onOpenSingle: () => void;
  onOpenUpload: () => void;
  onPreviewGenerateMissing: () => void | Promise<void>;
  onGenerateMissing: () => void | Promise<void>;
}

export const PortraitToolbar: React.FC<PortraitToolbarProps> = ({
  visiblePortraitCount,
  totalPortraitCount,
  profiles,
  selectedProfileId,
  generating,
  onProfileChange,
  onOpenSingle,
  onOpenUpload,
  onPreviewGenerateMissing,
  onGenerateMissing,
}) => {
  const selectedProfile =
    selectedProfileId == null
      ? null
      : profiles.find((profile) => profile.id === selectedProfileId) ?? null;

  return (
    <section className="a4n-flat-surface overflow-hidden">
      <header className="flex flex-col gap-2 px-3 pb-3 pt-3 sm:flex-row sm:items-center sm:justify-between sm:px-4">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold text-book-text-main">立绘素材</h2>
          <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
            {selectedProfile
              ? `${selectedProfile.name} ${visiblePortraitCount} 张，全项目共 ${totalPortraitCount} 张`
              : `共 ${totalPortraitCount} 张立绘，来自 ${profiles.length} 位角色档案`}
          </p>
        </div>
        <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
          {!generating ? (
            <Dropdown
              items={[
                {
                  label: "生成单个角色立绘",
                  icon: <Camera size={12} />,
                  onClick: onOpenSingle,
                },
                {
                  label: "上传立绘素材",
                  icon: <Upload size={12} />,
                  onClick: onOpenUpload,
                },
                { type: "divider" },
                {
                  label: "查看批量补缺模型请求",
                  icon: <Eye size={12} />,
                  onClick: () => void onPreviewGenerateMissing(),
                },
              ]}
            />
          ) : null}
          <BookButton
            variant="primary"
            size="sm"
            onClick={() => void onGenerateMissing()}
            disabled={generating}
            className="min-w-0 flex-1 !text-xs sm:flex-none"
          >
            {generating ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" /> 生成中…
              </span>
            ) : (
              "批量补缺"
            )}
          </BookButton>
        </div>
      </header>
      {profiles.length > 0 ? (
        <div className="border-t border-book-border/40 px-3 py-2.5 sm:px-4">
        <div className="a4n-filter-chips custom-scrollbar" aria-label="按角色筛选立绘">
          <button
            type="button"
            className="a4n-filter-chip"
            data-active={selectedProfileId == null ? "1" : undefined}
            aria-pressed={selectedProfileId == null}
            onClick={() => onProfileChange(null)}
          >
            全部
          </button>
          {profiles.map((profile) => (
            <button
              key={profile.id}
              type="button"
              className="a4n-filter-chip"
              data-active={selectedProfileId === profile.id ? "1" : undefined}
              aria-pressed={selectedProfileId === profile.id}
              onClick={() => onProfileChange(profile.id)}
              title={`只看 ${profile.name} 的立绘`}
            >
              {profile.name}
            </button>
          ))}
        </div>
        </div>
      ) : null}
    </section>
  );
};
