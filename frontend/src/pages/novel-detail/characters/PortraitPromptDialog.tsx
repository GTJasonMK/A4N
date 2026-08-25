import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  generatePortraitPrompt,
  previewGeneratePortraitPromptRequest,
  type GeneratePortraitPromptInput,
  type Portrait,
} from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { useToast } from "../../../components/feedback/Toast";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { useI18n } from "../../../i18n";

interface PortraitPromptDialogProps {
  portrait: Portrait;
  onClose: () => void;
}

export const PortraitPromptDialog: React.FC<PortraitPromptDialogProps> = ({
  portrait,
  onClose,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
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
  const requestInput = {
    project_id: portrait.project_id,
    profile_id: portrait.profile_id,
    portrait_type: portrait.portrait_type,
    variables: null,
  } satisfies GeneratePortraitPromptInput;

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
    if (renderBusyRef.current) return;
    renderBusyRef.current = true;
    const requestScope = `${portrait.project_id}:${portrait.id}`;
    setBusy(true);
    try {
      const result = await generatePortraitPrompt(requestInput);
      if (!isCurrentPortrait(requestScope)) return;
      setRendered(result);
    } catch (err) {
      if (!isCurrentPortrait(requestScope)) return;
      addToast(extractErrorMessage(err, t("characterDetail.portrait.promptRenderFailed")), "error");
    } finally {
      if (isCurrentPortrait(requestScope)) setBusy(false);
      if (renderBusyRef.current) renderBusyRef.current = false;
    }
  };

  const handlePreview = async () => {
    if (requestPreviewBusyRef.current) return;
    requestPreviewBusyRef.current = true;
    const requestScope = `${portrait.project_id}:${portrait.id}`;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGeneratePortraitPromptRequest(requestInput);
      if (!isCurrentPortrait(requestScope) || requestPreviewSeqRef.current !== requestSeq) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentPortrait(requestScope) || requestPreviewSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, t("characterDetail.portrait.promptPreviewFailed")), "error");
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
      title={t("characterDetail.portrait.promptTitle")}
      maxWidthClassName="max-w-2xl"
      footer={
        <div className="flex w-full items-center gap-2">
          <Dropdown
            items={[
              {
                label: t("characterDetail.portrait.viewRequest"),
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
                <LoadingSpinner size="sm" /> {t("characterDetail.portrait.rendering")}
              </span>
            ) : rendered === null ? (
              t("characterDetail.portrait.renderPrompt")
            ) : (
              t("characterDetail.portrait.rerender")
            )}
          </BookButton>
        </div>
      }
    >
      <div className="space-y-3 text-sm">
        <div>
          <div className="mb-1 text-xs text-book-text-muted">{t("characterDetail.portrait.currentPrompt")}</div>
          <pre className="a4n-flat-surface whitespace-pre-wrap break-words p-3 text-xs leading-relaxed text-book-text-sub">
            {portrait.prompt || t("characterDetail.portrait.none")}
          </pre>
        </div>
        {rendered !== null ? (
          <div>
            <div className="mb-1 text-xs text-book-text-muted">{t("characterDetail.portrait.renderedResult")}</div>
            <pre className="a4n-flat-surface whitespace-pre-wrap break-words p-3 text-xs leading-relaxed text-book-text-sub">
              {rendered || t("characterDetail.portrait.emptyResult")}
            </pre>
          </div>
        ) : null}
      </div>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("characterDetail.portrait.promptPreviewTitle")}
        onClose={closeRequestPreview}
      />
    </Modal>
  );
};
