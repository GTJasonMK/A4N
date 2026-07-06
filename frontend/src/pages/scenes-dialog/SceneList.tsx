import React from "react";
import {
  ChevronDown,
  ChevronRight,
  ChevronsDown,
  ChevronsUp,
  Loader2,
  Trash2,
} from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import type { ChapterScene } from "../../api/chapter";
import { nonEmptyText } from "../../utils/text";
import { SceneEditForm } from "./SceneEditForm";
import type { SceneEditForm as SceneEditFormState } from "./scenesDialogUtils";

interface SceneListProps {
  loading: boolean;
  loadError: string | null;
  scenes: ChapterScene[];
  expanded: Set<number>;
  editingId: number | null;
  editForm: SceneEditFormState;
  saving: boolean;
  actingKeys: Set<string>;
  onToggleExpand: (sceneId: number) => void;
  onStartEdit: (scene: ChapterScene) => void;
  onCancelEdit: () => void;
  onEditFormChange: (patch: Partial<SceneEditFormState>) => void;
  onSave: () => void;
  onMove: (scene: ChapterScene, direction: -1 | 1) => void;
  onDelete: (scene: ChapterScene) => void;
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
        <Loader2 size={12} className="animate-spin" /> 加载场景…
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="rounded-sm border border-error/35 bg-error-bg/40 px-3 py-4 text-center text-xs leading-relaxed text-error">
        {loadError}
      </div>
    );
  }

  if (scenes.length === 0) {
    return (
      <div className="py-8 text-center text-sm text-book-text-muted">
        本章尚无场景。点击「生成下一场景」开始拆解。
      </div>
    );
  }

  return (
    <ul className="flex-1 overflow-y-auto custom-scrollbar space-y-2 pr-1">
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
  scene: ChapterScene;
  isExpanded: boolean;
  isEditing: boolean;
  isFirst: boolean;
  isLast: boolean;
  editForm: SceneEditFormState;
  saving: boolean;
  actingKeys: Set<string>;
  onToggleExpand: (sceneId: number) => void;
  onStartEdit: (scene: ChapterScene) => void;
  onCancelEdit: () => void;
  onEditFormChange: (patch: Partial<SceneEditFormState>) => void;
  onSave: () => void;
  onMove: (scene: ChapterScene, direction: -1 | 1) => void;
  onDelete: (scene: ChapterScene) => void;
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
}) => (
  <li className="pyqt-card p-3">
    <div className="flex items-start gap-2">
      <button
        type="button"
        onClick={() => onToggleExpand(scene.id)}
        className="mt-0.5 shrink-0 text-book-text-muted hover:text-book-text-main"
      >
        {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
      </button>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap mb-0.5">
          <span className="pyqt-pill !text-[10px] font-mono">#{scene.scene_number}</span>
          {scene.scene_type && <span className="pyqt-pill !text-[10px]">{scene.scene_type}</span>}
          {scene.word_count > 0 && (
            <span className="text-[10px] text-book-text-muted">
              {scene.word_count.toLocaleString()} 字
            </span>
          )}
          {scene.location && (
            <span className="text-[10px] text-book-text-muted">· {scene.location}</span>
          )}
        </div>
        <div className="text-sm font-semibold text-book-text-main truncate">
          {nonEmptyText(scene.title) ?? `场景 #${scene.scene_number}`}
        </div>
        {!isExpanded && scene.summary && (
          <div className="text-xs text-book-text-muted line-clamp-1 mt-0.5">
            {scene.summary}
          </div>
        )}
      </div>
      <div className="flex items-center gap-1 shrink-0">
        <button
          type="button"
          onClick={() => onMove(scene, -1)}
          disabled={isFirst || actingKeys.has(`move-${scene.id}`)}
          title="上移"
          className="p-1 rounded text-book-text-muted hover:text-book-text-main hover:bg-book-bg-tertiary disabled:opacity-40 disabled:cursor-not-allowed"
        >
          <ChevronsUp size={12} />
        </button>
        <button
          type="button"
          onClick={() => onMove(scene, 1)}
          disabled={isLast || actingKeys.has(`move-${scene.id}`)}
          title="下移"
          className="p-1 rounded text-book-text-muted hover:text-book-text-main hover:bg-book-bg-tertiary disabled:opacity-40 disabled:cursor-not-allowed"
        >
          <ChevronsDown size={12} />
        </button>
        <button
          type="button"
          onClick={() => onDelete(scene)}
          disabled={actingKeys.has(`del-${scene.id}`)}
          title="删除"
          className="p-1 rounded text-error hover:bg-error-bg disabled:opacity-40"
        >
          {actingKeys.has(`del-${scene.id}`) ? (
            <Loader2 size={12} className="animate-spin" />
          ) : (
            <Trash2 size={12} />
          )}
        </button>
      </div>
    </div>

    {isExpanded && (
      <div className="mt-3 pt-3 border-t border-book-border/30">
        {!isEditing ? (
          <>
            {scene.summary && (
              <div className="text-xs text-book-text-sub mb-2 leading-relaxed">
                <span className="text-book-text-muted">摘要：</span>
                {scene.summary}
              </div>
            )}
            {scene.content?.trim() ? (
              <pre className="text-xs whitespace-pre-wrap font-serif text-book-text-main leading-relaxed max-h-48 overflow-y-auto custom-scrollbar">
                {scene.content}
              </pre>
            ) : (
              <div className="text-xs text-error">场景正文缺失</div>
            )}
            <div className="mt-2 text-right">
              <BookButton variant="secondary" onClick={() => onStartEdit(scene)}>
                编辑
              </BookButton>
            </div>
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
