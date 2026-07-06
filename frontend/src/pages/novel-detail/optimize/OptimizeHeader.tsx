import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Bot, Check, ChevronDown, Loader2, Plus, RefreshCw, Trash2 } from "lucide-react";
import type { ChapterData } from "../../../api/chapter";
import type { OptimizeSession } from "../../../api/optimize";
import { getPortalRoot } from "../../../components/ui/portalRoot";
import {
  chapterOptionLabel,
  sessionDisplayName,
  statusLabel,
} from "./optimizeTabUtils";

interface OptimizeHeaderProps {
  activeChapter: ChapterData | null;
  activeSession: OptimizeSession | null;
  activeSessionId: number | null;
  chapterSessions: OptimizeSession[];
  chapters: ChapterData[];
  isPinned: boolean;
  loading: boolean;
  selectedChapter: number | "";
  taskRunning: boolean;
  onDeleteSession: () => void;
  onNewChatSession: () => void;
  onRefresh: () => void;
  onSelectChapter: (chapterId: number | "") => void;
  onSelectSession: (sessionId: number) => void;
}

type SelectValue = string | number;

interface HeaderSelectOption<T extends SelectValue> {
  value: T;
  label: string;
  description?: string;
}

interface HeaderSelectProps<T extends SelectValue> {
  value: T;
  options: HeaderSelectOption<T>[];
  onChange: (value: T) => void;
  disabled?: boolean;
  title: string;
  className?: string;
  menuMinWidth?: number;
}

function HeaderSelect<T extends SelectValue>({
  value,
  options,
  onChange,
  disabled = false,
  title,
  className = "",
  menuMinWidth = 180,
}: HeaderSelectProps<T>) {
  const [open, setOpen] = useState(false);
  const [menuPosition, setMenuPosition] = useState({ top: 0, left: 0, width: menuMinWidth });
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const selectedOption = options.find((option) => option.value === value) ?? null;

  const syncMenuPosition = useCallback(() => {
    const rect = buttonRef.current?.getBoundingClientRect();
    if (!rect) return;
    const padding = 8;
    const availableWidth = Math.max(160, window.innerWidth - padding * 2);
    const width = Math.min(Math.max(rect.width, menuMinWidth), availableWidth);
    const left = Math.max(
      padding,
      Math.min(rect.left, window.innerWidth - width - padding),
    );
    setMenuPosition({
      top: rect.bottom + 3,
      left,
      width,
    });
  }, [menuMinWidth]);

  useEffect(() => {
    if (disabled) setOpen(false);
  }, [disabled]);

  useEffect(() => {
    if (!open) return undefined;
    syncMenuPosition();

    const handleMouseDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (buttonRef.current?.contains(target) || menuRef.current?.contains(target)) {
        return;
      }
      setOpen(false);
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };

    document.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("resize", syncMenuPosition);
    window.addEventListener("scroll", syncMenuPosition, true);
    return () => {
      document.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("resize", syncMenuPosition);
      window.removeEventListener("scroll", syncMenuPosition, true);
    };
  }, [open, syncMenuPosition]);

  const menu = open ? (
    <div
      ref={menuRef}
      role="listbox"
      className="fixed max-h-64 overflow-y-auto rounded border border-book-border bg-book-bg-paper py-1 shadow-md"
      style={{
        top: menuPosition.top,
        left: menuPosition.left,
        width: menuPosition.width,
        zIndex: 9999,
      }}
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={`${option.value}`}
            type="button"
            role="option"
            aria-selected={selected}
            onClick={() => {
              onChange(option.value);
              setOpen(false);
            }}
            className={`flex w-full items-center gap-2 px-2 py-1.5 text-left text-[11px] ${
              selected
                ? "bg-[rgb(var(--color-primary-pale))] text-book-primary"
                : "text-book-text-main hover:bg-book-bg-tertiary"
            }`}
          >
            <span className="min-w-0 flex-1">
              <span className="block truncate font-medium">{option.label}</span>
              {option.description ? (
                <span className="block truncate text-[10px] text-book-text-muted">
                  {option.description}
                </span>
              ) : null}
            </span>
            {selected ? <Check size={12} className="shrink-0" /> : null}
          </button>
        );
      })}
    </div>
  ) : null;

  return (
    <div className={`relative min-w-0 ${className}`}>
      <button
        ref={buttonRef}
        type="button"
        title={title}
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => {
          if (disabled) return;
          setOpen((current) => !current);
        }}
        className="inline-flex h-9 w-full min-w-0 items-center gap-1 rounded border border-book-border bg-book-bg-paper px-2 text-left text-[11px] text-book-text-main hover:border-book-primary/55 hover:bg-book-bg-tertiary disabled:cursor-not-allowed disabled:bg-book-bg-tertiary disabled:text-book-text-muted sm:h-7"
      >
        <span className="min-w-0 flex-1 truncate">
          {selectedOption?.label ?? title}
        </span>
        <ChevronDown
          size={12}
          className={`shrink-0 text-book-text-muted transition-transform ${
            open ? "rotate-180" : ""
          }`}
        />
      </button>
      {menu ? createPortal(menu, getPortalRoot()) : null}
    </div>
  );
}

export function OptimizeHeader({
  activeChapter,
  activeSession,
  activeSessionId,
  chapterSessions,
  chapters,
  isPinned,
  loading,
  selectedChapter,
  taskRunning,
  onDeleteSession,
  onNewChatSession,
  onRefresh,
  onSelectChapter,
  onSelectSession,
}: OptimizeHeaderProps) {
  const chapterOptions = useMemo<HeaderSelectOption<number | "">[]>(
    () => [
      { value: "", label: "选择章节" },
      ...chapters.map((chapter) => ({
        value: chapter.id,
        label: chapterOptionLabel(chapter),
      })),
    ],
    [chapters],
  );
  const sessionOptions = useMemo<HeaderSelectOption<number | "">[]>(
    () => [
      { value: "", label: "选择会话" },
      ...chapterSessions.map((session) => ({
        value: session.id,
        label: sessionDisplayName(session),
        description: statusLabel(session.status),
      })),
    ],
    [chapterSessions],
  );

  return (
    <div className="flex min-w-0 shrink-0 flex-wrap items-start gap-2 border-b border-book-border bg-book-bg-tertiary px-2 py-2">
      <div className="min-w-0 flex-1 basis-36">
        <div className="flex min-w-0 items-center gap-2">
          <Bot size={14} className="shrink-0 text-book-primary" />
          <span className="truncate text-xs font-semibold text-book-text-main">
            内容优化 Agent
          </span>
          {taskRunning ? (
            <span className="inline-flex shrink-0 items-center gap-1 rounded-sm bg-[rgb(var(--color-primary-pale))] px-1.5 py-0.5 text-[10px] text-book-primary">
              <Loader2 size={10} className="animate-spin" />
              运行中
            </span>
          ) : null}
        </div>
        <div className="mt-0.5 truncate text-[10px] text-book-text-sub">
          {activeChapter
            ? `${chapterOptionLabel(activeChapter)}${
                activeSession ? ` · ${sessionDisplayName(activeSession)}` : " · 尚未分析"
              }`
            : "未选择章节"}
        </div>
      </div>

      <div className="flex min-w-0 flex-1 basis-40 flex-wrap items-center justify-end gap-1">
        {!isPinned ? (
          <HeaderSelect
            value={selectedChapter}
            options={chapterOptions}
            onChange={onSelectChapter}
            disabled={taskRunning}
            title="选择要优化的章节"
            className="flex-1 basis-28"
            menuMinWidth={220}
          />
        ) : null}
        {activeChapter && chapterSessions.length > 1 ? (
          <HeaderSelect
            value={activeSessionId ?? ""}
            options={sessionOptions}
            onChange={(id) => {
              if (typeof id === "number") onSelectSession(id);
            }}
            disabled={taskRunning}
            title="切换本章历史会话"
            className="flex-1 basis-24"
            menuMinWidth={200}
          />
        ) : null}
        <button
          type="button"
          onClick={onNewChatSession}
          disabled={!activeChapter || taskRunning}
          className="inline-flex h-9 w-9 items-center justify-center rounded border border-book-border bg-book-bg-paper text-book-text-sub hover:bg-book-bg-tertiary hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-7 sm:w-7"
          title="新建普通 Agent 会话"
        >
          <Plus size={13} />
        </button>
        <button
          type="button"
          onClick={onDeleteSession}
          disabled={!activeSession || taskRunning}
          className="inline-flex h-9 w-9 items-center justify-center rounded border border-error/40 bg-book-bg-paper text-error hover:bg-error-bg disabled:cursor-not-allowed disabled:opacity-50 sm:h-7 sm:w-7"
          title="删除当前 Agent 会话"
        >
          <Trash2 size={13} />
        </button>
        <button
          type="button"
          onClick={onRefresh}
          disabled={loading || taskRunning}
          className="inline-flex h-9 w-9 items-center justify-center rounded border border-book-border bg-book-bg-paper text-book-text-sub hover:bg-book-bg-tertiary hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 sm:h-7 sm:w-7"
          title="刷新 Agent 状态"
        >
          <RefreshCw size={13} className={loading ? "animate-spin" : undefined} />
        </button>
      </div>
    </div>
  );
}
