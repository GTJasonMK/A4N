import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye, Loader2 } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  generateAgeVariant,
  previewGenerateAgeVariantRequest,
  type Portrait,
} from "../../../api/portrait";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { Modal } from "../../../components/ui/Modal";
import { createProgressTaskId, useProgressTask } from "../../../hooks/useProgressTask";
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
    const nextTaskId = createProgressTaskId("portrait-age-variant", portrait.project_id);
    submitBusyRef.current = true;
    setBusy(true);
    try {
      await runProgressTask(
        {
          title: "生成年龄变体",
          startMessage: "正在准备年龄变体",
          successMessage: "年龄变体生成完成",
          errorMessage: "生成失败",
          total: 4,
          taskId: nextTaskId,
          cancelLabel: "停止生成",
          runningHint: "年龄变体生成过程中请保持当前窗口打开",
        },
        () => {
          if (!isCurrentPortrait(requestScope)) {
            return Promise.reject(new Error("任务已停止"));
          }
          return generateAgeVariant(requestPortraitId, trimmedAge, nextTaskId);
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
        onClose={onClose}
        title={`生成年龄变体 #${portrait.id}`}
        maxWidthClassName="max-w-sm"
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
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <BookButton variant="secondary" onClick={onClose} disabled={busy}>
              取消
            </BookButton>
            <BookButton variant="secondary" onClick={handlePreview} disabled={busy}>
              <span className="inline-flex items-center gap-1.5">
                <Eye size={12} />
                预览请求
              </span>
            </BookButton>
            <BookButton variant="primary" onClick={handleSubmit} disabled={busy}>
              {busy ? (
                <span className="inline-flex items-center gap-1.5">
                  <Loader2 size={12} className="animate-spin" /> 生成中…
                </span>
              ) : (
                "生成"
              )}
            </BookButton>
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
