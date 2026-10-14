import type { GalgameBranchDesign, MediaAuthoringWorkspace, AdaptationPlanV1 } from "../../api/mediaAuthoring";
import type { AdaptationUnitRef, GalgamePipelineBeatRefV4 } from "../../api/generated/host";
import { BookButton } from "../../components/ui/BookButton";
import { ThemedSelect } from "../../components/ui/ThemedSelect";
import { PlanTextField } from "./AdaptationPlanEditor";
import { useI18n } from "../../i18n";

const unitKey = (reference: AdaptationUnitRef) => JSON.stringify([reference.source_chapter_id, reference.unit_id]);
const beatKey = (reference: GalgamePipelineBeatRefV4) => JSON.stringify([reference.source_chapter_id, reference.unit_id, reference.beat_id]);

export function GalgameBranchPlanEditor({ value, workspace, onChange }: {
  value: GalgameBranchDesign; workspace: MediaAuthoringWorkspace; onChange: (value: GalgameBranchDesign) => void;
}) {
  const { t } = useI18n();
  const plansByChapter = new Map<number, AdaptationPlanV1>();
  for (const { record, stale } of workspace.documents) {
    if (
      !stale &&
      record.kind === "adaptation_plan" &&
      record.status === "adopted" &&
      record.source_chapter_id != null &&
      !plansByChapter.has(record.source_chapter_id)
    ) {
      plansByChapter.set(record.source_chapter_id, record.document as AdaptationPlanV1);
    }
  }
  const units = workspace.chapters.flatMap((chapter) => {
    const chapterId = chapter.source.source_chapter_id;
    const plan = plansByChapter.get(chapterId);
    return plan?.units.map((unit) => ({
      unit,
      chapterId,
      label: `${chapter.chapter_number} · ${unit.title}`,
      reference: { source_chapter_id: chapterId, unit_id: unit.id },
    })) ?? [];
  });
  const beats = units.flatMap((unit) => unit.unit.beats.map((beat) => ({
    label: `${unit.label} · ${beat.action}`,
    reference: { ...unit.reference, beat_id: beat.id },
  })));
  return <div className="space-y-5">
    {value.choices.length === 0 && <p className="text-sm text-book-text-muted">{t("common.mediaAuthoring.linearStory")}</p>}
    {value.choices.map((choice, index) => {
      const updateChoice = (patch: Partial<typeof choice>) => onChange({ ...value, choices: value.choices.map((item, i) => i === index ? { ...item, ...patch } : item) });
      return <section key={index} className="space-y-3 border-b border-book-border/60 pb-4">
        <div className="flex flex-wrap items-center gap-2"><h3 className="flex-1 text-sm font-semibold">{t("common.mediaAuthoring.choice")} {index + 1}</h3>
          <BookButton size="sm" variant="ghost" onClick={() => onChange({ ...value, choices: value.choices.filter((_, i) => i !== index) })}>{t("common.delete")}</BookButton>
        </div>
        <label className="grid gap-1 text-xs text-book-text-sub"><span>{t("common.mediaAuthoring.afterBeat")}</span>
          <ThemedSelect aria-label={t("common.mediaAuthoring.afterBeat")} className="w-full min-w-0" value={beatKey(choice.after_beat_ref)} onChange={(value) => { const beat = beats.find((item) => beatKey(item.reference) === value); if (beat) updateChoice({ after_beat_ref: beat.reference }); }}>
            {beats.map((beat) => <option key={beatKey(beat.reference)} value={beatKey(beat.reference)}>{beat.label}</option>)}
          </ThemedSelect>
        </label>
        <PlanTextField label={t("common.mediaAuthoring.question")} value={choice.question} onChange={(question) => updateChoice({ question })} />
        <div className="grid gap-4 md:grid-cols-2">
          {choice.options.map((option, optionIndex) => {
            const update = (patch: Partial<typeof option>) => updateChoice({ options: choice.options.map((item, i) => i === optionIndex ? { ...item, ...patch } : item) });
            return <div key={option.branch_id} className="space-y-2 border-l-2 border-book-border/60 pl-3">
              <PlanTextField label={t("common.mediaAuthoring.option")} value={option.option_text} onChange={(option_text) => update({ option_text })} />
              <PlanTextField label={t("common.mediaAuthoring.unitTitle")} value={option.branch_title} onChange={(branch_title) => update({ branch_title })} />
              <PlanTextField label={t("common.mediaAuthoring.objective")} value={option.branch_objective} onChange={(branch_objective) => update({ branch_objective })} />
              <PlanTextField label={t("common.mediaAuthoring.action")} value={option.dramatic_action} onChange={(dramatic_action) => update({ dramatic_action })} />
              <PlanTextField label={t("common.mediaAuthoring.reaction")} value={option.reaction} onChange={(reaction) => update({ reaction })} />
              <PlanTextField label={t("common.mediaAuthoring.playerKnowledge")} value={option.player_knowledge.join("\n")} onChange={(text) => update({ player_knowledge: text.split("\n") })} />
              <label className="grid gap-1 text-xs text-book-text-sub"><span>{t("common.mediaAuthoring.returnsTo")}</span>
                <ThemedSelect aria-label={t("common.mediaAuthoring.returnsTo")} className="w-full min-w-0" value={unitKey(option.returns_to)} onChange={(value) => { const unit = units.find((item) => unitKey(item.reference) === value); if (unit) update({ returns_to: unit.reference }); }}>
                  {units.map((unit) => <option key={unitKey(unit.reference)} value={unitKey(unit.reference)}>{unit.label}</option>)}
                </ThemedSelect>
              </label>
            </div>;
          })}
        </div>
      </section>;
    })}
    <BookButton variant="secondary" size="sm" disabled={units.length < 2 || beats.length === 0} onClick={() => onChange({ ...value, choices: [...value.choices, {
      after_beat_ref: beats[0].reference, question: "", options: [0, 1].map(() => ({
        branch_id: crypto.randomUUID(), option_text: "", branch_title: "", branch_objective: "", dramatic_action: "", reaction: "", player_knowledge: [], returns_to: units[1].reference,
      })),
    }] })}>{t("common.mediaAuthoring.addChoice")}</BookButton>
  </div>;
}
