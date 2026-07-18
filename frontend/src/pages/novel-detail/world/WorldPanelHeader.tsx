import React from "react";
import { Edit3, Loader2, ScrollText } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import type { WorldObject } from "./worldTabUtils";

interface WorldPanelHeaderProps {
  editing: boolean;
  saving: boolean;
  canEdit: boolean;
  hasBlueprint: boolean;
  hasWorldContent: boolean;
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
  hasBlueprint,
  hasWorldContent,
  worldObject,
  onStartEdit,
  onCancelEdit,
  onSave,
  onOpenDetail,
}) => {
  const description = editing
    ? "调整世界规则、关键地点、主要阵营和其他扩展设定"
    : !hasBlueprint
      ? "生成蓝图后，可以在这里维护故事发生世界的基础规则"
      : hasWorldContent
        ? worldObject
          ? "集中查看故事世界的核心规则、地点与阵营"
          : "当前世界观以纯文本记录"
        : "尚未建立世界观，可以从核心规则开始";
  const headerClassName = editing
    ? "mb-4 flex flex-col gap-3 border-b border-book-border/45 pb-3 sm:flex-row sm:items-center sm:justify-between"
    : "flex flex-col gap-3 px-1 sm:flex-row sm:items-center sm:justify-between";

  return (
    <header className={headerClassName}>
      <div className="min-w-0">
        <h2 className="text-sm font-semibold text-book-text-main">
          {editing ? "编辑世界观" : "世界观设定"}
        </h2>
        <p className="mt-0.5 text-[11px] leading-relaxed text-book-text-muted">
          {description}
        </p>
      </div>
      <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
        {!editing ? (
          <>
            {worldObject && hasWorldContent ? (
              <Dropdown
                items={[
                  {
                    label: "查看完整设定",
                    icon: <ScrollText size={12} />,
                    onClick: onOpenDetail,
                  },
                ]}
              />
            ) : null}
            {canEdit ? (
              <BookButton
                variant="primary"
                size="sm"
                onClick={onStartEdit}
                className="min-w-0 flex-1 !text-xs sm:flex-none"
              >
                <Edit3 size={12} /> 编辑世界观
              </BookButton>
            ) : null}
          </>
        ) : (
          <>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={onCancelEdit}
              disabled={saving}
              className="!text-xs"
            >
              退出编辑
            </BookButton>
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onSave()}
              disabled={saving}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {saving ? (
                <span className="inline-flex items-center gap-1.5">
                  <Loader2 size={13} className="animate-spin" /> 保存中…
                </span>
              ) : (
                "保存世界观"
              )}
            </BookButton>
          </>
        )}
      </div>
    </header>
  );
};
