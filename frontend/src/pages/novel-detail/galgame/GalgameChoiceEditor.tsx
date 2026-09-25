import React, { useMemo } from "react";
import { Plus, Trash2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { useI18n, type MessageTranslator } from "../../../i18n";
import type {
  GalgameChoice,
  GalgameLine,
  GalgameSceneDetail,
  UpdateGalgameChoiceOptionInput,
} from "../../../api/galgame";
import { sceneSelectLabel } from "./galgameTabShared";

export interface ChoiceDraft {
  option_text: string;
  target_scene_id: string;
}

export const ChoiceEditor: React.FC<{
  line: GalgameLine;
  drafts: ChoiceDraft[];
  onDraftsChange: (drafts: ChoiceDraft[]) => void;
  scenes: GalgameSceneDetail[];
  sceneById: Map<number, GalgameSceneDetail>;
  disabled: boolean;
  onUpdate: (choices: UpdateGalgameChoiceOptionInput[]) => void;
}> = ({ line, drafts, onDraftsChange, scenes, sceneById, disabled, onUpdate }) => {
  const { t } = useI18n();

  const targetInspections = useMemo(
    () =>
      drafts.map((draft) =>
        inspectChoiceTarget(draft.target_scene_id, sceneById, t),
      ),
    [drafts, sceneById, t],
  );
  const normalizedChoices = useMemo(() => {
    return drafts
      .map((draft) => {
        const optionText = draft.option_text.trim();
        const targetSceneId = Number(draft.target_scene_id);
        if (
          !optionText ||
          !Number.isInteger(targetSceneId) ||
          !sceneById.has(targetSceneId)
        ) {
          return null;
        }
        return {
          option_text: optionText,
          target_scene_id: targetSceneId,
        };
      })
      .filter((item): item is UpdateGalgameChoiceOptionInput => item != null);
  }, [drafts, sceneById]);

  const structurallyComplete =
    drafts.length >= 2 &&
    drafts.length <= 3 &&
    normalizedChoices.length === drafts.length;
  const targetIssueCount = targetInspections.filter(
    (item) => item.kind === "error",
  ).length;
  const canSave = structurallyComplete && targetIssueCount === 0;
  const validationMessage =
    scenes.length === 0
      ? t("galgame.targetUnavailable")
      : structurallyComplete
        ? targetIssueCount > 0
          ? t("galgame.targetNeedsFix", { count: targetIssueCount })
          : t("galgame.targetReady")
        : t("galgame.choiceCountInvalid");

  const updateDraft = (index: number, patch: Partial<ChoiceDraft>) => {
    onDraftsChange(
      drafts.map((draft, itemIndex) =>
        itemIndex === index ? { ...draft, ...patch } : draft,
      ),
    );
  };

  const addDraft = () => {
    if (drafts.length >= 3) return;
    onDraftsChange([
      ...drafts,
      {
        option_text: "",
        target_scene_id: "",
      },
    ]);
  };

  const removeDraft = (index: number) => {
    if (drafts.length <= 2) return;
    onDraftsChange(
      drafts.filter((_, itemIndex) => itemIndex !== index),
    );
  };

  return (
    <div className="mt-2 rounded-xl bg-book-bg-paper/45 p-2.5">
      <div className="space-y-2">
        {drafts.map((draft, index) => {
          const inspection = targetInspections[index];
          return (
            <div key={`${line.id}:${index}`} className="space-y-1">
              <div className="grid gap-2 md:grid-cols-[minmax(0,1fr)_minmax(140px,220px)_auto]">
                <input
                  className="border border-book-border bg-book-bg px-2 py-1 text-xs text-book-text-main outline-none focus:border-book-primary"
                  value={draft.option_text}
                  disabled={disabled}
                  placeholder={t("galgame.choicePlaceholder", { index: index + 1 })}
                  onChange={(event) =>
                    updateDraft(index, { option_text: event.target.value })
                  }
                />
                <ThemedSelect
                  className="pyqt-input text-xs text-book-text-main"
                  value={draft.target_scene_id}
                  disabled={disabled || scenes.length === 0}
                  onChange={(selectedValue) =>
                    updateDraft(index, { target_scene_id: selectedValue })
                  }
                >
                  <option value="">
                    {scenes.length === 0
                      ? t("galgame.noTargetScene")
                      : t("galgame.chooseTargetScene")}
                  </option>
                  {scenes.map((item) => (
                    <option key={item.scene.id} value={item.scene.id}>
                      {sceneSelectLabel(item.scene, t)}
                    </option>
                  ))}
                </ThemedSelect>
                {drafts.length > 2 ? (
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={() => removeDraft(index)}
                    disabled={disabled}
                    aria-label={t("galgame.deleteChoice", { index: index + 1 })}
                  >
                    <Trash2 size={13} />
                  </BookButton>
                ) : null}
              </div>
              {inspection ? (
                <ChoiceTargetSummary inspection={inspection} />
              ) : null}
            </div>
          );
        })}
      </div>
      {targetInspections.some((item) => item.kind === "error") ? (
        <div className="mt-2 border border-error/50 bg-book-bg px-2 py-1 text-small text-error">
          {t("galgame.choiceTargetWarning")}
        </div>
      ) : null}
      <div className="mt-2 flex flex-col gap-2 border-t border-book-border/35 pt-2 sm:flex-row sm:items-center">
        <span
          className={`min-w-0 flex-1 text-small ${
            canSave && targetIssueCount === 0
              ? "text-book-text-muted"
              : "text-error"
          }`}
        >
          {validationMessage}
        </span>
        <div className="flex items-center justify-end gap-2">
          <BookButton
            variant="secondary"
            size="sm"
            onClick={addDraft}
            disabled={disabled || drafts.length >= 3 || scenes.length === 0}
          >
            <Plus size={13} />
            {t("galgame.addChoice")}
          </BookButton>
          <BookButton
            variant="primary"
            size="sm"
            onClick={() => onUpdate(normalizedChoices)}
            disabled={disabled || !canSave}
          >
            {t("galgame.saveChoices")}
          </BookButton>
        </div>
      </div>
    </div>
  );
};

type ChoiceTargetInspection =
  | { kind: "empty"; message: string }
  | { kind: "error"; message: string }
  | { kind: "main"; message: string }
  | { kind: "branch"; message: string };

const ChoiceTargetSummary: React.FC<{ inspection: ChoiceTargetInspection }> = ({
  inspection,
}) => {
  const className =
    inspection.kind === "error" ? "text-error" : "text-book-text-muted";
  return (
    <div className={`px-1 text-small ${className}`}>{inspection.message}</div>
  );
};

function inspectChoiceTarget(
  targetSceneId: string,
  sceneById: Map<number, GalgameSceneDetail>,
  t: MessageTranslator,
): ChoiceTargetInspection {
  if (!targetSceneId) {
    return { kind: "empty", message: t("galgame.choiceTarget.notSelected") };
  }
  const sceneId = Number(targetSceneId);
  if (!Number.isInteger(sceneId)) {
    return { kind: "error", message: t("galgame.choiceTarget.invalid") };
  }
  const target = sceneById.get(sceneId);
  if (!target) {
    return { kind: "error", message: t("galgame.choiceTarget.missing") };
  }
  if (target.scene.entry_kind !== "branch") {
    return {
      kind: "main",
      message: t("galgame.choiceTarget.main", {
        number: target.scene.scene_index,
        title: target.scene.title,
      }),
    };
  }
  const returnSceneId = target.scene.next_scene_id;
  if (returnSceneId == null) {
    return {
      kind: "error",
      message: t("galgame.choiceTarget.branchNoReturn", {
        number: target.scene.scene_index,
        title: target.scene.title,
      }),
    };
  }
  const returnScene = sceneById.get(returnSceneId);
  if (!returnScene) {
    return {
      kind: "error",
      message: t("galgame.choiceTarget.returnMissing", {
        number: target.scene.scene_index,
      }),
    };
  }
  if (returnScene.scene.entry_kind !== "main") {
    return {
      kind: "error",
      message: t("galgame.choiceTarget.returnNotMain", {
        number: target.scene.scene_index,
        returnNumber: returnScene.scene.scene_index,
      }),
    };
  }
  return {
    kind: "branch",
    message: t("galgame.choiceTarget.branch", {
      number: target.scene.scene_index,
      title: target.scene.title,
      returnNumber: returnScene.scene.scene_index,
      returnTitle: returnScene.scene.title,
    }),
  };
}

export function buildChoiceDrafts(
  choices: GalgameChoice[],
): ChoiceDraft[] {
  if (choices.length > 0) {
    return [...choices]
      .sort((left, right) => left.option_index - right.option_index)
      .map((choice) => ({
        option_text: choice.option_text,
        target_scene_id: choice.target_scene_id?.toString() ?? "",
      }));
  }
  return [
    { option_text: "", target_scene_id: "" },
    { option_text: "", target_scene_id: "" },
  ];
}
