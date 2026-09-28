import { useCallback, useEffect, useRef, useState } from "react";
import { getChapter, type ChapterSummary } from "../api/chapter";
import { extractErrorMessage } from "../api";
import { useI18n } from "../i18n";

/** Keep only the selected body; an older read must not replace a later selection or write. */
export function useChapterDetail(projectId: number, chapterId: number | null) {
  const { t } = useI18n();
  const selected = useRef({ projectId, chapterId });
  selected.current = { projectId, chapterId };
  const sequence = useRef(0);
  const [state, setState] = useState<{
    projectId: number;
    chapterId: number;
    chapter: ChapterSummary | null;
    error: string | null;
  } | null>(null);

  const apply = useCallback((chapter: ChapterSummary) => {
    if (selected.current.projectId !== chapter.project_id
      || selected.current.chapterId !== chapter.id) return;
    sequence.current += 1;
    setState({ projectId: chapter.project_id, chapterId: chapter.id, chapter, error: null });
  }, []);

  useEffect(() => {
    const requestSequence = ++sequence.current;
    const isCurrent = () => sequence.current === requestSequence
      && selected.current.projectId === projectId && selected.current.chapterId === chapterId;
    setState(null);
    if (chapterId != null) {
      void getChapter(projectId, chapterId).then((chapter) => {
        if (isCurrent()) setState({ projectId, chapterId, chapter, error: null });
      }).catch((error) => {
        if (isCurrent()) setState({
          projectId, chapterId, chapter: null,
          error: extractErrorMessage(error, t("novel.loadChaptersFailed")),
        });
      });
    }
    return () => { sequence.current += 1; };
  }, [chapterId, projectId, t]);

  const current = state?.projectId === projectId && state.chapterId === chapterId ? state : null;
  return {
    chapter: current?.chapter ?? null,
    error: current?.error ?? null,
    loading: chapterId != null && !current,
    apply,
  };
}
