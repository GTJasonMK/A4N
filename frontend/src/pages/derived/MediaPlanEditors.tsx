import type { GalgameBranchDesign, MediaAuthoringWorkspace, AdaptationPlanV1 } from "../../api/mediaAuthoring";
import type { AdaptationUnitRef, GalgamePipelineBeatRefV4, MangaPagePlanV4, MangaPageV4 } from "../../api/generated/host";
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
  const units = workspace.chapters.flatMap((chapter) => {
    const document = workspace.documents.find(({ record, stale }) => !stale && record.kind === "adaptation_plan" && record.source_chapter_id === chapter.source.source_chapter_id && record.status === "adopted");
    const plan = document?.record.document as AdaptationPlanV1 | undefined;
    return plan?.units.map((unit) => ({
      unit, chapterId: chapter.source.source_chapter_id,
      label: `${chapter.chapter_number} · ${unit.title}`,
      reference: { source_chapter_id: chapter.source.source_chapter_id, unit_id: unit.id },
    })) ?? [];
  });
  const beats = units.flatMap((unit) => unit.unit.beats.map((beat) => ({ label: `${unit.label} · ${beat.action}`, reference: { ...unit.reference, beat_id: beat.id } })));
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

export function MangaPagePlanEditor({ value, onChange }: { value: MangaPagePlanV4; onChange: (value: MangaPagePlanV4) => void }) {
  const { t } = useI18n();
  const updatePage = (index: number, patch: Partial<MangaPageV4>) => onChange({ ...value, pages: value.pages.map((page, i) => i === index ? { ...page, ...patch } : page) });
  return <div className="space-y-4">{value.pages.map((page, index) => <details key={page.page_number} open={index === 0} className="border-b border-book-border/60 pb-3">
    <summary className="cursor-pointer py-2 text-sm font-semibold">{t("common.mediaAuthoring.page")} {page.page_number} · {page.page_goal}</summary>
    <div className="mt-3 grid gap-3 sm:grid-cols-2">
      <PlanTextField label={t("common.mediaAuthoring.pageGoal")} value={page.page_goal} onChange={(page_goal) => updatePage(index, { page_goal })} />
      <PlanTextField label={t("common.mediaAuthoring.visualThesis")} value={page.visual_thesis} onChange={(visual_thesis) => updatePage(index, { visual_thesis })} />
      <PlanTextField label={t("common.mediaAuthoring.entryState")} value={page.entry_state} onChange={(entry_state) => updatePage(index, { entry_state })} />
      <PlanTextField label={t("common.mediaAuthoring.exitState")} value={page.exit_state} onChange={(exit_state) => updatePage(index, { exit_state })} />
    </div>
    <div className="mt-4 space-y-3">{page.panels.map((panel, panelIndex) => {
      const update = (patch: Partial<typeof panel>) => updatePage(index, { panels: page.panels.map((item, i) => i === panelIndex ? { ...item, ...patch } : item) });
      return <div key={panel.panel_number} className="space-y-2 border-l-2 border-book-border/60 pl-3">
        <h4 className="text-xs font-medium">{t("common.mediaAuthoring.panel")} {panel.panel_number}</h4>
        <div className="grid gap-3 sm:grid-cols-2">
          <PlanTextField label={t("common.mediaAuthoring.composition")} value={panel.composition} onChange={(composition) => update({ composition })} />
          <PlanTextField label={t("common.mediaAuthoring.action")} value={panel.observable_action} onChange={(observable_action) => update({ observable_action })} />
          <PlanTextField label={t("common.mediaAuthoring.reaction")} value={panel.reaction ?? ""} onChange={(reaction) => update({ reaction: reaction || null })} />
          <PlanTextField label={t("common.mediaAuthoring.spatialRelationship")} value={panel.spatial_relationship} onChange={(spatial_relationship) => update({ spatial_relationship })} />
        </div>
        {panel.text_layers.map((layer, layerIndex) => <PlanTextField key={layer.layer_index} label={`${t("common.mediaAuthoring.dialogue")} ${layerIndex + 1}${layer.speaker ? ` · ${layer.speaker}` : ""}`} value={layer.text} onChange={(text) => update({ text_layers: panel.text_layers.map((item, i) => i === layerIndex ? { ...item, text } : item) })} />)}
      </div>;
    })}</div>
  </details>)}</div>;
}
