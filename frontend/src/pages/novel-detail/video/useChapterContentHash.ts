import { useEffect, useState } from "react";
import { sha256Hex } from "../../../utils/contentHash";
import type { ChapterRow } from "../shared";
import { useI18n } from "../../../i18n";

export const useChapterContentHash = (chapter: ChapterRow | null) => {
  const { t } = useI18n();
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
          setChapterContentHashError(t("video.chapter.contentHashUnavailable"));
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
            error instanceof Error
              ? error.message
              : t("video.chapter.contentHashFailed"),
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, [chapter?.content, chapter?.id, t]);

  return { currentContentHash, chapterContentReady, chapterContentHashError };
};
