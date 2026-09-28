import type { AdaptationBeat, AdaptationOrigin, AdaptationPlanV1, AdaptationUnit, AuthoringChapter } from "../../api/mediaAuthoring";
import { BookButton } from "../../components/ui/BookButton";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { useI18n } from "../../i18n";

export function PlanTextField({ label, value, onChange, rows = 2 }: { label: string; value: string; onChange: (value: string) => void; rows?: number }) {
  return <label className="grid min-w-0 gap-1 text-xs text-book-text-sub">
    <span>{label}</span>
    <textarea className="pyqt-input w-full resize-y text-sm leading-relaxed" value={value} rows={rows} onChange={(event) => onChange(event.target.value)} />
  </label>;
}

export function newAdaptationUnit(): AdaptationUnit {
  return {
    id: crypto.randomUUID(), origin: "added", title: "", location: null, participants: [],
    dramatic_question: "", objective: "", obstacle: "", entry_state: "", exit_state: "", source_span_ids: [],
    beats: [newBeat()],
  };
}

function newBeat(): AdaptationBeat {
  return {
    id: crypto.randomUUID(), origin: "added", kind: "action", importance: "required", is_climax: false,
    action: "", reaction: null, dialogue_intent: null, entry_state: "", exit_state: "", source_span_ids: [],
  };
}

export function AdaptationPlanEditor({ value, chapter, onChange }: {
  value: AdaptationPlanV1; chapter: AuthoringChapter; onChange: (value: AdaptationPlanV1) => void;
}) {
  const { t } = useI18n();
  const updateUnit = (index: number, patch: Partial<AdaptationUnit>) => onChange({ ...value, units: value.units.map((unit, i) => i === index ? { ...unit, ...patch } : unit) });
  const origins: AdaptationOrigin[] = ["source", "adapted", "added", "historical"];
  const originLabels = { source: t("common.mediaAuthoring.originSource"), adapted: t("common.mediaAuthoring.originAdapted"), added: t("common.mediaAuthoring.originAdded"), historical: t("common.mediaAuthoring.originHistorical") };
  const sourceSelector = (origin: AdaptationOrigin, ids: string[], change: (ids: string[]) => void) => origin === "added" ? null : (
    <details className="text-xs text-book-text-muted">
      <summary className="cursor-pointer py-2">{t("common.mediaAuthoring.sourceEvidence")} · {ids.length}</summary>
      <div className="max-h-48 space-y-2 overflow-y-auto p-2">
        {chapter.source_spans.map((span, index) => <label key={span.id} className="flex items-start gap-2">
          <input type="checkbox" checked={ids.includes(span.id)} onChange={(event) => change(event.target.checked ? [...ids, span.id] : ids.filter((id) => id !== span.id))} />
          <span className="min-w-0 break-words">{index + 1}. {span.text}</span>
        </label>)}
      </div>
    </details>
  );
  return <div className="space-y-4">
    <PlanTextField label={t("common.mediaAuthoring.intention")} value={value.intention} onChange={(intention) => onChange({ ...value, intention })} />
    {value.decisions.length > 0 && <details className="border-b border-book-border/50 pb-3">
      <summary className="cursor-pointer text-sm font-medium">{t("common.mediaAuthoring.decisions")}</summary>
      <div className="mt-3 space-y-3">{value.decisions.map((decision, index) => <div key={index} className="grid gap-2 sm:grid-cols-2">
        <PlanTextField label={t("common.mediaAuthoring.decision")} value={decision.description} onChange={(description) => onChange({ ...value, decisions: value.decisions.map((item, i) => i === index ? { ...item, description } : item) })} />
        <PlanTextField label={t("common.mediaAuthoring.reason")} value={decision.reason} onChange={(reason) => onChange({ ...value, decisions: value.decisions.map((item, i) => i === index ? { ...item, reason } : item) })} />
      </div>)}</div>
    </details>}
    {value.units.map((unit, index) => <details key={unit.id} open={index === 0} className="border-b border-book-border/60 pb-3">
      <summary className="cursor-pointer py-2 text-sm font-semibold text-book-text-main">{index + 1}. {unit.title || t("common.mediaAuthoring.untitledUnit")}</summary>
      <div className="mt-2 space-y-3">
        <div className="grid gap-3 sm:grid-cols-2">
          <PlanTextField label={t("common.mediaAuthoring.unitTitle")} value={unit.title} rows={1} onChange={(title) => updateUnit(index, { title })} />
          <label className="grid gap-1 text-xs text-book-text-sub"><span>{t("common.mediaAuthoring.origin")}</span>
            <ThemedSelect aria-label={t("common.mediaAuthoring.origin")} value={unit.origin} onChange={(origin) => updateUnit(index, { origin: origin as AdaptationOrigin, ...(origin === "added" ? { source_span_ids: [] } : {}) })}>
              {origins.map((origin) => <option key={origin} value={origin}>{originLabels[origin]}</option>)}
            </ThemedSelect>
          </label>
          <PlanTextField label={t("common.mediaAuthoring.objective")} value={unit.objective} onChange={(objective) => updateUnit(index, { objective })} />
          <PlanTextField label={t("common.mediaAuthoring.obstacle")} value={unit.obstacle} onChange={(obstacle) => updateUnit(index, { obstacle })} />
          <PlanTextField label={t("common.mediaAuthoring.dramaticQuestion")} value={unit.dramatic_question} onChange={(dramatic_question) => updateUnit(index, { dramatic_question })} />
          <PlanTextField label={t("common.mediaAuthoring.location")} value={unit.location ?? ""} onChange={(location) => updateUnit(index, { location: location || null })} />
          <PlanTextField label={t("common.mediaAuthoring.entryState")} value={unit.entry_state} onChange={(entry_state) => updateUnit(index, { entry_state })} />
          <PlanTextField label={t("common.mediaAuthoring.exitState")} value={unit.exit_state} onChange={(exit_state) => updateUnit(index, { exit_state })} />
        </div>
        {sourceSelector(unit.origin, unit.source_span_ids, (source_span_ids) => updateUnit(index, { source_span_ids }))}
        <ol className="space-y-3 border-l-2 border-book-border/60 pl-3">
          {unit.beats.map((beat, beatIndex) => {
            const update = (patch: Partial<AdaptationBeat>) => updateUnit(index, { beats: unit.beats.map((item, i) => i === beatIndex ? { ...item, ...patch } : item) });
            return <li key={beat.id} className="space-y-2">
              <div className="flex flex-wrap items-center gap-2 text-xs">
                <span className="font-semibold">{t("common.mediaAuthoring.beat")} {beatIndex + 1}</span>
                <ThemedSelect aria-label={t("common.mediaAuthoring.origin")} value={beat.origin} onChange={(origin) => update({ origin: origin as AdaptationOrigin, ...(origin === "added" ? { source_span_ids: [] } : {}) })}>
                  {origins.map((origin) => <option key={origin} value={origin}>{originLabels[origin]}</option>)}
                </ThemedSelect>
                <label className="inline-flex items-center gap-1"><input type="checkbox" checked={beat.importance === "required"} onChange={(event) => update({ importance: event.target.checked ? "required" : "supporting" })} />{t("common.mediaAuthoring.requiredBeat")}</label>
                <label className="inline-flex items-center gap-1"><input type="checkbox" checked={beat.is_climax} onChange={(event) => update({ is_climax: event.target.checked })} />{t("common.mediaAuthoring.climax")}</label>
                {unit.beats.length > 1 && <BookButton size="sm" variant="ghost" onClick={() => updateUnit(index, { beats: unit.beats.filter((_, i) => i !== beatIndex) })}>{t("common.delete")}</BookButton>}
              </div>
              <PlanTextField label={t("common.mediaAuthoring.action")} value={beat.action} onChange={(action) => update({ action })} />
              <div className="grid gap-2 sm:grid-cols-2">
                <PlanTextField label={t("common.mediaAuthoring.reaction")} value={beat.reaction ?? ""} onChange={(reaction) => update({ reaction: reaction || null })} />
                <PlanTextField label={t("common.mediaAuthoring.dialogueIntent")} value={beat.dialogue_intent ?? ""} onChange={(dialogue_intent) => update({ dialogue_intent: dialogue_intent || null })} />
              </div>
              {sourceSelector(beat.origin, beat.source_span_ids, (source_span_ids) => update({ source_span_ids }))}
            </li>;
          })}
        </ol>
        <div className="flex flex-wrap gap-2">
          <BookButton size="sm" variant="secondary" onClick={() => updateUnit(index, { beats: [...unit.beats, newBeat()] })}>{t("common.mediaAuthoring.addBeat")}</BookButton>
          <BookButton size="sm" variant="ghost" disabled={index === 0} onClick={() => { const units = [...value.units]; [units[index - 1], units[index]] = [units[index], units[index - 1]]; onChange({ ...value, units }); }}>{t("common.mediaAuthoring.moveEarlier")}</BookButton>
          {value.units.length > 1 && <BookButton size="sm" variant="ghost" onClick={() => onChange({ ...value, units: value.units.filter((_, i) => i !== index) })}>{t("common.mediaAuthoring.removeUnit")}</BookButton>}
        </div>
      </div>
    </details>)}
    <BookButton variant="secondary" size="sm" onClick={() => onChange({ ...value, units: [...value.units, newAdaptationUnit()] })}>{t("common.mediaAuthoring.addUnit")}</BookButton>
  </div>;
}
