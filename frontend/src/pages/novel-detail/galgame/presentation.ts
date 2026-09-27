import type {
  GalgameCgImportance,
  GalgameLinePresentationDocumentV2,
  GalgameTransition,
} from "../../../api/galgame";

export interface PresentationPatch {
  cgEnabled?: boolean;
  cgNote?: string;
  cgImportance?: GalgameCgImportance;
  transition?: GalgameTransition | "";
}

export interface CgDraft {
  cgEnabled: boolean;
  cgNote: string;
  cgImportance: GalgameCgImportance | "";
}

export function buildCgDraft(value: GalgameLinePresentationDocumentV2): CgDraft {
  return {
    cgEnabled: value.cg === true,
    cgNote: presentationCgNote(value),
    cgImportance: value.cg_importance ?? "",
  };
}

export function presentationCgNote(
  value: GalgameLinePresentationDocumentV2,
): string {
  return value.cg_note ?? "";
}

export function updatePresentationJson(
  current: GalgameLinePresentationDocumentV2,
  patch: PresentationPatch,
): GalgameLinePresentationDocumentV2 {
  const next = { ...current };

  if (patch.cgEnabled === false) clearEventCgPresentation(next);

  if (patch.cgNote != null) {
    const note = patch.cgNote.trim();
    if (note) {
      next.cg = true;
      next.cg_note = note;
    } else {
      clearEventCgPresentation(next);
    }
  }

  if (patch.cgImportance != null) next.cg_importance = patch.cgImportance;

  if (presentationCgNote(next)) {
    if (next.cg_importance == null) {
      throw new Error("事件 CG 必须显式选择优先级");
    }
    next.cg = true;
  } else {
    clearEventCgPresentation(next);
  }

  if (patch.transition != null) {
    next.transition = patch.transition || null;
  }

  return next;
}

function clearEventCgPresentation(
  value: GalgameLinePresentationDocumentV2,
): void {
  value.cg = null;
  value.cg_note = null;
  value.cg_importance = null;
  value.transition = null;
}
