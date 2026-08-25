import { useMemo } from "react";
import { Plus, RefreshCw, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { ChapterSummary } from "../../../api/chapter";
import type { OptimizeSession } from "../../../api/optimize";
import { BookIconButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { useI18n } from "../../../i18n";
import {
  chapterOptionLabel,
  sessionDisplayName,
  statusLabel,
} from "./optimizeTabUtils";

interface OptimizeHeaderProps {
  activeChapter: ChapterSummary | null;
  activeSession: OptimizeSession | null;
  activeSessionId: number | null;
  chapterSessions: OptimizeSession[];
  chapters: ChapterSummary[];
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
}

function HeaderSelect<T extends SelectValue>({
  value,
  options,
  onChange,
  disabled = false,
  title,
  className = "",
}: HeaderSelectProps<T>) {
  return (
    <label className={`relative min-w-0 ${className}`}>
      <span className="sr-only">{title}</span>
      <select
        value={String(value)}
        title={title}
        disabled={disabled}
        aria-label={title}
        onChange={(event) => {
          const selected = options.find(
            (option) => String(option.value) === event.target.value,
          );
          if (selected) onChange(selected.value);
        }}
        className="pyqt-input h-9 w-full min-w-0 !py-1.5 text-small sm:h-8"
      >
        {options.map((option) => (
          <option key={`${option.value}`} value={String(option.value)}>
            {option.label}{option.description ? ` · ${option.description}` : ""}
          </option>
        ))}
      </select>
    </label>
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
  const { t } = useI18n();
  const chapterOptions = useMemo<HeaderSelectOption<number | "">[]>(
    () => [
      { value: "", label: t("optimize.selectChapter") },
      ...chapters.map((chapter) => ({
        value: chapter.id,
        label: chapterOptionLabel(chapter, t),
      })),
    ],
    [chapters, t],
  );
  const sessionOptions = useMemo<HeaderSelectOption<number | "">[]>(
    () => [
      { value: "", label: t("optimize.selectSession") },
      ...chapterSessions.map((session) => ({
        value: session.id,
        label: sessionDisplayName(session, t),
        description: statusLabel(session.status, t),
      })),
    ],
    [chapterSessions, t],
  );

  return (
    <div className="flex min-w-0 shrink-0 flex-wrap items-center gap-2 border-b border-book-border/45 bg-book-bg-paper/75 px-3 py-2.5">
      <div className="w-full min-w-0 sm:flex-1 sm:basis-36">
        <div className="flex min-w-0 items-center gap-2 text-xs font-semibold text-book-text-main">
          <span className="truncate">
            {activeChapter ? chapterOptionLabel(activeChapter, t) : t("optimize.chooseChapter")}
          </span>
          {taskRunning ? (
            <span className="inline-flex shrink-0 items-center gap-1 text-small text-book-primary" role="status">
              <LoadingSpinner size="sm" />
              {t("optimize.running")}
            </span>
          ) : null}
        </div>
        <div className="mt-0.5 truncate text-small text-book-text-muted">
          {activeSession ? sessionDisplayName(activeSession, t) : activeChapter ? t("optimize.noAnalysis") : t("optimize.chooseChapterHint")}
        </div>
      </div>

      <div className="flex w-full min-w-0 flex-wrap items-center justify-end gap-1 sm:flex-1 sm:basis-40">
        {!isPinned ? (
          <HeaderSelect
            value={selectedChapter}
            options={chapterOptions}
            onChange={onSelectChapter}
            disabled={taskRunning}
            title={t("optimize.chooseChapterToOptimize")}
            className="flex-1 basis-28"
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
            title={t("optimize.switchSession")}
            className="flex-1 basis-24"
          />
        ) : null}
        {activeChapter ? (
          <BookIconButton
            label={t("optimize.newSession")}
            variant="secondary"
            size="sm"
            onClick={onNewChatSession}
            disabled={taskRunning}
          >
            <Plus size={13} />
          </BookIconButton>
        ) : null}
        {activeSession && !taskRunning ? (
          <Dropdown
            items={[
              {
                label: t("optimize.deleteSession"),
                onClick: onDeleteSession,
                danger: true,
                icon: <Trash2 size={12} />,
              },
            ]}
          />
        ) : null}
        <BookIconButton
          label={t("optimize.refreshStatus")}
          variant="secondary"
          size="sm"
          onClick={onRefresh}
          disabled={loading || taskRunning}
        >
          <RefreshCw size={13} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
    </div>
  );
}
