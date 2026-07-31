import { useEffect, useState } from "react";
import { sha256Hex } from "../../../utils/contentHash";
import type { ChapterRow } from "../shared";

export const useChapterContentHash = (chapter: ChapterRow | null) => {
  const [currentContentHash, setCurrentContentHash] = useState<string | null>(null);
  const [chapterContentReady, setChapterContentReady] = useState(false);
  const [chapterContentHashError, setChapterContentHashError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const content = chapter?.content ?? "";
    if (!chapter?.id || !content.trim()) {
      setCurrentContentHash(null);
      setChapterContentReady(false);
      setChapterContentHashError(null);
      return;
    }
    setCurrentContentHash(null);
    setChapterContentReady(false);
    setChapterContentHashError(null);
    void sha256Hex(content)
      .then((hash) => {
        if (cancelled) return;
        if (!hash) {
          setChapterContentHashError("当前环境无法计算章节正文 SHA-256");
          return;
        }
        setCurrentContentHash(hash);
        setChapterContentReady(true);
      })
      .catch((error) => {
        if (!cancelled) {
          setCurrentContentHash(null);
          setChapterContentReady(false);
          setChapterContentHashError(
            error instanceof Error ? error.message : "计算章节正文 SHA-256 失败",
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, [chapter?.content, chapter?.id]);

  return { currentContentHash, chapterContentReady, chapterContentHashError };
};
