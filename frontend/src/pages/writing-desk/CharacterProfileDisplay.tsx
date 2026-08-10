import React from "react";
import { type CharacterProfile } from "../../api/character";
import {
  type CharacterAttributeCategory,
  characterAttributeText,
  characterAttributeValueText,
} from "../../utils/characterAttributes";
import { nonEmptyText } from "../../utils/text";
import { attributeKeyLabel } from "../character-detail-dialog/manageViewUtils";

export const describeProfile = (profile: CharacterProfile): string | null =>
  characterAttributeText(profile, "implicit", "personality") ??
  characterAttributeText(profile, "explicit", "background") ??
  characterAttributeText(profile, "explicit", "appearance");

const compactValue = (value: unknown): string | null => {
  if (value == null) return null;
  if (Array.isArray(value)) {
    const parts = value
      .slice(0, 4)
      .map((item) => compactValue(item))
      .filter((item): item is string => item != null);
    if (parts.length === 0) return null;
    return `${parts.join("、")}${value.length > 4 ? ` 等 ${value.length} 项` : ""}`;
  }
  if (typeof value === "object") return null;
  return nonEmptyText(characterAttributeValueText(value));
};

const clampText = (value: string, max = 72): string =>
  value.length > max ? `${value.slice(0, max)}...` : value;

export const ProfileField: React.FC<{ label: string; value: string | null }> = ({
  label,
  value,
}) => {
  const text = nonEmptyText(value);
  if (!text) return null;

  return (
    <div className="grid grid-cols-[48px_minmax(0,1fr)] gap-2 leading-relaxed">
      <dt className="text-book-text-muted">{label}</dt>
      <dd className="whitespace-pre-wrap text-book-text-sub">{text}</dd>
    </div>
  );
};

export const AttributePreview: React.FC<{
  label: string;
  category: CharacterAttributeCategory;
  entries: Array<[string, unknown]>;
  onOpenHistory: (category: CharacterAttributeCategory, key: string) => void;
}> = ({ label, category, entries, onOpenHistory }) => {
  const visibleEntries = entries
    .map(([key, value]) => {
      const text = compactValue(value);
      return text == null ? null : { key, text };
    })
    .filter((entry): entry is { key: string; text: string } => entry != null);

  if (visibleEntries.length === 0) return null;

  return (
    <section className="px-3 py-3 sm:px-4">
      <h3 className="mb-2 text-small font-semibold text-book-text-main">{label}</h3>
      <ul className="divide-y divide-book-border/35">
        {visibleEntries.slice(0, 6).map(({ key, text }) => (
          <li key={key}>
            <button
              type="button"
              onClick={() => onOpenHistory(category, key)}
              className="flex w-full flex-col gap-1 py-2 text-left transition-colors hover:text-book-primary sm:flex-row sm:items-start sm:justify-between sm:gap-4"
              title="查看该属性的事件溯源"
            >
              <span className="font-medium text-book-text-main">{attributeKeyLabel(key)}</span>
              <span className="line-clamp-2 min-w-0 leading-relaxed text-book-text-muted sm:max-w-[70%] sm:text-right">
                {clampText(text)}
              </span>
            </button>
          </li>
        ))}
      </ul>
      {visibleEntries.length > 6 && (
        <div className="mt-2 text-caption text-book-text-muted">
          另有 {visibleEntries.length - 6} 项，进入深度详情查看完整状态和溯源。
        </div>
      )}
    </section>
  );
};

export const ProfileTextarea: React.FC<{
  label: string;
  value: string;
  disabled: boolean;
  placeholder: string;
  onChange: (value: string) => void;
}> = ({ label, value, disabled, placeholder, onChange }) => (
  <label className="flex flex-col gap-1.5 sm:col-span-2">
    <span className="text-xs text-book-text-sub">{label}</span>
    <textarea
      value={value}
      onChange={(event) => onChange(event.target.value)}
      disabled={disabled}
      className="pyqt-input min-h-[60px] resize-none"
      placeholder={placeholder}
    />
  </label>
);
