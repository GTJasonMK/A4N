import React from "react";
import { Camera,
  ChevronRight,
  Clock3,
  Eye,
  FileText,
  RefreshCw,
  Star, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { Portrait } from "../../../api/portrait";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { toMediaSrc } from "../../../utils/mediaSrc";

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
  const imageSrc = toMediaSrc(portrait.image_path);
  const metadata = [portrait.portrait_type, portrait.expression, portrait.pose]
    .map((item) => item?.trim())
    .filter(Boolean)
    .join(" · ");

  return (
    <Modal
      isOpen
      onClose={() => {
        if (!busy) onClose();
      }}
      title="立绘详情"
      subtitle={metadata || "人物视觉素材"}
      icon={<Camera size={15} />}
      maxWidthClassName="max-w-2xl"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
      footer={
        <div className="flex w-full items-center gap-2">
          <Dropdown
            items={[
              {
                label: "查看重绘模型请求",
                icon: <Eye size={12} />,
                onClick: () => void onPreviewRegenerate(portrait),
                disabled: busy,
              },
            ]}
          />
          <span className="min-w-0 flex-1" />
          <BookButton
            variant="primary"
            size="sm"
            onClick={() => void onRegenerate(portrait)}
            disabled={busy}
            className="!text-xs"
          >
            {busy ? (
              <LoadingSpinner size="sm" />
            ) : (
              <RefreshCw size={13} />
            )}
            {busy ? "重绘中…" : "重绘立绘"}
          </BookButton>
        </div>
      }
    >
      <div className="grid gap-4 sm:grid-cols-[180px_minmax(0,1fr)]">
        <div className="min-w-0">
          <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden rounded-[14px] border border-book-border/55 bg-book-bg-tertiary text-xs text-book-text-muted">
            {imageSrc ? (
              <img
                src={imageSrc}
                alt="人物三视图预览"
                className="h-full w-full object-contain"
              />
            ) : (
              "无图"
            )}
            {portrait.is_primary ? (
              <span className="absolute right-2 top-2 inline-flex items-center gap-1 rounded-lg bg-book-primary/90 px-2 py-1 text-caption font-medium text-white backdrop-blur-sm">
                <Star size={9} fill="currentColor" /> 主图
              </span>
            ) : null}
          </div>
        </div>

        <div className="min-w-0">
          <div className="space-y-2">
            {!portrait.is_primary ? (
              <PortraitActionButton
                icon={<Star size={14} />}
                title="设为主图"
                description="作为该角色默认引用的立绘"
                disabled={busy}
                onClick={() => void onSetPrimary(portrait)}
              />
            ) : null}
            <PortraitActionButton
              icon={<Clock3 size={14} />}
              title="生成年龄变体"
              description="基于当前形象生成少年、中年或老年版本"
              disabled={busy}
              onClick={() => onAgeVariant(portrait)}
            />
            <PortraitActionButton
              icon={<FileText size={14} />}
              title="查看提示词"
              description="查看当前提示词或重新渲染模板"
              disabled={busy}
              onClick={() => onPrompt(portrait)}
            />
          </div>
          <p className="mt-2 px-1 text-caption leading-relaxed text-book-text-muted">
            重绘会保留当前图片，并生成一张新的候选立绘。
          </p>
        </div>
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
    className="a4n-flat-surface flex w-full items-start gap-3 px-3 py-3 text-left transition-colors hover:border-book-primary/30 hover:bg-book-bg-tertiary/45 disabled:cursor-not-allowed disabled:opacity-55"
  >
    <span className="mt-0.5 inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-book-primary/10 text-book-primary">
      {icon}
    </span>
    <span className="min-w-0 flex-1">
      <span className="block text-sm font-semibold text-book-text-main">{title}</span>
      <span className="mt-1 block text-small leading-relaxed text-book-text-muted">
        {description}
      </span>
    </span>
    <ChevronRight size={14} className="mt-2 shrink-0 text-book-text-muted" />
  </button>
);
