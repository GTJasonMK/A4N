import React, { useEffect, useState } from "react";
import { Copy, Eye, Loader2 } from "lucide-react";
import { extractErrorMessage } from "../../api";
import type { CanvasNodeRequestPreview } from "../../api/canvas";
import { BookButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import { copyTextToClipboard } from "../../utils/clipboard";
import { nodeKindMeta, stateLabel } from "./StudioCanvasNode";

interface CanvasNodePreviewDialogProps {
  open: boolean;
  loading: boolean;
  preview: CanvasNodeRequestPreview | null;
  onClose: () => void;
  onCopyError: (message: string) => void;
}

export const CanvasNodePreviewDialog: React.FC<CanvasNodePreviewDialogProps> = ({
  open,
  loading,
  preview,
  onClose,
  onCopyError,
}) => {
  const [copied, setCopied] = useState(false);
  const requestJson = preview ? formatPreviewJson(preview.request) : "";
  const title = preview ? `${nodeKindMeta(preview.node_kind).label} 请求预览` : "节点请求预览";

  useEffect(() => {
    if (!open) setCopied(false);
  }, [open]);

  const copyPreview = async () => {
    if (!requestJson) return;
    try {
      await copyTextToClipboard(requestJson);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch (err) {
      setCopied(false);
      onCopyError(extractErrorMessage(err, "复制节点请求预览失败"));
    }
  };

  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title={title}
      subtitle={
        preview
          ? `${stateLabel(preview.status)} · 仅预览，不会执行节点`
          : "构造将用于运行节点的请求，不执行节点、不调用外部模型"
      }
      icon={loading ? <Loader2 size={15} className="animate-spin" /> : <Eye size={15} />}
      maxWidthClassName="max-w-[min(96vw,72rem)]"
      bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4"
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          {copied ? <span className="text-xs text-book-text-muted">已复制</span> : null}
          <BookButton
            variant="secondary"
            size="sm"
            onClick={() => void copyPreview()}
            disabled={!requestJson || loading}
          >
            <Copy size={13} />
            复制请求
          </BookButton>
        </div>
      }
    >
      {loading ? (
        <div className="flex min-h-[18rem] flex-col items-center justify-center gap-3 text-book-text-muted">
          <Loader2 size={24} className="animate-spin text-book-primary" />
          <span className="text-sm">正在构造节点请求预览</span>
        </div>
      ) : preview ? (
        <div className="space-y-3 text-xs">
          <div className="rounded-xl border border-book-border bg-book-bg p-3 text-book-text-sub">
            {preview.note}
          </div>
          <details className="overflow-hidden rounded-xl border border-book-border bg-book-bg-paper">
            <summary className="flex cursor-pointer items-center justify-between gap-3 px-3 py-2.5 text-[11px] text-book-text-sub">
              <span>完整请求</span>
              <span className="text-book-text-muted">{requestJson.length.toLocaleString()} 字符</span>
            </summary>
            <pre className="whitespace-pre-wrap break-words border-t border-book-border px-3 py-2 font-mono text-[11px] leading-relaxed text-book-text-main">
              {requestJson}
            </pre>
          </details>
        </div>
      ) : (
        <div className="py-10 text-center text-sm text-book-text-muted">暂无预览数据</div>
      )}
    </Modal>
  );
};

function formatPreviewJson(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2);
  } catch (_err) {
    return String(value);
  }
}
