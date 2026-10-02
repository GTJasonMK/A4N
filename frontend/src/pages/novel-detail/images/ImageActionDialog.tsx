import { useEffect, useMemo, useRef, useState } from "react";
import { Clipboard,
  Image as ImageIcon,
  Save,
  Star,
  Trash2, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { ZoomableImage } from "../../../components/ui/ImageLightbox";
import { Modal } from "../../../components/ui/Modal";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { useToast } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
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
import { useI18n } from "../../../i18n";
import { useNovelDetailDirty } from "../novelDetailDirty";

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
  labelKey: "novel.imageLibrary.overviewTab" | "novel.imageLibrary.metadataTab" | "novel.imageLibrary.manageTab";
}> = [
  { id: "overview", labelKey: "novel.imageLibrary.overviewTab" },
  { id: "metadata", labelKey: "novel.imageLibrary.metadataTab" },
  { id: "manage", labelKey: "novel.imageLibrary.manageTab" },
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
  const { locale, t } = useI18n();
  const [tab, setTab] = useState<ImageDialogTab>("overview");
  const [copied, setCopied] = useState(false);
  const [editPrompt, setEditPrompt] = useState("");
  const [editType, setEditType] = useState("");
  const [editPanelId, setEditPanelId] = useState("");
  const [savingEdit, setSavingEdit] = useState(false);
  const [scoreEdited, setScoreEdited] = useState(false);
  const savingEditRef = useRef(false);
  const rawJson = useMemo(() => (image ? imageMetadataRawJson(image) : ""), [image]);
  const visibleTabs = useMemo(
    () => (canEditImages ? IMAGE_DIALOG_TABS : IMAGE_DIALOG_TABS.filter((item) => item.id !== "manage")),
    [canEditImages],
  );
  const activeTab: ImageDialogTab = canEditImages || tab !== "manage" ? tab : "overview";
  const scoreValue = scoreInput.trim() ? Number(scoreInput) : Number.NaN;
  const scoreValid = Number.isFinite(scoreValue) && scoreValue >= 0 && scoreValue <= 1;
  const scoreDirty = image != null && scoreValid && (
    image.ai_score == null || Math.abs(scoreValue - image.ai_score) > 0.000_001
  );

  useEffect(() => {
    savingEditRef.current = false;
    setSavingEdit(false);
    setScoreEdited(false);
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

  const imageDraftDirty = image != null && (
    editPrompt.trim() !== image.prompt.trim() ||
    (!image.is_system_managed && editType.trim() !== image.image_type.trim()) ||
    (!image.is_system_managed && editPanelId.trim() !== (image.panel_id ?? "")) ||
    (scoreEdited && (!scoreValid || scoreDirty))
  );
  useNovelDetailDirty("image-action-editor", imageDraftDirty);

  useEffect(() => {
    if (!canEditImages && tab === "manage") setTab("overview");
  }, [canEditImages, tab]);

  if (!image) return null;

  const busy = busyId != null || scoringId != null || savingEdit;
  const src = toMediaSrc(image.file_path);
  const generationFields = imageGenerationMetadataFields(image, t, locale);
  const sourceFields = imageSourceMetadataFields(image, t, locale);
  const businessFields = imageBusinessMetadataFields(image, t, locale);
  const snapshotFields = imagePromptSnapshotFields(image, t, locale);
  const detailSections = [
    { title: t("novel.imageLibrary.generationInfo"), fields: generationFields },
    { title: t("novel.imageLibrary.businessSource"), fields: businessFields },
    { title: t("novel.imageLibrary.contentSource"), fields: sourceFields },
    { title: t("novel.imageLibrary.promptSnapshot"), fields: snapshotFields },
  ].filter((section) => section.fields.length > 0);
  const systemManagedImageType = image.is_system_managed;
  const overviewFields = imageBaseMetadataFields(image, t, locale).filter(
    (field) => ![
      t("novel.imageLibrary.imageType"),
      t("novel.imageLibrary.model"),
      t("novel.imageLibrary.size"),
    ].includes(field.label),
  );
  const editDirty =
    editPrompt.trim() !== image.prompt.trim() ||
    (!systemManagedImageType && editType.trim() !== image.image_type.trim()) ||
    (!systemManagedImageType && editPanelId.trim() !== (image.panel_id ?? ""));
  const requestClose = async () => {
    if (busy) return;
    if ((editDirty || (scoreEdited && (!scoreValid || scoreDirty))) && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: imageTypeLabel(image.image_type, t), action: t("common.close"),
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    onClose();
  };

  const copyRawJson = async () => {
    try {
      await copyTextToClipboard(rawJson);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch (err) {
      setCopied(false);
      addToast(extractErrorMessage(err, t("errors.requestFailed")), "error");
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
      onClose={() => void requestClose()}
      title={imageTypeLabel(image.image_type, t)}
      subtitle={`${image.width} × ${image.height}${image.is_favorite ? ` · ${t("novel.imageLibrary.favorite")}` : ""}`}
      icon={<ImageIcon size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
      showCloseButton={!busy}
    >
      <SegmentedControl
        ariaLabel={t("novel.imageLibrary.detailsTabs")}
        value={activeTab}
        onChange={setTab}
        options={visibleTabs.map((item) => ({ id: item.id, label: t(item.labelKey) }))}
        className="mb-4 w-full [&_.a4n-seg__btn]:flex-1 sm:w-auto sm:[&_.a4n-seg__btn]:flex-none"
      />

      {activeTab === "overview" ? (
        <div className="grid gap-4 md:grid-cols-[280px_minmax(0,1fr)]">
          <div className="min-w-0">
            <div className="relative mx-auto flex aspect-square w-full max-w-sm items-center justify-center overflow-hidden rounded-[14px] border border-book-border/50 bg-book-bg-tertiary text-xs text-book-text-muted">
              {src ? (
                <ZoomableImage
                  src={src}
                  alt={t("novel.imageLibrary.alt", { type: imageTypeLabel(image.image_type, t) })}
                  className="h-full w-full object-contain"
                />
              ) : (
                t("novel.imageLibrary.noImage")
              )}
            </div>
          </div>
          <div className="min-w-0 space-y-4">
            <MetadataSection title={t("novel.imageLibrary.overviewTab")} fields={overviewFields} empty={t("novel.imageLibrary.noRecord")} />
            <MetadataTextBlock title={t("novel.imageLibrary.promptTitle")} text={image.prompt} />
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
            <div className="py-8 text-center text-xs text-book-text-muted">{t("novel.imageLibrary.noDetails")}</div>
          )}
          <details className="a4n-flat-surface overflow-hidden">
            <summary className="flex cursor-pointer items-center justify-between gap-3 px-3 py-2.5 text-xs font-medium text-book-text-sub">
              <span>{t("novel.imageLibrary.diagnostics")}</span>
              <span className="text-caption font-normal text-book-text-muted">{t("novel.imageLibrary.rawRecord")}</span>
            </summary>
            <div className="border-t border-book-border/40">
              <div className="flex justify-end px-3 pt-2">
                <button
                  type="button"
                  onClick={() => void copyRawJson()}
                  className="pyqt-btn-secondary inline-flex !min-h-7 items-center gap-1 !px-3 text-small"
                >
                  <Clipboard size={11} />
                  {copied ? t("novel.imageLibrary.copied") : t("novel.imageLibrary.copyJson")}
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
              disabled={busy}
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
                  <span className="block text-xs font-medium text-book-text-main">{t("novel.imageLibrary.addFavorite")}</span>
                  <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
                    {t("novel.imageLibrary.favoriteHint")}
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
                <h3 className="text-xs font-semibold text-book-text-main">{t("novel.imageLibrary.promptTitle")}</h3>
                <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
                  {t("novel.imageLibrary.promptHint")}
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
                    {t("novel.imageLibrary.advancedFields")}
                  </summary>
                  <div className="grid gap-2 border-t border-book-border/40 px-3 py-3">
                    <label className="grid gap-1 text-small text-book-text-muted">
                      {t("novel.imageLibrary.assetCategory")}
                      <input
                        value={editType}
                        onChange={(event) => setEditType(event.target.value)}
                        className="pyqt-input !px-2 !py-1 text-xs"
                        disabled={savingEdit || busy}
                      />
                    </label>
                    <label className="grid gap-1 text-small text-book-text-muted">
                      {t("novel.imageLibrary.panelId")}
                      <input
                        value={editPanelId}
                        onChange={(event) => setEditPanelId(event.target.value)}
                        className="pyqt-input !px-2 !py-1 text-xs"
                        placeholder={t("novel.imageLibrary.optional")}
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
                  {t("novel.imageLibrary.saveChanges")}
                </BookButton>
              </div>
            </section>

            <section className="a4n-flat-surface space-y-3 p-3">
              <div>
                <h3 className="text-xs font-semibold text-book-text-main">{t("novel.imageLibrary.qualityScore")}</h3>
                <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
                  {t("novel.imageLibrary.qualityHint")}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  min="0"
                  max="1"
                  step="0.01"
                  inputMode="decimal"
                  aria-label={t("novel.imageLibrary.scoreAria")}
                  value={scoreInput}
                  onChange={(event) => {
                    setScoreEdited(true);
                    onScoreInputChange(event.target.value);
                  }}
                  placeholder={t("novel.imageLibrary.scorePlaceholder")}
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
                    t("novel.imageLibrary.saveScore")
                  )}
                </BookButton>
              </div>
            </section>
          </div>

          <details className="a4n-flat-surface overflow-hidden">
            <summary className="cursor-pointer px-3 py-2.5 text-small font-medium text-book-text-sub">
              {t("novel.imageLibrary.dangerZone")}
            </summary>
            <div className="flex flex-col gap-3 border-t border-book-border/40 px-3 py-3 sm:flex-row sm:items-center sm:justify-between">
              <p className="text-small leading-relaxed text-book-text-muted">
                {t("novel.imageLibrary.deleteHint")}
              </p>
              <BookButton
                variant="danger"
                size="sm"
                disabled={busy}
                onClick={() => onDelete(image)}
                className="w-full shrink-0 !text-xs sm:w-auto"
              >
                <Trash2 size={12} /> {t("novel.imageLibrary.deleteTitle")}
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
  empty = "—",
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
