import React from "react";
import { Loader2, UserPlus } from "lucide-react";
import { Modal } from "../../components/ui/Modal";
import { BookButton } from "../../components/ui/BookButton";
import { ProfileTextarea } from "./CharacterProfileDisplay";

export interface ProfileForm {
  name: string;
  gender: string;
  age: string;
  personality: string;
  background: string;
  appearance: string;
}

export const EMPTY_PROFILE_FORM: ProfileForm = {
  name: "",
  gender: "",
  age: "",
  personality: "",
  background: "",
  appearance: "",
};

export const CharacterProfileEditorModal: React.FC<{
  open: boolean;
  form: ProfileForm;
  saving: boolean;
  onChange: (form: ProfileForm) => void;
  onSave: () => void | Promise<void>;
  onClose: () => void;
}> = ({ open, form, saving, onChange, onSave, onClose }) => {
  if (!open) return null;

  return (
    <Modal
      isOpen
      onClose={() => !saving && onClose()}
      title="新建角色档案"
      icon={<UserPlus size={15} />}
      subtitle="建立随章节同步维护的角色档案"
      maxWidthClassName="max-w-lg"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!saving}
      closeOnEscape={!saving}
      showCloseButton={!saving}
      footer={
        <>
          <BookButton variant="secondary" onClick={onClose} disabled={saving}>
            取消
          </BookButton>
          <BookButton variant="primary" onClick={onSave} disabled={saving}>
            {saving ? (
              <span className="inline-flex items-center gap-2">
                <Loader2 size={14} className="animate-spin" /> 保存中…
              </span>
            ) : (
              "保存"
            )}
          </BookButton>
        </>
      }
    >
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <label className="flex flex-col gap-1.5 sm:col-span-2">
          <span className="text-xs text-book-text-sub">姓名 *</span>
          <input
            type="text"
            value={form.name}
            onChange={(event) => onChange({ ...form, name: event.target.value })}
            disabled={saving}
            className="pyqt-input"
            placeholder="角色姓名"
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">性别</span>
          <input
            type="text"
            value={form.gender}
            onChange={(event) => onChange({ ...form, gender: event.target.value })}
            disabled={saving}
            className="pyqt-input"
            placeholder="男 / 女 / 其他"
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">年龄</span>
          <input
            type="text"
            value={form.age}
            onChange={(event) => onChange({ ...form, age: event.target.value })}
            disabled={saving}
            className="pyqt-input"
            placeholder="18 / 弱冠 / 不详"
          />
        </label>
        <ProfileTextarea
          label="性格"
          value={form.personality}
          disabled={saving}
          placeholder="冷静、固执、外热内冷..."
          onChange={(value) => onChange({ ...form, personality: value })}
        />
        <ProfileTextarea
          label="背景"
          value={form.background}
          disabled={saving}
          placeholder="出身、成长经历、关键事件"
          onChange={(value) => onChange({ ...form, background: value })}
        />
        <ProfileTextarea
          label="外观"
          value={form.appearance}
          disabled={saving}
          placeholder="身形、五官、常穿衣物"
          onChange={(value) => onChange({ ...form, appearance: value })}
        />
      </div>
    </Modal>
  );
};
