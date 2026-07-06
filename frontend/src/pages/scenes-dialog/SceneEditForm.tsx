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
    <div className="grid grid-cols-2 gap-2">
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
          placeholder="dialog / action / introspection"
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
        className="book-control min-h-[40px] rounded p-2 text-xs resize-none"
      />
    </label>
    <label className="flex flex-col gap-1">
      <span className="text-[10px] text-book-text-muted">正文</span>
      <textarea
        value={editTextValue(form.content)}
        onChange={(event) => onChange({ content: event.target.value })}
        disabled={saving}
        spellCheck={false}
        className="book-control min-h-[140px] rounded p-2 text-xs font-serif leading-relaxed resize-y"
      />
    </label>
    <div className="flex items-center justify-end gap-2 mt-1">
      <BookButton variant="secondary" onClick={onCancel} disabled={saving}>
        取消
      </BookButton>
      <BookButton variant="primary" onClick={onSave} disabled={saving}>
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
