import React from "react";
import { ScrollText } from "lucide-react";
import { Modal } from "../../../components/ui/Modal";
import { WorldObjectView } from "./WorldReadViews";
import { stringifyPretty, type WorldObject } from "./worldTabUtils";

interface WorldDetailModalProps {
  open: boolean;
  worldObject: WorldObject | null;
  showJsonView: boolean;
  onClose: () => void;
  onToggleJsonView: () => void;
}

export const WorldDetailModal: React.FC<WorldDetailModalProps> = ({
  open,
  worldObject,
  showJsonView,
  onClose,
  onToggleJsonView,
}) => {
  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title="完整世界观"
      subtitle="完整设定收纳在弹窗中，主页面只保留核心摘要。"
      icon={<ScrollText size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="px-4 py-4"
    >
      {worldObject ? <WorldObjectView obj={worldObject} /> : null}
      {worldObject ? (
        <div className="mt-4 rounded-sm border border-book-border/40 bg-book-bg-tertiary/35 p-3">
          <button
            type="button"
            className="text-xs font-semibold text-book-text-sub hover:text-book-text-main"
            onClick={onToggleJsonView}
          >
            {showJsonView ? "隐藏 JSON 原文" : "查看 JSON 原文"}
          </button>
          {showJsonView && (
            <pre className="mt-2 max-h-80 overflow-y-auto whitespace-pre-wrap break-words rounded-sm bg-book-bg-paper p-3 font-mono text-[10px] leading-relaxed text-book-text-sub custom-scrollbar">
              {stringifyPretty(worldObject)}
            </pre>
          )}
        </div>
      ) : null}
    </Modal>
  );
};
