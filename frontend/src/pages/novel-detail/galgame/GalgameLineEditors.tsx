import React, { useEffect, useMemo, useState } from "react";
import { ChevronRight, Plus, Trash2 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { Dropdown } from "../../../components/ui/Dropdown";
import { ThemedCombobox } from "../../../components/ui/ThemedCombobox";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { BookInput, BookTextarea } from "../../../components/ui/BookInput";
import { MediaAssetIdBinder } from "./MediaAssetIdBinder";
import { presentationCgNote, updatePresentationJson, type CgDraft } from "./presentation";
import { ChoiceEditor, type ChoiceDraft } from "./GalgameChoiceEditor";
import {
  ImageBindingStrip,
  LineMediaBindings,
} from "./GalgameMediaBindings";
import { useI18n } from "../../../i18n";
import type {
  GalgameFieldDraft,
  GalgameSceneFieldValues,
} from "./GalgameDrafts";
import type { SourceSnapshotChapterIndexEntry } from "../../../api/project";
import type {
  GalgameCgImportance,
  GalgameLine,
  GalgameLinePresentationDocumentV3,
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
  sourceChapter: SourceSnapshotChapterIndexEntry | null;
  mediaBindings: GalgameMediaBindingSummary[];
  choiceDrafts: ChoiceDraft[];
  onChoiceDraftsChange: (drafts: ChoiceDraft[]) => void;
  cgDraft: CgDraft;
  onCgDraftChange: (draft: CgDraft) => void;
  onCgSave: (draft: CgDraft) => void;
  scenes: GalgameSceneDetail[];
  sceneById: Map<number, GalgameSceneDetail>;
  disabled: boolean;
  saving: boolean;
  fieldDraftResetRevision: number;
  fieldDraft: GalgameFieldDraft | null;
  mediaAssetDrafts: Record<string, string>;
  onMediaAssetDraftChange: (key: string, value: string) => void;
  onPatch: (patch: Partial<GalgameLine>) => void;
  onDraftChange: (key: string, draft: GalgameFieldDraft | null) => void;
  onChoicesUpdate: (choices: UpdateGalgameChoiceOptionInput[]) => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  canMoveUp: boolean;
  canMoveDown: boolean;
  onInsertAfter: () => void;
  onDelete: () => void;
  onImportAudio: (role: "bgm" | "sfx") => void;
  onBindMedia: (role: "cg" | "bgm" | "sfx", mediaAssetId: number) => Promise<boolean>;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}

type SceneMetaDraft = GalgameSceneFieldValues;

const sameSceneMetaDraft = (left: SceneMetaDraft, right: SceneMetaDraft) =>
  left.title === right.title &&
  left.location === right.location &&
  left.time_of_day === right.time_of_day &&
  left.background_note === right.background_note &&
  left.next_scene_id === right.next_scene_id;

const createSceneMetaDraft = (scene: GalgameScene): SceneMetaDraft => ({
  title: scene.title,
  location: scene.location ?? "",
  time_of_day: scene.time_of_day ?? "",
  background_note: scene.background_note ?? "",
  next_scene_id: scene.next_scene_id?.toString() ?? "",
});

export const SceneMetaEditor: React.FC<{
  scene: GalgameScene;
  scenes: GalgameSceneDetail[];
  mediaBindings: GalgameMediaBindingSummary[];
  disabled: boolean;
  fieldDraftResetRevision: number;
  fieldDraft: GalgameFieldDraft | null;
  mediaAssetIdDraft: string;
  onMediaAssetIdDraftChange: (value: string) => void;
  onPatch: (patch: Partial<GalgameScene>) => void;
  onDraftChange: (key: string, draft: GalgameFieldDraft | null) => void;
  onBindMedia: (mediaAssetId: number) => Promise<boolean>;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}> = ({
  scene,
  scenes,
  mediaBindings,
  disabled,
  fieldDraftResetRevision,
  fieldDraft,
  mediaAssetIdDraft,
  onMediaAssetIdDraftChange,
  onPatch,
  onDraftChange,
  onBindMedia,
  onUnbindMedia,
}) => {
  const { t } = useI18n();
  const draftKey = `scene:${scene.id}`;
  const savedDraft = fieldDraft?.kind === "scene" ? fieldDraft.value : null;
  const [draft, setDraft] = useState(() => savedDraft ?? createSceneMetaDraft(scene));
  const updateDraft = (patch: Partial<SceneMetaDraft>) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    onDraftChange(
      draftKey,
      sameSceneMetaDraft(next, createSceneMetaDraft(scene))
        ? null
        : { kind: "scene", value: next },
    );
  };
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
    const next = fieldDraft?.kind === "scene" ? fieldDraft.value : createSceneMetaDraft(scene);
    setDraft((current) => sameSceneMetaDraft(current, next) ? current : next);
  }, [
    scene.background_note,
    scene.id,
    scene.location,
    scene.next_scene_id,
    scene.time_of_day,
    scene.title,
    fieldDraftResetRevision,
    fieldDraft,
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
      <div className="grid grid-cols-2 content-start items-start gap-2 md:grid-cols-[1.2fr_0.8fr_0.8fr]">
        <div className="col-span-2 md:col-span-1">
        <BookInput
          label={t("galgame.sceneTitle")}
          className="pyqt-input text-book-text-main"
          value={draft.title}
          disabled={disabled}
          onChange={(event) => updateDraft({ title: event.target.value })}
          onBlur={() => commit("title", draft.title)}
        />
        </div>
        <BookInput
          label={t("galgame.location")}
          className="pyqt-input text-book-text-main"
          value={draft.location}
          disabled={disabled}
          placeholder={t("galgame.location")}
          onChange={(event) => updateDraft({ location: event.target.value })}
          onBlur={() => commit("location", draft.location)}
        />
        <BookInput
          label={t("galgame.time")}
          className="pyqt-input text-book-text-main"
          value={draft.time_of_day}
          disabled={disabled}
          placeholder={t("galgame.time")}
          onChange={(event) => updateDraft({ time_of_day: event.target.value })}
          onBlur={() => commit("time_of_day", draft.time_of_day)}
        />
        <div className="col-span-2 md:col-span-3">
        <BookTextarea
          label={t("galgame.backgroundNeed")}
          className="min-h-16 text-book-text-main"
          value={draft.background_note}
          disabled={disabled}
          placeholder={t("galgame.backgroundNeed")}
          onChange={(event) => updateDraft({ background_note: event.target.value })}
          onBlur={() => commit("background_note", draft.background_note)}
        />
        </div>
        <div className="col-span-2 md:col-span-3">
        <div className="mb-1.5 text-xs font-semibold text-book-text-sub">{t("galgame.nextScene")}</div>
        <ThemedSelect
          aria-label={t("galgame.nextScene")}
          className="pyqt-input text-book-text-main"
          value={draft.next_scene_id}
          disabled={disabled}
          onChange={(value) => {
            updateDraft({ next_scene_id: value });
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
      </div>
      <details className="self-start rounded-xl border border-book-border/40 bg-book-bg-paper/45 p-2.5">
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
                value={mediaAssetIdDraft}
                onChange={onMediaAssetIdDraftChange}
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
  choiceDrafts,
  onChoiceDraftsChange,
  cgDraft,
  onCgDraftChange,
  onCgSave,
  scenes,
  sceneById,
  disabled,
  saving,
  fieldDraftResetRevision,
  fieldDraft,
  mediaAssetDrafts,
  onMediaAssetDraftChange,
  onPatch,
  onDraftChange,
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
  const draftKey = `line:${line.id}`;
  const savedDraft = fieldDraft?.kind === "line" ? fieldDraft.value : null;
  const [text, setText] = useState(savedDraft?.text ?? line.text);
  const [pose, setPose] = useState(savedDraft?.pose ?? line.pose ?? "");
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

  const reportDraftChange = (nextText: string, nextPose: string) => {
    const dirty = nextText !== line.text || poseStoredValue(nextPose, t) !== (line.pose ?? "");
    onDraftChange(
      draftKey,
      dirty ? { kind: "line", value: { text: nextText, pose: nextPose } } : null,
    );
  };

  useEffect(() => {
    const nextText = fieldDraft?.kind === "line" ? fieldDraft.value.text : line.text;
    const nextPose = fieldDraft?.kind === "line" ? fieldDraft.value.pose : line.pose ?? "";
    setText((current) => current === nextText ? current : nextText);
    setPose((current) => current === nextPose ? current : nextPose);
  }, [fieldDraftResetRevision, fieldDraft, line.id, line.pose, line.text]);

  const firstSpeakerId = speakerProfiles[0]?.id ?? null;
  const expressionLabel = EXPRESSIONS.find((item) => item.value === expressionValue);
  const transitionLabel = TRANSITION_OPTIONS.find((item) => item.value === line.presentation_json.transition);

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
        {isSpeakingLine ? (
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
              <option key={item.value} value={item.value} disabled={item.value === ""}>
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
        aria-label={`${t("galgame.lineNumber", { number: line.line_index })} · ${t("galgame.lineText")}`}
        className="pyqt-input min-h-20 w-full resize-y px-3 py-2 text-xs leading-relaxed text-book-text-main"
        value={text}
        disabled={disabled}
        onChange={(event) => {
          setText(event.target.value);
          reportDraftChange(event.target.value, pose);
        }}
        onBlur={() => {
          if (text !== line.text) onPatch({ text });
          else reportDraftChange(text, pose);
        }}
      />
      {!isChoiceLine ? (
        <details className="group/performance mt-2" open={line.presentation_json.cg === true}>
          <summary className="flex cursor-pointer list-none items-center gap-2 py-2 text-small text-book-text-sub [&::-webkit-details-marker]:hidden">
            <ChevronRight size={13} className="shrink-0 transition-transform group-open/performance:rotate-90" />
            <span>{t("galgame.performance")}</span>
            <span className="min-w-0 flex-1 truncate text-book-text-muted">
              {[expressionValue && expressionLabel ? t(expressionLabel.labelKey) : null,
                pose ? poseDisplayValue(pose, t) : null,
                line.presentation_json.cg ? "CG" : null,
                transitionLabel ? t(transitionLabel.labelKey) : line.presentation_json.transition,
              ].filter(Boolean).join(" · ")}
            </span>
          </summary>
          <div className="grid gap-2 md:grid-cols-2">
          {isSpeakingLine ? <>
          <div>
          <div className="mb-1.5 text-xs text-book-text-sub">{t("galgame.expressionLabel")}</div>
          <ThemedSelect
            aria-label={t("galgame.expressionLabel")}
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
          </div>
          <div>
          <div className="mb-1.5 text-xs text-book-text-sub">{t("galgame.posePlaceholder")}</div>
          <ThemedCombobox
            value={poseDisplayValue(pose, t)}
            options={POSE_SUGGESTIONS.map((item) => ({
              value: t(item.labelKey),
              label: t(item.labelKey),
            }))}
            aria-label={t("galgame.posePlaceholder")}
            disabled={disabled}
            placeholder={t("galgame.posePlaceholder")}
            onChange={(value) => {
              setPose(value);
              reportDraftChange(text, value);
            }}
            onBlur={() => {
              const storedPose = poseStoredValue(pose, t);
              if (storedPose !== (line.pose ?? ""))
                onPatch({ pose: storedPose || null });
              else reportDraftChange(text, pose);
            }}
          />
          </div>
          </> : null}
          <PresentationEditor
            line={line}
            draft={cgDraft}
            onDraftChange={onCgDraftChange}
            onSave={onCgSave}
            disabled={disabled}
            onPatch={(presentationJson) =>
              onPatch({ presentation_json: presentationJson })
            }
          />
          </div>
        </details>
      ) : (
        <ChoiceEditor
          line={line}
          drafts={choiceDrafts}
          onDraftsChange={onChoiceDraftsChange}
          scenes={scenes}
          sceneById={sceneById}
          disabled={disabled}
          onUpdate={onChoicesUpdate}
        />
      )}
      <LineMediaBindings
        lineId={line.id}
        bindings={mediaBindings}
        disabled={disabled}
        onImportAudio={onImportAudio}
        onBindMedia={onBindMedia}
        onUnbindMedia={onUnbindMedia}
        mediaAssetDrafts={mediaAssetDrafts}
        onMediaAssetDraftChange={onMediaAssetDraftChange}
      />
    </article>
  );
};

const PresentationEditor: React.FC<{
  line: GalgameLine;
  draft: CgDraft;
  onDraftChange: (draft: CgDraft) => void;
  onSave: (draft: CgDraft) => void;
  disabled: boolean;
  onPatch: (presentationJson: GalgameLinePresentationDocumentV3) => void;
}> = ({ line, draft, onDraftChange, onSave, disabled, onPatch }) => {
  const { t } = useI18n();
  const presentation = line.presentation_json;
  const { cgEnabled, cgNote, cgImportance } = draft;
  const currentTransition: GalgameTransition | "" =
    presentation.transition || "";

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
            const next = checked
              ? { ...draft, cgEnabled: true }
              : { cgEnabled: false, cgNote: "", cgImportance: "" as const };
            onDraftChange(next);
            onSave(next);
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
            aria-label={t("galgame.selectCgImportance")}
            className="pyqt-input text-xs text-book-text-main"
            value={cgImportance}
            disabled={disabled}
            onChange={(selectedValue) => {
              const importance = selectedValue as GalgameCgImportance;
              const next = { ...draft, cgImportance: importance };
              onDraftChange(next);
              onSave(next);
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
          aria-label={t("galgame.transitionLabel")}
          className="pyqt-input text-xs text-book-text-main"
          value={currentTransition}
          disabled={disabled || presentation.cg !== true}
          onChange={(selectedValue) => {
            const nextTransition = selectedValue as GalgameTransition | "";
            onPatch(updatePresentationJson(presentation, { transition: nextTransition }));
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
          aria-label={t("galgame.cgDescriptionPlaceholder")}
          className="min-h-16 w-full resize-y rounded-xl border border-book-border/55 bg-book-bg px-2.5 py-2 text-xs leading-relaxed text-book-text-main outline-none focus:border-book-primary"
          value={cgNote}
          disabled={disabled}
          placeholder={t("galgame.cgDescriptionPlaceholder")}
          onChange={(event) => onDraftChange({ ...draft, cgNote: event.target.value })}
          onBlur={() => {
            if (cgNote !== presentationCgNote(presentation)) {
              onSave(draft);
            }
          }}
        />
      ) : null}
      {cgEnabled && !cgNote.trim() ? (
        <div className="mt-1 text-small text-error">
          {t("galgame.requireCgDescription")}
        </div>
      ) : cgEnabled && !cgImportance ? (
        <div className="mt-1 text-small text-error">
          {t("galgame.requireCgImportance")}
        </div>
      ) : null}
    </div>
  );
};
