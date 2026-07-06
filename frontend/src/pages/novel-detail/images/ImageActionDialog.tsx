import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  Clipboard,
  Database,
  FileJson,
  Image as ImageIcon,
  Info,
  Loader2,
  Save,
  Star,
  Trash2,
} from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { useToast } from "../../../components/feedback/Toast";
import type { GeneratedImage, UpdateImageInput } from "../../../api/image";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { toMediaSrc } from "../../../utils/mediaSrc";
import {
  imageBaseMetadataFields,
  imageBusinessMetadataFields,
  imageGenerationMetadataFields,
  imageMetadataRawJson,
  imagePromptSnapshotFields,
  imageSourceMetadataFields,
  type MetadataField,
} from "./imageMetadata";

type ImageDialogTab = "overview" | "metadata" | "manage";

interface ImageActionDialogProps {
  image: GeneratedImage | null;
  busyId: number | null;
  scoringId: number | null;
  canEditImages?: boolean;
  scoreInput: string;
  onClose: () => void;
  onToggleFavorite: (image: GeneratedImage) => void;
  onUpdate: (image: GeneratedImage, input: UpdateImageInput) => Promise<void>;
  onScoreInputChange: (value: string) => void;
  onScore: (image: GeneratedImage) => void;
  onDelete: (image: GeneratedImage) => void;
}

const IMAGE_DIALOG_TABS: Array<{
  id: ImageDialogTab;
  label: string;
  icon: ReactNode;
}> = [
  { id: "overview", label: "概览", icon: <Info size={12} /> },
  { id: "metadata", label: "元数据", icon: <FileJson size={12} /> },
  { id: "manage", label: "管理", icon: <Database size={12} /> },
];

const SYSTEM_MANAGED_IMAGE_TYPES = new Set([
  "scene",
  "manga_page",
  "scene_material",
  "object_material",
  "video_key_frame",
  "video_start_frame",
  "video_storyboard_frame",
  "video_end_frame",
  "video_tail_frame",
  "canvas_image",
  "canvas_frame",
  "portrait",
  "portrait_reference",
  "chapter_reference",
  "project_reference",
]);

function isSystemManagedImageType(imageType: string): boolean {
  return SYSTEM_MANAGED_IMAGE_TYPES.has(imageType.trim());
}

export function ImageActionDialog({
  image,
  busyId,
  scoringId,
  canEditImages = true,
  scoreInput,
  onClose,
  onToggleFavorite,
  onUpdate,
  onScoreInputChange,
  onScore,
  onDelete,
}: ImageActionDialogProps) {
  const addToast = useToast((state) => state.addToast);
  const [tab, setTab] = useState<ImageDialogTab>("overview");
  const [copied, setCopied] = useState(false);
  const [editPrompt, setEditPrompt] = useState("");
  const [editType, setEditType] = useState("");
  const [editPanelId, setEditPanelId] = useState("");
  const [savingEdit, setSavingEdit] = useState(false);
  const savingEditRef = useRef(false);
  const rawJson = useMemo(() => (image ? imageMetadataRawJson(image) : ""), [image]);
  const visibleTabs = useMemo(
    () => (canEditImages ? IMAGE_DIALOG_TABS : IMAGE_DIALOG_TABS.filter((item) => item.id !== "manage")),
    [canEditImages],
  );
  const activeTab: ImageDialogTab = canEditImages || tab !== "manage" ? tab : "overview";

  useEffect(() => {
    savingEditRef.current = false;
    setSavingEdit(false);
    if (!image) {
      setEditPrompt("");
      setEditType("");
      setEditPanelId("");
      return;
    }
    setEditPrompt(image.prompt);
    setEditType(image.image_type);
    setEditPanelId(image.panel_id ?? "");
  }, [image?.id]);

  useEffect(() => {
    if (!canEditImages && tab === "manage") setTab("overview");
  }, [canEditImages, tab]);

  if (!image) return null;

  const busy = busyId != null || scoringId != null;
  const src = toMediaSrc(image.file_path);
  const generationFields = imageGenerationMetadataFields(image);
  const sourceFields = imageSourceMetadataFields(image);
  const businessFields = imageBusinessMetadataFields(image);
  const snapshotFields = imagePromptSnapshotFields(image);
  const systemManagedImageType = isSystemManagedImageType(image.image_type);
  const editDirty =
    editPrompt.trim() !== image.prompt.trim() ||
    (!systemManagedImageType && editType.trim() !== image.image_type.trim()) ||
    (!systemManagedImageType && editPanelId.trim() !== (image.panel_id ?? ""));

  const copyRawJson = async () => {
    try {
      await copyTextToClipboard(rawJson);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch (err) {
      setCopied(false);
      addToast(err, "error");
    }
  };

  const saveEdit = async () => {
    if (savingEditRef.current || busy) return;
    const prompt = editPrompt.trim();
    const imageType = editType.trim();
    if (!prompt || (!systemManagedImageType && !imageType)) return;
    savingEditRef.current = true;
    setSavingEdit(true);
    try {
      const input: UpdateImageInput = {
        prompt,
      };
      if (!systemManagedImageType) {
        input.image_type = imageType;
        input.panel_id = editPanelId.trim() || null;
      }
      await onUpdate(image, input);
    } catch {
      // 调用方负责 toast；保留编辑内容便于修正后重试。
    } finally {
      savingEditRef.current = false;
      setSavingEdit(false);
    }
  };

  return (
    <Modal
      isOpen
      onClose={onClose}
      title={`图片 #${image.id}`}
      subtitle={`${image.image_type} · ${image.model}`}
      icon={<ImageIcon size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
      headerExtra={
        <div className="flex overflow-hidden rounded-sm border border-book-border/50 bg-book-bg-paper/35">
          {visibleTabs.map((item) => (
            <button
              key={item.id}
              type="button"
              data-active={activeTab === item.id ? "1" : undefined}
              onClick={() => setTab(item.id)}
              className="pyqt-tab !px-3"
            >
              {item.icon}
              {item.label}
            </button>
          ))}
        </div>
      }
    >
      {activeTab === "overview" ? (
        <div className="grid gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
          <div className="min-w-0">
            <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden rounded border border-book-border/50 bg-book-bg-tertiary text-xs text-book-text-muted">
              {src ? (
                <img src={src} alt={`图片 ${image.id}`} className="h-full w-full object-contain" />
              ) : (
                "无图"
              )}
            </div>
            <div className="mt-3 flex flex-wrap items-center gap-1 text-xs">
              <span className="pyqt-pill !text-[10px]">{image.image_type}</span>
              {image.panel_id ? (
                <span className="pyqt-pill !text-[10px] font-mono">{image.panel_id}</span>
              ) : null}
              {image.ai_score != null ? (
                <span className="pyqt-pill !text-[10px]">评分 {image.ai_score.toFixed(3)}</span>
              ) : null}
              {image.is_favorite ? (
                <span className="pyqt-pill !text-[10px] inline-flex items-center gap-0.5">
                  <Star size={9} fill="currentColor" />
                  收藏
                </span>
              ) : null}
            </div>
          </div>
          <div className="min-w-0 space-y-4">
            <MetadataSection title="基础记录" fields={imageBaseMetadataFields(image)} />
            <MetadataTextBlock title="生成提示词" text={image.prompt} />
          </div>
        </div>
      ) : activeTab === "metadata" ? (
        <div className="space-y-4">
          <div className="grid gap-4 lg:grid-cols-2">
            <MetadataSection title="生成链路" fields={generationFields} empty="没有 generation 元数据。" />
            <MetadataSection title="业务来源" fields={businessFields} empty="没有业务来源元数据。" />
            <MetadataSection title="正文来源" fields={sourceFields} empty="没有 source 元数据。" />
            <MetadataSection title="提示词快照" fields={snapshotFields} empty="没有 prompt_snapshot。" />
          </div>
          <div className="a4n-page-surface overflow-hidden">
            <div className="flex items-center justify-between gap-2 border-b border-book-border/40 px-3 py-2">
              <div className="inline-flex items-center gap-1.5 text-xs font-semibold text-book-text-main">
                <FileJson size={12} className="text-book-primary" />
                原始记录
              </div>
              <button
                type="button"
                onClick={() => void copyRawJson()}
                className="pyqt-btn-secondary inline-flex !min-h-7 items-center gap-1 !px-3 text-[11px]"
              >
                <Clipboard size={11} />
                {copied ? "已复制" : "复制 JSON"}
              </button>
            </div>
            <pre className="max-h-80 overflow-auto p-3 text-[11px] leading-relaxed text-book-text-sub">
              {rawJson}
            </pre>
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          <ImageActionButton
            icon={<Star size={14} fill={image.is_favorite ? "currentColor" : "none"} />}
            title={image.is_favorite ? "取消收藏" : "收藏图片"}
            description="调整图片是否进入收藏筛选。"
            disabled={busyId === image.id}
            onClick={() => onToggleFavorite(image)}
          />
          <div className="a4n-page-surface p-3">
            <div className="mb-2 text-xs font-semibold text-book-text-main">记录编辑</div>
            <div className="grid gap-2">
              <label className="grid gap-1 text-[11px] text-book-text-muted">
                图片类型
                <input
                  value={editType}
                  onChange={(event) => setEditType(event.target.value)}
                  className="pyqt-input !px-2 !py-1 text-xs"
                  disabled={savingEdit || busy || systemManagedImageType}
                />
              </label>
              <label className="grid gap-1 text-[11px] text-book-text-muted">
                面板 ID
                <input
                  value={editPanelId}
                  onChange={(event) => setEditPanelId(event.target.value)}
                  className="pyqt-input !px-2 !py-1 text-xs"
                  placeholder="可留空"
                  disabled={savingEdit || busy || systemManagedImageType}
                />
              </label>
              <label className="grid gap-1 text-[11px] text-book-text-muted">
                Prompt
                <textarea
                  value={editPrompt}
                  onChange={(event) => setEditPrompt(event.target.value)}
                  className="pyqt-input min-h-[120px] resize-y !px-2 !py-1 text-xs"
                  disabled={savingEdit || busy}
                />
              </label>
              <div className="flex justify-end">
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={() => void saveEdit()}
                  disabled={
                    savingEdit ||
                    busy ||
                    !editDirty ||
                    !editPrompt.trim() ||
                    (!systemManagedImageType && !editType.trim())
                  }
                >
                  {savingEdit ? <Loader2 size={11} className="animate-spin" /> : <Save size={11} />}
                  保存记录
                </BookButton>
              </div>
            </div>
          </div>
          <div className="a4n-page-surface p-3">
            <div className="mb-2 text-xs font-semibold text-book-text-main">AI 评分</div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={scoreInput}
                onChange={(event) => onScoreInputChange(event.target.value)}
                placeholder="0.8"
                className="pyqt-input !px-2 !py-1 w-24 text-xs"
              />
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => onScore(image)}
                disabled={scoringId === image.id}
              >
                {scoringId === image.id ? (
                  <Loader2 size={11} className="animate-spin" />
                ) : (
                  "保存评分"
                )}
              </BookButton>
            </div>
          </div>
          <ImageActionButton
            icon={<Trash2 size={14} />}
            title="删除图片"
            description="解除关联并删除该生成图片记录。"
            tone="danger"
            disabled={busyId === image.id}
            onClick={() => onDelete(image)}
          />
        </div>
      )}
    </Modal>
  );
}

function MetadataSection({
  title,
  fields,
  empty = "没有记录。",
}: {
  title: string;
  fields: MetadataField[];
  empty?: string;
}) {
  return (
    <section className="a4n-page-surface overflow-hidden">
      <div className="border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
        {title}
      </div>
      {fields.length === 0 ? (
        <div className="px-3 py-3 text-xs text-book-text-muted">{empty}</div>
      ) : (
        <dl className="divide-y divide-book-border/35">
          {fields.map((item) => (
            <div key={`${item.label}:${item.value}`} className="grid gap-2 px-3 py-2 sm:grid-cols-[130px_minmax(0,1fr)]">
              <dt className="text-[11px] text-book-text-muted">{item.label}</dt>
              <dd
                className={`min-w-0 break-words text-xs text-book-text-main ${
                  item.mono ? "font-mono" : ""
                }`}
              >
                {item.value}
              </dd>
            </div>
          ))}
        </dl>
      )}
    </section>
  );
}

function MetadataTextBlock({ title, text }: { title: string; text: string }) {
  return (
    <section className="a4n-page-surface overflow-hidden">
      <div className="border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
        {title}
      </div>
      <div className="max-h-44 overflow-auto p-3 text-xs leading-relaxed text-book-text-sub">
        {text}
      </div>
    </section>
  );
}

function ImageActionButton({
  icon,
  title,
  description,
  disabled,
  tone = "normal",
  onClick,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  disabled?: boolean;
  tone?: "normal" | "danger";
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`a4n-interactive-row flex w-full items-start gap-3 px-3 py-3 text-left disabled:cursor-not-allowed disabled:opacity-55 ${
        tone === "danger"
          ? "border-error/50 bg-error-bg/40 text-error hover:bg-error-bg dark:hover:bg-error-bg"
          : "text-book-text-main"
      }`}
    >
      <span
        className={`mt-0.5 inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border ${
          tone === "danger"
            ? "border-error/40 bg-error/10 text-error"
            : "border-book-primary/30 bg-book-primary/10 text-book-primary"
        }`}
      >
        {icon}
      </span>
      <span className="min-w-0">
        <span className="block text-sm font-semibold">{title}</span>
        <span className="mt-1 block text-[11px] leading-relaxed text-book-text-muted">
          {description}
        </span>
      </span>
    </button>
  );
}
