import React, { useImperativeHandle, useState } from "react";
import { Check, ChevronDown, Plus, Save, Trash2, Wand2 } from "lucide-react";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { BookButton } from "../../../components/ui/BookButton";
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
import { MAX_REFERENCE_IMAGES, VIDEO_ASPECT_RATIOS } from "./videoTabUtils";

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

const gapKey = (gap: InsertedSegmentGap): string =>
  `${gap.previous_item_id ?? 0}:${gap.next_item_id ?? 0}`;

export const gapLabel = (
  gap: InsertedSegmentGap,
  items: VideoTimelineItemSummary[],
): string => {
  const previous = items.find((item) => item.id === gap.previous_item_id);
  const next = items.find((item) => item.id === gap.next_item_id);
  return `${previous?.title ?? (previous ? `片段 ${previous.display_index}` : "开头")} → ${next?.title ?? (next ? `片段 ${next.display_index}` : "结尾")}`;
};

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
    const items = timeline?.items ?? [];
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
    const [workflowRunId, setWorkflowRunId] = useState<number | null>(null);
    const [draftRevision, setDraftRevision] = useState<number | null>(null);
    const [editingItemId, setEditingItemId] = useState<number | null>(null);
    const [edit, setEdit] = useState<InsertedPromptEdit | null>(null);
    const editingItem = items.find((item) => item.id === editingItemId) ?? null;
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
      if (loading || acting != null) return;
      const durationOptions = Array.from(
        new Set([...defaultDurationOptions, defaultDurationSecs]),
      ).sort((left, right) => left - right);
      setWorkflowRunId(null);
      setDraftRevision(null);
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
      setWorkflowRunId(null);
      setDraftRevision(null);
      setEditingItemId(null);
      setEdit(null);
    };

    const closeModal = () => {
      if (acting != null) return;
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
      setWorkflowRunId(null);
      setDraftRevision(null);
      setForm((current) => (current ? { ...current, ...patch } : current));
    };

    if (!timeline) return null;

    return (
      <>
        <Modal
          isOpen={form != null}
          onClose={closeModal}
          title="生成插入片段"
          subtitle={form ? gapLabel(form.gap, items) : undefined}
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
                  取消
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={
                    acting != null ||
                    workflowRunId == null ||
                    draftRevision == null ||
                    draftRevision !== timeline.revision
                  }
                  onClick={async () => {
                    if (workflowRunId == null || draftRevision == null) return;
                    if (
                      await onCommit(timeline.id, draftRevision, workflowRunId)
                    ) {
                      resetModalState();
                    }
                  }}
                >
                  <Check size={12} />
                  确认插入
                </BookButton>
                <BookButton
                  variant="primary"
                  size="sm"
                  disabled={
                    acting != null || form.user_prompt.trim().length === 0
                  }
                  onClick={async () => {
                    const revision = timeline.revision;
                    setWorkflowRunId(null);
                    setDraftRevision(null);
                    const runId = await onGenerate(
                      buildInsertedSegmentGenerationOptions(
                        timeline.id,
                        revision,
                        form,
                      ),
                    );
                    if (runId != null) {
                      setDraftRevision(revision);
                      setWorkflowRunId(runId);
                    }
                  }}
                >
                  <Wand2 size={12} />
                  {workflowRunId == null ? "生成描述" : "重新生成描述"}
                </BookButton>
              </div>
            ) : undefined
          }
        >
          {form ? (
            <div className="space-y-3">
              <label className="block">
                <span className="text-xs font-medium text-book-text-main">
                  用户提示词
                </span>
                <textarea
                  value={form.user_prompt}
                  disabled={acting != null}
                  onChange={(event) =>
                    updateForm({ user_prompt: event.target.value })
                  }
                  className="book-control mt-1 min-h-28 w-full resize-y p-2 text-xs leading-relaxed"
                  placeholder="例如：让两人短暂沉默后交换一个确认眼神，镜头从门口缓慢推进。"
                />
              </label>
              <div className="grid gap-2 sm:grid-cols-2">
                <label className="block">
                  <span className="text-xs text-book-text-muted">风格</span>
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
                    提示词语言
                  </span>
                  <select
                    value={form.prompt_output_language}
                    disabled={acting != null}
                    onChange={(event) =>
                      updateForm({
                        prompt_output_language: event.target
                          .value as PromptOutputLanguage,
                      })
                    }
                    className="book-control mt-1 h-9 w-full px-2 text-xs"
                  >
                    <option value="zh_cn">简体中文</option>
                    <option value="en">English</option>
                  </select>
                </label>
                <label className="block">
                  <span className="text-xs text-book-text-muted">
                    时长（秒）
                  </span>
                  <select
                    value={form.duration_secs}
                    disabled={acting != null}
                    onChange={(event) =>
                      updateForm({ duration_secs: Number(event.target.value) })
                    }
                    className="book-control mt-1 h-9 w-full px-2 text-xs"
                  >
                    {form.duration_secs_options.map((duration) => (
                      <option key={duration} value={duration}>
                        {duration} 秒
                      </option>
                    ))}
                  </select>
                </label>
                <label className="block">
                  <span className="text-xs text-book-text-muted">画幅</span>
                  <select
                    value={form.aspect_ratio}
                    disabled={acting != null}
                    onChange={(event) =>
                      updateForm({ aspect_ratio: event.target.value })
                    }
                    className="book-control mt-1 h-9 w-full px-2 text-xs"
                  >
                    {VIDEO_ASPECT_RATIOS.map((ratio) => (
                      <option key={ratio} value={ratio}>
                        {ratio}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
              <div>
                <div className="mb-1 text-xs text-book-text-muted">
                  参考图（可选）
                </div>
                <div className="grid max-h-36 grid-cols-3 gap-2 overflow-y-auto sm:grid-cols-5">
                  {referenceImages.map((image) => {
                    const checked = form.reference_image_ids.includes(image.id);
                    return (
                      <button
                        type="button"
                        key={image.id}
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
                        className={`relative overflow-hidden rounded border ${
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
                没有前项尾帧/参考图时允许纯文本生成
              </label>
              {workflowRunId != null && draftRevision !== timeline.revision ? (
                <div className="rounded-lg border border-warning/35 bg-warning/8 px-3 py-2 text-caption text-warning">
                  时间线已变化，当前草稿不能直接提交；表单已保留，请重新生成描述。
                </div>
              ) : workflowRunId != null ? (
                <div className="rounded-lg border border-success/35 bg-success/8 px-3 py-2 text-caption text-success">
                  描述草稿已生成（workflow #{workflowRunId}
                  ）。确认后才会写入时间线；若 revision 冲突，表单内容会保留。
                </div>
              ) : null}
            </div>
          ) : null}
        </Modal>

        <Modal
          isOpen={editingItem != null && editingPrompt != null && edit != null}
          onClose={closeModal}
          title="插入片段详情"
          subtitle="该片段已进入时间线，可编辑提示词并复用视频生成"
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
                    预览视频
                  </BookButton>
                ) : null}
                {editingVideo ? (
                  <BookButton
                    variant="danger"
                    size="sm"
                    disabled={acting != null}
                    onClick={() => void onDeleteVideo(editingVideo)}
                  >
                    <Trash2 size={12} /> 删除视频
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
                  <Save size={12} /> 保存提示词
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
                  {editingPromptDirty ? "请先保存" : "生成视频"}
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={closeModal}
                  disabled={acting != null}
                >
                  关闭
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
                    时间线第 {editingItem.display_index} 位
                  </div>
                  <select
                    aria-label={`移动插入片段 ${editingItem.display_index}`}
                    value=""
                    disabled={acting != null}
                    onChange={(event) => {
                      const selected = editingMoveGaps.find(
                        (candidate) => gapKey(candidate) === event.target.value,
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
                    <option value="">移动位置…</option>
                    {editingMoveGaps
                      .filter((candidate) => !gapUsesReanchorItem(candidate))
                      .map((candidate) => (
                        <option
                          key={gapKey(candidate)}
                          value={gapKey(candidate)}
                        >
                          {gapLabel(candidate, items)}
                        </option>
                      ))}
                  </select>
                  <BookButton
                    variant="danger"
                    size="sm"
                    disabled={acting != null}
                    onClick={async () => {
                      const confirmed = await confirmDialog({
                        title: "删除插入片段",
                        message: "删除该插入片段？历史视频不会被物理删除。",
                        confirmText: "删除",
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
                    <Trash2 size={11} /> 移除片段
                  </BookButton>
                </div>
              ) : null}
              {editingItemNeedsReanchor ? (
                <div className="rounded-lg border border-warning/35 bg-warning/8 px-3 py-2 text-caption text-warning">
                  原生片段规划已变化，请先关闭窗口并通过“移动…”重新定位该片段，再生成视频或进入成片。
                </div>
              ) : null}
              <div className="grid gap-2 sm:grid-cols-2">
                <div className="rounded-lg border border-book-border/35 bg-book-bg-paper/70 px-3 py-2">
                  <div className="text-xs text-book-text-muted">标题</div>
                  <div className="mt-1 text-xs text-book-text-main">
                    {editingPrompt.title?.trim() || "未命名插入片段"}
                  </div>
                </div>
                <div className="rounded-lg border border-book-border/35 bg-book-bg-paper/70 px-3 py-2">
                  <div className="text-xs text-book-text-muted">场景摘要</div>
                  <div className="mt-1 whitespace-pre-wrap text-xs text-book-text-main">
                    {editingPrompt.scene_summary?.trim() || "暂无场景摘要"}
                  </div>
                </div>
              </div>
              {(
                ["visual_prompt", "motion_prompt", "negative_prompt"] as const
              ).map((field) => (
                <label key={field} className="block">
                  <span className="text-xs text-book-text-muted">
                    {field === "visual_prompt"
                      ? "画面提示词"
                      : field === "motion_prompt"
                        ? "运动提示词"
                        : "限制与避免项"}
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
