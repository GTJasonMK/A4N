import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useImperativeHandle, useMemo, useState } from "react";
import { Check, ChevronDown, Plus, Save, Trash2, Wand2 } from "lucide-react";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { BookButton } from "../../../components/ui/BookButton";
import { ImageZoomButton } from "../../../components/ui/ImageLightbox";
import { Modal } from "../../../components/ui/Modal";
import type { GeneratedImage } from "../../../api/image";
import type {
  ChapterVideoPrompt,
  GeneratedVideo,
  InsertedSegmentGenerationOptions,
  PromptOutputLanguage,
  VideoTimelineItemSummary,
  VideoTimelineSummary,
} from "../../../api/video";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { MAX_REFERENCE_IMAGES, VIDEO_ASPECT_RATIOS, type VideoAspectRatio } from "./videoTabConstants";
import { useI18n, type MessageTranslator } from "../../../i18n";

export interface InsertedSegmentGap {
  previous_item_id: number | null;
  next_item_id: number | null;
}

interface InsertedSegmentDraftForm {
  gap: InsertedSegmentGap;
  user_prompt: string;
  style: string;
  prompt_output_language: PromptOutputLanguage;
  duration_secs: number;
  duration_secs_options: number[];
  aspect_ratio: string;
  reference_image_ids: number[];
  allow_without_visual_anchor: boolean;
}

interface InsertedPromptEdit {
  visual_prompt: string;
  motion_prompt: string;
  negative_prompt: string;
}

export interface InsertedSegmentTimelinePanelProps {
  timeline: VideoTimelineSummary | null;
  prompts: ChapterVideoPrompt[];
  videos: GeneratedVideo[];
  referenceImages: GeneratedImage[];
  defaultStyle: string;
  defaultPromptOutputLanguage: PromptOutputLanguage;
  defaultDurationSecs: number;
  defaultDurationOptions: number[];
  defaultAspectRatio: string;
  loading: boolean;
  acting: string | null;
  onGenerate: (
    options: InsertedSegmentGenerationOptions,
  ) => Promise<number | null>;
  onCommit: (
    timelineId: number,
    expectedRevision: number,
    workflowRunId: number,
  ) => Promise<boolean>;
  onMove: (input: {
    timeline_id: number;
    expected_timeline_revision: number;
    timeline_item_id: number;
    previous_item_id: number | null;
    next_item_id: number | null;
  }) => Promise<void>;
  onDelete: (input: {
    timeline_id: number;
    expected_timeline_revision: number;
    timeline_item_id: number;
  }) => Promise<void>;
  onGenerateVideo: (prompt: ChapterVideoPrompt) => Promise<void>;
  onUpdatePrompt: (
    prompt: ChapterVideoPrompt,
    input: {
      visual_prompt: string;
      motion_prompt: string;
      negative_prompt: string;
    },
  ) => Promise<void>;
  onPreviewVideo: (video: GeneratedVideo) => Promise<void>;
  onDeleteVideo: (video: GeneratedVideo) => Promise<void>;
}

export interface InsertedSegmentTimelinePanelHandle {
  openCreate: (gap: InsertedSegmentGap) => void;
  openEdit: (timelineItemId: number) => void;
}

export const insertedSegmentGapKey = (gap: InsertedSegmentGap): string =>
  `${gap.previous_item_id == null ? "head" : gap.previous_item_id}:${
    gap.next_item_id == null ? "tail" : gap.next_item_id
  }`;

const timelineItemsById = (
  items: ReadonlyArray<VideoTimelineItemSummary>,
): Map<number, VideoTimelineItemSummary> => {
  const index = new Map<number, VideoTimelineItemSummary>();
  for (const item of items) {
    if (!index.has(item.id)) index.set(item.id, item);
  }
  return index;
};

const gapLabelFromIndex = (
  gap: InsertedSegmentGap,
  itemsById: ReadonlyMap<number, VideoTimelineItemSummary>,
  t: MessageTranslator,
): string => {
  const previous =
    gap.previous_item_id == null ? null : (itemsById.get(gap.previous_item_id) ?? null);
  const next = gap.next_item_id == null ? null : (itemsById.get(gap.next_item_id) ?? null);
  const segment = (number: number) => t("video.chapter.segment", { number });
  return `${previous?.title ?? (previous ? segment(previous.display_index) : t("video.chapter.assemblyStart"))} → ${next?.title ?? (next ? segment(next.display_index) : t("video.chapter.assemblyEnd"))}`;
};

export const gapLabel = (
  gap: InsertedSegmentGap,
  items: VideoTimelineItemSummary[],
  t: MessageTranslator,
): string => gapLabelFromIndex(gap, timelineItemsById(items), t);

export const timelineGaps = (
  items: ReadonlyArray<Pick<VideoTimelineItemSummary, "id">>,
): InsertedSegmentGap[] => {
  const gaps: InsertedSegmentGap[] = [];
  for (let index = 0; index <= items.length; index += 1) {
    gaps.push({
      previous_item_id: index === 0 ? null : items[index - 1].id,
      next_item_id: index === items.length ? null : items[index].id,
    });
  }
  return gaps;
};

export const timelineGapsForMove = (
  items: ReadonlyArray<Pick<VideoTimelineItemSummary, "id">>,
  movingItemId: number,
): InsertedSegmentGap[] =>
  timelineGaps(items.filter((item) => item.id !== movingItemId));

export const buildInsertedSegmentGenerationOptions = (
  timelineId: number,
  revision: number,
  form: InsertedSegmentDraftForm,
): InsertedSegmentGenerationOptions => {
  const { gap, ...options } = form;
  return {
    timeline_id: timelineId,
    expected_timeline_revision: revision,
    previous_item_id: gap.previous_item_id,
    next_item_id: gap.next_item_id,
    ...options,
  };
};

export const InsertedSegmentTimelinePanel = React.forwardRef<
  InsertedSegmentTimelinePanelHandle,
  InsertedSegmentTimelinePanelProps
>(
  (
    {
      timeline,
      prompts,
      videos,
      referenceImages,
      defaultStyle,
      defaultPromptOutputLanguage,
      defaultDurationSecs,
      defaultDurationOptions,
      defaultAspectRatio,
      loading,
      acting,
      onGenerate,
      onCommit,
      onMove,
      onDelete,
      onGenerateVideo,
      onUpdatePrompt,
      onPreviewVideo,
      onDeleteVideo,
    },
    ref,
  ) => {
    const { t } = useI18n();
    const items = timeline?.items ?? [];
    const itemsById = useMemo(() => timelineItemsById(items), [items]);
    const reanchorItemIds = new Set(
      items
        .filter((item) => item.status === "needs_reanchor")
        .map((item) => item.id),
    );
    const gapUsesReanchorItem = (gap: InsertedSegmentGap): boolean =>
      (gap.previous_item_id != null &&
        reanchorItemIds.has(gap.previous_item_id)) ||
      (gap.next_item_id != null && reanchorItemIds.has(gap.next_item_id));
    const [form, setForm] = useState<InsertedSegmentDraftForm | null>(null);
    const [draft, setDraft] = useState<{
      workflowRunId: number;
      revision: number;
    } | null>(null);
    const [editingItemId, setEditingItemId] = useState<number | null>(null);
    const [edit, setEdit] = useState<InsertedPromptEdit | null>(null);
    const editingItem =
      editingItemId == null ? null : (itemsById.get(editingItemId) ?? null);
    const editingPrompt =
      editingItem?.plan_prompt_id == null
        ? null
        : (prompts.find((prompt) => prompt.id === editingItem.plan_prompt_id) ??
          null);
    const editingVideo =
      editingItem?.generated_video_id == null
        ? null
        : (videos.find(
            (video) => video.id === editingItem.generated_video_id,
          ) ?? null);
    const editingItemNeedsReanchor = editingItem?.status === "needs_reanchor";
    const editingMoveGaps = editingItem
      ? timelineGapsForMove(items, editingItem.id)
      : [];
    const editingPromptDirty = Boolean(
      edit &&
      editingPrompt &&
      (edit.visual_prompt.trim() !== editingPrompt.visual_prompt.trim() ||
        edit.motion_prompt.trim() !==
          (editingPrompt.motion_prompt ?? "").trim() ||
        edit.negative_prompt.trim() !==
          (editingPrompt.negative_prompt ?? "").trim()),
    );

    const openCreate = (nextGap: InsertedSegmentGap) => {
      if (
        loading ||
        acting != null
      ) {
        return;
      }
      const durationOptions = Array.from(
        new Set([...defaultDurationOptions, defaultDurationSecs]),
      ).sort((left, right) => left - right);
      setDraft(null);
      setForm({
        gap: nextGap,
        user_prompt: "",
        style: defaultStyle,
        prompt_output_language: defaultPromptOutputLanguage,
        duration_secs: defaultDurationSecs,
        duration_secs_options: durationOptions,
        aspect_ratio: defaultAspectRatio,
        reference_image_ids: [],
        allow_without_visual_anchor: false,
      });
    };

    const resetModalState = () => {
      setForm(null);
      setDraft(null);
      setEditingItemId(null);
      setEdit(null);
    };

    const closeModal = async () => {
      if (acting != null) return;
      const formDirty = Boolean(
        form && (
          form.user_prompt.trim() ||
          form.style !== defaultStyle ||
          form.prompt_output_language !== defaultPromptOutputLanguage ||
          form.duration_secs !== defaultDurationSecs ||
          form.aspect_ratio !== defaultAspectRatio ||
          form.reference_image_ids.length > 0 ||
          form.allow_without_visual_anchor
        ),
      );
      if (formDirty || draft != null || editingPromptDirty) {
        if (!(await confirmDialog({
          title: t("common.unsavedChangesTitle"),
          message: t("common.unsavedChangesMessage", { name: t("video.chapter.insertSegment"), action: t("common.close") }),
          confirmText: t("common.discardDraft"),
          dialogType: "warning",
        }))) return;
      }
      resetModalState();
    };

    const openEdit = (item: VideoTimelineItemSummary) => {
      const prompt =
        item.plan_prompt_id == null
          ? null
          : (prompts.find(
              (candidate) => candidate.id === item.plan_prompt_id,
            ) ?? null);
      if (!prompt) return;
      setEditingItemId(item.id);
      setEdit({
        visual_prompt: prompt.visual_prompt,
        motion_prompt: prompt.motion_prompt ?? "",
        negative_prompt: prompt.negative_prompt ?? "",
      });
    };

    useImperativeHandle(ref, () => ({
      openCreate,
      openEdit: (timelineItemId) => {
        const item = items.find((candidate) => candidate.id === timelineItemId);
        if (item?.kind === "inserted") openEdit(item);
      },
    }));

    const updateForm = (patch: Partial<InsertedSegmentDraftForm>) => {
      setDraft(null);
      setForm((current) => (current ? { ...current, ...patch } : current));
    };

    if (!timeline) return null;

    return (
      <>
        <Modal
          isOpen={form != null}
          onClose={closeModal}
          title={t("video.chapter.generateInsertedTitle")}
          subtitle={form ? gapLabelFromIndex(form.gap, itemsById, t) : undefined}
          icon={<Plus size={15} />}
          maxWidthClassName="max-w-3xl"
          footer={
            form ? (
              <div className="flex flex-wrap justify-end gap-2">
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={closeModal}
                  disabled={acting != null}
                >
                  {t("common.cancel")}
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={
                    acting != null ||
                    draft == null ||
                    draft.revision !== timeline.revision
                  }
                  onClick={async () => {
                    if (draft == null) return;
                    if (
                      await onCommit(
                        timeline.id,
                        draft.revision,
                        draft.workflowRunId,
                      )
                    ) {
                      resetModalState();
                    }
                  }}
                >
                  <Check size={12} />
                  {t("video.chapter.confirmInsert")}
                </BookButton>
                <BookButton
                  variant="primary"
                  size="sm"
                  disabled={
                    acting != null ||
                    form.user_prompt.trim().length === 0 ||
                    !VIDEO_ASPECT_RATIOS.includes(
                      form.aspect_ratio as VideoAspectRatio,
                    )
                  }
                  onClick={async () => {
                    const revision = timeline.revision;
                    setDraft(null);
                    const runId = await onGenerate(
                      buildInsertedSegmentGenerationOptions(
                        timeline.id,
                        revision,
                        form,
                      ),
                    );
                    if (runId != null) {
                      setDraft({ revision, workflowRunId: runId });
                    }
                  }}
                >
                  <Wand2 size={12} />
                  {draft == null ? t("video.chapter.generateDescription") : t("video.chapter.regenerateDescription")}
                </BookButton>
              </div>
            ) : undefined
          }
        >
          {form ? (
            <div className="space-y-3">
              <label className="block">
                <span className="text-xs font-medium text-book-text-main">
                  {t("video.chapter.userPrompt")}
                </span>
                <textarea
                  value={form.user_prompt}
                  disabled={acting != null}
                  onChange={(event) =>
                    updateForm({ user_prompt: event.target.value })
                  }
                  className="book-control mt-1 min-h-28 w-full resize-y p-2 text-xs leading-relaxed"
                  placeholder={t("video.chapter.insertPromptPlaceholder")}
                />
              </label>
              <div className="grid gap-2 sm:grid-cols-2">
                <label className="block">
                  <span className="text-xs text-book-text-muted">{t("video.chapter.style")}</span>
                  <input
                    value={form.style}
                    disabled={acting != null}
                    onChange={(event) =>
                      updateForm({ style: event.target.value })
                    }
                    className="book-control mt-1 h-9 w-full px-2 text-xs"
                  />
                </label>
                <label className="block">
                  <span className="text-xs text-book-text-muted">
                    {t("video.chapter.promptLanguageLabel")}
                  </span>
                  <ThemedSelect
                    value={form.prompt_output_language}
                    disabled={acting != null}
                    onChange={(selectedValue) =>
                      updateForm({
                        prompt_output_language: selectedValue as PromptOutputLanguage,
                      })
                    }
                    className="book-control mt-1 h-9 w-full px-2 text-xs"
                  >
                    <option value="zh_cn">{t("settings.chinese")}</option>
                    <option value="en">{t("settings.english")}</option>
                  </ThemedSelect>
                </label>
                <label className="block">
                  <span className="text-xs text-book-text-muted">
                    {t("video.chapter.durationSeconds")}
                  </span>
                  <ThemedSelect
                    value={form.duration_secs}
                    disabled={acting != null}
                    onChange={(selectedValue) =>
                      updateForm({ duration_secs: Number(selectedValue) })
                    }
                    className="book-control mt-1 h-9 w-full px-2 text-xs"
                  >
                    {form.duration_secs_options.map((duration) => (
                      <option key={duration} value={duration}>
                        {t("video.chapter.seconds", { count: duration })}
                      </option>
                    ))}
                  </ThemedSelect>
                </label>
                <label className="block">
                  <span className="text-xs text-book-text-muted">{t("video.chapter.aspectRatioLabel")}</span>
                  <ThemedSelect
                    value={form.aspect_ratio}
                    disabled={acting != null}
                    onChange={(selectedValue) =>
                      updateForm({ aspect_ratio: selectedValue })
                    }
                    className="book-control mt-1 h-9 w-full px-2 text-xs"
                  >
                    {!VIDEO_ASPECT_RATIOS.includes(
                      form.aspect_ratio as VideoAspectRatio,
                    ) && (
                      <option value={form.aspect_ratio} disabled>
                        {form.aspect_ratio} · {t("video.chapter.unsupportedAspectRatio")}
                      </option>
                    )}
                    {VIDEO_ASPECT_RATIOS.map((ratio) => (
                      <option key={ratio} value={ratio}>
                        {ratio}
                      </option>
                    ))}
                  </ThemedSelect>
                </label>
              </div>
              <div>
                <div className="mb-1 text-xs text-book-text-muted">
                  {t("video.chapter.referenceImagesOptional")}
                </div>
                <div className="grid max-h-36 grid-cols-3 gap-2 overflow-y-auto sm:grid-cols-5">
                  {referenceImages.map((image) => {
                    const checked = form.reference_image_ids.includes(image.id);
                    return (
                      <div key={image.id} className="relative min-w-0">
                      <button
                        type="button"
                        aria-label={t("video.reference.imageNumber", { id: image.id })}
                        aria-pressed={checked}
                        disabled={acting != null}
                        onClick={() => {
                          const next = checked
                            ? form.reference_image_ids.filter(
                                (id) => id !== image.id,
                              )
                            : form.reference_image_ids.length >=
                                MAX_REFERENCE_IMAGES
                              ? form.reference_image_ids
                              : [...form.reference_image_ids, image.id];
                          updateForm({ reference_image_ids: next });
                        }}
                        className={`relative w-full overflow-hidden rounded border ${
                          checked
                            ? "border-book-primary ring-1 ring-book-primary"
                            : "border-book-border/40"
                        }`}
                      >
                        <img
                          src={toMediaSrc(image.file_path)}
                          alt=""
                          className="aspect-square w-full object-cover"
                        />
                        {checked ? (
                          <span className="absolute right-1 top-1 rounded-full bg-book-primary p-0.5 text-white">
                            <Check size={10} />
                          </span>
                        ) : null}
                      </button>
                      <ImageZoomButton src={toMediaSrc(image.file_path)} alt={image.prompt} className="absolute bottom-1 left-1" />
                      </div>
                    );
                  })}
                </div>
              </div>
              <label className="flex items-center gap-2 text-caption text-book-text-muted">
                <input
                  type="checkbox"
                  checked={form.allow_without_visual_anchor}
                  disabled={acting != null}
                  onChange={(event) =>
                    updateForm({
                      allow_without_visual_anchor: event.target.checked,
                    })
                  }
                />
                {t("video.chapter.allowTextOnly")}
              </label>
              {draft != null && draft.revision !== timeline.revision ? (
                <div className="rounded-lg border border-warning/35 bg-warning/8 px-3 py-2 text-caption text-warning">
                  {t("video.chapter.timelineChanged")}
                </div>
              ) : draft != null ? (
                <div className="rounded-lg border border-success/35 bg-success/8 px-3 py-2 text-caption text-success">
                  {t("video.chapter.descriptionDraftGenerated", { id: draft.workflowRunId })}
                </div>
              ) : null}
            </div>
          ) : null}
        </Modal>

        <Modal
          isOpen={editingItem != null && editingPrompt != null && edit != null}
          onClose={closeModal}
          title={t("video.chapter.insertedDetailTitle")}
          subtitle={t("video.chapter.insertedDetailSubtitle")}
          icon={<ChevronDown size={15} />}
          maxWidthClassName="max-w-3xl"
          footer={
            editingPrompt != null && edit != null ? (
              <div className="flex flex-wrap justify-end gap-2">
                {editingVideo ? (
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={() => void onPreviewVideo(editingVideo)}
                  >
                    {t("video.chapter.previewVideo")}
                  </BookButton>
                ) : null}
                {editingVideo ? (
                  <BookButton
                    variant="danger"
                    size="sm"
                    disabled={acting != null}
                    onClick={() => void onDeleteVideo(editingVideo)}
                  >
                    <Trash2 size={12} /> {t("video.chapter.deleteVideo")}
                  </BookButton>
                ) : null}
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={acting != null}
                  onClick={() =>
                    void onUpdatePrompt(editingPrompt, {
                      visual_prompt: edit.visual_prompt,
                      motion_prompt: edit.motion_prompt,
                      negative_prompt: edit.negative_prompt,
                    })
                  }
                >
                  <Save size={12} /> {t("video.chapter.saveChanges")}
                </BookButton>
                <BookButton
                  variant="primary"
                  size="sm"
                  disabled={
                    acting != null ||
                    editingItemNeedsReanchor ||
                    editingPromptDirty
                  }
                  onClick={() => void onGenerateVideo(editingPrompt)}
                >
                  <Wand2 size={12} />
                  {editingPromptDirty ? t("video.chapter.saveBeforeAction") : t("video.chapter.generateVideo")}
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={closeModal}
                  disabled={acting != null}
                >
                  {t("common.close")}
                </BookButton>
              </div>
            ) : undefined
          }
        >
          {edit && editingPrompt ? (
            <div className="space-y-3">
              {editingItem ? (
                <div className="flex flex-wrap items-center gap-2 rounded-lg border border-book-border/35 bg-book-bg-paper/70 px-3 py-2">
                  <div className="min-w-0 flex-1 text-caption text-book-text-muted">
                    {t("video.chapter.timelinePosition", { number: editingItem.display_index })}
                  </div>
                  <ThemedSelect
                    aria-label={t("video.chapter.moveInserted", { number: editingItem.display_index })}
                    value=""
                    disabled={acting != null}
                    onChange={(selectedValue) => {
                      const selected = editingMoveGaps.find(
                        (candidate) =>
                          insertedSegmentGapKey(candidate) === selectedValue,
                      );
                      if (!selected) return;
                      void onMove({
                        timeline_id: timeline.id,
                        expected_timeline_revision: timeline.revision,
                        timeline_item_id: editingItem.id,
                        ...selected,
                      });
                    }}
                    className="book-control h-8 max-w-48 px-2 text-caption"
                  >
                    <option value="">{t("video.chapter.movePosition")}</option>
                    {editingMoveGaps
                      .filter((candidate) => !gapUsesReanchorItem(candidate))
                      .map((candidate) => (
                        <option
                          key={insertedSegmentGapKey(candidate)}
                          value={insertedSegmentGapKey(candidate)}
                        >
                          {gapLabelFromIndex(candidate, itemsById, t)}
                        </option>
                      ))}
                  </ThemedSelect>
                  <BookButton
                    variant="danger"
                    size="sm"
                    disabled={acting != null}
                    onClick={async () => {
                      const confirmed = await confirmDialog({
                        title: t("video.chapter.deleteInsertedTimelineTitle"),
                        message: t("video.chapter.deleteInsertedTimelineMessage"),
                        confirmText: t("common.delete"),
                        dialogType: "danger",
                      });
                      if (!confirmed) return;
                      await onDelete({
                        timeline_id: timeline.id,
                        expected_timeline_revision: timeline.revision,
                        timeline_item_id: editingItem.id,
                      });
                    }}
                  >
                    <Trash2 size={11} /> {t("video.chapter.removeSegment")}
                  </BookButton>
                </div>
              ) : null}
              {editingItemNeedsReanchor ? (
                <div className="rounded-lg border border-warning/35 bg-warning/8 px-3 py-2 text-caption text-warning">
                  {t("video.chapter.nativePlanChanged")}
                </div>
              ) : null}
              <div className="grid gap-2 sm:grid-cols-2">
                <div className="rounded-lg border border-book-border/35 bg-book-bg-paper/70 px-3 py-2">
                  <div className="text-xs text-book-text-muted">{t("novel.novel")}</div>
                  <div className="mt-1 text-xs text-book-text-main">
                    {editingPrompt.title?.trim() || t("video.chapter.unnamedInserted")}
                  </div>
                </div>
                <div className="rounded-lg border border-book-border/35 bg-book-bg-paper/70 px-3 py-2">
                  <div className="text-xs text-book-text-muted">{t("video.chapter.sceneSummary")}</div>
                  <div className="mt-1 whitespace-pre-wrap text-xs text-book-text-main">
                    {editingPrompt.scene_summary?.trim() || t("video.chapter.noSceneSummary")}
                  </div>
                </div>
              </div>
              {(
                ["visual_prompt", "motion_prompt", "negative_prompt"] as const
              ).map((field) => (
                <label key={field} className="block">
                  <span className="text-xs text-book-text-muted">
                    {field === "visual_prompt"
                      ? t("video.chapter.visualPrompt")
                      : field === "motion_prompt"
                        ? t("video.chapter.motionPrompt")
                        : t("video.chapter.constraints")}
                  </span>
                  <textarea
                    value={edit[field]}
                    disabled={acting != null}
                    onChange={(event) =>
                      setEdit({ ...edit, [field]: event.target.value })
                    }
                    className="book-control mt-1 min-h-20 w-full resize-y p-2 text-xs leading-relaxed"
                  />
                </label>
              ))}
            </div>
          ) : null}
        </Modal>
      </>
    );
  },
);

InsertedSegmentTimelinePanel.displayName = "InsertedSegmentTimelinePanel";
