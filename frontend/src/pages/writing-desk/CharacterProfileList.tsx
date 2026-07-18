import React from "react";
import { Eye, Loader2, RefreshCw, Search, UserPlus } from "lucide-react";
import { type CharacterProfile } from "../../api/character";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { describeProfile } from "./CharacterProfileDisplay";

export const CharacterProfileList: React.FC<{
  profiles: CharacterProfile[];
  selectedId: number | null;
  loading: boolean;
  discovering: boolean;
  syncingAll: boolean;
  canSyncAll: boolean;
  onRefresh: () => void | Promise<void>;
  onCreate: () => void;
  onPreviewDiscover: () => void | Promise<void>;
  onDiscover: () => void | Promise<void>;
  onPreviewSyncAll: () => void | Promise<void>;
  onSyncAll: () => void | Promise<void>;
  onSelect: (profileId: number) => void;
}> = ({
  profiles,
  selectedId,
  loading,
  discovering,
  syncingAll,
  canSyncAll,
  onRefresh,
  onCreate,
  onPreviewDiscover,
  onDiscover,
  onPreviewSyncAll,
  onSyncAll,
  onSelect,
}) => (
  <aside className="flex min-h-0 min-w-0 flex-col border-b border-book-border bg-book-bg-paper/45 p-3 md:border-b-0 md:border-r">
    <div className="mb-3 flex items-center gap-2">
      <p className="min-w-0 flex-1 text-[11px] text-book-text-muted">
        {profiles.length > 0 ? `${profiles.length} 位角色` : "尚无角色"}
      </p>
      <Dropdown
        items={[
          {
            label: "刷新角色档案",
            icon: <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />,
            disabled: loading || discovering || syncingAll,
            onClick: () => void onRefresh(),
          },
          {
            label: "新建角色",
            icon: <UserPlus size={12} />,
            disabled: discovering || syncingAll,
            onClick: onCreate,
          },
          { type: "divider" },
          {
            label: "预览角色发现请求",
            icon: <Eye size={12} />,
            disabled: discovering || syncingAll || loading,
            onClick: () => void onPreviewDiscover(),
          },
          {
            label: "预览全部同步请求",
            icon: <Eye size={12} />,
            disabled: syncingAll || discovering || loading || profiles.length === 0 || !canSyncAll,
            onClick: () => void onPreviewSyncAll(),
          },
          {
            label: "同步全部角色",
            icon: <RefreshCw size={12} />,
            disabled: syncingAll || discovering || loading || profiles.length === 0 || !canSyncAll,
            onClick: () => void onSyncAll(),
          },
        ]}
      />
      <BookButton
        variant="primary"
        size="sm"
        onClick={() => void onDiscover()}
        disabled={discovering || syncingAll || loading}
        className="min-w-0 flex-1 md:flex-none"
      >
        {syncingAll ? (
          <span className="inline-flex items-center gap-1.5">
            <Loader2 size={12} className="animate-spin" /> 同步中…
          </span>
        ) : discovering ? (
          <span className="inline-flex items-center gap-1.5">
            <Loader2 size={12} className="animate-spin" /> 发现中…
          </span>
        ) : (
          <span className="inline-flex items-center gap-1.5">
            <Search size={12} /> 发现角色
          </span>
        )}
      </BookButton>
    </div>

    {loading && profiles.length === 0 ? (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <Loader2 size={12} className="animate-spin" /> 加载中…
      </div>
    ) : profiles.length === 0 ? (
      <div className="flex flex-col items-center gap-3 py-6 text-center">
        <p className="text-xs text-book-text-muted">可以自动发现，也可以手动建立第一个角色。</p>
        <BookButton variant="secondary" size="sm" onClick={onCreate}>
          <UserPlus size={12} /> 新建角色
        </BookButton>
      </div>
    ) : (
      <ul
        aria-label="角色档案"
        className="a4n-mobile-scroll-rail custom-scrollbar flex max-w-full gap-2 overflow-x-auto pb-1 md:min-h-0 md:flex-1 md:flex-col md:overflow-x-hidden md:overflow-y-auto md:pb-0 md:pr-1"
      >
        {profiles.map((profile) => {
          const active = selectedId === profile.id;
          const initial = profile.name.trim().charAt(0).toUpperCase();
          return (
            <li key={profile.id}>
              <button
                type="button"
                onClick={() => onSelect(profile.id)}
                aria-pressed={active}
                data-active={active ? "1" : undefined}
                className="a4n-flat-surface group flex min-w-[11rem] items-start gap-2 px-2.5 py-2 text-left transition-colors hover:border-book-primary/55 hover:bg-book-bg-tertiary/45 data-[active=1]:border-book-primary/70 data-[active=1]:bg-book-primary/10 md:w-full md:min-w-0 md:px-3 md:py-2.5"
              >
                <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-book-primary/10 text-xs font-semibold text-book-primary">
                  {initial}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="flex items-center gap-1.5">
                    <span className="truncate text-xs font-semibold text-book-text-main">
                      {profile.name}
                    </span>
                  </span>
                  {describeProfile(profile) && (
                    <span className="mt-0.5 block truncate text-[10px] text-book-text-muted">
                      {describeProfile(profile)}
                    </span>
                  )}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    )}
  </aside>
);
