import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
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
import { Dropdown } from "../../../components/ui/Dropdown";
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
      title="人物三视图拆解提示词"
      maxWidthClassName="max-w-2xl"
      footer={
        <div className="flex w-full items-center gap-2">
          <Dropdown
            items={[
              {
                label: "查看模型请求",
                icon: <Eye size={12} />,
                onClick: handlePreview,
                disabled: busy,
              },
            ]}
          />
          <span className="min-w-0 flex-1" />
          <BookButton variant="primary" onClick={handleRender} disabled={busy}>
            {busy ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" /> 渲染中…
              </span>
            ) : rendered === null ? (
              "渲染提示词"
            ) : (
              "重新渲染"
            )}
          </BookButton>
        </div>
      }
    >
      <div className="space-y-3 text-sm">
        <div>
          <div className="mb-1 text-xs text-book-text-muted">当前提示词</div>
          <pre className="a4n-flat-surface whitespace-pre-wrap break-words p-3 text-xs leading-relaxed text-book-text-sub">
            {portrait.prompt || "（无）"}
          </pre>
        </div>
        {rendered !== null ? (
          <div>
            <div className="mb-1 text-xs text-book-text-muted">渲染结果</div>
            <pre className="a4n-flat-surface whitespace-pre-wrap break-words p-3 text-xs leading-relaxed text-book-text-sub">
              {rendered || "（空）"}
            </pre>
          </div>
        ) : null}
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
