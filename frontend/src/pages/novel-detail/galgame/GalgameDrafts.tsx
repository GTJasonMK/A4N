import { useCallback, useEffect, useState } from "react";
import type { GalgameChoice, GalgameLine } from "../../../api/galgame";
import { buildChoiceDrafts, type ChoiceDraft } from "./GalgameChoiceEditor";
import { buildCgDraft, updatePresentationJson, type CgDraft } from "./presentation";

export function useGalgameDrafts(projectId: number, choicesByLine: ReadonlyMap<number, GalgameChoice[]>) {
  const [choiceDrafts, setChoiceDrafts] = useState<Record<number, ChoiceDraft[]>>({});
  const [cgDrafts, setCgDrafts] = useState<Record<number, CgDraft>>({});
  const clearDrafts = useCallback(() => {
    setChoiceDrafts({});
    setCgDrafts({});
  }, []);
  useEffect(clearDrafts, [clearDrafts, projectId]);

  const clearChoiceDraft = (lineId: number) => {
    setChoiceDrafts((current) => {
      const next = { ...current };
      delete next[lineId];
      return next;
    });
  };
  const clearCgDraft = (lineId: number, submitted?: CgDraft) => {
    setCgDrafts((current) => {
      if (submitted && current[lineId] !== submitted) return current;
      const next = { ...current };
      delete next[lineId];
      return next;
    });
  };
  const acceptChoiceDraft = (lineId: number, submitted: ChoiceDraft[] | undefined) => {
    setChoiceDrafts((current) => {
      if (current[lineId] !== submitted) return current;
      const next = { ...current };
      delete next[lineId];
      return next;
    });
  };
  const updateChoiceDraft = (lineId: number, drafts: ChoiceDraft[]) => {
    if (JSON.stringify(drafts) === JSON.stringify(buildChoiceDrafts(choicesByLine.get(lineId) ?? []))) {
      clearChoiceDraft(lineId);
    } else {
      setChoiceDrafts((current) => ({ ...current, [lineId]: drafts }));
    }
  };
  const updateCgDraft = (line: GalgameLine, draft: CgDraft) => {
    if (JSON.stringify(draft) === JSON.stringify(buildCgDraft(line.presentation_json))) {
      clearCgDraft(line.id);
    } else {
      setCgDrafts((current) => ({ ...current, [line.id]: draft }));
    }
  };
  const saveCgDraft = async (
    line: GalgameLine,
    draft: CgDraft,
    save: (line: GalgameLine, patch: Partial<GalgameLine>) => Promise<boolean | undefined>,
  ) => {
    if (JSON.stringify(draft) === JSON.stringify(buildCgDraft(line.presentation_json))) return;
    if (draft.cgEnabled && (!draft.cgNote.trim() || !draft.cgImportance)) return;
    const presentation = updatePresentationJson(line.presentation_json,
      draft.cgEnabled && draft.cgImportance
        ? { cgNote: draft.cgNote, cgImportance: draft.cgImportance }
        : { cgEnabled: false },
    );
    if (await save(line, { presentation_json: presentation })) clearCgDraft(line.id, draft);
  };
  return {
    choiceDrafts, cgDrafts, acceptChoiceDraft,
    hasUnsavedDrafts: Object.keys(choiceDrafts).length > 0 || Object.keys(cgDrafts).length > 0,
    updateChoiceDraft, updateCgDraft, clearChoiceDraft, clearCgDraft, clearDrafts, saveCgDraft,
  };
}
