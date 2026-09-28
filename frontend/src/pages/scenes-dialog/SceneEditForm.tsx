import React from "react";
import { Save } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { editTextValue, type SceneEditForm as SceneEditFormState } from "./scenesDialogUtils";
import { useI18n } from "../../i18n";

interface SceneEditFormProps {
  form: SceneEditFormState;
  dirty: boolean;
  saving: boolean;
  disabled: boolean;
  onChange: (patch: Partial<SceneEditFormState>) => void;
  onCancel: () => void;
  onSave: () => void;
}

export const SceneEditForm: React.FC<SceneEditFormProps> = ({
  form,
  dirty,
  saving,
  disabled,
  onChange,
  onCancel,
  onSave,
}) => {
  const { t } = useI18n();
  return (
  <div className="flex flex-col gap-2">
    <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
      <label className="flex flex-col gap-1">
        <span className="text-caption text-book-text-muted">{t("scenes.title")}</span>
        <input
          type="text"
          value={editTextValue(form.title)}
          onChange={(event) => onChange({ title: event.target.value })}
          disabled={disabled || saving}
          className="pyqt-input !py-1 text-xs"
        />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-caption text-book-text-muted">{t("scenes.type")}</span>
        <input
          type="text"
          value={editTextValue(form.scene_type)}
          onChange={(event) => onChange({ scene_type: event.target.value })}
          disabled={disabled || saving}
          placeholder={t("scenes.typePlaceholder")}
          className="pyqt-input !py-1 text-xs"
        />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-caption text-book-text-muted">{t("scenes.location")}</span>
        <input
          type="text"
          value={editTextValue(form.location)}
          onChange={(event) => onChange({ location: event.target.value })}
          disabled={disabled || saving}
          className="pyqt-input !py-1 text-xs"
        />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-caption text-book-text-muted">{t("scenes.timeline")}</span>
        <input
          type="text"
          value={editTextValue(form.timeline_position)}
          onChange={(event) => onChange({ timeline_position: event.target.value })}
          disabled={disabled || saving}
          className="pyqt-input !py-1 text-xs"
        />
      </label>
    </div>
    <label className="flex flex-col gap-1">
      <span className="text-caption text-book-text-muted">{t("scenes.summary")}</span>
      <textarea
        value={editTextValue(form.summary)}
        onChange={(event) => onChange({ summary: event.target.value })}
        disabled={disabled || saving}
        className="book-control min-h-[48px] resize-none rounded-xl p-2 text-xs"
      />
    </label>
    <label className="flex flex-col gap-1">
      <span className="text-caption text-book-text-muted">{t("scenes.content")}</span>
      <textarea
        value={editTextValue(form.content)}
        onChange={(event) => onChange({ content: event.target.value })}
        disabled={disabled || saving}
        spellCheck={false}
        className="book-control min-h-[140px] resize-y rounded-xl p-2 font-serif text-xs leading-relaxed"
      />
    </label>
    <div className="flex items-center justify-end gap-2 mt-1">
      <BookButton variant="secondary" size="sm" onClick={onCancel} disabled={disabled || saving}>
        {t("common.cancel")}
      </BookButton>
      <BookButton variant="primary" size="sm" onClick={onSave} disabled={disabled || saving || !dirty}>
        {saving ? (
          <span className="inline-flex items-center gap-2">
            <LoadingSpinner size="sm" /> {t("scenes.saving")}
          </span>
        ) : (
          <span className="inline-flex items-center gap-2">
            <Save size={14} /> {t("scenes.save")}
          </span>
        )}
      </BookButton>
    </div>
  </div>
  );
};
