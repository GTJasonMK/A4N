import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import {
  previewGenerateAgeVariantRequest,
  startPortraitGeneration,
  verifyPortraitGenerationResult,
  type Portrait,
} from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import { useProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";

interface AgeVariantDialogProps {
  portrait: Portrait;
  onClose: () => void;
  onCreated: () => void;
}

export const AgeVariantDialog: React.FC<AgeVariantDialogProps> = ({
  portrait,
  onClose,
  onCreated,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { runProgressTask } = useProgressTask();
  const [age, setAge] = useState("");
  const [busy, setBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const activePortraitScopeRef = useRef(`${portrait.project_id}:${portrait.id}`);
  const mountedRef = useRef(true);
  const requestPreviewSeqRef = useRef(0);
  const submitBusyRef = useRef(false);
  const previewBusyRef = useRef(false);

  activePortraitScopeRef.current = `${portrait.project_id}:${portrait.id}`;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      submitBusyRef.current = false;
      previewBusyRef.current = false;
      requestPreviewSeqRef.current += 1;
    };
  }, []);

  useEffect(() => {
    submitBusyRef.current = false;
    previewBusyRef.current = false;
    requestPreviewSeqRef.current += 1;
    setBusy(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [portrait.id, portrait.project_id]);

  const isCurrentPortrait = useCallback((scope: string) => {
    return mountedRef.current && activePortraitScopeRef.current === scope;
  }, []);

  const handleSubmit = async () => {
    if (submitBusyRef.current) return;
    const trimmedAge = age.trim();
    if (!trimmedAge) {
      addToast("请输入年龄段（如：少年 / 老年 / 25 岁）", "error");
      return;
    }
    const requestScope = `${portrait.project_id}:${portrait.id}`;
    const requestPortraitId = portrait.id;
    const requestProjectId = portrait.project_id;
    submitBusyRef.current = true;
    setBusy(true);
    try {
      const handle = await startPortraitGeneration({
        project_id: requestProjectId,
        operation: {
          kind: "generate_age_variant",
          portrait_id: requestPortraitId,
          age: trimmedAge,
          reference_image_ids: [],
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: "生成年龄变体",
          startMessage: "正在准备年龄变体",
          successMessage: "年龄变体生成完成",
          errorMessage: "生成失败",
          total: 4,
          cancelLabel: "停止生成",
        },
        async (task) => {
          await verifyPortraitGenerationResult(task, requestProjectId);
        },
      );
      if (!isCurrentPortrait(requestScope)) return;
      addToast(`已生成 ${trimmedAge} 年龄变体`, "success");
      onCreated();
    } catch (err) {
      if (!isCurrentPortrait(requestScope)) return;
      const message = extractErrorMessage(err, "生成失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentPortrait(requestScope)) {
        submitBusyRef.current = false;
        setBusy(false);
      }
    }
  };

  const handlePreview = async () => {
    if (previewBusyRef.current) return;
    const trimmedAge = age.trim();
    if (!trimmedAge) {
      addToast("请输入年龄段（如：少年 / 老年 / 25 岁）", "error");
      return;
    }
    const requestScope = `${portrait.project_id}:${portrait.id}`;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    const requestPortraitId = portrait.id;
    previewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGenerateAgeVariantRequest(requestPortraitId, trimmedAge);
      if (!isCurrentPortrait(requestScope) || requestPreviewSeqRef.current !== requestSeq) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentPortrait(requestScope) || requestPreviewSeqRef.current !== requestSeq) return;
      addToast(extractErrorMessage(err, "预览年龄变体请求失败"), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentPortrait(requestScope) && requestPreviewSeqRef.current === requestSeq) {
        previewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  };

  return (
    <>
      <Modal
        isOpen
        onClose={() => {
          if (!busy) onClose();
        }}
        title="生成年龄变体"
        maxWidthClassName="max-w-sm"
        closeOnBackdrop={!busy}
        closeOnEscape={!busy}
        showCloseButton={!busy}
        footer={
          <div className="flex w-full items-center gap-2">
            <Dropdown
              items={[
                {
                  label: "查看模型请求",
                  icon: <Eye size={12} />,
                  onClick: handlePreview,
                  disabled: busy || !age.trim(),
                },
              ]}
            />
            <span className="min-w-0 flex-1" />
            <BookButton
              variant="primary"
              onClick={handleSubmit}
              disabled={busy || !age.trim()}
            >
              {busy ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> 生成中…
                </span>
              ) : (
                "生成变体"
              )}
            </BookButton>
          </div>
        }
      >
        <div className="space-y-3 text-sm">
          <div className="text-xs text-book-text-muted">
            基于现有人物三视图拆解生成不同年龄段的变体，原图保留。
          </div>
          <div>
            <label className="mb-1 block text-xs text-book-text-muted">年龄段</label>
            <BookInput
              value={age}
              onChange={(event) => setAge(event.target.value)}
              placeholder="少年 / 中年 / 老年 / 25 岁 ..."
              autoFocus
            />
          </div>
        </div>
      </Modal>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="年龄变体请求预览"
        onClose={() => {
          requestPreviewSeqRef.current += 1;
          previewBusyRef.current = false;
          setRequestPreviewOpen(false);
          setRequestPreviewLoading(false);
        }}
      />
    </>
  );
};
