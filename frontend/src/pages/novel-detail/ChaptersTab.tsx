import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ChevronRight, Upload } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import {
  chapterContentCommitToastType,
  importChapter,
  listChapters,
  type ChapterSummary,
} from "../../api/chapter";
import { EmptyHint } from "./shared";
import { useNovelDetailLoader } from "./useNovelDetailLoader";
import { nonEmptyText } from "../../utils/text";
import { getOperationStatusLabel } from "../../utils/workflowPresentation";

const chapterWordCount = (chapter: ChapterSummary): number => {
  return chapter.word_count == null ? 0 : chapter.word_count;
};

const nextImportChapterNumber = (chapters: ChapterSummary[]): number => {
  if (chapters.length === 0) return 1;
  return Math.max(...chapters.map((chapter) => chapter.chapter_number)) + 1;
};

const chapterStatusLabel = (status: string | null | undefined): string | null => {
  const normalized = status?.trim().toLowerCase();
  if (!normalized || normalized === "completed") return null;
  return getOperationStatusLabel(normalized);
};

const derivedSyncStatusLabel = (chapter: ChapterSummary): string | null => {
  if (chapter.derived_sync_status === "running") return "资料同步中";
  if (chapter.derived_sync_status === "failed") return "资料同步失败";
  return null;
};

interface ChaptersTabProps {
  projectId: number;
  canEditChapters: boolean;
}

export const ChaptersTab: React.FC<ChaptersTabProps> = ({ projectId, canEditChapters }) => {
  const { addToast } = useToast();
  const navigate = useNavigate();
  const [chapters, setChapters] = useState<ChapterSummary[]>([]);
  const [importOpen, setImportOpen] = useState(false);
  const [importForm, setImportForm] = useState({
    chapter_number: 1,
    title: "",
    content: "",
  });
  const [importing, setImporting] = useState(false);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const importSeqRef = useRef(0);
  const importingRef = useRef(false);

  const loadChapters = useCallback(async (isCurrent: () => boolean) => {
    try {
      const data = await listChapters(projectId);
      if (!isCurrent()) return;
      setChapters(data);
      setLoadedProjectId(projectId);
    } catch (err) {
      if (isCurrent()) {
        setChapters([]);
        setLoadedProjectId(projectId);
      }
      throw err;
    }
  }, [projectId]);
  const { loading, load } = useNovelDetailLoader(loadChapters, "加载章节失败");

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      importSeqRef.current += 1;
      importingRef.current = false;
    };
  }, []);

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    importSeqRef.current += 1;
    importingRef.current = false;
    setImportOpen(false);
    setImporting(false);
  }, [projectId]);

  useEffect(() => {
    if (!canEditChapters) {
      importSeqRef.current += 1;
      importingRef.current = false;
      setImportOpen(false);
      setImporting(false);
    }
  }, [canEditChapters]);

  const isCurrentImport = (requestProjectId: number, requestSeq: number) =>
    mountedRef.current &&
    activeProjectIdRef.current === requestProjectId &&
    importSeqRef.current === requestSeq;

  const openImportDialog = () => {
    if (!canEditChapters) {
      addToast("当前账号没有导入章节的权限", "error");
      return;
    }
    setImportForm({
      chapter_number: nextImportChapterNumber(displayChapters),
      title: "",
      content: "",
    });
    setImportOpen(true);
  };

  const openWritingDesk = (chapterNumber?: number) => {
    if (!canEditChapters) {
      addToast("当前账号没有进入写作桌的权限", "error");
      return;
    }
    navigate(chapterNumber ? `/write/${projectId}?chapter=${chapterNumber}` : `/write/${projectId}`);
  };

  const handleImport = async () => {
    if (!canEditChapters) {
      addToast("当前账号没有导入章节的权限", "error");
      return;
    }
    const requestProjectId = projectId;
    const requestForm = {
      chapter_number: importForm.chapter_number,
      title: importForm.title,
      content: importForm.content,
    };
    if (!requestForm.content.trim()) {
      addToast("请粘贴章节正文", "error");
      return;
    }
    if (!Number.isInteger(requestForm.chapter_number) || requestForm.chapter_number <= 0) {
      addToast("章节序号必须是正整数", "error");
      return;
    }
    if (importingRef.current) return;
    importingRef.current = true;
    const requestSeq = importSeqRef.current + 1;
    importSeqRef.current = requestSeq;
    setImporting(true);
    try {
      const imported = await importChapter({
        project_id: requestProjectId,
        chapter_number: requestForm.chapter_number,
        content: requestForm.content,
        title: nonEmptyText(requestForm.title),
      });
      if (!isCurrentImport(requestProjectId, requestSeq)) return;
      const message = imported.derived_sync_status === "failed"
        ? `已导入第 ${requestForm.chapter_number} 章，但派生资料同步失败；可打开章节重试`
        : imported.derived_sync_status === "running"
          ? `已导入第 ${requestForm.chapter_number} 章，派生资料正在同步`
          : `已导入第 ${requestForm.chapter_number} 章`;
      addToast(message, chapterContentCommitToastType(imported.derived_sync_status));
      setImportOpen(false);
      setImportForm({ chapter_number: requestForm.chapter_number + 1, title: "", content: "" });
      await load({ force: true });
    } catch (err) {
      if (isCurrentImport(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, "导入失败"), "error");
      }
    } finally {
      if (isCurrentImport(requestProjectId, requestSeq)) {
        setImporting(false);
      }
      if (importSeqRef.current === requestSeq) {
        importingRef.current = false;
      }
    }
  };

  const displayChapters = loadedProjectId === projectId ? chapters : [];
  const displayLoading = loading || loadedProjectId !== projectId;
  const importReady =
    Number.isInteger(importForm.chapter_number) &&
    importForm.chapter_number > 0 &&
    importForm.content.trim().length > 0;
  const totalWords = useMemo(
    () => displayChapters.reduce((sum, chapter) => sum + chapterWordCount(chapter), 0),
    [displayChapters],
  );

  if (displayLoading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> 加载章节…
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-3">
      <div className="flex min-h-8 items-center gap-3">
        {displayChapters.length > 0 ? (
          <span className="min-w-0 flex-1 truncate text-xs text-book-text-muted">
            {displayChapters.length} 章 · {totalWords.toLocaleString("zh-CN")} 字
          </span>
        ) : null}
        {canEditChapters ? (
          <BookButton
            variant="secondary"
            size="sm"
            onClick={openImportDialog}
            className="ml-auto !min-h-8 !text-xs"
          >
            导入章节
          </BookButton>
        ) : null}
      </div>
      <section className="a4n-flat-surface overflow-hidden">
        {displayChapters.length === 0 ? (
          <div className="p-4">
            <EmptyHint
              text="还没有章节。"
              actionText={canEditChapters ? "进入写作桌生成第一章" : undefined}
              onAction={canEditChapters ? () => openWritingDesk() : undefined}
            />
          </div>
        ) : (
          <ul className="divide-y divide-book-border/50">
            {displayChapters.map((c) => (
              <ChapterRow
                key={c.id}
                chapter={c}
                onOpen={
                  canEditChapters
                    ? () => openWritingDesk(c.chapter_number)
                    : undefined
                }
              />
            ))}
          </ul>
        )}
      </section>

      {/* 导入章节 Modal */}
      {importOpen && (
        <Modal
          isOpen
          onClose={() => !importing && setImportOpen(false)}
          title="导入章节"
          icon={<Upload size={15} />}
          subtitle="粘贴外部正文并写入当前项目"
          maxWidthClassName="max-w-2xl"
          bodyClassName="px-4 py-4"
          closeOnBackdrop={!importing}
          closeOnEscape={!importing}
          showCloseButton={!importing}
          footer={
            <BookButton
              variant="primary"
              onClick={handleImport}
              disabled={importing || !importReady}
              className="w-full sm:w-auto"
            >
              {importing ? (
                <span className="inline-flex items-center gap-2">
                  <LoadingSpinner size="sm" /> 导入中…
                </span>
              ) : (
                "确认导入"
              )}
            </BookButton>
          }
        >
          <div className="space-y-3">
            <div className="mb-3 grid gap-3 sm:grid-cols-[100px_1fr]">
              <label className="flex flex-col gap-1">
                <span className="text-xs text-book-text-sub">章节序号</span>
                <input
                  type="number"
                  min={1}
                  value={importForm.chapter_number}
                  onChange={(e) =>
                    setImportForm({
                      ...importForm,
                      chapter_number: Number(e.target.value),
                    })
                  }
                  disabled={importing}
                  className="pyqt-input !py-1"
                />
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-xs text-book-text-sub">标题（可选）</span>
                <input
                  type="text"
                  value={importForm.title}
                  onChange={(e) => setImportForm({ ...importForm, title: e.target.value })}
                  disabled={importing}
                  className="pyqt-input !py-1"
                />
              </label>
            </div>
            <label className="flex flex-col gap-1">
              <span className="text-xs text-book-text-sub">正文</span>
              <textarea
                value={importForm.content}
                onChange={(e) =>
                  setImportForm({ ...importForm, content: e.target.value })
                }
                disabled={importing}
                spellCheck={false}
                placeholder="粘贴本章正文；导入后会标记为外部导入"
                className="book-control min-h-[220px] resize-y rounded-xl p-3 font-serif text-sm leading-relaxed sm:min-h-[280px]"
              />
            </label>
          </div>
        </Modal>
      )}
    </div>
  );
};

const ChapterRow: React.FC<{
  chapter: ChapterSummary;
  onOpen?: () => void;
}> = ({ chapter, onOpen }) => {
  const status = chapterStatusLabel(chapter.status);
  const derivedSyncStatus = derivedSyncStatusLabel(chapter);
  const content = (
    <>
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium text-book-text-main">
          第 {chapter.chapter_number} 章
          {chapter.title?.trim() ? ` · ${chapter.title.trim()}` : ""}
        </div>
        <div className="mt-1 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-small text-book-text-muted">
          <span>{chapterWordCount(chapter).toLocaleString("zh-CN")} 字</span>
          {status ? (
            <>
              <span aria-hidden="true">·</span>
              <span>{status}</span>
            </>
          ) : null}
          {derivedSyncStatus ? (
            <>
              <span aria-hidden="true">·</span>
              <span className={chapter.derived_sync_status === "failed" ? "text-error" : ""}>
                {derivedSyncStatus}
              </span>
            </>
          ) : null}
        </div>
      </div>
      {onOpen ? (
        <ChevronRight
          size={14}
          className="shrink-0 text-book-text-muted"
          aria-hidden
        />
      ) : null}
    </>
  );

  return (
    <li>
      {onOpen ? (
        <button
          type="button"
          onClick={onOpen}
          className="flex min-h-14 w-full items-center gap-3 px-3 py-3 text-left transition-colors hover:bg-book-bg-tertiary/55 focus-visible:bg-book-bg-tertiary/55 focus-visible:outline-none sm:px-4"
        >
          {content}
        </button>
      ) : (
        <div className="flex min-h-14 items-center gap-3 px-3 py-3 sm:px-4">
          {content}
        </div>
      )}
    </li>
  );
};
