import React from "react";
import { Braces, Edit3, Loader2, ScrollText } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import type { WorldObject } from "./worldTabUtils";

interface WorldPanelHeaderProps {
  editing: boolean;
  saving: boolean;
  canEdit: boolean;
  worldObject: WorldObject | null;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onSave: () => void | Promise<void>;
  onOpenDetail: () => void;
}

export const WorldPanelHeader: React.FC<WorldPanelHeaderProps> = ({
  editing,
  saving,
  canEdit,
  worldObject,
  onStartEdit,
  onCancelEdit,
  onSave,
  onOpenDetail,
}) => {
  return (
    <div className="mb-4 flex items-center justify-between gap-2 border-b border-book-border/50 pb-3">
      <div className="min-w-0">
        <h3 className="inline-flex items-center gap-2 text-sm font-serif text-book-text-main">
          <ScrollText size={14} className="text-book-primary" /> 世界观设定
        </h3>
        <div className="mt-1 text-[11px] text-book-text-muted">
          用于约束剧情生成、角色行为与章节细节的一致性。
        </div>
      </div>

      {!editing ? (
        <div className="flex items-center gap-2">
          {worldObject && (
            <BookButton variant="secondary" size="sm" onClick={onOpenDetail}>
              <span className="inline-flex items-center gap-1.5">
                <Braces size={12} />
                详情
              </span>
            </BookButton>
          )}
          <BookButton variant="primary" size="sm" onClick={onStartEdit} disabled={!canEdit}>
            <span className="inline-flex items-center gap-1.5">
              <Edit3 size={12} /> 编辑
            </span>
          </BookButton>
        </div>
      ) : (
        <div className="flex items-center gap-2">
          <BookButton variant="secondary" size="sm" onClick={onCancelEdit} disabled={saving}>
            取消
          </BookButton>
          <BookButton variant="primary" size="sm" onClick={() => void onSave()} disabled={saving}>
            {saving ? (
              <span className="inline-flex items-center gap-2">
                <Loader2 size={13} className="animate-spin" /> 保存中…
              </span>
            ) : (
              "保存"
            )}
          </BookButton>
        </div>
      )}
    </div>
  );
};
