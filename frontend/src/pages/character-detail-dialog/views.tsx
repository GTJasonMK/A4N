import React from "react";
import { Check } from "lucide-react";
import type { CharacterConflictSummary } from "../../api/character";
import { formatNumber, useI18n, type Locale, type MessageKey, type Messages, type MessageTranslator } from "../../i18n";

export const TabBtn: React.FC<{
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  children: React.ReactNode;
}> = ({ active, onClick, icon, children }) => (
  <button
    type="button"
    onClick={onClick}
    data-active={active ? "1" : undefined}
    className="pyqt-tab inline-flex shrink-0 items-center gap-1 !text-small"
  >
    {icon}
    {children}
  </button>
);

export const ArcView: React.FC<{ data: unknown }> = ({ data }) => {
  const { locale, t } = useI18n();
  const a = data as
    | {
        arc_name?: string;
        current_stage?: string;
        progress?: number;
        stages?: string[];
      }
    | null;
  if (!a) return <div className="py-4 text-xs text-book-text-muted">{t("characterDetail.arcNoData")}</div>;
  const arcName = typeof a.arc_name === "string" ? a.arc_name.trim() : "";
  const currentStage = typeof a.current_stage === "string" ? a.current_stage.trim() : "";
  const stages = Array.isArray(a.stages) ? a.stages : [];
  return (
    <div className="a4n-flat-surface divide-y divide-book-border/45 text-xs">
      {(arcName || currentStage || typeof a.progress === "number") && (
        <section className="p-3">
          {arcName ? <div className="text-sm font-semibold text-book-text-main">{arcName}</div> : null}
          {currentStage ? (
            <div className={`${arcName ? "mt-1 " : ""}text-book-text-sub`}>{currentStage}</div>
          ) : null}
          {typeof a.progress === "number" && (
            <div className="mt-2">
              <div className="h-1.5 w-full overflow-hidden rounded-full bg-book-bg-tertiary">
                <div
                  className="h-full rounded-full bg-book-primary/75 transition-[width] duration-300"
                  style={{ width: `${Math.max(0, Math.min(100, a.progress * 100))}%` }}
                />
              </div>
              <div className="mt-1 text-caption text-book-text-muted">
                {t("characterDetail.progress", { value: formatNumber(a.progress * 100, locale, { maximumFractionDigits: 0 }) })}
              </div>
            </div>
          )}
        </section>
      )}
      {stages.length > 0 && (
        <section className="p-3">
          <div className="mb-2 text-caption font-semibold text-book-text-muted">
            {t("characterDetail.developmentStages")}
          </div>
          <ol className="list-inside list-decimal space-y-1 text-book-text-sub">
            {stages.map((stage, index) => (
              <li key={index}>{stage}</li>
            ))}
          </ol>
        </section>
      )}
    </div>
  );
};

type VoiceFieldKey = keyof Messages["characterDetail"]["voiceFields"];
const VOICE_FIELD_LABELS: Record<string, VoiceFieldKey> = {
  voice_style: "voiceStyle",
  style: "style",
  tone: "tone",
  speech_pattern: "speechPattern",
  vocabulary: "vocabulary",
  vocabulary_level: "vocabularyLevel",
  sentence_length: "sentenceLength",
  rhythm: "rhythm",
  formality: "formality",
  catchphrases: "catchphrases",
  favorite_phrases: "favoritePhrases",
  emotional_expression: "emotionalExpression",
  examples: "examples",
};

const voiceFieldLabel = (value: string, locale: Locale, t: MessageTranslator, index = 1): string => {
  const key = VOICE_FIELD_LABELS[value];
  return key
    ? t(`characterDetail.voiceFields.${key}` as MessageKey)
    : t("characterDetail.feature", { index: formatNumber(index, locale) });
};

const structuredValueText = (
  value: unknown,
  t: MessageTranslator,
  locale: Locale,
  depth = 0,
): string => {
  if (value == null) return t("characterDetail.notRecorded");
  if (typeof value === "string") return value.trim() || t("characterDetail.notRecorded");
  if (typeof value === "number") return formatNumber(value, locale);
  if (typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    const items = value
      .map((item) => structuredValueText(item, t, locale, depth + 1))
      .filter(Boolean);
    return items.length > 0 ? items.join(t("common.listSeparator")) : t("characterDetail.notRecorded");
  }
  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 0) return t("characterDetail.notRecorded");
    return entries
      .slice(0, depth > 0 ? 6 : 10)
      .map(([key, item], index) => {
        const text = structuredValueText(item, t, locale, depth + 1);
        return depth > 0 ? `${voiceFieldLabel(key, locale, t, index + 1)}: ${text}` : text;
      })
      .join(t("common.summarySeparator"));
  }
  return String(value);
};

export const VoiceView: React.FC<{ data: unknown }> = ({ data }) => {
  const { locale, t } = useI18n();
  if (data == null) {
    return <div className="py-4 text-xs text-book-text-muted">{t("characterDetail.voiceNoData")}</div>;
  }
  const entries =
    typeof data === "object" && !Array.isArray(data)
      ? Object.entries(data as Record<string, unknown>)
      : [["voice_style", data] as const];
  return (
    <div className="space-y-3">
      <div className="a4n-flat-surface divide-y divide-book-border/45">
        {entries.map(([key, value], index) => (
          <div key={key} className="grid gap-1 px-3 py-2.5 text-xs sm:grid-cols-[88px_minmax(0,1fr)] sm:gap-3">
            <div className="text-small font-medium text-book-text-muted">
              {voiceFieldLabel(key, locale, t, index + 1)}
            </div>
            <div className="min-w-0 whitespace-pre-wrap break-words leading-relaxed text-book-text-main">
              {Array.isArray(value) && value.length > 0 && ["catchphrases", "favorite_phrases", "examples"].includes(key) ? (
                <ul className="space-y-1">
                  {value.map((item, itemIndex) => <li key={itemIndex}>{structuredValueText(item, t, locale)}</li>)}
                </ul>
              ) : structuredValueText(value, t, locale)}
            </div>
          </div>
        ))}
      </div>
      <details className="text-small text-book-text-muted">
        <summary className="w-fit cursor-pointer rounded px-1 py-0.5 hover:bg-book-bg-tertiary hover:text-book-text-main">
          {t("characterDetail.diagnostics")}
        </summary>
        <pre className="mt-2 whitespace-pre-wrap break-words rounded-xl bg-book-bg-tertiary/55 p-3 font-mono leading-relaxed text-book-text-sub">
          {JSON.stringify(data, null, 2)}
        </pre>
      </details>
    </div>
  );
};

const CONFLICT_FIELD_LABELS: Record<string, MessageKey> = {
  appearance: "characterDetail.labels.appearance",
  background: "characterDetail.labels.background",
  personality: "characterDetail.labels.personality",
  age: "characterDetail.labels.age",
  gender: "characterDetail.labels.gender",
  current_state: "characterDetail.labels.state",
  character_arc: "characterDetail.labels.characterArc",
  voice_fingerprint: "characterDetail.labels.voiceFingerprint",
};

const conflictFieldLabel = (value: string, t: MessageTranslator): string => {
  const normalized = value.trim();
  if (!normalized) return t("characterDetail.profileField");
  if (/[^\x00-\x7F]/.test(normalized)) return normalized;
  const key = CONFLICT_FIELD_LABELS[normalized.toLowerCase()];
  return key ? t(key) : t("characterDetail.profileField");
};

export const ConflictView: React.FC<{
  conflicts: CharacterConflictSummary[];
  scopeLabel: string;
}> = ({ conflicts, scopeLabel }) => {
  const { locale, t } = useI18n();
  const scopeSummary = (
    <p className="mb-1 text-caption text-book-text-muted">
      {t("characterDetail.checkedScope")}<span className="font-medium text-book-text-sub">{scopeLabel}</span>
    </p>
  );
  if (conflicts.length === 0) {
    return (
      <>
        {scopeSummary}
        <div className="inline-flex items-center gap-2 py-4 text-xs text-success">
          <Check size={16} aria-hidden="true" /> {t("characterDetail.noConflicts")}
        </div>
      </>
    );
  }
  const conflictList = (
    <ul className="a4n-flat-surface divide-y divide-book-border/45 overflow-hidden">
      {conflicts.map((conflict, index) => {
        const severity = conflict.severity;
        const severityLabel =
          severity === "major"
            ? t("characterDetail.severityMajor")
            : severity === "moderate"
              ? t("characterDetail.severityModerate")
              : t("characterDetail.severityMinor");
        return (
          <li
            key={`${conflict.chapter_number}:${conflict.field}:${index}`}
            className={`px-3 py-2.5 text-xs ${
              severity === "major"
                ? "bg-error-bg/35"
                : severity === "moderate"
                  ? "bg-warning-bg/35"
                  : ""
            }`}
          >
            <div className="mb-1 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-caption text-book-text-muted">
              <span>{t("characterDetail.chapter", { chapter: formatNumber(conflict.chapter_number, locale) })}</span>
              <span aria-hidden="true">·</span>
              <span>{conflictFieldLabel(conflict.field, t)}</span>
              <span aria-hidden="true">·</span>
              <span className={severity === "major" ? "text-error" : severity === "moderate" ? "text-warning" : undefined}>
                {severityLabel}
              </span>
            </div>
            <div className="text-book-text-main">
              <div className="text-book-text-muted">{t("characterDetail.expected")}</div>
              <span className="text-small leading-relaxed">{structuredValueText(conflict.expected, t, locale)}</span>
            </div>
            <div className="mt-1 text-book-text-main">
              <div className="text-book-text-muted">{t("characterDetail.actual")}</div>
              <span className="text-small leading-relaxed">{structuredValueText(conflict.actual, t, locale)}</span>
            </div>
          </li>
        );
      })}
    </ul>
  );
  return (
    <>
      {scopeSummary}
      {conflictList}
    </>
  );
};
