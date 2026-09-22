import React, { useEffect, useMemo, useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { Dropdown } from "../../../components/ui/Dropdown";
import { ThemedCombobox } from "../../../components/ui/ThemedCombobox";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { MediaAssetIdBinder } from "./MediaAssetIdBinder";
import { presentationCgNote, updatePresentationJson, type PresentationPatch } from "./presentation";
import { ChoiceEditor } from "./GalgameChoiceEditor";
import {
  ImageBindingStrip,
  LineMediaBindings,
} from "./GalgameMediaBindings";
import { useI18n } from "../../../i18n";
import type { SourceSnapshotChapterSummary } from "../../../api/project";
import type {
  GalgameCgImportance,
  GalgameChoice,
  GalgameLine,
  GalgameLinePresentationDocumentV2,
  GalgameLineType,
  GalgameMediaBindingSummary,
  GalgameScene,
  GalgameSceneDetail,
  GalgameSpeakerProfile,
  GalgameStagePositionV2,
  GalgameTransition,
  GalgameTextAnchorSummary,
  UpdateGalgameChoiceOptionInput,
} from "../../../api/galgame";
import {
  CG_IMPORTANCE_OPTIONS,
  EXPRESSIONS,
  LINE_TYPES,
  POSE_SUGGESTIONS,
  POSITIONS,
  SPEAKING_LINE_TYPES,
  TRANSITION_OPTIONS,
  poseDisplayValue,
  poseStoredValue,
  sceneSelectLabel,
  selectValue,
} from "./galgameTabShared";

interface LineEditorProps {
  line: GalgameLine;
  speakerProfiles: GalgameSpeakerProfile[];
  speakerProfileById: ReadonlyMap<number, GalgameSpeakerProfile>;
  sourceAnchor: GalgameTextAnchorSummary | null;
  sourceChapter: SourceSnapshotChapterSummary | null;
  mediaBindings: GalgameMediaBindingSummary[];
  choices: GalgameChoice[];
  scenes: GalgameSceneDetail[];
  sceneById: Map<number, GalgameSceneDetail>;
  disabled: boolean;
  saving: boolean;
  onPatch: (patch: Partial<GalgameLine>) => void;
  onChoicesUpdate: (choices: UpdateGalgameChoiceOptionInput[]) => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  canMoveUp: boolean;
  canMoveDown: boolean;
  onInsertAfter: () => void;
  onDelete: () => void;
  onImportAudio: (role: "bgm" | "sfx") => void;
  onBindMedia: (role: "cg" | "bgm" | "sfx", mediaAssetId: number) => void;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}

export const SceneMetaEditor: React.FC<{
  scene: GalgameScene;
  scenes: GalgameSceneDetail[];
  mediaBindings: GalgameMediaBindingSummary[];
  disabled: boolean;
  onPatch: (patch: Partial<GalgameScene>) => void;
  onBindMedia: (mediaAssetId: number) => void;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}> = ({
  scene,
  scenes,
  mediaBindings,
  disabled,
  onPatch,
  onBindMedia,
  onUnbindMedia,
}) => {
  const { t } = useI18n();
  const [draft, setDraft] = useState({
    title: scene.title,
    location: scene.location ?? "",
    time_of_day: scene.time_of_day ?? "",
    background_note: scene.background_note ?? "",
    next_scene_id: scene.next_scene_id?.toString() ?? "",
  });
  const availableNextScenes = useMemo(
    () =>
      scenes.filter(
        (item) =>
          item.scene.id !== scene.id &&
          (scene.entry_kind !== "branch" || item.scene.entry_kind === "main"),
      ),
    [scene.entry_kind, scene.id, scenes],
  );
  const selectedNextSceneSelectable =
    scene.next_scene_id == null ||
    availableNextScenes.some((item) => item.scene.id === scene.next_scene_id);

  useEffect(() => {
    setDraft({
      title: scene.title,
      location: scene.location ?? "",
      time_of_day: scene.time_of_day ?? "",
      background_note: scene.background_note ?? "",
      next_scene_id: scene.next_scene_id?.toString() ?? "",
    });
  }, [
    scene.background_note,
    scene.location,
    scene.next_scene_id,
    scene.time_of_day,
    scene.title,
  ]);

  const commit = (
    key: "title" | "location" | "time_of_day" | "background_note",
    value: string,
  ) => {
    const current =
      key === "title"
        ? scene.title
        : key === "location"
          ? (scene.location ?? "")
          : key === "time_of_day"
            ? (scene.time_of_day ?? "")
            : (scene.background_note ?? "");
    if (value === current) return;
    onPatch({ [key]: key === "title" ? value : value || null });
  };
  const backgroundBindings = mediaBindings.filter(
    (item) => item.binding.role === "background",
  );

  return (
    <div className="grid gap-3 border-b border-book-border bg-book-bg px-4 py-3 text-xs lg:grid-cols-[minmax(0,1fr)_220px]">
      <div className="grid gap-2 md:grid-cols-[1.2fr_0.8fr_0.8fr]">
        <input
          className="pyqt-input text-book-text-main"
          value={draft.title}
          disabled={disabled}
          onChange={(event) =>
            setDraft((current) => ({ ...current, title: event.target.value }))
          }
          onBlur={() => commit("title", draft.title)}
        />
        <input
          className="pyqt-input text-book-text-main"
          value={draft.location}
          disabled={disabled}
          placeholder={t("galgame.location")}
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              location: event.target.value,
            }))
          }
          onBlur={() => commit("location", draft.location)}
        />
        <input
          className="pyqt-input text-book-text-main"
          value={draft.time_of_day}
          disabled={disabled}
          placeholder={t("galgame.time")}
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              time_of_day: event.target.value,
            }))
          }
          onBlur={() => commit("time_of_day", draft.time_of_day)}
        />
        <textarea
          className="pyqt-input min-h-16 text-book-text-main md:col-span-3"
          value={draft.background_note}
          disabled={disabled}
          placeholder={t("galgame.backgroundNeed")}
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              background_note: event.target.value,
            }))
          }
          onBlur={() => commit("background_note", draft.background_note)}
        />
        <ThemedSelect
          className="pyqt-input text-book-text-main md:col-span-3"
          value={draft.next_scene_id}
          disabled={disabled}
          onChange={(value) => {
            setDraft((current) => ({ ...current, next_scene_id: value }));
            onPatch({ next_scene_id: value ? Number(value) : null });
          }}
        >
          <option value="">
            {scene.entry_kind === "branch" ? t("galgame.noConvergence") : t("galgame.noNextScene")}
          </option>
          {!selectedNextSceneSelectable && scene.next_scene_id != null ? (
            <option value={scene.next_scene_id}>
              {scenes.some((item) => item.scene.id === scene.next_scene_id)
                ? t("galgame.nextSceneNotMain")
                : t("galgame.nextSceneMissing")}
            </option>
          ) : null}
          {availableNextScenes.map((item) => (
            <option key={item.scene.id} value={item.scene.id}>
              {sceneSelectLabel(item.scene, t)}
            </option>
          ))}
        </ThemedSelect>
      </div>
      <details className="rounded-xl border border-book-border/40 bg-book-bg-paper/45 p-2.5">
        <summary className="cursor-pointer text-small font-medium text-book-text-main">
          {t("galgame.background")} ·{" "}
          {backgroundBindings.length
            ? t("galgame.bindingCount", { count: backgroundBindings.length })
            : t("galgame.unbound")}
        </summary>
        <div className="mt-2">
          <ImageBindingStrip
            title={t("galgame.background")}
            emptyLabel=""
            bindings={backgroundBindings}
            disabled={disabled}
            bindControl={
              <MediaAssetIdBinder
                label={t("galgame.bindBackground")}
                disabled={disabled}
                onBind={onBindMedia}
              />
            }
            onUnbindMedia={onUnbindMedia}
          />
        </div>
      </details>
    </div>
  );
};

export const LineEditor: React.FC<LineEditorProps> = ({
  line,
  speakerProfiles,
  speakerProfileById,
  sourceAnchor,
  sourceChapter,
  mediaBindings,
  choices,
  scenes,
  sceneById,
  disabled,
  saving,
  onPatch,
  onChoicesUpdate,
  onMoveUp,
  onMoveDown,
  canMoveUp,
  canMoveDown,
  onInsertAfter,
  onDelete,
  onImportAudio,
  onBindMedia,
  onUnbindMedia,
}) => {
  const { t } = useI18n();
  const [text, setText] = useState(line.text);
  const [pose, setPose] = useState(line.pose ?? "");
  const isSpeakingLine = SPEAKING_LINE_TYPES.has(line.line_type);
  const isChoiceLine = line.line_type === "choice";
  const expressionValue = selectValue(line.expression, EXPRESSIONS);
  const missingSpeakingExpression =
    isSpeakingLine && expressionValue.length === 0;
  const speaker = useMemo(
    () =>
      line.speaker_profile_id == null
        ? null
        : (speakerProfileById.get(line.speaker_profile_id) ?? null),
    [line.speaker_profile_id, speakerProfileById],
  );

  useEffect(() => {
    setText(line.text);
    setPose(line.pose ?? "");
  }, [line.pose, line.text]);

  const firstSpeakerId = speakerProfiles[0]?.id ?? null;

  return (
    <article className="py-4 first:pt-0 last:pb-0">
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <span className="min-w-8 text-small font-semibold text-book-text-muted">
          {t("galgame.lineNumber", { number: line.line_index })}
        </span>
        <ThemedSelect
          className="pyqt-input text-xs text-book-text-main"
          value={line.line_type}
          disabled={disabled}
          onChange={(selectedValue) => {
            const nextType = selectedValue as GalgameLineType;
            onPatch({
              line_type: nextType,
              speaker_profile_id: SPEAKING_LINE_TYPES.has(nextType)
                ? (line.speaker_profile_id ?? firstSpeakerId)
                : null,
              expression: SPEAKING_LINE_TYPES.has(nextType)
                ? expressionValue || "neutral"
                : null,
            });
          }}
        >
          {LINE_TYPES.map((item) => (
            <option
              key={item.value}
              value={item.value}
              disabled={
                SPEAKING_LINE_TYPES.has(item.value) &&
                speakerProfiles.length === 0
              }
            >
              {t(item.labelKey)}
            </option>
          ))}
        </ThemedSelect>
        {isSpeakingLine ? (
          <ThemedSelect
            className="pyqt-input text-xs text-book-text-main"
            value={line.speaker_profile_id ?? ""}
            disabled={disabled || speakerProfiles.length === 0}
            onChange={(selectedValue) =>
              onPatch({
                speaker_profile_id: selectedValue
                  ? Number(selectedValue)
                  : null,
                expression: selectedValue
                  ? expressionValue || "neutral"
                  : line.expression,
              })
            }
          >
            <option value="">
              {speakerProfiles.length === 0
                ? t("galgame.noSpeakerProfiles")
                : t("galgame.unboundSpeaker")}
            </option>
            {speakerProfiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name}
              </option>
            ))}
          </ThemedSelect>
        ) : null}
        {!isChoiceLine ? (
          <ThemedSelect
            className="pyqt-input text-xs text-book-text-main"
            value={line.position ?? ""}
            disabled={disabled}
            onChange={(selectedValue) => {
              const position = selectedValue as GalgameStagePositionV2 | "";
              onPatch({ position: position || null });
            }}
          >
            {POSITIONS.map((item) => (
              <option key={item.value} value={item.value}>
                {t(item.labelKey)}
              </option>
            ))}
          </ThemedSelect>
        ) : null}
        {isSpeakingLine ? (
          <span
            className={`max-w-[180px] truncate text-small ${
              speaker && !missingSpeakingExpression
                ? "text-book-text-muted"
                : "text-error"
            }`}
            title={
              speaker
                ? missingSpeakingExpression
                  ? t("galgame.expressionRequired")
                  : speaker.name
                : t("galgame.unboundSpeaker")
            }
          >
            {speaker
              ? missingSpeakingExpression
                ? t("galgame.expressionUnset")
                : t("galgame.speakerLabel", { name: speaker.name })
              : t("galgame.lineUnbound")}
          </span>
        ) : null}
        {line.text_anchor_id ? (
          <span
            className={`max-w-[220px] truncate text-small ${
              sourceAnchor ? "text-book-text-muted" : "text-error"
            }`}
            title={
              sourceAnchor
                ? `${sourceChapter
                    ? t("novel.chapterRangeSingle", {
                        number: sourceChapter.chapter_number,
                      })
                    : t("galgame.sourceChapter")} · P${
                    sourceAnchor.paragraph_index + 1
                  } · ${sourceAnchor.quote}`
                : t("galgame.sourceAnchorMissing")
            }
          >
            {sourceAnchor
              ? `${sourceChapter
                  ? t("novel.chapterRangeSingle", {
                      number: sourceChapter.chapter_number,
                    })
                  : t("galgame.sourceSnapshot")} · ${
                  sourceAnchor.quote
                }`
              : t("galgame.sourceAnchorUnavailable")}
          </span>
        ) : null}
        <div className="ml-auto flex items-center gap-1">
          {saving ? (
            <LoadingSpinner size="sm" className="text-book-text-muted" />
          ) : null}
          <Dropdown
            items={[
              {
                label: t("galgame.moveUp"),
                onClick: onMoveUp,
                disabled: disabled || !canMoveUp,
              },
              {
                label: t("galgame.moveDown"),
                onClick: onMoveDown,
                disabled: disabled || !canMoveDown,
              },
              {
                label: t("galgame.insertNarration"),
                icon: <Plus size={13} />,
                onClick: onInsertAfter,
                disabled,
              },
              { type: "divider" },
              {
                label: t("galgame.deleteLine"),
                icon: <Trash2 size={13} />,
                onClick: onDelete,
                disabled,
                danger: true,
              },
            ]}
          />
        </div>
      </div>
      <textarea
        className="pyqt-input min-h-20 w-full resize-y px-3 py-2 text-xs leading-relaxed text-book-text-main"
        value={text}
        disabled={disabled}
        onChange={(event) => setText(event.target.value)}
        onBlur={() => {
          if (text !== line.text) onPatch({ text });
        }}
      />
      {!isChoiceLine ? (
        <div className="mt-2 grid gap-2 md:grid-cols-2">
          <ThemedSelect
            className="pyqt-input text-xs text-book-text-main"
            value={expressionValue}
            disabled={disabled}
            onChange={(selectedValue) =>
              onPatch({ expression: selectedValue || null })
            }
          >
            {EXPRESSIONS.map((item) => (
              <option
                key={item.value || "none"}
                value={item.value}
                disabled={isSpeakingLine && item.value === ""}
              >
                {t(item.labelKey)}
              </option>
            ))}
          </ThemedSelect>
          <ThemedCombobox
            value={poseDisplayValue(pose, t)}
            options={POSE_SUGGESTIONS.map((item) => ({
              value: t(item.labelKey),
              label: t(item.labelKey),
            }))}
            aria-label={t("galgame.posePlaceholder")}
            disabled={disabled}
            placeholder={t("galgame.posePlaceholder")}
            onChange={setPose}
            onBlur={() => {
              const storedPose = poseStoredValue(pose, t);
              if (storedPose !== (line.pose ?? ""))
                onPatch({ pose: storedPose || null });
            }}
          />
          <PresentationEditor
            line={line}
            disabled={disabled}
            onPatch={(presentationJson) =>
              onPatch({ presentation_json: presentationJson })
            }
          />
        </div>
      ) : (
        <ChoiceEditor
          line={line}
          choices={choices}
          scenes={scenes}
          sceneById={sceneById}
          disabled={disabled}
          onUpdate={onChoicesUpdate}
        />
      )}
      <LineMediaBindings
        bindings={mediaBindings}
        disabled={disabled}
        onImportAudio={onImportAudio}
        onBindMedia={onBindMedia}
        onUnbindMedia={onUnbindMedia}
      />
    </article>
  );
};

const PresentationEditor: React.FC<{
  line: GalgameLine;
  disabled: boolean;
  onPatch: (presentationJson: GalgameLinePresentationDocumentV2) => void;
}> = ({ line, disabled, onPatch }) => {
  const { t } = useI18n();
  const presentation = line.presentation_json;
  const currentCgEnabled = presentation.cg === true;
  const currentCgImportance: GalgameCgImportance | "" =
    presentation.cg_importance ?? "";
  const currentTransition: GalgameTransition | "" =
    presentation.transition || "";
  const [cgNote, setCgNote] = useState(presentationCgNote(presentation));
  const [cgEnabled, setCgEnabled] = useState(currentCgEnabled);
  const [cgImportance, setCgImportance] =
    useState<GalgameCgImportance | "">(currentCgImportance);
  const [transition, setTransition] =
    useState<GalgameTransition | "">(currentTransition);

  useEffect(() => {
    setCgNote(presentationCgNote(presentation));
    setCgEnabled(currentCgEnabled);
    setCgImportance(currentCgImportance);
    setTransition(currentTransition);
  }, [currentCgEnabled, currentCgImportance, currentTransition, presentation]);

  const update = (patch: PresentationPatch) => {
    onPatch(updatePresentationJson(presentation, patch));
  };

  return (
    <div className="rounded-xl bg-book-bg-paper/45 px-2.5 py-2.5 md:col-span-2">
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <button
          type="button"
          role="switch"
          aria-checked={cgEnabled}
          disabled={disabled}
          onClick={() => {
            const checked = !cgEnabled;
            setCgEnabled(checked);
            if (!checked) {
              setCgNote("");
              setCgImportance("");
              update({ cgEnabled: false });
              return;
            }
            if (cgNote.trim() && cgImportance) {
              update({ cgNote, cgImportance });
            }
          }}
          className="inline-flex items-center gap-1.5 text-small text-book-text-main disabled:opacity-50"
        >
          <span
            className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
              cgEnabled ? "bg-book-primary" : "bg-book-text-muted/25"
            }`}
          >
            <span
              className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
                cgEnabled ? "translate-x-4" : "translate-x-[2px]"
              }`}
            />
          </span>
          CG
        </button>
        {cgEnabled ? (
          <ThemedSelect
            className="pyqt-input text-xs text-book-text-main"
            value={cgImportance}
            disabled={disabled}
            onChange={(selectedValue) => {
              const importance = selectedValue as GalgameCgImportance;
              setCgImportance(importance);
              if (cgNote.trim()) update({ cgNote, cgImportance: importance });
            }}
          >
            <option value="" disabled>
              {t("galgame.selectCgImportance")}
            </option>
            {CG_IMPORTANCE_OPTIONS.map((item) => (
              <option key={item.value} value={item.value}>
                {t(item.labelKey)}
              </option>
            ))}
          </ThemedSelect>
        ) : null}
        <ThemedSelect
          className="pyqt-input text-xs text-book-text-main"
          value={transition}
          disabled={disabled}
          onChange={(selectedValue) => {
            const nextTransition = selectedValue as GalgameTransition | "";
            setTransition(nextTransition);
            update({ transition: nextTransition });
          }}
        >
          {TRANSITION_OPTIONS.map((item) => (
            <option key={item.value || "none"} value={item.value}>
              {t(item.labelKey)}
            </option>
          ))}
        </ThemedSelect>
      </div>
      {cgEnabled ? (
        <textarea
          className="min-h-16 w-full resize-y rounded-xl border border-book-border/55 bg-book-bg px-2.5 py-2 text-xs leading-relaxed text-book-text-main outline-none focus:border-book-primary"
          value={cgNote}
          disabled={disabled}
          placeholder={t("galgame.cgDescriptionPlaceholder")}
          onChange={(event) => setCgNote(event.target.value)}
          onBlur={() => {
            if (cgNote !== presentationCgNote(presentation)) {
              if (cgNote.trim()) {
                setCgEnabled(true);
                if (cgImportance) update({ cgNote, cgImportance });
              } else {
                update({ cgNote: "" });
              }
            }
          }}
        />
      ) : null}
      {cgEnabled && cgNote.trim() && !cgImportance ? (
        <div className="mt-1 text-small text-error">
          {t("galgame.requireCgImportance")}
        </div>
      ) : null}
    </div>
  );
};
