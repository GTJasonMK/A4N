import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../api";
import { previewChapterPrompt } from "../../api/chapter";
import type { ModelRequestPreview } from "../../api/requestPreview";
import type { ToastType } from "../../components/feedback/Toast";
import { nonEmptyText } from "../../utils/text";
import type { GenerationWordCountRange } from "./generationWordCount";

type AddToast = (message: string, type: ToastType) => void;

interface UseChapterPromptPreviewParams {
  projectId: number;
  activeNumber: number | null;
  writingNotes: string;
  wordCountRange: GenerationWordCountRange | null;
  wordCountRangeError: string | null;
  addToast: AddToast;
}

export function useChapterPromptPreview({
  projectId,
  activeNumber,
  writingNotes,
  wordCountRange,
  wordCountRangeError,
  addToast,
}: UseChapterPromptPreviewParams) {
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewing, setPreviewing] = useState(false);
  const [previewData, setPreviewData] = useState<ModelRequestPreview | null>(
    null,
  );
  const mountedRef = useRef(false);
  const projectIdRef = useRef(projectId);
  const activeNumberRef = useRef(activeNumber);
  const previewBusyRef = useRef(false);
  const previewSeqRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      previewBusyRef.current = false;
      previewSeqRef.current += 1;
    };
  }, []);

  useEffect(() => {
    projectIdRef.current = projectId;
    activeNumberRef.current = activeNumber;
    previewBusyRef.current = false;
    previewSeqRef.current += 1;
    setPreviewOpen(false);
    setPreviewing(false);
    setPreviewData(null);
  }, [activeNumber, projectId]);

  const isCurrentPreview = useCallback(
    (requestSeq: number, requestProjectId: number, requestChapterNumber: number) =>
      mountedRef.current &&
      previewSeqRef.current === requestSeq &&
      projectIdRef.current === requestProjectId &&
      activeNumberRef.current === requestChapterNumber,
    [],
  );

  const handlePreviewPrompt = useCallback(async () => {
    if (previewBusyRef.current) {
      addToast("正在生成请求预览", "error");
      return;
    }
    if (!activeNumber) {
      addToast("请先选择一章", "error");
      return;
    }
    if (wordCountRangeError || !wordCountRange) {
      addToast(wordCountRangeError ?? "正文字数范围无效", "error");
      return;
    }

    const requestProjectId = projectId;
    const requestChapterNumber = activeNumber;
    const requestSeq = previewSeqRef.current + 1;
    previewSeqRef.current = requestSeq;
    previewBusyRef.current = true;
    setPreviewing(true);
    setPreviewOpen(true);
    setPreviewData(null);
    try {
      const result = await previewChapterPrompt({
        project_id: projectId,
        chapter_number: activeNumber,
        writing_notes: nonEmptyText(writingNotes),
        min_word_count: wordCountRange.min,
        max_word_count: wordCountRange.max,
      });
      if (!isCurrentPreview(requestSeq, requestProjectId, requestChapterNumber)) return;
      setPreviewData(result);
    } catch (err) {
      if (!isCurrentPreview(requestSeq, requestProjectId, requestChapterNumber)) return;
      addToast(extractErrorMessage(err, "预览失败"), "error");
      setPreviewOpen(false);
    } finally {
      if (isCurrentPreview(requestSeq, requestProjectId, requestChapterNumber)) {
        previewBusyRef.current = false;
        setPreviewing(false);
      }
    }
  }, [
    activeNumber,
    addToast,
    isCurrentPreview,
    projectId,
    wordCountRange,
    wordCountRangeError,
    writingNotes,
  ]);

  const closePromptPreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyRef.current = false;
    setPreviewOpen(false);
    setPreviewing(false);
  }, []);

  const resetPromptPreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyRef.current = false;
    setPreviewOpen(false);
    setPreviewing(false);
    setPreviewData(null);
  }, []);

  return {
    previewOpen,
    previewing,
    previewData,
    handlePreviewPrompt,
    closePromptPreview,
    resetPromptPreview,
  };
}
