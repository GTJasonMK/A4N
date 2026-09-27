import React, { useEffect, useRef } from "react";
import { Pencil, UserPlus } from "lucide-react";
import type { CharacterProfile } from "../../api/character";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { Modal } from "../../components/ui/Modal";
import { BookButton } from "../../components/ui/BookButton";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { ProfileTextarea } from "./CharacterProfileDisplay";
import { characterAttributeText } from "../../utils/characterAttributes";
import { useI18n } from "../../i18n";

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

const attributeCategory = (
  state: unknown,
  key: "explicit_attributes" | "implicit_attributes" | "social_attributes",
): Record<string, unknown> => {
  if (state == null || typeof state !== "object" || Array.isArray(state)) return {};
  const value = (state as Record<string, unknown>)[key];
  return value != null && typeof value === "object" && !Array.isArray(value)
    ? { ...(value as Record<string, unknown>) }
    : {};
};

const setOptionalText = (
  attributes: Record<string, unknown>,
  key: string,
  value: string,
) => {
  const text = value.trim();
  if (text) attributes[key] = text;
  else delete attributes[key];
};

export const profileFormBaseAttributes = (form: ProfileForm, currentState?: unknown) => {
  const explicit = attributeCategory(currentState, "explicit_attributes");
  const implicit = attributeCategory(currentState, "implicit_attributes");
  const social = attributeCategory(currentState, "social_attributes");
  setOptionalText(explicit, "gender", form.gender);
  setOptionalText(explicit, "age", form.age);
  setOptionalText(explicit, "background", form.background);
  setOptionalText(explicit, "appearance", form.appearance);
  setOptionalText(implicit, "personality", form.personality);
  return {
    explicit_attributes: explicit,
    implicit_attributes: implicit,
    social_attributes: social,
  };
};

export const profileRelationshipBaseAttributes = (
  currentState: unknown,
  target: string,
  description: string | null,
) => {
  const explicit = attributeCategory(currentState, "explicit_attributes");
  const implicit = attributeCategory(currentState, "implicit_attributes");
  const social = attributeCategory(currentState, "social_attributes");
  const key = target.trim();
  if (key) {
    if (description?.trim()) social[key] = description.trim();
    else delete social[key];
  }
  return {
    explicit_attributes: explicit,
    implicit_attributes: implicit,
    social_attributes: social,
  };
};

export const profileFormFromProfile = (profile: CharacterProfile): ProfileForm => ({
  name: profile.name,
  gender: characterAttributeText(profile, "explicit", "gender") ?? "",
  age: characterAttributeText(profile, "explicit", "age") ?? "",
  personality: characterAttributeText(profile, "implicit", "personality") ?? "",
  background: characterAttributeText(profile, "explicit", "background") ?? "",
  appearance: characterAttributeText(profile, "explicit", "appearance") ?? "",
});

export const CharacterProfileEditorModal: React.FC<{
  open: boolean;
  form: ProfileForm;
  saving: boolean;
  mode?: "create" | "edit";
  onChange: (form: ProfileForm) => void;
  onSave: () => void | Promise<void>;
  onClose: () => void;
}> = ({ open, form, saving, mode = "create", onChange, onSave, onClose }) => {
  const { t } = useI18n();
  const baselineRef = useRef<ProfileForm>(form);
  useEffect(() => {
    if (open) baselineRef.current = form;
  }, [mode, open]);
  if (!open) return null;
  const editing = mode === "edit";
  const close = async () => {
    if (saving) return;
    if (JSON.stringify(form) !== JSON.stringify(baselineRef.current) && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("characterDetail.profilesTitle"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  return (
    <Modal
      isOpen
      onClose={() => void close()}
      title={editing ? t("characterDetail.editInitialProfileTitle") : t("characterDetail.newProfileTitle")}
      icon={editing ? <Pencil size={15} /> : <UserPlus size={15} />}
      subtitle={
        editing
          ? t("characterDetail.editProfileSubtitle")
          : t("characterDetail.createProfileSubtitle")
      }
      maxWidthClassName="max-w-lg"
      bodyClassName="px-4 py-4"
      closeOnBackdrop={!saving}
      closeOnEscape={!saving}
      showCloseButton={!saving}
      footer={
        <BookButton
          variant="primary"
          onClick={onSave}
          disabled={saving || !form.name.trim()}
        >
          {saving ? (
            <span className="inline-flex items-center gap-2">
              <LoadingSpinner size="sm" /> {t("characterDetail.savingProfile")}
            </span>
          ) : (
            editing ? t("characterDetail.saveChanges") : t("characterDetail.saveProfile")
          )}
        </BookButton>
      }
    >
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <label className="flex flex-col gap-1.5 sm:col-span-2">
          <span className="text-xs text-book-text-sub">{t("characterDetail.nameRequired")}</span>
          <input
            type="text"
            value={form.name}
            onChange={(event) => onChange({ ...form, name: event.target.value })}
            disabled={saving || editing}
            className="pyqt-input"
            placeholder={t("characterDetail.namePlaceholder")}
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">{t("characterDetail.labels.gender")}</span>
          <input
            type="text"
            value={form.gender}
            onChange={(event) => onChange({ ...form, gender: event.target.value })}
            disabled={saving}
            className="pyqt-input"
            placeholder={t("characterDetail.genderPlaceholder")}
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-xs text-book-text-sub">{t("characterDetail.labels.age")}</span>
          <input
            type="text"
            value={form.age}
            onChange={(event) => onChange({ ...form, age: event.target.value })}
            disabled={saving}
            className="pyqt-input"
            placeholder={t("characterDetail.agePlaceholder")}
          />
        </label>
        <ProfileTextarea
          label={t("characterDetail.labels.personality")}
          value={form.personality}
          disabled={saving}
          placeholder={t("characterDetail.personalityPlaceholder")}
          onChange={(value) => onChange({ ...form, personality: value })}
        />
        <ProfileTextarea
          label={t("characterDetail.labels.background")}
          value={form.background}
          disabled={saving}
          placeholder={t("characterDetail.backgroundPlaceholder")}
          onChange={(value) => onChange({ ...form, background: value })}
        />
        <ProfileTextarea
          label={t("characterDetail.labels.appearance")}
          value={form.appearance}
          disabled={saving}
          placeholder={t("characterDetail.appearancePlaceholder")}
          onChange={(value) => onChange({ ...form, appearance: value })}
        />
      </div>
    </Modal>
  );
};
