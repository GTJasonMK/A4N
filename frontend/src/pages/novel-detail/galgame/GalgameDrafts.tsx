import { useCallback, useEffect, useState } from "react";
import { useBeforeUnload, useBlocker } from "react-router-dom";
import type { GalgameChoice, GalgameLine } from "../../../api/galgame";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { useI18n } from "../../../i18n";
import { buildChoiceDrafts, type ChoiceDraft } from "./GalgameChoiceEditor";
import { buildCgDraft, updatePresentationJson, type CgDraft } from "./presentation";

export type GalgameSceneFieldValues = {
  title: string;
  location: string;
  time_of_day: string;
  background_note: string;
  next_scene_id: string;
};

export type GalgameLineFieldValues = { text: string; pose: string };

export type GalgameFieldDraft =
  | { kind: "scene"; value: GalgameSceneFieldValues }
  | { kind: "line"; value: GalgameLineFieldValues };

export function useGalgameDrafts(projectId: number, choicesByLine: ReadonlyMap<number, GalgameChoice[]>) {
  const [choiceDrafts, setChoiceDrafts] = useState<Record<number, ChoiceDraft[]>>({});
  const [cgDrafts, setCgDrafts] = useState<Record<number, CgDraft>>({});
  const [fieldDrafts, setFieldDrafts] = useState<Record<string, GalgameFieldDraft>>({});
  const [fieldDraftResetRevision, setFieldDraftResetRevision] = useState(0);
  const clearDrafts = useCallback(() => {
    setChoiceDrafts({});
    setCgDrafts({});
    setFieldDrafts({});
    setFieldDraftResetRevision((revision) => revision + 1);
  }, []);
  useEffect(clearDrafts, [clearDrafts, projectId]);

  const updateFieldDraft = useCallback((key: string, draft: GalgameFieldDraft | null) => {
    setFieldDrafts((current) => {
      if (!draft && current[key] == null) return current;
      const next = { ...current };
      if (draft) next[key] = draft;
      else delete next[key];
      return next;
    });
  }, []);

  const clearFieldDraft = useCallback((key: string) => {
    setFieldDrafts((current) => {
      if (current[key] == null) return current;
      const next = { ...current };
      delete next[key];
      return next;
    });
  }, []);

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
    hasUnsavedDrafts: Object.keys(choiceDrafts).length > 0 || Object.keys(cgDrafts).length > 0 || Object.keys(fieldDrafts).length > 0,
    fieldDrafts,
    fieldDraftResetRevision,
    updateFieldDraft,
    clearFieldDraft,
    updateChoiceDraft, updateCgDraft, clearChoiceDraft, clearCgDraft, clearDrafts, saveCgDraft,
  };
}

export function GalgameDraftGuard({ dirty, saving, onDiscard }: { dirty: boolean; saving: boolean; onDiscard: () => void }) {
  const { t } = useI18n();
  const { state, proceed, reset } = useBlocker(dirty);
  useEffect(() => {
    if (!dirty && state === "blocked") proceed();
  }, [dirty, proceed, state]);
  useBeforeUnload((event) => {
    if (!dirty) return;
    event.preventDefault();
    event.returnValue = "";
  });
  if (state !== "blocked") return null;
  return (
    <Modal
      isOpen
      onClose={reset}
      title={t("common.unsavedChangesTitle")}
      tone="warning"
      maxWidthClassName="max-w-xl"
      footer={
        <>
          <BookButton variant="secondary" onClick={reset}>{t("common.cancel")}</BookButton>
          <BookButton variant="warning" disabled={saving} onClick={() => {
            onDiscard();
          }}>{saving ? t("common.loading") : t("common.discardDraft")}</BookButton>
        </>
      }
    >
      {t("galgame.unsavedChoices")}
    </Modal>
  );
}
