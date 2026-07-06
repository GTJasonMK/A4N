import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye, Loader2 } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  generatePortraitPrompt,
  previewGeneratePortraitPromptRequest,
  type Portrait,
} from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { useToast } from "../../../components/feedback/Toast";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";

interface PortraitPromptDialogProps {
  portrait: Portrait;
  onClose: () => void;
}

export const PortraitPromptDialog: React.FC<PortraitPromptDialogProps> = ({
  portrait,
  onClose,
}) => {
  const addToast = useToast((state) => state.addToast);
  const [rendered, setRendered] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const activePortraitScopeRef = useRef(`${portrait.project_id}:${portrait.id}`);
  const mountedRef = useRef(true);
  const requestPreviewSeqRef = useRef(0);
  const renderBusyRef = useRef(false);
  const requestPreviewBusyRef = useRef(false);

  activePortraitScopeRef.current = `${portrait.project_id}:${portrait.id}`;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    requestPreviewSeqRef.current += 1;
    renderBusyRef.current = false;
    requestPreviewBusyRef.current = false;
    setRendered(null);
    setBusy(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [portrait.id, portrait.project_id]);

  const isCurrentPortrait = useCallback((scope: string) => {
    return mountedRef.current && activePortraitScopeRef.current === scope;
  }, []);

  const closeRequestPreview = useCallback(() => {
    requestPreviewSeqRef.current += 1;
    requestPreviewBusyRef.current = false;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, []);

  const handleRender = async () => {
    const portraitType = portrait.portrait_type?.trim();
    if (!portraitType) {
      addToast("人物三视图拆解类型缺失，不能渲染提示词", "error");
      return;
    }
    if (renderBusyRef.current) return;
    renderBusyRef.current = true;
    const requestScope = `${portrait.project_id}:${portrait.id}`;
    const requestInput = {
      project_id: portrait.project_id,
      profile_id: portrait.profile_id,
      portrait_type: portraitType,
      variables: null,
    };
    setBusy(true);
    try {
      const result = await generatePortraitPrompt(requestInput);
      if (!isCurrentPortrait(requestScope)) return;
      setRendered(result);
    } catch (err) {
      if (!isCurrentPortrait(requestScope)) return;
      addToast(extractErrorMessage(err, "渲染失败"), "error");
    } finally {
      if (isCurrentPortrait(requestScope)) setBusy(false);
      if (renderBusyRef.current) renderBusyRef.current = false;
    }
  };

  const handlePreview = async () => {
    const portraitType = portrait.portrait_type?.trim();
    if (!portraitType) {
      addToast("人物三视图拆解类型缺失，不能预览提示词请求", "error");
      return;
    }
    if (requestPreviewBusyRef.current) return;
    requestPreviewBusyRef.current = true;
    const requestScope = `${portrait.project_id}:${portrait.id}`;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    const requestInput = {
      project_id: portrait.project_id,
      profile_id: portrait.profile_id,
      portrait_type: portraitType,
      variables: null,
    };
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGeneratePortraitPromptRequest(requestInput);
      if (!isCurrentPortrait(requestScope) || requestPreviewSeqRef.current !== requestSeq) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentPortrait(requestScope) || requestPreviewSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "预览提示词请求失败"), "error");
      setRequestPreviewOpen(false);
      requestPreviewBusyRef.current = false;
    } finally {
      if (isCurrentPortrait(requestScope) && requestPreviewSeqRef.current === requestSeq) {
        setRequestPreviewLoading(false);
      }
    }
  };

  return (
    <Modal
      isOpen
      onClose={onClose}
      title={`人物三视图拆解提示词 #${portrait.id}`}
      maxWidthClassName="max-w-2xl"
    >
      <div className="space-y-3 text-sm">
        <div>
          <div className="mb-1 text-xs text-book-text-muted">原始 Prompt</div>
          <pre className="pyqt-card custom-scrollbar max-h-48 overflow-y-auto whitespace-pre-wrap break-words bg-book-bg-tertiary/40 p-3 text-xs">
            {portrait.prompt || "（无）"}
          </pre>
        </div>
        <div className="flex items-center gap-2">
          <BookButton variant="secondary" onClick={handleRender} disabled={busy}>
            {busy ? (
              <span className="inline-flex items-center gap-1.5">
                <Loader2 size={12} className="animate-spin" /> 渲染中…
              </span>
            ) : (
              "渲染提示词"
            )}
          </BookButton>
          <BookButton variant="secondary" onClick={handlePreview} disabled={busy}>
            <span className="inline-flex items-center gap-1.5">
              <Eye size={12} />
              预览请求
            </span>
          </BookButton>
        </div>
        {rendered !== null ? (
          <div>
            <div className="mb-1 text-xs text-book-text-muted">渲染结果</div>
            <pre className="pyqt-card custom-scrollbar max-h-64 overflow-y-auto whitespace-pre-wrap break-words bg-book-bg-tertiary/40 p-3 text-xs">
              {rendered || "（空）"}
            </pre>
          </div>
        ) : null}
        <div className="flex justify-end pt-2">
          <BookButton variant="secondary" onClick={onClose}>
            关闭
          </BookButton>
        </div>
      </div>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="人物三视图拆解提示词请求预览"
        onClose={closeRequestPreview}
      />
    </Modal>
  );
};
