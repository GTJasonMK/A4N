import React from "react";
import { ChevronDown,
  ChevronRight,
  ChevronsDown,
  ChevronsUp,
  Pencil,
  Trash2, } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import type { ChapterSceneSummary } from "../../api/chapter";
import { Dropdown, type DropdownItem } from "../../components/ui/Dropdown";
import { nonEmptyText } from "../../utils/text";
import { SceneEditForm } from "./SceneEditForm";
import type { SceneEditForm as SceneEditFormState } from "./scenesDialogUtils";

interface SceneListProps {
  loading: boolean;
  loadError: string | null;
  scenes: ChapterSceneSummary[];
  expanded: Set<number>;
  editingId: number | null;
  editForm: SceneEditFormState;
  saving: boolean;
  actingKeys: Set<string>;
  onToggleExpand: (sceneId: number) => void;
  onStartEdit: (scene: ChapterSceneSummary) => void;
  onCancelEdit: () => void;
  onEditFormChange: (patch: Partial<SceneEditFormState>) => void;
  onSave: () => void;
  onMove: (scene: ChapterSceneSummary, direction: -1 | 1) => void;
  onDelete: (scene: ChapterSceneSummary) => void;
}

export const SceneList: React.FC<SceneListProps> = ({
  loading,
  loadError,
  scenes,
  expanded,
  editingId,
  editForm,
  saving,
  actingKeys,
  onToggleExpand,
  onStartEdit,
  onCancelEdit,
  onEditFormChange,
  onSave,
  onMove,
  onDelete,
}) => {
  if (loading && scenes.length === 0) {
    return (
      <div className="py-6 text-center text-xs text-book-text-muted inline-flex items-center justify-center gap-2">
        <LoadingSpinner size="sm" /> 加载场景…
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="border-l-2 border-error/55 py-2 pl-3 text-xs leading-relaxed text-error">
        {loadError}
      </div>
    );
  }

  if (scenes.length === 0) {
    return (
      <div className="py-8 text-center text-sm text-book-text-muted">
        本章尚未拆分场景。
      </div>
    );
  }

  return (
    <ul className="a4n-flat-surface divide-y divide-book-border/35">
      {scenes.map((scene, index) => (
        <SceneListItem
          key={scene.id}
          scene={scene}
          isExpanded={expanded.has(scene.id)}
          isEditing={editingId === scene.id}
          isFirst={index === 0}
          isLast={index === scenes.length - 1}
          editForm={editForm}
          saving={saving}
          actingKeys={actingKeys}
          onToggleExpand={onToggleExpand}
          onStartEdit={onStartEdit}
          onCancelEdit={onCancelEdit}
          onEditFormChange={onEditFormChange}
          onSave={onSave}
          onMove={onMove}
          onDelete={onDelete}
        />
      ))}
    </ul>
  );
};

interface SceneListItemProps {
  scene: ChapterSceneSummary;
  isExpanded: boolean;
  isEditing: boolean;
  isFirst: boolean;
  isLast: boolean;
  editForm: SceneEditFormState;
  saving: boolean;
  actingKeys: Set<string>;
  onToggleExpand: (sceneId: number) => void;
  onStartEdit: (scene: ChapterSceneSummary) => void;
  onCancelEdit: () => void;
  onEditFormChange: (patch: Partial<SceneEditFormState>) => void;
  onSave: () => void;
  onMove: (scene: ChapterSceneSummary, direction: -1 | 1) => void;
  onDelete: (scene: ChapterSceneSummary) => void;
}

const SceneListItem: React.FC<SceneListItemProps> = ({
  scene,
  isExpanded,
  isEditing,
  isFirst,
  isLast,
  editForm,
  saving,
  actingKeys,
  onToggleExpand,
  onStartEdit,
  onCancelEdit,
  onEditFormChange,
  onSave,
  onMove,
  onDelete,
}) => {
  const actionBusy = actingKeys.has(`move-${scene.id}`) || actingKeys.has(`del-${scene.id}`);
  const actionItems: DropdownItem[] = [];

  if (!isFirst) {
    actionItems.push({
      label: "上移场景",
      icon: <ChevronsUp size={12} />,
      onClick: () => onMove(scene, -1),
    });
  }
  if (!isLast) {
    actionItems.push({
      label: "下移场景",
      icon: <ChevronsDown size={12} />,
      onClick: () => onMove(scene, 1),
    });
  }
  actionItems.push({
    label: "编辑场景",
    icon: <Pencil size={12} />,
    onClick: () => onStartEdit(scene),
  });
  actionItems.push({ type: "divider" });
  actionItems.push({
    label: "删除场景",
    icon: <Trash2 size={12} />,
    danger: true,
    onClick: () => onDelete(scene),
  });

  return (
    <li className="p-3">
      <div className="flex items-start gap-2">
        <button
          type="button"
          onClick={() => onToggleExpand(scene.id)}
          className="mt-0.5 shrink-0 text-book-text-muted hover:text-book-text-main"
        >
          {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </button>
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-semibold text-book-text-main">
            {nonEmptyText(scene.title) ?? "未命名场景"}
          </div>
          <div className="mt-0.5 truncate text-caption text-book-text-muted">
            {[
              `场景 ${scene.scene_number}`,
              scene.scene_type ? sceneTypeLabel(scene.scene_type) : null,
              scene.word_count > 0 ? `${scene.word_count.toLocaleString()} 字` : null,
              scene.location || null,
            ]
              .filter(Boolean)
              .join(" · ")}
          </div>
          {!isExpanded && scene.summary && (
            <div className="mt-0.5 line-clamp-1 text-xs text-book-text-muted">
              {scene.summary}
            </div>
          )}
        </div>
        <div className="shrink-0">
          {actionBusy ? (
            <span className="inline-flex h-9 w-9 items-center justify-center text-book-text-muted sm:h-8 sm:w-8">
              <LoadingSpinner size="sm" />
            </span>
          ) : isEditing ? null : (
            <Dropdown items={actionItems} />
          )}
        </div>
      </div>

      {isExpanded && (
        <div className="mt-3 border-t border-book-border/30 pt-3">
          {!isEditing ? (
            <>
              {scene.summary && (
                <div className="mb-2 text-xs leading-relaxed text-book-text-sub">
                  <span className="text-book-text-muted">摘要：</span>
                  {scene.summary}
                </div>
              )}
              {scene.content?.trim() ? (
                <pre className="whitespace-pre-wrap font-serif text-xs leading-relaxed text-book-text-main">
                  {scene.content}
                </pre>
              ) : (
                <div className="text-xs text-error">场景正文缺失</div>
              )}
            </>
          ) : (
            <SceneEditForm
              form={editForm}
              saving={saving}
              onChange={onEditFormChange}
              onCancel={onCancelEdit}
              onSave={onSave}
            />
          )}
        </div>
      )}
    </li>
  );
};

const SCENE_TYPE_LABELS: Record<string, string> = {
  dialog: "对话",
  action: "行动",
  introspection: "内心",
  exposition: "叙述",
  transition: "过渡",
  climax: "高潮",
};

const sceneTypeLabel = (value: string): string =>
  SCENE_TYPE_LABELS[value.trim().toLowerCase()] ?? "其他场景";
