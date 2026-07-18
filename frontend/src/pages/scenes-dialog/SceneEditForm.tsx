import React from "react";
import { Loader2, Save } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { editTextValue, type SceneEditForm as SceneEditFormState } from "./scenesDialogUtils";

interface SceneEditFormProps {
  form: SceneEditFormState;
  saving: boolean;
  onChange: (patch: Partial<SceneEditFormState>) => void;
  onCancel: () => void;
  onSave: () => void;
}

export const SceneEditForm: React.FC<SceneEditFormProps> = ({
  form,
  saving,
  onChange,
  onCancel,
  onSave,
}) => (
  <div className="flex flex-col gap-2">
    <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
      <label className="flex flex-col gap-1">
        <span className="text-[10px] text-book-text-muted">标题</span>
        <input
          type="text"
          value={editTextValue(form.title)}
          onChange={(event) => onChange({ title: event.target.value })}
          disabled={saving}
          className="pyqt-input !py-1 text-xs"
        />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-[10px] text-book-text-muted">场景类型</span>
        <input
          type="text"
          value={editTextValue(form.scene_type)}
          onChange={(event) => onChange({ scene_type: event.target.value })}
          disabled={saving}
          placeholder="例如：对话、行动、内心"
          className="pyqt-input !py-1 text-xs"
        />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-[10px] text-book-text-muted">地点</span>
        <input
          type="text"
          value={editTextValue(form.location)}
          onChange={(event) => onChange({ location: event.target.value })}
          disabled={saving}
          className="pyqt-input !py-1 text-xs"
        />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-[10px] text-book-text-muted">时间点</span>
        <input
          type="text"
          value={editTextValue(form.timeline_position)}
          onChange={(event) => onChange({ timeline_position: event.target.value })}
          disabled={saving}
          className="pyqt-input !py-1 text-xs"
        />
      </label>
    </div>
    <label className="flex flex-col gap-1">
      <span className="text-[10px] text-book-text-muted">摘要</span>
      <textarea
        value={editTextValue(form.summary)}
        onChange={(event) => onChange({ summary: event.target.value })}
        disabled={saving}
        className="book-control min-h-[48px] resize-none rounded-xl p-2 text-xs"
      />
    </label>
    <label className="flex flex-col gap-1">
      <span className="text-[10px] text-book-text-muted">正文</span>
      <textarea
        value={editTextValue(form.content)}
        onChange={(event) => onChange({ content: event.target.value })}
        disabled={saving}
        spellCheck={false}
        className="book-control min-h-[140px] resize-y rounded-xl p-2 font-serif text-xs leading-relaxed"
      />
    </label>
    <div className="flex items-center justify-end gap-2 mt-1">
      <BookButton variant="secondary" size="sm" onClick={onCancel} disabled={saving}>
        取消
      </BookButton>
      <BookButton variant="primary" size="sm" onClick={onSave} disabled={saving}>
        {saving ? (
          <span className="inline-flex items-center gap-2">
            <Loader2 size={14} className="animate-spin" /> 保存中…
          </span>
        ) : (
          <span className="inline-flex items-center gap-2">
            <Save size={14} /> 保存
          </span>
        )}
      </BookButton>
    </div>
  </div>
);
