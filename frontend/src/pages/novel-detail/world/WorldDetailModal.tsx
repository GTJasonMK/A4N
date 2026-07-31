import React from "react";
import { ScrollText } from "lucide-react";
import { Modal } from "../../../components/ui/Modal";
import { WorldObjectView } from "./WorldReadViews";
import { stringifyPretty, type WorldObject } from "./worldTabUtils";

interface WorldDetailModalProps {
  open: boolean;
  worldObject: WorldObject | null;
  onClose: () => void;
}

export const WorldDetailModal: React.FC<WorldDetailModalProps> = ({
  open,
  worldObject,
  onClose,
}) => {
  return (
    <Modal
      isOpen={open}
      onClose={onClose}
      title="完整世界观"
      subtitle="世界规则、地点、阵营与其他扩展设定"
      icon={<ScrollText size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="px-4 py-4"
    >
      {worldObject ? <WorldObjectView obj={worldObject} /> : null}
      {worldObject ? (
        <details className="mt-4 text-caption text-book-text-muted">
          <summary className="w-fit cursor-pointer text-xs hover:text-book-text-main">
            诊断信息
          </summary>
          <pre className="mt-1 whitespace-pre-wrap break-words rounded-[10px] bg-book-bg-tertiary/45 p-3 font-mono text-caption leading-relaxed text-book-text-sub">
            {stringifyPretty(worldObject)}
          </pre>
        </details>
      ) : null}
    </Modal>
  );
};
