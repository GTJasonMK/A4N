import type { FC, ReactNode } from "react";
import {
  BookOpen,
  Clock3,
  Layers3,
  MapPin,
  MessageSquareText,
  Package,
  Sparkles,
  UserRound,
  type LucideIcon,
} from "lucide-react";
import { useI18n } from "../../../i18n";

type JsonObject = Record<string, unknown>;

const asObject = (value: unknown): JsonObject =>
  value != null && typeof value === "object" && !Array.isArray(value)
    ? value as JsonObject
    : {};

const asArray = (value: unknown): unknown[] => Array.isArray(value) ? value : [];

const ContentSection: FC<{
  title: string;
  count?: number;
  icon: LucideIcon;
  children: ReactNode;
}> = ({ title, count, icon: Icon, children }) => (
  <section className="min-w-0 py-4 first:pt-0 last:pb-0">
    <header className="mb-3 flex min-w-0 items-center gap-2">
      <Icon size={15} className="shrink-0 text-book-accent" aria-hidden="true" />
      <h3 className="min-w-0 text-xs font-semibold text-book-text-main">{title}</h3>
      {count != null ? (
        <span className="text-small text-book-text-muted">{count}</span>
      ) : null}
    </header>
    {children}
  </section>
);

export const MangaPreprocessingView: FC<{ payload: unknown }> = ({ payload }) => {
  const { t } = useI18n();
  const data = asObject(payload);
  const synthesis = asObject(data.comic_synthesis);
  const characters = asObject(data.characters);
  const events = asArray(asObject(data.source_events).events).map(asObject);
  const speech = asObject(data.speech);
  const dialogues = asArray(speech.dialogues).map(asObject);
  const narrations = asArray(speech.narrations).map(asObject);
  const scenes = asArray(asObject(data.visual_anchors).scenes).map(asObject);
  const items = asArray(asObject(data.items).items).map(asObject);
  const moods = asArray(synthesis.mood_progression);

  const fieldLabels: Record<string, string> = {
    age_description: t("comic.preprocessingView.field.age"),
    appearance: t("comic.preprocessingView.field.appearance"),
    atmosphere: t("comic.preprocessingView.field.atmosphere"),
    bubble_type: t("comic.preprocessingView.field.bubbleType"),
    chapter_summary: t("comic.preprocessingView.field.chapterSummary"),
    content: t("comic.preprocessingView.field.content"),
    description: t("comic.preprocessingView.field.description"),
    dialogues: t("comic.preprocessingView.dialogue"),
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
    name: t("comic.preprocessingView.field.name"),
    narration_type: t("comic.preprocessingView.field.narrationType"),
    participants: t("comic.preprocessingView.field.participants"),
    personality: t("comic.preprocessingView.field.personality"),
    position: t("comic.preprocessingView.field.position"),
    role: t("comic.preprocessingView.field.role"),
    scenes: t("comic.preprocessingView.scenes"),
    schema_version: t("comic.preprocessingView.field.schemaVersion"),
    source_events: t("comic.preprocessingView.field.sourceEvents"),
    source_event_id: t("comic.preprocessingView.field.sourceEvent"),
    source_event_ids: t("comic.preprocessingView.field.sourceEvents"),
    source_span_ids: t("comic.preprocessingView.field.sourceSpans"),
    speech: t("comic.preprocessingView.speech"),
    speaker: t("comic.preprocessingView.field.speaker"),
    target: t("comic.preprocessingView.field.target"),
    time_of_day: t("comic.preprocessingView.field.timeOfDay"),
    type: t("comic.preprocessingView.field.type"),
    visual_features: t("comic.preprocessingView.field.visualFeatures"),
    visual_anchors: t("comic.preprocessingView.scenes"),
    weather: t("comic.preprocessingView.field.weather"),
    narrations: t("comic.preprocessingView.narration"),
    items: t("comic.preprocessingView.items"),
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
    role: {
      antagonist: t("comic.preprocessingView.value.antagonist"),
      background: t("comic.preprocessingView.value.background"),
      minor: t("comic.preprocessingView.value.minor"),
      protagonist: t("comic.preprocessingView.value.protagonist"),
      supporting: t("comic.preprocessingView.value.supporting"),
    },
    time_of_day: {
      dawn: t("comic.preprocessingView.value.dawn"),
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

  const fieldLabel = (key: string) =>
    Object.prototype.hasOwnProperty.call(fieldLabels, key)
      ? fieldLabels[key]
      : key.replace(/_/g, " ");

  const renderValue = (value: unknown, field = ""): ReactNode => {
    if (value == null || value === "") {
      return <span className="text-book-text-muted">{t("comic.preprocessingView.emptyValue")}</span>;
    }
    if (typeof value === "string") {
      const labels = Object.prototype.hasOwnProperty.call(valueLabels, field)
        ? valueLabels[field]
        : undefined;
      const translated = labels && Object.prototype.hasOwnProperty.call(labels, value)
        ? labels[value]
        : undefined;
      return <span className="break-words">{translated ?? value}</span>;
    }
    if (typeof value === "number") return <span>{value}</span>;
    if (typeof value === "boolean") {
      return <span>{value ? t("comic.preprocessingView.yes") : t("comic.preprocessingView.no")}</span>;
    }
    if (Array.isArray(value)) {
      if (value.length === 0) {
        return <span className="text-book-text-muted">{t("comic.preprocessingView.emptyValue")}</span>;
      }
      const primitiveItems = value.every((item) => item == null || ["string", "number", "boolean"].includes(typeof item));
      return (
        <ul className={primitiveItems ? "flex min-w-0 flex-wrap gap-x-3 gap-y-1" : "min-w-0 space-y-2"}>
          {value.map((item, index) => (
            <li
              key={`${field}-${index}`}
              className={field === "source_span_ids" ? "break-all font-mono text-small text-book-text-muted" : "min-w-0 break-words text-body-sm text-book-text-main"}
            >
              {renderValue(item, field)}
            </li>
          ))}
        </ul>
      );
    }
    const object = asObject(value);
    return (
      <dl className="min-w-0 space-y-2">
        {Object.entries(object).map(([key, nestedValue]) => (
          <div key={key} className="grid min-w-0 gap-1 sm:grid-cols-[minmax(7rem,0.35fr)_minmax(0,1fr)] sm:gap-x-4">
            <dt className="text-small text-book-text-muted">{fieldLabel(key)}</dt>
            <dd className="min-w-0 text-body-sm text-book-text-main">{renderValue(nestedValue, key)}</dd>
          </div>
        ))}
      </dl>
    );
  };

  const renderFields = (value: JsonObject, excluded: string[] = []) => {
    const entries = Object.entries(value).filter(([key]) => !excluded.includes(key));
    if (entries.length === 0) return null;
    return (
      <dl className="min-w-0 space-y-2">
        {entries.map(([key, fieldValue]) => (
          <div key={key} className="grid min-w-0 gap-1 sm:grid-cols-[minmax(7rem,0.32fr)_minmax(0,1fr)] sm:gap-x-5">
            <dt className="text-small text-book-text-muted">{fieldLabel(key)}</dt>
            <dd className="min-w-0 text-body-sm leading-relaxed text-book-text-main">
              {renderValue(fieldValue, key)}
            </dd>
          </div>
        ))}
      </dl>
    );
  };

  const extraSynthesis = Object.fromEntries(
    Object.entries(synthesis).filter(([key]) =>
      !["chapter_summary", "mood_progression"].includes(key),
    ),
  );
  const extraData = Object.fromEntries(
    Object.entries(data).filter(([key]) =>
      !["schema_version", "comic_synthesis", "characters", "source_events", "speech", "visual_anchors", "items"].includes(key),
    ),
  );
  const extensionGroups = [
    ["source_events", data.source_events, ["events"]],
    ["speech", data.speech, ["dialogues", "narrations"]],
    ["visual_anchors", data.visual_anchors, ["scenes"]],
    ["items", data.items, ["items"]],
  ] as Array<[string, unknown, string[]]>;
  for (const [key, value, knownFields] of extensionGroups) {
    const fields = Object.fromEntries(
      Object.entries(asObject(value)).filter(([field]) => !knownFields.includes(field)),
    );
    if (Object.keys(fields).length > 0) extraData[key] = fields;
  }

  if (Object.keys(data).length === 0) {
    return <p className="text-small text-book-text-muted">{t("comic.preprocessingView.noData")}</p>;
  }

  return (
    <div className="min-w-0 divide-y divide-book-border/35">
      {synthesis.chapter_summary != null ? (
        <section className="min-w-0 pb-4">
          <div className="min-w-0">
            <div className="mb-2 flex items-center gap-2">
              <BookOpen size={15} className="shrink-0 text-book-accent" aria-hidden="true" />
              <h3 className="text-xs font-semibold text-book-text-main">{t("comic.preprocessingView.chapterSummary")}</h3>
            </div>
            {synthesis.chapter_summary != null ? (
              <p className="whitespace-pre-wrap break-words text-body-sm leading-relaxed text-book-text-main">
                {renderValue(synthesis.chapter_summary, "chapter_summary")}
              </p>
            ) : null}
          </div>
        </section>
      ) : null}

      {moods.length > 0 ? (
        <ContentSection title={t("comic.preprocessingView.moodProgression")} count={moods.length} icon={Sparkles}>
          <ol className="grid min-w-0 grid-cols-1 gap-y-2 sm:grid-cols-2 xl:grid-cols-3">
            {moods.map((mood, index) => (
              <li key={`mood-${index}`} className="flex min-w-0 items-start gap-2 border-l-2 border-book-accent/50 pl-2">
                <span className="w-5 shrink-0 font-mono text-small text-book-text-muted">{String(index + 1).padStart(2, "0")}</span>
                <span className="min-w-0 break-words text-body-sm text-book-text-main">{renderValue(mood)}</span>
              </li>
            ))}
          </ol>
          {renderFields(extraSynthesis)}
        </ContentSection>
      ) : Object.keys(extraSynthesis).length > 0 ? (
        <ContentSection title={t("comic.preprocessingView.chapterSummary")} icon={BookOpen}>
          {renderFields(extraSynthesis)}
        </ContentSection>
      ) : null}

      {Object.keys(characters).length > 0 ? (
        <ContentSection title={t("comic.preprocessingView.characters")} count={Object.keys(characters).length} icon={UserRound}>
          <div className="grid min-w-0 gap-2 xl:grid-cols-2">
            {Object.entries(characters).map(([characterKey, rawCharacter]) => {
              const character = asObject(rawCharacter);
              const name = typeof character.name === "string" ? character.name : characterKey;
              return (
                <article key={characterKey} className="min-w-0 rounded-md border border-book-border/45 px-3 py-2.5">
                  <header className="mb-2 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                    <h4 className="min-w-0 break-words text-sm font-semibold text-book-text-main">{name}</h4>
                    {character.role != null ? (
                      <span className="text-small text-book-accent">{renderValue(character.role, "role")}</span>
                    ) : null}
                  </header>
                  {renderFields(character, ["name", "role"])}
                </article>
              );
            })}
          </div>
        </ContentSection>
      ) : null}

      {events.length > 0 ? (
        <ContentSection title={t("comic.preprocessingView.events")} count={events.length} icon={Clock3}>
          <ol className="space-y-1">
            {events.map((event, index) => (
              <li key={String(event.event_id ?? index)} className="border-l-2 border-book-border/60 py-1 pl-3">
                <div className="mb-1 flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-1">
                  <span className="font-mono text-small text-book-text-muted">{String(event.event_id ?? `#${index + 1}`)}</span>
                  {event.importance != null ? <span className="text-small text-amber-300">{renderValue(event.importance, "importance")}</span> : null}
                </div>
                {renderFields(event, ["event_id", "importance"])}
              </li>
            ))}
          </ol>
        </ContentSection>
      ) : null}

      {scenes.length > 0 ? (
        <ContentSection title={t("comic.preprocessingView.scenes")} count={scenes.length} icon={MapPin}>
          <div className="grid min-w-0 gap-x-5 gap-y-4 xl:grid-cols-2">
            {scenes.map((scene, index) => (
              <article key={String(scene.index ?? index)} className="min-w-0 border-l-2 border-sky-400/50 pl-3">
                <h4 className="mb-2 break-words text-xs font-semibold text-book-text-main">
                  {scene.location != null ? renderValue(scene.location, "location") : `${t("comic.preprocessingView.sceneLabel")} ${index + 1}`}
                  {typeof scene.location_en === "string" ? <span className="ml-2 font-normal text-book-text-muted">{scene.location_en}</span> : null}
                </h4>
                {renderFields(scene, ["location", "location_en", "index"])}
              </article>
            ))}
          </div>
        </ContentSection>
      ) : null}

      {items.length > 0 ? (
        <ContentSection title={t("comic.preprocessingView.items")} count={items.length} icon={Package}>
          <div className="grid min-w-0 gap-x-5 gap-y-4 xl:grid-cols-2">
            {items.map((item, index) => (
              <article key={String(item.name ?? index)} className="min-w-0 border-l-2 border-emerald-400/50 pl-3">
                <h4 className="mb-2 break-words text-xs font-semibold text-book-text-main">
                  {item.name != null ? renderValue(item.name, "name") : `${t("comic.preprocessingView.itemLabel")} ${index + 1}`}
                  {item.importance != null ? <span className="ml-2 font-normal text-book-text-muted">{renderValue(item.importance, "importance")}</span> : null}
                </h4>
                {renderFields(item, ["name", "importance"])}
              </article>
            ))}
          </div>
        </ContentSection>
      ) : null}

      {dialogues.length + narrations.length > 0 ? (
        <ContentSection title={t("comic.preprocessingView.speech")} count={dialogues.length + narrations.length} icon={MessageSquareText}>
          <div className="grid min-w-0 gap-x-6 gap-y-4 xl:grid-cols-2">
            {dialogues.length > 0 ? (
              <div className="min-w-0">
                <h4 className="mb-2 text-small font-semibold text-book-text-sub">{t("comic.preprocessingView.dialogue")}</h4>
                <ol className="space-y-3">
                  {dialogues.map((dialogue, index) => (
                    <li key={String(dialogue.index ?? index)} className="min-w-0 border-l-2 border-rose-400/50 pl-3">
                      <p className="break-words text-body-sm leading-relaxed text-book-text-main"><q>{renderValue(dialogue.content, "content")}</q></p>
                      <div className="mt-1 flex min-w-0 flex-wrap gap-x-2 text-small text-book-text-muted">
                        {dialogue.speaker != null ? <span>{renderValue(dialogue.speaker, "speaker")}</span> : null}
                        {dialogue.emotion != null ? <span>{renderValue(dialogue.emotion, "emotion")}</span> : null}
                      </div>
                      {renderFields(dialogue, ["index", "content", "speaker", "emotion"])}
                    </li>
                  ))}
                </ol>
              </div>
            ) : null}
            {narrations.length > 0 ? (
              <div className="min-w-0">
                <h4 className="mb-2 text-small font-semibold text-book-text-sub">{t("comic.preprocessingView.narration")}</h4>
                <ol className="space-y-3">
                  {narrations.map((narration, index) => (
                    <li key={String(narration.index ?? index)} className="min-w-0 border-l-2 border-violet-400/50 pl-3">
                      <p className="break-words text-body-sm leading-relaxed text-book-text-main"><q>{renderValue(narration.content, "content")}</q></p>
                      {renderFields(narration, ["index", "content"])}
                    </li>
                  ))}
                </ol>
              </div>
            ) : null}
          </div>
        </ContentSection>
      ) : null}

      {Object.keys(extraData).length > 0 ? (
        <ContentSection title={t("comic.preprocessingView.additionalData")} icon={Layers3}>
          {renderFields(extraData)}
        </ContentSection>
      ) : null}

      {events.length + Object.keys(characters).length + scenes.length + items.length + dialogues.length + narrations.length === 0 &&
        synthesis.chapter_summary == null && moods.length === 0 &&
        Object.keys(extraSynthesis).length === 0 && Object.keys(extraData).length === 0 ? (
        <p className="py-2 text-small text-book-text-muted">{t("comic.preprocessingView.noData")}</p>
      ) : null}
    </div>
  );
};
