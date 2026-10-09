import type { FC, ReactNode } from "react";
import { useEffect, useState } from "react";
import { BookOpen, Clock3, MapPin, MessageSquareText, Package, Plus, Sparkles, Trash2, UserRound, type LucideIcon } from "lucide-react";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { BookInput, BookTextarea } from "../../../components/ui/BookInput";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { useI18n } from "../../../i18n";

type JsonObject = Record<string, unknown>;
type PathPart = string | number;

const asObject = (value: unknown): JsonObject =>
  value != null && typeof value === "object" && !Array.isArray(value)
    ? value as JsonObject
    : {};

const asArray = (value: unknown): unknown[] => Array.isArray(value) ? value : [];

const updateAtPath = (value: unknown, path: PathPart[], next: unknown): unknown => {
  if (path.length === 0) return next;
  const [part, ...rest] = path;
  if (Array.isArray(value) && typeof part === "number") {
    const copy = [...value];
    copy[part] = updateAtPath(copy[part], rest, next);
    return copy;
  }
  const object = asObject(value);
  return { ...object, [String(part)]: updateAtPath(object[String(part)], rest, next) };
};

const EditorSection: FC<{
  title: string;
  icon: LucideIcon;
  children: ReactNode;
}> = ({ title, icon: Icon, children }) => (
  <section className="min-w-0 border-b border-book-border/40 py-4 first:pt-0 last:border-b-0 last:pb-0">
    <header className="mb-3 flex items-center gap-2">
      <Icon size={15} className="shrink-0 text-book-accent" aria-hidden="true" />
      <h3 className="text-xs font-semibold text-book-text-main">{title}</h3>
    </header>
    {children}
  </section>
);

const CharacterNameInput: FC<{
  value: string;
  disabled: boolean;
  onCommit: (value: string) => boolean;
}> = ({ value, disabled, onCommit }) => {
  const { t } = useI18n();
  const [draft, setDraft] = useState(value);

  useEffect(() => setDraft(value), [value]);

  const commit = () => {
    if (!onCommit(draft)) setDraft(value);
  };

  return (
    <BookInput
      label={t("comic.preprocessingView.field.name")}
      value={draft}
      disabled={disabled}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          event.currentTarget.blur();
        }
      }}
    />
  );
};

export const MangaPreprocessingEditor: FC<{
  value: unknown;
  disabled?: boolean;
  onChange: (value: unknown) => void;
}> = ({ value, disabled = false, onChange }) => {
  const { t } = useI18n();
  const data = asObject(value);
  const events = asArray(asObject(data.source_events).events).map(asObject);
  const eventIds = events.flatMap((event) =>
    typeof event.event_id === "string" && event.event_id.trim() ? [event.event_id] : [],
  );
  const speech = asObject(data.speech);
  const scenes = asArray(asObject(data.visual_anchors).scenes).map(asObject);
  const items = asArray(asObject(data.items).items).map(asObject);
  const characters = asObject(data.characters);

  const fieldLabels: Record<string, string> = {
    age_description: t("comic.preprocessingView.field.age"),
    appearance: t("comic.preprocessingView.field.appearance"),
    atmosphere: t("comic.preprocessingView.field.atmosphere"),
    bubble_type: t("comic.preprocessingView.field.bubbleType"),
    chapter_summary: t("comic.preprocessingView.field.chapterSummary"),
    content: t("comic.preprocessingView.field.content"),
    description: t("comic.preprocessingView.field.description"),
    emotion: t("comic.preprocessingView.field.emotion"),
    event_id: t("comic.preprocessingView.field.eventId"),
    events: t("comic.preprocessingView.events"),
    first_appearance_source_event_id: t("comic.preprocessingView.field.firstAppearance"),
    gender: t("comic.preprocessingView.field.gender"),
    importance: t("comic.preprocessingView.field.importance"),
    indoor_outdoor: t("comic.preprocessingView.field.indoorOutdoor"),
    is_internal: t("comic.preprocessingView.field.isInternal"),
    lighting: t("comic.preprocessingView.field.lighting"),
    location: t("comic.preprocessingView.field.location"),
    location_en: t("comic.preprocessingView.field.locationEnglish"),
    mood_progression: t("comic.preprocessingView.moodProgression"),
    name: t("comic.preprocessingView.field.name"),
    narration_type: t("comic.preprocessingView.field.narrationType"),
    narrations: t("comic.preprocessingView.narration"),
    participants: t("comic.preprocessingView.field.participants"),
    personality: t("comic.preprocessingView.field.personality"),
    position: t("comic.preprocessingView.field.position"),
    role: t("comic.preprocessingView.field.role"),
    scenes: t("comic.preprocessingView.scenes"),
    schema_version: t("comic.preprocessingView.field.schemaVersion"),
    source_event_id: t("comic.preprocessingView.field.sourceEvent"),
    source_event_ids: t("comic.preprocessingView.field.sourceEvents"),
    source_span_ids: t("comic.preprocessingView.field.sourceSpans"),
    speaker: t("comic.preprocessingView.field.speaker"),
    target: t("comic.preprocessingView.field.target"),
    time_of_day: t("comic.preprocessingView.field.timeOfDay"),
    type: t("comic.preprocessingView.field.type"),
    visual_features: t("comic.preprocessingView.field.visualFeatures"),
    weather: t("comic.preprocessingView.field.weather"),
  };
  const valueLabels: Record<string, Record<string, string>> = {
    bubble_type: {
      normal: t("comic.preprocessingView.value.normalBubble"),
      thought: t("comic.preprocessingView.value.thoughtBubble"),
    },
    gender: {
      female: t("comic.preprocessingView.value.female"),
      male: t("comic.preprocessingView.value.male"),
      unknown: t("comic.preprocessingView.value.unknown"),
    },
    importance: {
      critical: t("comic.preprocessingView.value.critical"),
      important: t("comic.preprocessingView.value.important"),
      minor: t("comic.preprocessingView.value.minor"),
      prop: t("comic.preprocessingView.value.prop"),
      supporting: t("comic.preprocessingView.value.supporting"),
    },
    indoor_outdoor: {
      indoor: t("comic.preprocessingView.value.indoor"),
      outdoor: t("comic.preprocessingView.value.outdoor"),
    },
    narration_type: {
      cognition: t("comic.preprocessingView.value.cognition"),
      time: t("comic.preprocessingView.value.time"),
    },
    position: {
      top: t("comic.preprocessingView.value.top"),
      bottom: t("comic.preprocessingView.value.bottom"),
      left: t("comic.preprocessingView.value.left"),
      right: t("comic.preprocessingView.value.right"),
    },
    role: {
      antagonist: t("comic.preprocessingView.value.antagonist"),
      background: t("comic.preprocessingView.value.background"),
      minor: t("comic.preprocessingView.value.minor"),
      protagonist: t("comic.preprocessingView.value.protagonist"),
      supporting: t("comic.preprocessingView.value.supporting"),
    },
    time_of_day: {
      dawn: t("comic.preprocessingView.value.dawn"),
      morning: t("comic.preprocessingView.value.morning"),
      day: t("comic.preprocessingView.value.day"),
      dusk: t("comic.preprocessingView.value.dusk"),
      evening: t("comic.preprocessingView.value.evening"),
      night: t("comic.preprocessingView.value.night"),
    },
    type: {
      action: t("comic.preprocessingView.value.action"),
      dialogue: t("comic.preprocessingView.dialogue"),
      reaction: t("comic.preprocessingView.value.reaction"),
      revelation: t("comic.preprocessingView.value.revelation"),
      transition: t("comic.preprocessingView.value.transition"),
    },
  };
  const enumOptions: Record<string, string[]> = {
    bubble_type: ["normal", "thought"],
    gender: ["female", "male", "unknown"],
    importance: ["critical", "important", "supporting", "minor", "prop"],
    indoor_outdoor: ["indoor", "outdoor"],
    narration_type: ["time", "cognition"],
    position: ["top", "bottom", "left", "right"],
    role: ["protagonist", "antagonist", "supporting", "minor", "background"],
    time_of_day: ["dawn", "morning", "day", "evening", "night", "dusk"],
    type: ["action", "dialogue", "reaction", "revelation", "transition"],
  };
  const longTextFields = new Set([
    "appearance", "atmosphere", "chapter_summary", "content", "description",
    "lighting", "personality", "visual_features",
  ]);
  const rootKeys = [
    "schema_version", "comic_synthesis", "characters", "source_events",
    "speech", "visual_anchors", "items",
  ];

  const fieldLabel = (key: string) =>
    Object.prototype.hasOwnProperty.call(fieldLabels, key)
      ? fieldLabels[key]
      : key.replace(/_/g, " ");
  const setAtPath = (path: PathPart[], next: unknown) =>
    onChange(updateAtPath(value, path, next));

  const isEventReferenced = (eventId: string) => {
    const dialogueReferences = asArray(speech.dialogues).some(
      (entry) => asObject(entry).source_event_id === eventId,
    );
    const narrationReferences = asArray(speech.narrations).some(
      (entry) => asObject(entry).source_event_id === eventId,
    );
    const sceneReferences = scenes.some((scene) =>
      asArray(scene.source_event_ids).includes(eventId),
    );
    const itemReferences = items.some(
      (item) => item.first_appearance_source_event_id === eventId,
    );
    return dialogueReferences || narrationReferences || sceneReferences || itemReferences;
  };

  const eventSpanDefault = events
    .flatMap((event) => asArray(event.source_span_ids))
    .find((span) => typeof span === "string" && span.trim());
  const firstEventId = eventIds[0] ?? "";

  const newArrayEntry = (field: string, current: unknown[]): unknown => {
    switch (field) {
      case "events": {
        let sequence = 1;
        while (eventIds.includes(`event-${sequence}`)) sequence += 1;
        return {
          event_id: `event-${sequence}`,
          type: "action",
          description: "",
          participants: [],
          importance: "important",
          source_span_ids: eventSpanDefault ? [eventSpanDefault] : [],
        };
      }
      case "dialogues":
        return { index: current.length, speaker: "", content: "", source_event_id: firstEventId, emotion: "", target: null, is_internal: false, bubble_type: "normal" };
      case "narrations":
        return { index: current.length, content: "", source_event_id: firstEventId, narration_type: "time", position: "top" };
      case "scenes":
        return { index: current.length, location: "", location_en: "", time_of_day: "night", atmosphere: "", weather: null, lighting: "", indoor_outdoor: "indoor", description: "", source_event_ids: firstEventId ? [firstEventId] : [], source_span_ids: eventSpanDefault ? [eventSpanDefault] : [] };
      case "items":
        return { name: "", description: "", description_en: "", importance: "prop", first_appearance_source_event_id: firstEventId, visual_features: "", source_span_ids: eventSpanDefault ? [eventSpanDefault] : [] };
      case "mood_progression":
      case "participants":
      case "source_span_ids":
      case "source_event_ids":
        return "";
      default:
        return current.some((item) => item != null && typeof item === "object") ? {} : "";
    }
  };

  const addLabel = (field: string) => {
    const labels: Record<string, string> = {
      dialogues: t("comic.preprocessingEditor.addDialogue"),
      events: t("comic.preprocessingEditor.addEvent"),
      items: t("comic.preprocessingEditor.addObject"),
      mood_progression: t("comic.preprocessingEditor.addMoodPhase"),
      narrations: t("comic.preprocessingEditor.addNarration"),
      participants: t("comic.preprocessingEditor.addParticipant"),
      scenes: t("comic.preprocessingEditor.addScene"),
      source_event_ids: t("comic.preprocessingEditor.addEventReference"),
      source_span_ids: t("comic.preprocessingEditor.addSourceSpan"),
    };
    return labels[field] ?? t("comic.preprocessingEditor.addValue");
  };

  const itemLabel = (field: string, item: unknown, index: number) => {
    if (item != null && typeof item !== "object") return String(item || `${index + 1}`);
    const object = asObject(item);
    const title = object.event_id ?? object.name ?? object.location ?? object.speaker;
    return title != null ? String(title) : `${fieldLabel(field)} ${index + 1}`;
  };

  const canRemoveEntry = (field: string, path: PathPart[], list: unknown[], item: unknown) => {
    if (field === "events" && path[0] === "source_events") {
      const eventId = asObject(item).event_id;
      if (typeof eventId === "string" && isEventReferenced(eventId)) return "referenced" as const;
      if (list.length <= 1) return "required" as const;
    }
    if (field === "mood_progression" && list.length <= 3) return "required" as const;
    if (field === "source_event_ids" && path[0] === "visual_anchors" && list.length <= 1) return "required" as const;
    if (field === "source_span_ids" && path[0] === "source_events" && list.length <= 1) return "required" as const;
    return null;
  };

  const renderObjectFields: (object: JsonObject, path: PathPart[], excluded?: string[]) => ReactNode =
    (object, path, excluded = []) => (
      <dl className="min-w-0 space-y-3">
        {Object.entries(object).filter(([key]) => !excluded.includes(key)).map(([key, fieldValue]) => (
          <div key={key} className="grid min-w-0 gap-1 sm:grid-cols-[minmax(9rem,0.3fr)_minmax(0,1fr)] sm:gap-x-4">
            <dt className="text-small text-book-text-muted">{fieldLabel(key)}</dt>
            <dd className="min-w-0">{renderField(key, fieldValue, [...path, key])}</dd>
          </div>
        ))}
      </dl>
    );

  const renderScalar = (field: string, fieldValue: unknown, path: PathPart[]): ReactNode => {
    const label = fieldLabel(field);
    if (["event_id", "index", "schema_version"].includes(field)) {
      return <span className="break-words font-mono text-body-sm text-book-text-muted">{String(fieldValue ?? "")}</span>;
    }
    if (typeof fieldValue === "boolean") {
      return (
        <label className="inline-flex min-h-10 items-center gap-2 text-body-sm text-book-text-main">
          <input
            type="checkbox"
            checked={fieldValue}
            disabled={disabled}
            aria-label={label}
            onChange={(event) => setAtPath(path, event.target.checked)}
            className="size-4 accent-book-accent"
          />
          {fieldValue ? t("comic.preprocessingView.yes") : t("comic.preprocessingView.no")}
        </label>
      );
    }
    if (typeof fieldValue === "number") {
      return (
        <BookInput
          aria-label={label}
          type="number"
          step="1"
          value={fieldValue}
          disabled={disabled}
          onChange={(event) => setAtPath(path, Number(event.target.value))}
        />
      );
    }

    const current = typeof fieldValue === "string" ? fieldValue : "";
    const eventReference = ["source_event_id", "first_appearance_source_event_id", "source_event_ids"].includes(field);
    const choices = eventReference ? eventIds : enumOptions[field];
    if (choices) {
      const options = current && !choices.includes(current) ? [current, ...choices] : choices;
      const selectedValues = current ? options : ["", ...options];
      return (
        <ThemedSelect
          aria-label={label}
          className="w-full"
          value={current}
          disabled={disabled}
          onChange={(next) => setAtPath(path, next)}
        >
          {selectedValues.map((option) => (
            <option key={option || "empty"} value={option}>
              {option
                ? Object.prototype.hasOwnProperty.call(valueLabels[field] ?? {}, option)
                  ? valueLabels[field][option]
                  : option
                : t("comic.preprocessingView.emptyValue")}
            </option>
          ))}
        </ThemedSelect>
      );
    }

    const onChange = (next: string) => setAtPath(path, next);
    if (longTextFields.has(field)) {
      return (
        <BookTextarea
          aria-label={label}
          value={current}
          rows={field === "chapter_summary" || field === "description" ? 3 : 2}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
          className="text-body-sm leading-relaxed"
        />
      );
    }
    return (
      <BookInput
        aria-label={label}
        value={current}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
      />
    );
  };

  const renderArray = (field: string, list: unknown[], path: PathPart[]): ReactNode => {
    return (
      <div className="min-w-0 space-y-2">
        {list.length === 0 ? (
          <p className="text-small text-book-text-muted">{t("comic.preprocessingEditor.emptyList")}</p>
        ) : null}
        {list.map((item, index) => {
          const pathToItem = [...path, index];
          const label = itemLabel(field, item, index);
          const removeBlocked = canRemoveEntry(field, path, list, item);
          const removeTitle = removeBlocked === "referenced"
            ? t("comic.preprocessingEditor.eventInUse")
            : removeBlocked === "required"
              ? t("comic.preprocessingEditor.keepRequired")
              : t("comic.preprocessingEditor.removeEntry", { item: label });
          return item != null && typeof item === "object" && !Array.isArray(item) ? (
            <article key={`${field}-${label}-${index}`} className="min-w-0 rounded-md border border-book-border/45 px-3 py-2.5">
              <header className="mb-2 flex min-w-0 items-center justify-between gap-2">
                <h4 className="min-w-0 break-words text-small font-semibold text-book-text-main">{label}</h4>
                <BookIconButton
                  label={removeTitle}
                  title={removeTitle}
                  disabled={disabled || removeBlocked != null}
                  onClick={() => setAtPath(path, list.filter((_, itemIndex) => itemIndex !== index))}
                  className="shrink-0"
                >
                  <Trash2 size={13} />
                </BookIconButton>
              </header>
              {renderObjectFields(asObject(item), pathToItem)}
            </article>
          ) : (
            <div key={`${field}-${index}`} className="flex min-w-0 items-center gap-2">
              <div className="min-w-0 flex-1">{renderScalar(field, item, pathToItem)}</div>
              <BookIconButton
                label={t("comic.preprocessingEditor.removeEntry", { item: label })}
                title={removeTitle}
                disabled={disabled || removeBlocked != null}
                onClick={() => setAtPath(path, list.filter((_, itemIndex) => itemIndex !== index))}
                className="shrink-0"
              >
                <Trash2 size={13} />
              </BookIconButton>
            </div>
          );
        })}
        <BookButton
          size="sm"
          variant="secondary"
          disabled={disabled}
          onClick={() => setAtPath(path, [...list, newArrayEntry(field, list)])}
        >
          <Plus size={13} /> {addLabel(field)}
        </BookButton>
      </div>
    );
  };

  const renderField = (field: string, fieldValue: unknown, path: PathPart[]): ReactNode => {
    if (Array.isArray(fieldValue)) return renderArray(field, fieldValue, path);
    if (fieldValue != null && typeof fieldValue === "object") {
      return (
        <div className="min-w-0 border-l border-book-border/45 pl-3">
          {renderObjectFields(asObject(fieldValue), path)}
        </div>
      );
    }
    return renderScalar(field, fieldValue, path);
  };

  const commitCharacterName = (oldName: string, requestedName: string) => {
    const nextName = requestedName.trim();
    const currentCharacters = asObject(asObject(value).characters);
    if (!nextName || (nextName !== oldName && Object.prototype.hasOwnProperty.call(currentCharacters, nextName))) {
      return false;
    }
    const profile = asObject(currentCharacters[oldName]);
    const nextCharacters = { ...currentCharacters };
    if (nextName !== oldName) delete nextCharacters[oldName];
    nextCharacters[nextName] = { ...profile, name: nextName };
    setAtPath(["characters"], nextCharacters);
    return true;
  };

  const addCharacter = () => {
    let number = Object.keys(characters).length + 1;
    let name = t("comic.preprocessingEditor.newCharacter", { number });
    while (Object.prototype.hasOwnProperty.call(characters, name)) {
      number += 1;
      name = t("comic.preprocessingEditor.newCharacter", { number });
    }
    setAtPath(["characters"], {
      ...characters,
      [name]: {
        name,
        age_description: "",
        appearance: "",
        personality: "",
        role: "supporting",
        gender: "unknown",
        source_span_ids: [],
      },
    });
  };

  const extraData = Object.fromEntries(
    Object.entries(data).filter(([key]) => !rootKeys.includes(key)),
  );

  return (
    <div className="min-w-0 divide-y divide-book-border/35">
      <EditorSection title={t("comic.preprocessingView.chapterSummary")} icon={BookOpen}>
        <div className="space-y-4">
          {renderObjectFields(asObject(data.comic_synthesis), ["comic_synthesis"])}
        </div>
      </EditorSection>

      <EditorSection title={t("comic.preprocessingView.characters")} icon={UserRound}>
        <div className="space-y-2">
          {Object.entries(characters).map(([name, profileValue]) => {
            const profile = asObject(profileValue);
            const shownName = typeof profile.name === "string" ? profile.name : name;
            return (
              <article key={name} className="min-w-0 rounded-md border border-book-border/45 p-3">
                <div className="mb-3 flex min-w-0 items-start gap-3">
                  <div className="min-w-0 flex-1">
                    <CharacterNameInput
                      value={shownName}
                      disabled={disabled}
                      onCommit={(next) => commitCharacterName(name, next)}
                    />
                  </div>
                  <BookIconButton
                    label={t("comic.preprocessingEditor.removeEntry", { item: shownName })}
                    title={t("comic.preprocessingEditor.removeEntry", { item: shownName })}
                    disabled={disabled}
                    onClick={() => {
                      const nextCharacters = { ...characters };
                      delete nextCharacters[name];
                      setAtPath(["characters"], nextCharacters);
                    }}
                    className="mt-6 shrink-0"
                  >
                    <Trash2 size={13} />
                  </BookIconButton>
                </div>
                {renderObjectFields(profile, ["characters", name], ["name"])}
              </article>
            );
          })}
          <BookButton size="sm" variant="secondary" disabled={disabled} onClick={addCharacter}>
            <Plus size={13} /> {t("comic.preprocessingEditor.addCharacter")}
          </BookButton>
        </div>
      </EditorSection>

      <EditorSection title={t("comic.preprocessingView.events")} icon={Clock3}>
        {renderObjectFields(asObject(data.source_events), ["source_events"])}
      </EditorSection>
      <EditorSection title={t("comic.preprocessingView.speech")} icon={MessageSquareText}>
        {renderObjectFields(speech, ["speech"])}
      </EditorSection>
      <EditorSection title={t("comic.preprocessingView.scenes")} icon={MapPin}>
        {renderObjectFields(asObject(data.visual_anchors), ["visual_anchors"])}
      </EditorSection>
      <EditorSection title={t("comic.preprocessingView.items")} icon={Package}>
        {renderObjectFields(asObject(data.items), ["items"])}
      </EditorSection>
      {Object.keys(extraData).length > 0 ? (
        <EditorSection title={t("comic.preprocessingView.additionalData")} icon={Sparkles}>
          {renderObjectFields(extraData, [])}
        </EditorSection>
      ) : null}
    </div>
  );
};
