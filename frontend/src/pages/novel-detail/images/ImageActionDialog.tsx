import { useEffect, useMemo, useRef, useState } from "react";
import { Clipboard,
  Image as ImageIcon,
  Save,
  Star,
  Trash2, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
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
import { imageTypeLabel } from "./imageTabUtils";

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
}> = [
  { id: "overview", label: "概览" },
  { id: "metadata", label: "来源" },
  { id: "manage", label: "管理" },
];

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

  const busy = busyId != null || scoringId != null || savingEdit;
  const src = toMediaSrc(image.file_path);
  const generationFields = imageGenerationMetadataFields(image);
  const sourceFields = imageSourceMetadataFields(image);
  const businessFields = imageBusinessMetadataFields(image);
  const snapshotFields = imagePromptSnapshotFields(image);
  const detailSections = [
    { title: "生成信息", fields: generationFields },
    { title: "业务来源", fields: businessFields },
    { title: "正文来源", fields: sourceFields },
    { title: "提示词快照", fields: snapshotFields },
  ].filter((section) => section.fields.length > 0);
  const systemManagedImageType = image.is_system_managed;
  const overviewFields = imageBaseMetadataFields(image).filter(
    (field) => !["图片类型", "模型", "尺寸"].includes(field.label),
  );
  const editDirty =
    editPrompt.trim() !== image.prompt.trim() ||
    (!systemManagedImageType && editType.trim() !== image.image_type.trim()) ||
    (!systemManagedImageType && editPanelId.trim() !== (image.panel_id ?? ""));
  const scoreValue = scoreInput.trim() ? Number(scoreInput) : Number.NaN;
  const scoreValid =
    Number.isFinite(scoreValue) && scoreValue >= 0 && scoreValue <= 1;
  const scoreDirty =
    scoreValid &&
    (image.ai_score == null || Math.abs(scoreValue - image.ai_score) > 0.000_001);

  const copyRawJson = async () => {
    try {
      await copyTextToClipboard(rawJson);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch (err) {
      setCopied(false);
      addToast(extractErrorMessage(err, "复制图片数据失败"), "error");
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
      title={imageTypeLabel(image.image_type)}
      subtitle={`${image.width} × ${image.height}${image.is_favorite ? " · 已收藏" : ""}`}
      icon={<ImageIcon size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
    >
      <SegmentedControl
        ariaLabel="图片详情栏目"
        value={activeTab}
        onChange={setTab}
        options={visibleTabs}
        className="mb-4 w-full [&_.a4n-seg__btn]:flex-1 sm:w-auto sm:[&_.a4n-seg__btn]:flex-none"
      />

      {activeTab === "overview" ? (
        <div className="grid gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
          <div className="min-w-0">
            <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden rounded-[14px] border border-book-border/50 bg-book-bg-tertiary text-xs text-book-text-muted">
              {src ? (
                <img
                  src={src}
                  alt={`${imageTypeLabel(image.image_type)}预览`}
                  className="h-full w-full object-contain"
                />
              ) : (
                "无图"
              )}
            </div>
          </div>
          <div className="min-w-0 space-y-4">
            <MetadataSection title="基础信息" fields={overviewFields} />
            <MetadataTextBlock title="生成提示词" text={image.prompt} />
          </div>
        </div>
      ) : activeTab === "metadata" ? (
        <div className="space-y-4">
          {detailSections.length > 0 ? (
            <div className="a4n-flat-surface divide-y divide-book-border/45 overflow-hidden">
              {detailSections.map((section) => (
                <MetadataSection
                  key={section.title}
                  title={section.title}
                  fields={section.fields}
                  embedded
                />
              ))}
            </div>
          ) : (
            <div className="py-8 text-center text-xs text-book-text-muted">没有可展示的图片详情。</div>
          )}
          <details className="a4n-flat-surface overflow-hidden">
            <summary className="flex cursor-pointer items-center justify-between gap-3 px-3 py-2.5 text-xs font-medium text-book-text-sub">
              <span>诊断信息</span>
              <span className="text-caption font-normal text-book-text-muted">原始生成记录</span>
            </summary>
            <div className="border-t border-book-border/40">
              <div className="flex justify-end px-3 pt-2">
                <button
                  type="button"
                  onClick={() => void copyRawJson()}
                  className="pyqt-btn-secondary inline-flex !min-h-7 items-center gap-1 !px-3 text-small"
                >
                  <Clipboard size={11} />
                  {copied ? "已复制" : "复制 JSON"}
                </button>
              </div>
          <pre className="whitespace-pre-wrap break-words p-3 font-mono text-caption leading-relaxed text-book-text-sub">
                {rawJson}
              </pre>
            </div>
          </details>
        </div>
      ) : (
        <div className="space-y-3">
          <div className="space-y-2">
            <button
              type="button"
              role="switch"
              aria-checked={image.is_favorite}
              disabled={busyId === image.id}
              onClick={() => onToggleFavorite(image)}
              className="a4n-flat-surface flex w-full items-center justify-between gap-3 overflow-hidden px-3 py-3 text-left transition-colors hover:bg-book-bg-tertiary/35 focus-visible:outline-2 focus-visible:outline-book-primary/35 disabled:cursor-not-allowed disabled:opacity-55"
            >
              <span className="flex min-w-0 items-start gap-2.5">
                <Star
                  size={14}
                  fill={image.is_favorite ? "currentColor" : "none"}
                  className="mt-0.5 shrink-0 text-book-primary"
                />
                <span className="min-w-0">
                  <span className="block text-xs font-medium text-book-text-main">加入收藏</span>
                  <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
                    在图库中通过收藏筛选快速找到这张图片
                  </span>
                </span>
              </span>
              <span
                className={`relative inline-flex h-6 w-11 shrink-0 rounded-full transition-colors ${
                  image.is_favorite ? "bg-book-primary" : "bg-book-text-muted/25"
                }`}
                aria-hidden="true"
              >
                <span
                  className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-transform ${
                    image.is_favorite ? "translate-x-[22px]" : "translate-x-0.5"
                  }`}
                />
              </span>
            </button>

            <section className="a4n-flat-surface space-y-3 p-3">
              <div>
                <h3 className="text-xs font-semibold text-book-text-main">生成提示词</h3>
                <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
                  修改这张图片保存的提示词记录
                </p>
              </div>
              <label className="grid gap-1 text-small text-book-text-muted">
                <textarea
                  value={editPrompt}
                  onChange={(event) => setEditPrompt(event.target.value)}
                  className="pyqt-input min-h-[132px] resize-y !rounded-[12px] !px-3 !py-2 text-xs leading-relaxed"
                  disabled={savingEdit || busy}
                />
              </label>

              {!systemManagedImageType ? (
                <details className="rounded-[12px] border border-book-border/45 bg-book-bg-paper/30">
                  <summary className="cursor-pointer px-3 py-2.5 text-small font-medium text-book-text-sub">
                    高级记录字段
                  </summary>
                  <div className="grid gap-2 border-t border-book-border/40 px-3 py-3">
                    <label className="grid gap-1 text-small text-book-text-muted">
                      素材分类
                      <input
                        value={editType}
                        onChange={(event) => setEditType(event.target.value)}
                        className="pyqt-input !px-2 !py-1 text-xs"
                        disabled={savingEdit || busy}
                      />
                    </label>
                    <label className="grid gap-1 text-small text-book-text-muted">
                      关联分镜标识
                      <input
                        value={editPanelId}
                        onChange={(event) => setEditPanelId(event.target.value)}
                        className="pyqt-input !px-2 !py-1 text-xs"
                        placeholder="可留空"
                        disabled={savingEdit || busy}
                      />
                    </label>
                  </div>
                </details>
              ) : null}

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
                  className="w-full !text-xs sm:w-auto"
                >
                  {savingEdit ? <LoadingSpinner size="sm" /> : <Save size={11} />}
                  保存修改
                </BookButton>
              </div>
            </section>

            <section className="a4n-flat-surface space-y-3 p-3">
              <div>
                <h3 className="text-xs font-semibold text-book-text-main">质量评分</h3>
                <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
                  使用 0 到 1 之间的数值记录质量评价
                </p>
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  min="0"
                  max="1"
                  step="0.01"
                  inputMode="decimal"
                  aria-label="质量评分"
                  value={scoreInput}
                  onChange={(event) => onScoreInputChange(event.target.value)}
                  placeholder="例如：0.8"
                  className="pyqt-input min-w-0 flex-1 !px-3 !py-1 text-xs sm:max-w-40"
                />
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={() => onScore(image)}
                  disabled={busy || !scoreValid || !scoreDirty}
                  className="shrink-0 !text-xs"
                >
                  {scoringId === image.id ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    "保存评分"
                  )}
                </BookButton>
              </div>
            </section>
          </div>

          <details className="a4n-flat-surface overflow-hidden">
            <summary className="cursor-pointer px-3 py-2.5 text-small font-medium text-book-text-sub">
              危险操作
            </summary>
            <div className="flex flex-col gap-3 border-t border-book-border/40 px-3 py-3 sm:flex-row sm:items-center sm:justify-between">
              <p className="text-small leading-relaxed text-book-text-muted">
                删除后会解除关联并移除这条图片记录。
              </p>
              <BookButton
                variant="danger"
                size="sm"
                disabled={busyId === image.id}
                onClick={() => onDelete(image)}
                className="w-full shrink-0 !text-xs sm:w-auto"
              >
                <Trash2 size={12} /> 删除图片
              </BookButton>
            </div>
          </details>
        </div>
      )}
    </Modal>
  );
}

function MetadataSection({
  title,
  fields,
  empty = "没有记录。",
  embedded = false,
}: {
  title: string;
  fields: MetadataField[];
  empty?: string;
  embedded?: boolean;
}) {
  return (
    <section className={embedded ? "overflow-hidden" : "a4n-flat-surface overflow-hidden"}>
      <div className="border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
        {title}
      </div>
      {fields.length === 0 ? (
        <div className="px-3 py-3 text-xs text-book-text-muted">{empty}</div>
      ) : (
        <dl className="divide-y divide-book-border/35">
          {fields.map((item) => (
            <div
              key={`${item.label}:${item.value}`}
              className="grid grid-cols-[88px_minmax(0,1fr)] items-start gap-3 px-3 py-2 sm:grid-cols-[130px_minmax(0,1fr)]"
            >
              <dt className="text-small text-book-text-muted">{item.label}</dt>
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
    <section className="a4n-flat-surface overflow-hidden">
      <div className="border-b border-book-border/40 px-3 py-2 text-xs font-semibold text-book-text-main">
        {title}
      </div>
      <div className="whitespace-pre-wrap p-3 text-xs leading-relaxed text-book-text-sub">
        {text}
      </div>
    </section>
  );
}
