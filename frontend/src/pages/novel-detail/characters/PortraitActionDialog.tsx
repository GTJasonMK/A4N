import React from "react";
import { Camera, Eye, RefreshCw, Star } from "lucide-react";
import type { Portrait } from "../../../api/portrait";
import { Modal } from "../../../components/ui/Modal";

interface PortraitActionDialogProps {
  portrait: Portrait;
  busyId: number | null;
  onClose: () => void;
  onPreviewRegenerate: (portrait: Portrait) => void | Promise<void>;
  onRegenerate: (portrait: Portrait) => void | Promise<void>;
  onSetPrimary: (portrait: Portrait) => void | Promise<void>;
  onAgeVariant: (portrait: Portrait) => void;
  onPrompt: (portrait: Portrait) => void;
}

export const PortraitActionDialog: React.FC<PortraitActionDialogProps> = ({
  portrait,
  busyId,
  onClose,
  onPreviewRegenerate,
  onRegenerate,
  onSetPrimary,
  onAgeVariant,
  onPrompt,
}) => {
  const busy = busyId === portrait.id;

  return (
    <Modal
      isOpen
      onClose={onClose}
      title={`立绘 #${portrait.id}`}
      subtitle="低频操作统一收纳，人物三视图拆解网格只保留核心预览信息。"
      icon={<Camera size={15} />}
      maxWidthClassName="max-w-md"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={busyId == null}
      closeOnEscape={busyId == null}
      showCloseButton={busyId == null}
    >
      <div className="space-y-3">
        <PortraitActionButton
          icon={<Eye size={14} />}
          title="预览重绘请求"
          description="查看将发送给 LLM 和图片供应商的请求。"
          disabled={busy}
          onClick={() => void onPreviewRegenerate(portrait)}
        />
        <PortraitActionButton
          icon={<RefreshCw size={14} className={busy ? "animate-spin" : undefined} />}
          title="重绘立绘"
          description="基于当前角色和原提示词重新生成。"
          disabled={busy}
          onClick={() => void onRegenerate(portrait)}
        />
        <PortraitActionButton
          icon={<Star size={14} fill={portrait.is_primary ? "currentColor" : "none"} />}
          title={portrait.is_primary ? "当前已是主图" : "设为主图"}
          description="作为该角色默认引用的立绘。"
          disabled={busy || portrait.is_primary}
          onClick={() => void onSetPrimary(portrait)}
        />
        <PortraitActionButton
          icon={<Camera size={14} />}
          title="年龄变体"
          description="基于现有人物三视图拆解生成少年、中年或老年等变体。"
          disabled={busy}
          onClick={() => onAgeVariant(portrait)}
        />
        <PortraitActionButton
          icon={<RefreshCw size={14} />}
          title="查看提示词"
          description="查看原始提示词，并可渲染模板结果。"
          disabled={busy}
          onClick={() => onPrompt(portrait)}
        />
      </div>
    </Modal>
  );
};

const PortraitActionButton: React.FC<{
  icon: React.ReactNode;
  title: string;
  description: string;
  disabled?: boolean;
  onClick: () => void;
}> = ({ icon, title, description, disabled, onClick }) => (
  <button
    type="button"
    disabled={disabled}
    onClick={onClick}
    className="flex w-full items-start gap-3 rounded-sm border border-book-border bg-book-bg-paper/40 px-3 py-3 text-left transition-colors hover:border-book-primary/60 hover:bg-book-bg-tertiary/45 disabled:cursor-not-allowed disabled:opacity-55"
  >
    <span className="mt-0.5 inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border border-book-primary/30 bg-book-primary/10 text-book-primary">
      {icon}
    </span>
    <span className="min-w-0">
      <span className="block text-sm font-semibold text-book-text-main">{title}</span>
      <span className="mt-1 block text-[11px] leading-relaxed text-book-text-muted">
        {description}
      </span>
    </span>
  </button>
);
