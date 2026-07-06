import { useEffect, useState } from "react";
import { sha256Hex } from "../../../utils/contentHash";
import type { ChapterRow } from "../shared";

export const useChapterContentHash = (chapter: ChapterRow | null) => {
  const [currentContentHash, setCurrentContentHash] = useState<string | null>(null);
  const [chapterContentReady, setChapterContentReady] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const content = chapter?.content ?? "";
    if (!chapter?.id || !content.trim()) {
      setCurrentContentHash(null);
      setChapterContentReady(false);
      return;
    }
    setCurrentContentHash(null);
    setChapterContentReady(true);
    void sha256Hex(content)
      .then((hash) => {
        if (!cancelled) setCurrentContentHash(hash);
      })
      .catch(() => {
        if (!cancelled) {
          setCurrentContentHash(null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [chapter?.content, chapter?.id]);

  return { currentContentHash, chapterContentReady };
};
