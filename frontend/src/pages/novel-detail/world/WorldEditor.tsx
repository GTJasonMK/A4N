import React, { useMemo } from "react";
import { BookTextarea } from "../../../components/ui/BookInput";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import {
  CORE_KEYS,
  FACTION_KEYS,
  LOCATION_KEYS,
  inputTextValue,
  namedListToEditText,
  pickFirst,
  textToNamedWorldList,
  tryParseObjectDraft,
  valueToText,
  type WorldEditMode,
  type WorldObject,
} from "./worldTabUtils";
import { useI18n } from "../../../i18n";

interface WorldEditorProps {
  editMode: WorldEditMode;
  jsonDraft: string;
  textDraft: string;
  saving: boolean;
  onEditModeChange: (mode: WorldEditMode) => void;
  onJsonDraftChange: (draft: string) => void;
  onTextDraftChange: (draft: string) => void;
}

export const WorldEditor: React.FC<WorldEditorProps> = ({
  editMode,
  jsonDraft,
  textDraft,
  saving,
  onEditModeChange,
  onJsonDraftChange,
  onTextDraftChange,
}) => {
  const { t } = useI18n();
  const draftObject = useMemo(() => tryParseObjectDraft(jsonDraft), [jsonDraft]);
  const coreDraft = draftObject ? pickFirst(draftObject, CORE_KEYS) : null;
  const locationsDraft = draftObject ? pickFirst(draftObject, LOCATION_KEYS) : null;
  const factionsDraft = draftObject ? pickFirst(draftObject, FACTION_KEYS) : null;
  const canStructured = Boolean(draftObject);
  const activeMode = editMode === "structured" && !canStructured ? "json" : editMode;
  const modeDescription = activeMode === "structured"
    ? t("novel.world.structuredModeDescription")
    : activeMode === "json"
      ? t("novel.world.jsonModeDescription")
      : t("novel.world.textModeDescription");

  const updateObjectDraft = (patch: (obj: WorldObject) => void) => {
    const base = tryParseObjectDraft(jsonDraft);
    if (!base) return;
    const next = { ...base };
    patch(next);
    onJsonDraftChange(JSON.stringify(next, null, 2));
  };

  const handleModeChange = (mode: WorldEditMode) => {
    if (mode === "text" && !textDraft.trim() && draftObject) {
      onTextDraftChange(valueToText(draftObject, t));
    }
    onEditModeChange(mode);
  };

  return (
    <div className="space-y-4">
      <div className="space-y-2 border-b border-book-border/45 pb-3">
        <SegmentedControl
          ariaLabel={t("novel.world.modeAria")}
          value={activeMode}
          onChange={handleModeChange}
          className="w-full [&_.a4n-seg__btn]:flex-1 sm:w-auto sm:[&_.a4n-seg__btn]:flex-none"
          options={[
            {
              id: "structured",
              label: t("novel.world.structured"),
              disabled: !canStructured,
            },
            { id: "json", label: t("novel.world.json"), title: t("novel.world.jsonTitle") },
            { id: "text", label: t("novel.world.text") },
          ]}
        />
        <p className="text-small leading-relaxed text-book-text-muted">
          {modeDescription}
        </p>
      </div>

      {activeMode === "structured" && draftObject ? (
        <StructuredWorldEditor
          coreDraft={coreDraft}
          locationsDraft={locationsDraft}
          factionsDraft={factionsDraft}
          saving={saving}
          onObjectDraftChange={updateObjectDraft}
        />
      ) : activeMode === "text" ? (
        <BookTextarea
          value={textDraft}
          onChange={(event) => onTextDraftChange(event.target.value)}
          disabled={saving}
          className="min-h-[320px] text-sm font-serif leading-relaxed sm:min-h-[400px]"
          placeholder={t("novel.world.textPlaceholder")}
        />
      ) : (
        <BookTextarea
          value={jsonDraft}
          onChange={(event) => onJsonDraftChange(event.target.value)}
          disabled={saving}
          spellCheck={false}
          error={jsonDraft.trim() && !draftObject ? t("novel.world.jsonObjectError") : undefined}
          className="min-h-[360px] font-mono text-xs leading-relaxed sm:min-h-[460px]"
          placeholder='{"core_rules":"","key_locations":[],"factions":[]}'
        />
      )}
    </div>
  );
};

interface StructuredWorldEditorProps {
  coreDraft: [string, unknown] | null;
  locationsDraft: [string, unknown] | null;
  factionsDraft: [string, unknown] | null;
  saving: boolean;
  onObjectDraftChange: (patch: (obj: WorldObject) => void) => void;
}

const StructuredWorldEditor: React.FC<StructuredWorldEditorProps> = ({
  coreDraft,
  locationsDraft,
  factionsDraft,
  saving,
  onObjectDraftChange,
}) => {
  const { t } = useI18n();
  return (
    <>
      <div className="border-b border-book-border/45 pb-4">
        <BookTextarea
          label={t("novel.world.coreRules")}
          value={inputTextValue(coreDraft?.[1])}
          onChange={(event) =>
            onObjectDraftChange((obj) => {
              obj[coreDraft?.[0] ?? "core_rules"] = event.target.value;
            })
          }
          disabled={saving}
          className="min-h-[140px] text-sm font-serif leading-relaxed"
          placeholder={t("novel.world.coreRulesPlaceholder")}
        />
      </div>

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <div className="border-b border-book-border/45 pb-4 lg:border-b-0 lg:border-r lg:pb-0 lg:pr-3">
          <BookTextarea
            label={t("novel.world.keyLocations")}
            value={namedListToEditText(locationsDraft?.[1])}
            onChange={(event) =>
              onObjectDraftChange((obj) => {
                obj[locationsDraft?.[0] ?? "key_locations"] = textToNamedWorldList(
                  event.target.value,
                  locationsDraft?.[1],
                );
              })
            }
            disabled={saving}
            className="min-h-[220px] text-xs font-mono leading-relaxed"
            placeholder={t("novel.world.keyLocationsPlaceholder")}
          />
          <div className="mt-1.5 text-small text-book-text-muted">
            {t("novel.world.listHint")}
          </div>
        </div>

        <div className="lg:pl-3">
          <BookTextarea
            label={t("novel.world.factions")}
            value={namedListToEditText(factionsDraft?.[1])}
            onChange={(event) =>
              onObjectDraftChange((obj) => {
                obj[factionsDraft?.[0] ?? "factions"] = textToNamedWorldList(
                  event.target.value,
                  factionsDraft?.[1],
                );
              })
            }
            disabled={saving}
            className="min-h-[220px] text-xs font-mono leading-relaxed"
            placeholder={t("novel.world.factionsPlaceholder")}
          />
          <div className="mt-1.5 text-small text-book-text-muted">
            {t("novel.world.advancedHint")}
          </div>
        </div>
      </div>
    </>
  );
};
