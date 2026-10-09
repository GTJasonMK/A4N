import React from "react";
import type {
  MangaPageAdaptationV1,
  MangaPageContentV1,
  MangaPageImportanceV2,
  MangaPagePacingV2,
  MangaPageRoleV2,
  MangaPageV4,
  MangaReadingFlowV2,
} from "../../../api/manga";
import type { MangaPagePromptLabels } from "./mangaPagePromptModel";
import { BookTextarea } from "../../../components/ui/BookInput";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { useI18n, type MessageTranslator } from "../../../i18n";

export type MangaPageEditableArtifactStage =
  | "page_content"
  | "page_adaptation"
  | "storyboard";

export const validateMangaPageArtifactDraft = (
  stage: MangaPageEditableArtifactStage,
  value: unknown,
  pageNumber: number,
  t: MessageTranslator,
): string | null => {
  if (!isRecord(value)) return t("comic.prompt.artifactEditor.invalidPayload");
  if (stage === "page_content") {
    if (!isMangaPageContent(value)) return t("comic.prompt.artifactEditor.invalidPayload");
    return validatePageContent(value, t);
  }
  if (stage === "page_adaptation") {
    if (!isMangaPageAdaptation(value)) return t("comic.prompt.artifactEditor.invalidPayload");
    if (!isNonEmptyText(value.visual_intent)) {
      return t("comic.prompt.artifactEditor.requiredField", {
        field: t("comic.prompt.visualIntent"),
      });
    }
    return hasBeatReferences(value.beat_refs)
      ? null
      : t("comic.prompt.artifactEditor.beatReferencesRequired");
  }

  if (!isMangaPageStoryboard(value)) {
    return t("comic.prompt.artifactEditor.invalidPayload");
  }
  if (value.page_number !== pageNumber) {
    return t("comic.prompt.artifactEditor.storyboardPageNumber", { page: pageNumber });
  }
  if (!isNonEmptyText(value.page_goal)) {
    return t("comic.prompt.artifactEditor.requiredField", {
      field: t("comic.prompt.pageGoal"),
    });
  }
  if (!isNonEmptyText(value.visual_thesis)) {
    return t("comic.prompt.artifactEditor.requiredField", {
      field: t("comic.prompt.visualThesis"),
    });
  }
  if (!Array.isArray(value.panels) || value.panels.length === 0) {
    return t("comic.prompt.artifactEditor.panelsRequired");
  }
  const requiredPanelFields = [
    ["observable_action", t("comic.prompt.artifactEditor.observableAction")],
    ["composition", t("comic.prompt.artifactEditor.composition")],
    ["spatial_relationship", t("comic.prompt.artifactEditor.spatialRelationship")],
  ] as const;
  for (const panel of value.panels) {
    if (!isRecord(panel)) return t("comic.prompt.artifactEditor.invalidPayload");
    for (const [field, label] of requiredPanelFields) {
      if (!isNonEmptyText(panel[field])) {
        return t("comic.prompt.artifactEditor.requiredField", {
          field: label,
        });
      }
    }
    if (Array.isArray(panel.text_layers)) {
      for (const layer of panel.text_layers) {
        if (!isRecord(layer) || !isNonEmptyText(layer.text)) {
          return t("comic.prompt.artifactEditor.requiredField", {
            field: t("comic.prompt.artifactEditor.textContent"),
          });
        }
      }
    }
  }
  return null;
};

export const validateMangaPageSlotsDraft = (
  value: unknown,
  t: MessageTranslator,
): string | null => {
  if (!isMangaPageSlotsDraft(value)) {
    return t("comic.prompt.artifactEditor.invalidPayload");
  }
  if (value.pages.length === 0) {
    return t("comic.prompt.artifactEditor.pagesRequired");
  }
  let previousExitState: string | null = null;
  for (const [index, page] of value.pages.entries()) {
    if (!isRecord(page) || page.page_number !== index + 1 || !isRecord(page.content)) {
      return t("comic.prompt.artifactEditor.pageSequenceInvalid");
    }
    const contentError = validatePageContent(page.content, t);
    if (contentError) return contentError;
    if (
      previousExitState != null &&
      page.content.entry_state !== previousExitState
    ) {
      return t("comic.prompt.artifactEditor.pageStateContinuityInvalid");
    }
    previousExitState = page.content.exit_state as string;
  }
  return null;
};

const validatePageContent = (
  value: Record<string, unknown>,
  t: MessageTranslator,
): string | null => {
  for (const [field, label] of [
    ["summary", t("comic.prompt.contentSummary")],
    ["action", t("comic.prompt.pageAction")],
    ["entry_state", t("comic.prompt.entryState")],
    ["exit_state", t("comic.prompt.exitState")],
  ]) {
    if (!isNonEmptyText(value[field])) {
      return t("comic.prompt.artifactEditor.requiredField", { field: label });
    }
  }
  return hasBeatReferences(value.beat_refs)
    ? null
    : t("comic.prompt.artifactEditor.beatReferencesRequired");
};

const hasBeatReferences = (value: unknown): boolean =>
  Array.isArray(value) &&
  value.length > 0 &&
  value.every(
    (reference) =>
      isRecord(reference) &&
      isNonEmptyText(reference.unit_id) &&
      isNonEmptyText(reference.beat_id),
  );

const isNonEmptyText = (value: unknown): value is string =>
  typeof value === "string" && value.trim().length > 0;

type MangaPageSlotsDraft = {
  schema_version: string;
  pages: Array<{
    page_number: number;
    content: MangaPageContentV1;
  }>;
};

type MangaPageContentFieldsProps = {
  value: MangaPageContentV1;
  disabled: boolean;
  onChange: (value: MangaPageContentV1) => void;
};

const TextareaField: React.FC<{
  label: string;
  value: string;
  onChange: (value: string) => void;
  disabled: boolean;
  rows?: number;
}> = ({ label, value, onChange, disabled, rows = 3 }) => (
  <BookTextarea
    label={label}
    value={value}
    rows={rows}
    disabled={disabled}
    onChange={(event) => onChange(event.target.value)}
    className="text-xs leading-relaxed"
  />
);

const LineListField: React.FC<{
  label: string;
  value: string[];
  onChange: (value: string[]) => void;
  disabled: boolean;
}> = ({ label, value, onChange, disabled }) => {
  const { t } = useI18n();
  return (
    <div className="min-w-0">
      <TextareaField
        label={label}
        value={value.join("\n")}
        disabled={disabled}
        rows={4}
        onChange={(next) =>
          onChange(
            next
              .split(/\r?\n/)
              .map((item) => item.trim())
              .filter(Boolean),
          )
        }
      />
      <p className="mt-1 text-caption text-book-text-muted">
        {t("comic.prompt.artifactEditor.listInputHint")}
      </p>
    </div>
  );
};

const SelectField = <T extends string>({
  label,
  value,
  options,
  disabled,
  onChange,
}: {
  label: string;
  value: T;
  options: Array<{ value: T; label: string }>;
  disabled: boolean;
  onChange: (value: T) => void;
}) => (
  <div className="min-w-0">
    <label className="mb-1.5 block text-xs font-semibold text-book-text-sub">
      {label}
    </label>
    <ThemedSelect
      aria-label={label}
      value={value}
      disabled={disabled}
      onChange={(next) => onChange(next as T)}
      className="pyqt-input h-9 w-full min-w-0 px-2 text-xs"
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </ThemedSelect>
  </div>
);

export const MangaPageContentFields: React.FC<MangaPageContentFieldsProps> = ({
  value,
  disabled,
  onChange,
}) => {
  const { t } = useI18n();
  const update = <K extends keyof MangaPageContentV1>(
    key: K,
    next: MangaPageContentV1[K],
  ) => onChange({ ...value, [key]: next });
  const updateOptional = (key: "reaction" | "dialogue_intent", next: string) =>
    update(key, next.trim() ? next : null);

  return (
    <div className="grid min-w-0 gap-3 md:grid-cols-2">
      <TextareaField
        label={t("comic.prompt.contentSummary")}
        value={value.summary}
        disabled={disabled}
        onChange={(next) => update("summary", next)}
        rows={3}
      />
      <TextareaField
        label={t("comic.prompt.pageAction")}
        value={value.action}
        disabled={disabled}
        onChange={(next) => update("action", next)}
        rows={3}
      />
      <TextareaField
        label={t("comic.prompt.pageReaction")}
        value={value.reaction ?? ""}
        disabled={disabled}
        onChange={(next) => updateOptional("reaction", next)}
        rows={2}
      />
      <TextareaField
        label={t("comic.prompt.dialogueIntent")}
        value={value.dialogue_intent ?? ""}
        disabled={disabled}
        onChange={(next) => updateOptional("dialogue_intent", next)}
        rows={2}
      />
      <TextareaField
        label={t("comic.prompt.entryState")}
        value={value.entry_state}
        disabled={disabled}
        onChange={(next) => update("entry_state", next)}
        rows={2}
      />
      <TextareaField
        label={t("comic.prompt.exitState")}
        value={value.exit_state}
        disabled={disabled}
        onChange={(next) => update("exit_state", next)}
        rows={2}
      />
    </div>
  );
};

export const MangaPageSlotsArtifactEditor: React.FC<{
  value: unknown;
  disabled: boolean;
  onChange: (value: unknown) => void;
}> = ({ value, disabled, onChange }) => {
  const { t } = useI18n();
  if (!isMangaPageSlotsDraft(value)) {
    return <InvalidArtifactPayload />;
  }
  const slots = value as unknown as MangaPageSlotsDraft;

  const updatePageContent = (index: number, content: MangaPageContentV1) => {
    onChange({
      ...value,
      pages: slots.pages.map((page, pageIndex) =>
        pageIndex === index ? { ...page, content } : page,
      ),
    });
  };

  return (
    <div className="space-y-3">
      <p className="text-small text-book-text-muted">
        {t("comic.prompt.artifactEditor.pageSlotsHint")}
      </p>
      <div className="divide-y divide-book-border/40">
        {slots.pages.map((page, index) => (
          <details
            key={page.page_number}
            open={index === 0}
            className="min-w-0 py-3 first:pt-0 last:pb-0"
          >
            <summary className="cursor-pointer text-xs font-semibold text-book-text-main">
              {t("comic.prompt.pageHeading", { page: page.page_number })}
            </summary>
            <div className="pt-3">
              <MangaPageContentFields
                value={page.content}
                disabled={disabled}
                onChange={(content) => updatePageContent(index, content)}
              />
            </div>
          </details>
        ))}
      </div>
    </div>
  );
};

export const MangaPageStageArtifactEditor: React.FC<{
  stage: MangaPageEditableArtifactStage;
  value: unknown;
  labels: MangaPagePromptLabels;
  disabled: boolean;
  onChange: (value: unknown) => void;
}> = ({ stage, value, labels, disabled, onChange }) => {
  const { t } = useI18n();
  if (
    (stage === "page_content" && !isMangaPageContent(value)) ||
    (stage === "page_adaptation" && !isMangaPageAdaptation(value)) ||
    (stage === "storyboard" && !isMangaPageStoryboard(value))
  ) return <InvalidArtifactPayload />;

  switch (stage) {
    case "page_content":
      return (
        <div className="space-y-3">
          <p className="text-caption text-book-text-muted">
            {t("comic.prompt.artifactEditor.pageReferencesHint")}
          </p>
          <MangaPageContentFields
            value={value as unknown as MangaPageContentV1}
            disabled={disabled}
            onChange={onChange}
          />
        </div>
      );
    case "page_adaptation":
      return (
        <MangaPageAdaptationEditor
          value={value as unknown as MangaPageAdaptationV1}
          disabled={disabled}
          onChange={onChange}
        />
      );
    case "storyboard":
      return (
        <MangaStoryboardEditor
          value={value as unknown as MangaPageV4}
          labels={labels}
          disabled={disabled}
          onChange={onChange}
        />
      );
  }
};

const MangaPageAdaptationEditor: React.FC<{
  value: MangaPageAdaptationV1;
  disabled: boolean;
  onChange: (value: MangaPageAdaptationV1) => void;
}> = ({ value, disabled, onChange }) => {
  const { t } = useI18n();
  const update = <K extends keyof MangaPageAdaptationV1>(
    key: K,
    next: MangaPageAdaptationV1[K],
  ) => onChange({ ...value, [key]: next });

  return (
    <div className="space-y-3">
      <p className="text-caption text-book-text-muted">
        {t("comic.prompt.artifactEditor.pageReferencesHint")}
      </p>
      <TextareaField
        label={t("comic.prompt.visualIntent")}
        value={value.visual_intent}
        disabled={disabled}
        onChange={(next) => update("visual_intent", next)}
        rows={4}
      />
      <div className="grid min-w-0 gap-3 md:grid-cols-2">
        <LineListField
          label={t("comic.prompt.continuityRequirements")}
          value={value.continuity_requirements}
          disabled={disabled}
          onChange={(next) => update("continuity_requirements", next)}
        />
        <LineListField
          label={t("comic.prompt.retainedDetails")}
          value={value.retained_details}
          disabled={disabled}
          onChange={(next) => update("retained_details", next)}
        />
        <LineListField
          label={t("comic.prompt.omittedDetails")}
          value={value.omitted_details}
          disabled={disabled}
          onChange={(next) => update("omitted_details", next)}
        />
      </div>
    </div>
  );
};

const MangaStoryboardEditor: React.FC<{
  value: MangaPageV4;
  labels: MangaPagePromptLabels;
  disabled: boolean;
  onChange: (value: MangaPageV4) => void;
}> = ({ value, labels, disabled, onChange }) => {
  const { t } = useI18n();
  const updatePage = <K extends keyof MangaPageV4>(
    key: K,
    next: MangaPageV4[K],
  ) => onChange({ ...value, [key]: next });
  const updatePanel = (index: number, patch: Record<string, unknown>) =>
    onChange({
      ...value,
      panels: value.panels.map((panel, panelIndex) =>
        panelIndex === index ? { ...panel, ...patch } : panel,
      ),
    });

  return (
    <div className="space-y-4">
      <p className="text-caption text-book-text-muted">
        {t("comic.prompt.artifactEditor.storyboardHint")}
      </p>
      <div className="grid min-w-0 gap-3 md:grid-cols-2">
        <TextareaField
          label={t("comic.prompt.pageGoal")}
          value={value.page_goal}
          disabled={disabled}
          onChange={(next) => updatePage("page_goal", next)}
          rows={3}
        />
        <TextareaField
          label={t("comic.prompt.visualThesis")}
          value={value.visual_thesis}
          disabled={disabled}
          onChange={(next) => updatePage("visual_thesis", next)}
          rows={3}
        />
        <SelectField<MangaPageRoleV2>
          label={t("comic.prompt.pageFunction")}
          value={value.role}
          options={Object.entries(labels.role).map(([option, label]) => ({
            value: option as MangaPageRoleV2,
            label,
          }))}
          disabled={disabled}
          onChange={(next) => updatePage("role", next)}
        />
        <SelectField<MangaPagePacingV2>
          label={t("comic.prompt.pagePacing")}
          value={value.pacing}
          options={Object.entries(labels.pacing).map(([option, label]) => ({
            value: option as MangaPagePacingV2,
            label,
          }))}
          disabled={disabled}
          onChange={(next) => updatePage("pacing", next)}
        />
        <SelectField<MangaPageImportanceV2>
          label={t("comic.prompt.importance")}
          value={value.importance}
          options={Object.entries(labels.importance).map(([option, label]) => ({
            value: option as MangaPageImportanceV2,
            label,
          }))}
          disabled={disabled}
          onChange={(next) => updatePage("importance", next)}
        />
        <SelectField<MangaReadingFlowV2>
          label={t("comic.prompt.readingFlow")}
          value={value.reading_flow}
          options={Object.entries(labels.readingFlow).map(([option, label]) => ({
            value: option as MangaReadingFlowV2,
            label,
          }))}
          disabled={disabled}
          onChange={(next) => updatePage("reading_flow", next)}
        />
      </div>
      <div className="divide-y divide-book-border/40 border-y border-book-border/40">
        {value.panels.map((panel, index) => (
          <details
            key={panel.panel_number}
            open={index === 0}
            className="min-w-0 py-3 first:pt-0 last:pb-0"
          >
            <summary className="cursor-pointer text-xs font-semibold text-book-text-main">
              {t("comic.prompt.panelHeading", { panel: panel.panel_number })}
              {panel.participants.length > 0
                ? ` · ${panel.participants.join(t("common.listSeparator"))}`
                : ""}
            </summary>
            <div className="grid min-w-0 gap-3 pt-3 md:grid-cols-2">
              <TextareaField
                label={t("comic.prompt.pageAction")}
                value={panel.observable_action}
                disabled={disabled}
                onChange={(next) => updatePanel(index, { observable_action: next })}
              />
              <TextareaField
                label={t("comic.prompt.artifactEditor.composition")}
                value={panel.composition}
                disabled={disabled}
                onChange={(next) => updatePanel(index, { composition: next })}
              />
              <TextareaField
                label={t("comic.prompt.pageReaction")}
                value={panel.reaction ?? ""}
                disabled={disabled}
                onChange={(next) =>
                  updatePanel(index, { reaction: next.trim() ? next : null })
                }
                rows={2}
              />
              <TextareaField
                label={t("comic.prompt.artifactEditor.spatialRelationship")}
                value={panel.spatial_relationship}
                disabled={disabled}
                onChange={(next) => updatePanel(index, { spatial_relationship: next })}
              />
              <TextareaField
                label={t("comic.prompt.artifactEditor.continuityFromPrevious")}
                value={panel.continuity_from_previous ?? ""}
                disabled={disabled}
                onChange={(next) =>
                  updatePanel(index, {
                    continuity_from_previous: next.trim() ? next : null,
                  })
                }
                rows={2}
              />
              {panel.text_layers.map((layer, layerIndex) => (
                <div
                  key={`${panel.panel_number}:${layer.layer_index}`}
                  className="min-w-0 space-y-3 border-t border-book-border/35 pt-3 md:col-span-2"
                >
                  <h5 className="text-xs font-semibold text-book-text-sub">
                    {t("comic.prompt.artifactEditor.textLayer", {
                      index: layerIndex + 1,
                    })}
                  </h5>
                  <div className="grid min-w-0 gap-3 md:grid-cols-2">
                    <TextareaField
                      label={t("comic.prompt.artifactEditor.textContent")}
                      value={layer.text}
                      disabled={disabled}
                      onChange={(next) =>
                        updateTextLayer(index, layerIndex, { text: next })
                      }
                      rows={2}
                    />
                    {layer.kind === "dialogue" || layer.kind === "thought" ? (
                      <SelectField<string>
                        label={t("comic.prompt.artifactEditor.speaker")}
                        value={layer.speaker ?? ""}
                        options={Array.from(
                          new Set([
                            ...panel.participants,
                            ...(layer.speaker ? [layer.speaker] : []),
                          ]),
                        ).map((speaker) => ({ value: speaker, label: speaker }))}
                        disabled={disabled}
                        onChange={(speaker) =>
                          updateTextLayer(index, layerIndex, { speaker })
                        }
                      />
                    ) : null}
                    <LineListField
                      label={t("comic.prompt.artifactEditor.avoidCovering")}
                      value={layer.avoid_covering}
                      disabled={disabled}
                      onChange={(next) =>
                        updateTextLayer(index, layerIndex, { avoid_covering: next })
                      }
                    />
                  </div>
                </div>
              ))}
            </div>
          </details>
        ))}
      </div>
    </div>
  );

  function updateTextLayer(
    panelIndex: number,
    layerIndex: number,
    patch: Record<string, unknown>,
  ) {
    onChange({
      ...value,
      panels: value.panels.map((panel, currentPanelIndex) =>
        currentPanelIndex === panelIndex
          ? {
              ...panel,
              text_layers: panel.text_layers.map((layer, currentLayerIndex) =>
                currentLayerIndex === layerIndex ? { ...layer, ...patch } : layer,
              ),
            }
          : panel,
      ),
    });
  }
};

const InvalidArtifactPayload: React.FC = () => {
  const { t } = useI18n();
  return (
    <p role="alert" className="text-small text-error">
      {t("comic.prompt.artifactEditor.invalidPayload")}
    </p>
  );
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  value != null && typeof value === "object" && !Array.isArray(value);

const isStringList = (value: unknown): value is string[] =>
  Array.isArray(value) && value.every((item) => typeof item === "string");

const isBeatReferenceList = (value: unknown): boolean =>
  Array.isArray(value) &&
  value.every(
    (item) =>
      isRecord(item) &&
      typeof item.unit_id === "string" &&
      typeof item.beat_id === "string",
  );

const isMangaPageContent = (value: unknown): value is MangaPageContentV1 =>
  isRecord(value) &&
  typeof value.summary === "string" &&
  typeof value.action === "string" &&
  (value.reaction === null || typeof value.reaction === "string") &&
  (value.dialogue_intent === null || typeof value.dialogue_intent === "string") &&
  typeof value.entry_state === "string" &&
  typeof value.exit_state === "string" &&
  isStringList(value.source_span_ids) &&
  isBeatReferenceList(value.beat_refs);

const isMangaPageAdaptation = (value: unknown): value is MangaPageAdaptationV1 =>
  isRecord(value) &&
  typeof value.visual_intent === "string" &&
  isStringList(value.continuity_requirements) &&
  isStringList(value.retained_details) &&
  isStringList(value.omitted_details) &&
  isBeatReferenceList(value.beat_refs);

const isMangaPageStoryboard = (value: unknown): value is MangaPageV4 =>
  isRecord(value) &&
  typeof value.page_number === "number" &&
  typeof value.page_goal === "string" &&
  typeof value.visual_thesis === "string" &&
  typeof value.entry_state === "string" &&
  typeof value.exit_state === "string" &&
  typeof value.role === "string" &&
  typeof value.pacing === "string" &&
  typeof value.importance === "string" &&
  typeof value.reading_flow === "string" &&
  isBeatReferenceList(value.beat_refs) &&
  Array.isArray(value.panels) &&
  value.panels.every(
    (panel) =>
      isRecord(panel) &&
      typeof panel.panel_number === "number" &&
      typeof panel.composition === "string" &&
      typeof panel.observable_action === "string" &&
      (panel.reaction === null || typeof panel.reaction === "string") &&
      typeof panel.spatial_relationship === "string" &&
      (panel.continuity_from_previous === null ||
        typeof panel.continuity_from_previous === "string") &&
      isStringList(panel.participants) &&
      Array.isArray(panel.text_layers) &&
      panel.text_layers.every(
        (layer) =>
          isRecord(layer) &&
          typeof layer.layer_index === "number" &&
          typeof layer.kind === "string" &&
          typeof layer.text === "string" &&
          typeof layer.placement === "string" &&
          (layer.speaker === null || typeof layer.speaker === "string") &&
          isStringList(layer.avoid_covering),
      ),
  );

const isMangaPageSlotsDraft = (value: unknown): value is MangaPageSlotsDraft =>
  isRecord(value) &&
  typeof value.schema_version === "string" &&
  Array.isArray(value.pages) &&
  value.pages.every(
    (page) =>
      isRecord(page) &&
      typeof page.page_number === "number" &&
      isMangaPageContent(page.content),
  );
