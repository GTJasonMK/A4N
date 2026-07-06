import React from "react";
import { type CharacterProfile } from "../../api/character";
import {
  type CharacterAttributeCategory,
  characterAttributeValueText,
} from "../../utils/characterAttributes";
import { nonEmptyText } from "../../utils/text";

export const describeProfile = (profile: CharacterProfile): string | null =>
  nonEmptyText(profile.personality) ??
  nonEmptyText(profile.background) ??
  nonEmptyText(profile.appearance);

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
    <section className="a4n-page-surface p-3">
      <div className="mb-1 text-[10px] font-semibold uppercase tracking-wide text-book-text-muted">
        {label}
      </div>
      <div className="whitespace-pre-wrap leading-relaxed text-book-text-sub">{text}</div>
    </section>
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
    <section className="a4n-page-surface p-3 md:col-span-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="text-[10px] font-semibold uppercase tracking-wide text-book-text-muted">
          {label}
        </div>
        <span className="pyqt-pill !text-[10px]">{visibleEntries.length} 项</span>
      </div>
      <div className="grid gap-1.5 md:grid-cols-2">
        {visibleEntries.slice(0, 6).map(({ key, text }) => (
          <button
            type="button"
            key={key}
            onClick={() => onOpenHistory(category, key)}
            className="a4n-interactive-row px-2 py-1.5 text-left"
            title="查看该属性的事件溯源"
          >
            <div className="truncate font-medium text-book-text-main">{key}</div>
            <div className="mt-0.5 line-clamp-2 leading-relaxed text-book-text-muted">
              {clampText(text)}
            </div>
          </button>
        ))}
      </div>
      {visibleEntries.length > 6 && (
        <div className="mt-2 text-[10px] text-book-text-muted">
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
  <label className="col-span-2 flex flex-col gap-1.5">
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
