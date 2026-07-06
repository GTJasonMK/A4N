import React from "react";
import { Eye, Loader2, RefreshCw, Search, UserPlus } from "lucide-react";
import { type CharacterProfile } from "../../api/character";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
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
  <aside className="min-h-0 border-b border-book-border bg-book-bg-paper/45 p-3 md:border-b-0 md:border-r">
    <div className="mb-3 flex items-center justify-between gap-2">
      <div className="min-w-0">
        <div className="text-[10px] font-semibold uppercase tracking-wide text-book-text-muted">
          角色档案
        </div>
        <div className="text-xs text-book-text-main">{profiles.length} 位角色</div>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <BookIconButton
          label="刷新角色档案"
          onClick={() => void onRefresh()}
          variant="secondary"
          size="sm"
          disabled={loading}
        >
          <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
        <BookIconButton
          label="新建角色"
          onClick={onCreate}
          variant="secondary"
          size="sm"
        >
          <UserPlus size={12} />
        </BookIconButton>
      </div>
    </div>

    <div className="mb-2 grid grid-cols-1 gap-1.5 sm:grid-cols-2 md:grid-cols-1 xl:grid-cols-2">
      <BookButton
        variant="secondary"
        size="sm"
        onClick={() => void onPreviewDiscover()}
        disabled={discovering || syncingAll || loading}
        className="justify-center"
      >
        <span className="inline-flex items-center gap-1.5">
          <Eye size={12} /> 预览发现
        </span>
      </BookButton>
      <BookButton
        variant="secondary"
        size="sm"
        onClick={() => void onDiscover()}
        disabled={discovering || syncingAll || loading}
        className="justify-center"
      >
        {discovering ? (
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
    <div className="mb-3 grid grid-cols-1 gap-1.5 sm:grid-cols-2 md:grid-cols-1 xl:grid-cols-2">
      <BookButton
        variant="secondary"
        size="sm"
        onClick={() => void onPreviewSyncAll()}
        disabled={syncingAll || discovering || loading || profiles.length === 0 || !canSyncAll}
        className="justify-center"
      >
        <span className="inline-flex items-center gap-1.5">
          <Eye size={12} /> 预览同步
        </span>
      </BookButton>
      <BookButton
        variant="secondary"
        size="sm"
        onClick={() => void onSyncAll()}
        disabled={syncingAll || discovering || loading || profiles.length === 0 || !canSyncAll}
        className="justify-center"
      >
        {syncingAll ? (
          <span className="inline-flex items-center gap-1.5">
            <Loader2 size={12} className="animate-spin" /> 同步中…
          </span>
        ) : (
          <span className="inline-flex items-center gap-1.5">
            <RefreshCw size={12} /> 同步全部
          </span>
        )}
      </BookButton>
    </div>

    {loading && profiles.length === 0 ? (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <Loader2 size={12} className="animate-spin" /> 加载中…
      </div>
    ) : profiles.length === 0 ? (
      <button
        type="button"
        onClick={onCreate}
        className="w-full rounded-sm border border-dashed border-book-border bg-book-bg-paper/40 px-3 py-6 text-center text-xs text-book-text-muted transition-colors hover:border-book-primary/60 hover:text-book-text-main"
      >
        创建第一个角色
      </button>
    ) : (
      <ul className="flex max-w-full gap-1.5 overflow-x-auto md:block md:space-y-1.5 md:overflow-x-visible custom-scrollbar">
        {profiles.map((profile) => {
          const active = selectedId === profile.id;
          const initial = profile.name.trim().charAt(0).toUpperCase();
          return (
            <li key={profile.id}>
              <button
                type="button"
                onClick={() => onSelect(profile.id)}
                data-active={active ? "1" : undefined}
                className="group flex min-w-[12rem] items-start gap-2 rounded-sm border border-book-border bg-book-bg-paper/55 px-2 py-2 text-left transition-colors hover:border-book-primary/55 hover:bg-book-bg-tertiary/45 data-[active=1]:border-book-primary/70 data-[active=1]:bg-book-primary/10 md:w-full md:min-w-0"
              >
                <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border border-book-primary/30 bg-book-primary/10 text-xs font-bold text-book-primary">
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
