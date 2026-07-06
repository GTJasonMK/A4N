import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Loader2, Upload } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { BookCard } from "../../components/ui/BookCard";
import { Modal } from "../../components/ui/Modal";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { importChapter, listChapters, type ChapterData } from "../../api/chapter";
import { EmptyHint } from "./shared";
import { useNovelDetailLoader } from "./useNovelDetailLoader";
import { nonEmptyText } from "../../utils/text";

const chapterWordCount = (chapter: ChapterData): number => {
  return chapter.word_count == null ? 0 : chapter.word_count;
};

const nextImportChapterNumber = (chapters: ChapterData[]): number => {
  if (chapters.length === 0) return 1;
  return chapters[chapters.length - 1].chapter_number + 1;
};

interface ChaptersTabProps {
  projectId: number;
  canEditChapters: boolean;
}

export const ChaptersTab: React.FC<ChaptersTabProps> = ({ projectId, canEditChapters }) => {
  const { addToast } = useToast();
  const navigate = useNavigate();
  const [chapters, setChapters] = useState<ChapterData[]>([]);
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
      await importChapter({
        project_id: requestProjectId,
        chapter_number: requestForm.chapter_number,
        content: requestForm.content,
        title: nonEmptyText(requestForm.title),
      });
      if (!isCurrentImport(requestProjectId, requestSeq)) return;
      addToast(`已导入第 ${requestForm.chapter_number} 章`, "success");
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
  const totalWords = useMemo(
    () => displayChapters.reduce((sum, chapter) => sum + chapterWordCount(chapter), 0),
    [displayChapters],
  );

  if (displayLoading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <Loader2 size={12} className="animate-spin" /> 加载章节…
      </div>
    );
  }

  return (
    <div className="content-main">
      <BookCard className="p-5">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-serif text-book-text-main">
            章节 <span className="text-xs text-book-text-muted">({displayChapters.length})</span>
          </h3>
          <div className="flex items-center gap-2">
            <span className="text-xs text-book-text-muted">
              共 {totalWords.toLocaleString()} 字
            </span>
            {canEditChapters && (
              <BookButton variant="secondary" onClick={openImportDialog}>
                导入章节
              </BookButton>
            )}
          </div>
        </div>
        {displayChapters.length === 0 ? (
          <EmptyHint
            text="还没有章节。"
            actionText={canEditChapters ? "进入写作桌生成第一章" : undefined}
            onAction={canEditChapters ? () => openWritingDesk() : undefined}
          />
        ) : (
          <ul className="space-y-1.5">
            {displayChapters.map((c) => (
              <li
                key={c.id}
                className={`rounded-md border border-book-border/30 bg-book-bg-paper/40 px-3 py-2 text-sm flex items-center gap-3 ${
                  canEditChapters ? "cursor-pointer hover:bg-book-bg-paper/80" : ""
                }`}
                onClick={canEditChapters ? () => openWritingDesk(c.chapter_number) : undefined}
              >
                <span className="font-mono text-book-text-muted text-xs shrink-0 min-w-[3em] text-right">
                  #{c.chapter_number}
                </span>
                <div className="flex-1 min-w-0">
                  <div className="text-book-text-main truncate">
                    {c.title || `第 ${c.chapter_number} 章`}
                  </div>
                </div>
                {c.status && <span className="pyqt-pill shrink-0">{c.status}</span>}
                {c.word_count != null && (
                  <span className="text-xs text-book-text-muted shrink-0">
                    {c.word_count.toLocaleString()} 字
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
      </BookCard>

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
            <>
              <BookButton
                variant="secondary"
                onClick={() => setImportOpen(false)}
                disabled={importing}
              >
                取消
              </BookButton>
              <BookButton variant="primary" onClick={handleImport} disabled={importing}>
                {importing ? (
                  <span className="inline-flex items-center gap-2">
                    <Loader2 size={14} className="animate-spin" /> 导入中…
                  </span>
                ) : (
                  "确认导入"
                )}
              </BookButton>
            </>
          }
        >
          <div className="flex max-h-[78dvh] min-h-0 flex-col">
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
            <label className="flex-1 min-h-0 flex flex-col gap-1">
              <span className="text-xs text-book-text-sub">正文</span>
              <textarea
                value={importForm.content}
                onChange={(e) =>
                  setImportForm({ ...importForm, content: e.target.value })
                }
                disabled={importing}
                spellCheck={false}
                placeholder="粘贴本章正文（status 会设为 imported）"
                className="book-control min-h-[220px] flex-1 resize-none rounded p-3 font-serif text-sm leading-relaxed sm:min-h-[280px]"
              />
            </label>
          </div>
        </Modal>
      )}
    </div>
  );
};
