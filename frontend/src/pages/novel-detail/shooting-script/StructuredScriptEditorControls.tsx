import React from "react";
import { ChevronRight, Plus, Trash2 } from "lucide-react";
import type { ShootingPerformanceBeatV1 } from "../../../api/shootingScript";
import { useI18n } from "../../../i18n";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";

export type ValuePath = Array<string | number>;
export type UpdateValue = (path: ValuePath, value: unknown) => void;

export const BeatEditor: React.FC<{
  beat: ShootingPerformanceBeatV1;
  path: ValuePath;
  onChange: UpdateValue;
  onRemove: () => void;
}> = ({ beat, path, onChange, onRemove }) => {
  const { t } = useI18n();
  return (
  <details className="group">
    <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-3 py-2.5 [&::-webkit-details-marker]:hidden">
      <div className="min-w-0">
        <div className="text-small font-medium text-book-text-main">
            {t("shootingScript.editor.beat", { index: beat.beat_index })}
            {beat.actor ? " · " + beat.actor : ""}
        </div>
        <div className="mt-0.5 truncate text-caption text-book-text-muted">
          {beat.visible_action}
        </div>
      </div>
        <div className="flex items-center gap-1">
          <ChevronRight
            size={13}
            className="text-book-text-muted transition-transform group-open:rotate-90"
          />
          <BookIconButton
            label={t("shootingScript.editor.deleteBeat")}
            onClick={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onRemove();
            }}
            className="text-error hover:bg-error-bg"
          >
            <Trash2 size={13} />
          </BookIconButton>
        </div>
    </summary>
    <div className="grid gap-3 pb-3 pl-2 sm:grid-cols-2 sm:pl-3">
        <TextField
          label={t("shootingScript.editor.actor")}
          value={beat.actor}
          path={[...path, "actor"]}
          onChange={onChange}
          nullable
        />
        <TextField
          label={t("shootingScript.editor.visibleAction")}
          value={beat.visible_action}
          path={[...path, "visible_action"]}
          onChange={onChange}
        />
        <TextField
          label={t("shootingScript.editor.objective")}
          value={beat.objective}
          path={[...path, "objective"]}
          onChange={onChange}
        />
        <TextField
          label={t("shootingScript.editor.obstacle")}
          value={beat.obstacle}
          path={[...path, "obstacle"]}
          onChange={onChange}
        />
        <TextField
          label={t("shootingScript.editor.reaction")}
          value={beat.reaction}
          path={[...path, "reaction"]}
          onChange={onChange}
          nullable
        />
        <TextField
          label={t("shootingScript.editor.expression")}
          value={beat.expression}
          path={[...path, "expression"]}
          onChange={onChange}
          nullable
        />
        <TextField
          label={t("shootingScript.editor.gaze")}
          value={beat.gaze}
          path={[...path, "gaze"]}
          onChange={onChange}
          nullable
        />
        <TextField
          label={t("shootingScript.editor.poseMovement")}
          value={beat.pose_or_movement}
          path={[...path, "pose_or_movement"]}
          onChange={onChange}
          nullable
        />
        <TextField
          label={t("shootingScript.editor.dialogueIntent")}
          value={beat.dialogue_intent}
          path={[...path, "dialogue_intent"]}
          onChange={onChange}
          nullable
        />
        <TextField
          label={t("shootingScript.editor.audienceQuestion")}
          value={beat.audience_question}
          path={[...path, "audience_question"]}
          onChange={onChange}
        />
        <TextField
          label={t("shootingScript.editor.answerOrDelay")}
          value={beat.answer_or_delay}
          path={[...path, "answer_or_delay"]}
          onChange={onChange}
        />
        <StringListField
          label={t("shootingScript.editor.knowledgeDelta")}
          value={beat.knowledge_delta}
          path={[...path, "knowledge_delta"]}
          onChange={onChange}
        />
        <TextField
          label={t("shootingScript.editor.relationshipDelta")}
          value={beat.relationship_delta}
          path={[...path, "relationship_delta"]}
          onChange={onChange}
          nullable
        />
        <TextField
          label={t("shootingScript.editor.spatialChange")}
          value={beat.spatial_change}
          path={[...path, "spatial_change"]}
          onChange={onChange}
          nullable
        />
        <TextField
          label={t("shootingScript.editor.entryState")}
          value={beat.entry_state}
          path={[...path, "entry_state"]}
          onChange={onChange}
        />
        <TextField
          label={t("shootingScript.editor.exitState")}
          value={beat.exit_state}
          path={[...path, "exit_state"]}
          onChange={onChange}
        />
        <NumberField
          label={t("shootingScript.editor.estimatedSeconds")}
          value={beat.estimated_seconds}
          path={[...path, "estimated_seconds"]}
          onChange={onChange}
        />
        <SwitchField
          label={t("shootingScript.editor.required")}
          value={beat.required}
          path={[...path, "required"]}
          onChange={onChange}
        />
      <div className="sm:col-span-2">
        <DiagnosticDetails
          nested
          rows={[
            [t("shootingScript.editor.beatId"), beat.beat_id],
              [
                t("shootingScript.editor.sceneSourceLocation"),
                beat.source_span_ids.join("、"),
              ],
          ]}
        />
      </div>
    </div>
  </details>
  );
};

export const TextField: React.FC<{
  label: string;
  value: string | null;
  path: ValuePath;
  onChange: UpdateValue;
  nullable?: boolean;
  compact?: boolean;
}> = ({ label, value, path, onChange, nullable = false, compact = false }) => (
  <label className="block min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <textarea
      value={value ?? ""}
      onChange={(event) =>
        onChange(
          path,
          nullable && event.target.value === "" ? null : event.target.value,
        )
      }
      rows={compact ? 1 : (value?.length ?? 0) > 80 ? 3 : 2}
      className="pyqt-input w-full resize-y text-xs"
    />
  </label>
);

export const NumberField: React.FC<{
  label: string;
  value: number | null;
  path: ValuePath;
  onChange: UpdateValue;
  nullable?: boolean;
}> = ({ label, value, path, onChange, nullable = false }) => (
  <label className="block min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <input
      type="number"
      step="any"
      value={value ?? ""}
      onChange={(event) =>
        onChange(
          path,
          nullable && event.target.value === ""
            ? null
            : Number(event.target.value),
        )
      }
      className="pyqt-input w-full text-xs"
    />
  </label>
);

export const SelectField: React.FC<{
  label: string;
  value: string;
  options: ReadonlyArray<readonly [string, string]>;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ label, value, options, path, onChange }) => (
  <label className="block min-w-0">
    <span className="mb-1 block text-small text-book-text-muted">{label}</span>
    <ThemedSelect
      value={value}
      onChange={(event) => onChange(path, event.target.value)}
      className="pyqt-input w-full text-xs"
    >
      {options.map(([optionValue, optionLabel]) => (
        <option key={optionValue} value={optionValue}>
          {optionLabel}
        </option>
      ))}
    </ThemedSelect>
  </label>
);

export const StringListField: React.FC<{
  label: string;
  value: string[];
  path: ValuePath;
  onChange: UpdateValue;
  compact?: boolean;
}> = ({ label, value, path, onChange, compact = false }) => {
  const { t } = useI18n();

  return (
    <div className="block min-w-0">
      <span className="mb-1 block text-small text-book-text-muted">
        {label}
      </span>
      <div className="space-y-1.5">
        {value.map((item, index) => (
          <div key={index} className="flex min-w-0 items-start gap-1.5">
      <textarea
              value={item}
              onChange={(event) =>
                onChange([...path, index], event.target.value)
              }
              onBlur={() => {
                const trimmed = item.trim();
                if (!trimmed) {
                  onChange(
                    path,
                    value.filter((_, itemIndex) => itemIndex !== index),
                  );
                } else if (trimmed !== item) {
                  onChange([...path, index], trimmed);
                }
              }}
              rows={compact ? 1 : 2}
        placeholder={t("shootingScript.editor.listPlaceholder")}
              className="pyqt-input min-w-0 flex-1 resize-y text-xs"
      />
            <BookIconButton
              label={t("shootingScript.editor.deleteListItem")}
              onClick={() =>
                onChange(
                  path,
                  value.filter((_, itemIndex) => itemIndex !== index),
                )
              }
              className="shrink-0 text-error hover:bg-error-bg"
            >
              <Trash2 size={13} />
            </BookIconButton>
          </div>
        ))}
      </div>
      <BookButton
        size="sm"
        variant="ghost"
        onClick={() => onChange(path, [...value, ""])}
        className="mt-1"
      >
        <Plus size={12} />
        {t("shootingScript.editor.addListItem")}
      </BookButton>
    </div>
  );
};

export const SwitchField: React.FC<{
  label: string;
  value: boolean;
  path: ValuePath;
  onChange: UpdateValue;
}> = ({ label, value, path, onChange }) => {
  const { t } = useI18n();
  return (
    <div className="min-w-0">
      <span className="mb-1 block text-small text-book-text-muted">
        {label}
      </span>
    <button
      type="button"
      role="switch"
      aria-checked={value}
      onClick={() => onChange(path, !value)}
      className="inline-flex min-h-9 items-center gap-2 text-small text-book-text-sub"
    >
        <span>
          {value
            ? t("shootingScript.editor.yes")
            : t("shootingScript.editor.no")}
        </span>
        <span
          className={`relative inline-flex h-[18px] w-8 rounded-full transition-colors ${value ? "bg-book-primary" : "bg-book-text-muted/25"}`}
        >
          <span
            className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${value ? "translate-x-4" : "translate-x-[2px]"}`}
          />
      </span>
    </button>
    </div>
  );
};

export const AdvancedDetails: React.FC<
  React.PropsWithChildren<{
    title: string;
    action?: React.ReactNode;
  }>
> = ({ title, children, action }) => (
  <details className="group border-t border-book-border/40 pt-2">
    <summary className="flex min-h-9 w-fit cursor-pointer list-none items-center gap-1.5 rounded-lg px-2 text-xs text-book-text-muted transition-colors hover:bg-book-bg-tertiary/60 hover:text-book-text-main [&::-webkit-details-marker]:hidden">
      <ChevronRight
        size={13}
        className="transition-transform group-open:rotate-90"
      />
      {title}
    </summary>
    {action ? <div className="mt-1 flex justify-end">{action}</div> : null}
    <div className="pt-2">{children}</div>
  </details>
);

export const DiagnosticDetails: React.FC<{
  rows: Array<readonly [string, string | number]>;
  nested?: boolean;
}> = ({ rows, nested = false }) => {
  const { t } = useI18n();
  return (
    <details
    className={
      nested
        ? "text-caption text-book-text-muted"
        : "border-t border-book-border/40 px-3 py-2 text-caption text-book-text-muted sm:px-4"
    }
  >
    <summary className="w-fit cursor-pointer rounded px-1 py-0.5 transition-colors hover:bg-book-bg-tertiary hover:text-book-text-main">
      {t("shootingScript.editor.diagnostics")}
    </summary>
    <dl className="mt-2 grid grid-cols-[5rem_minmax(0,1fr)] gap-x-2 gap-y-1 font-mono">
      {rows.map(([label, value]) => (
        <React.Fragment key={label}>
          <dt>{label}</dt>
          <dd className="min-w-0 break-all">{value}</dd>
        </React.Fragment>
      ))}
    </dl>
    </details>
  );
};
