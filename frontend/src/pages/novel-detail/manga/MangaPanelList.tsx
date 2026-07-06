import React, { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  FileText,
  Image,
  ImagePlus,
  Link2,
  Loader2,
  MessageSquareText,
  Pencil,
  Save,
  Trash2,
  Users,
  Wand2,
  X,
  Eye,
} from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type { Portrait } from "../../../api/portrait";
import type { BindMangaReferenceImageInput, MangaPanel, PipelineStatus } from "../../../api/manga";
import { BookButton } from "../../../components/ui/BookButton";
import { Modal } from "../../../components/ui/Modal";
import { EmptyHint } from "../shared";
import {
  MangaPageImagePreview,
  type ImagePreviewTarget,
} from "./MangaPageImagePreview";
import {
  LayoutSummaryGrid,
  PageStatusPill,
  statusLabel,
  type MangaPageDetailView,
} from "./MangaPanelListParts";
import { ReferenceBindingsPanel } from "./ReferenceBindingsPanel";
import {
  buildPagePrompts,
  findGeneratedPageImage,
  sumPipelinePrompts,
  type MangaPagePrompt,
} from "./mangaPagePromptModel";

interface MangaPanelListProps {
  panels: MangaPanel[];
  panelsLoading: boolean;
  currentContentHash: string | null;
  chapterLocked?: boolean;
  generatingPageNumber?: number | null;
  profiles: CharacterProfile[];
  portraits: Portrait[];
  projectImages: GeneratedImage[];
  assetsLoading?: boolean;
  pipelineStatus?: PipelineStatus | null;
  optimizingPromptId?: number | null;
  actionsDisabled?: boolean;
  canEditManga: boolean;
  onPreviewPageImage?: (pageNumber: number) => void | Promise<void>;
  onGeneratePageImage?: (pageNumber: number) => void | Promise<void>;
  onBindReferenceImage?: (input: BindMangaReferenceImageInput) => void | Promise<void>;
  onUpdatePrompt?: (promptId: number, imagePrompt: string) => void | Promise<void>;
  onOptimizePrompt?: (prompt: MangaPanel) => void | Promise<void>;
  onDeletePrompt?: (prompt: MangaPanel) => void | Promise<void>;
}

export const MangaPanelList: React.FC<MangaPanelListProps> = ({
  panels,
  panelsLoading,
  currentContentHash,
  chapterLocked = false,
  generatingPageNumber = null,
  profiles,
  portraits,
  projectImages,
  assetsLoading = false,
  pipelineStatus,
  optimizingPromptId = null,
  actionsDisabled = false,
  canEditManga,
  onPreviewPageImage,
  onGeneratePageImage,
  onBindReferenceImage,
  onUpdatePrompt,
  onOptimizePrompt,
  onDeletePrompt,
}) => {
  const pagePrompts = useMemo(
    () => buildPagePrompts(panels, currentContentHash),
    [currentContentHash, panels],
  );
  const [selectedPage, setSelectedPage] = useState<number | null>(null);
  const [detailView, setDetailView] = useState<MangaPageDetailView>("prompt");
  const [editingPromptId, setEditingPromptId] = useState<number | null>(null);
  const [promptDraft, setPromptDraft] = useState("");
  const [promptSaving, setPromptSaving] = useState(false);
  const [selectedCandidateBySlot, setSelectedCandidateBySlot] = useState<Record<string, string>>(
    {},
  );
  const [imagePreview, setImagePreview] = useState<ImagePreviewTarget | null>(null);
  const panelScopeKey = useMemo(
    () => panels.map((panel) => `${panel.id}:${panel.page_number}`).join("|"),
    [panels],
  );

  useEffect(() => {
    if (pagePrompts.length === 0) {
      if (selectedPage !== null) setSelectedPage(null);
      return;
    }
    if (
      selectedPage == null ||
      !pagePrompts.some((page) => page.pageNumber === selectedPage)
    ) {
      setSelectedPage(pagePrompts[0].pageNumber);
    }
  }, [pagePrompts, selectedPage]);

  const activePage =
    pagePrompts.find((page) => page.pageNumber === selectedPage) ?? pagePrompts[0] ?? null;
  const activePageImage = useMemo(
    () => findGeneratedPageImage(activePage, projectImages),
    [activePage, projectImages],
  );

  useEffect(() => {
    setDetailView(activePage?.prompt.generated_image_id != null ? "image" : "prompt");
  }, [activePage?.pageNumber, activePage?.prompt.generated_image_id]);

  useEffect(() => {
    setEditingPromptId(null);
    setPromptDraft("");
    setPromptSaving(false);
  }, [activePage?.prompt.id]);

  useEffect(() => {
    setImagePreview(null);
    setSelectedCandidateBySlot({});
    setEditingPromptId(null);
    setPromptDraft("");
    setPromptSaving(false);
  }, [panelScopeKey]);

  useEffect(() => {
    if (!canEditManga) {
      setEditingPromptId(null);
      setPromptDraft("");
      setPromptSaving(false);
    }
  }, [canEditManga]);

  const linkedImageCount = pagePrompts.filter(
    (page) => page.prompt.generated_image_id != null,
  ).length;
  const staleCount = pagePrompts.filter((page) => page.sourceStale).length;
  const bindingCount = pagePrompts.reduce(
    (sum, page) => sum + page.referenceBindings.length,
    0,
  );
  const boundBindingCount = pagePrompts.reduce(
    (sum, page) =>
      sum + page.referenceBindings.filter((binding) => binding.boundAsset != null).length,
    0,
  );
  const pipelinePromptCount = sumPipelinePrompts(pipelineStatus);
  const pipelineCompleted = pipelineStatus?.completed ?? 0;
  const pipelineFailed = pipelineStatus?.failed ?? 0;
  const pipelineCheckpointLabel = pipelineStatus?.has_checkpoint
    ? pipelineStatus.checkpoint_valid
      ? "可用"
      : "过期"
    : "无";
  const pipelineCheckpointClass = pipelineStatus?.has_checkpoint
    ? pipelineStatus.checkpoint_valid
      ? "text-warning"
      : "text-error"
    : "";

  if (panelsLoading) {
    return (
      <div className="flex h-full min-h-0 items-center justify-center gap-2 text-xs text-book-text-muted">
        <Loader2 size={13} className="animate-spin" /> 加载整页提示词…
      </div>
    );
  }

  if (pagePrompts.length === 0) {
    return (
      <div className="flex h-full min-h-0 items-center justify-center">
        <EmptyHint text="本章尚未生成整页漫画提示词。" />
      </div>
    );
  }

  const renderGenerateButton = (page: MangaPagePrompt) => {
    if (!canEditManga || !onGeneratePageImage) return null;
    const generating = generatingPageNumber === page.pageNumber;
    const generated = page.prompt.generated_image_id != null;
    return (
      <div className="flex flex-wrap items-center justify-end gap-2">
        {onPreviewPageImage ? (
          <BookButton
            size="sm"
            variant="secondary"
            onClick={() => void onPreviewPageImage(page.pageNumber)}
            disabled={actionsDisabled || chapterLocked || generating || generatingPageNumber != null}
          >
            <Eye size={13} />
            预览请求
          </BookButton>
        ) : null}
        <BookButton
          size="sm"
          variant="primary"
          onClick={() => void onGeneratePageImage(page.pageNumber)}
          disabled={actionsDisabled || chapterLocked || generating || generatingPageNumber != null}
        >
          {generating ? <Loader2 size={13} className="animate-spin" /> : <ImagePlus size={13} />}
          {generated ? "重新生成图片" : "生成图片"}
        </BookButton>
      </div>
    );
  };

  const beginPromptEdit = (page: MangaPagePrompt) => {
    if (!canEditManga) return;
    setEditingPromptId(page.prompt.id);
    setPromptDraft(page.prompt.image_prompt);
  };

  const cancelPromptEdit = () => {
    setEditingPromptId(null);
    setPromptDraft("");
  };

  const savePromptEdit = async (page: MangaPagePrompt) => {
    if (!canEditManga) {
      setEditingPromptId(null);
      setPromptDraft("");
      setPromptSaving(false);
      return;
    }
    const nextPrompt = promptDraft.trim();
    if (!nextPrompt || !onUpdatePrompt) return;
    setPromptSaving(true);
    try {
      await onUpdatePrompt(page.prompt.id, nextPrompt);
      setEditingPromptId(null);
      setPromptDraft("");
    } catch {
      // 调用方负责 toast；这里保留编辑状态，方便继续修改后重试。
    } finally {
      setPromptSaving(false);
    }
  };

  const activePromptDownstreamHint =
    activePage?.prompt.generated_image_id != null
      ? `该提示词已派生成图片 #${activePage.prompt.generated_image_id}，修改、优化或删除时需要选择如何处理下游图片`
      : undefined;
  const optimizingActivePrompt =
    activePage != null && optimizingPromptId === activePage.prompt.id;

  return (
    <div className="flex h-full min-h-0 flex-col">
        <div className="mb-2 flex shrink-0 flex-wrap items-center justify-between gap-2 text-xs">
          <div className="flex items-center gap-2 text-book-text-sub">
            <span className="font-semibold text-book-text-main">整页漫画</span>
            <span>{pagePrompts.length} 页提示词</span>
            <span>{linkedImageCount}/{pagePrompts.length} 页已生成图片</span>
            {bindingCount > 0 ? <span>{boundBindingCount}/{bindingCount} 参考图</span> : null}
            {pipelineStatus ? (
              <>
                <span
                  className={pipelineCompleted > 0 ? "font-medium text-success" : undefined}
                >
                  完成 {pipelineCompleted}/{pipelinePromptCount || pagePrompts.length}
                </span>
                <span className={pipelineFailed > 0 ? "font-medium text-error" : undefined}>
                  失败 {pipelineFailed}
                </span>
                <span className={pipelineCheckpointClass}>断点 {pipelineCheckpointLabel}</span>
              </>
            ) : null}
            {staleCount > 0 ? (
              <span className="inline-flex items-center gap-1 text-error">
                <AlertTriangle size={12} /> {staleCount} 页需更新
              </span>
            ) : null}
          </div>
          {activePage ? (
            <div className="font-mono text-[11px] text-book-text-muted">
              P{activePage.pageNumber} · prompt_id {activePage.prompt.id}
            </div>
          ) : null}
        </div>

        <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 xl:grid-cols-[220px_minmax(0,1fr)]">
          <aside className="min-h-[126px] overflow-y-auto border-book-border/50 pr-0 custom-scrollbar xl:min-h-0 xl:border-r xl:pr-2">
            <div className="grid grid-cols-2 gap-1.5 sm:grid-cols-3 xl:grid-cols-1">
              {pagePrompts.map((page) => {
                const selected = page.pageNumber === activePage?.pageNumber;
                const imageLabel =
                  page.prompt.generated_image_id != null
                    ? `图 ${page.prompt.generated_image_id}`
                    : "未生成";
                return (
                  <button
                    key={page.pageNumber}
                    type="button"
                    onClick={() => setSelectedPage(page.pageNumber)}
                    className={`rounded border px-2.5 py-2 text-left text-xs transition-colors ${
                      selected
                        ? "border-book-primary bg-book-primary/10 text-book-primary"
                        : "border-book-border/50 bg-book-bg-paper/50 text-book-text-sub hover:border-book-primary/50 hover:text-book-text-main"
                    }`}
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-mono text-sm font-semibold">P{page.pageNumber}</span>
                      <span
                        className={
                          selected
                            ? "text-book-primary"
                            : page.prompt.generated_image_id != null
                              ? "text-success"
                              : "text-book-text-muted"
                        }
                      >
                        {imageLabel}
                      </span>
                    </div>
                    <div
                      className={`mt-1 flex items-center gap-1.5 text-[10px] ${
                        selected ? "text-book-primary" : "text-book-text-muted"
                      }`}
                    >
                      <span>{statusLabel(page.prompt.status)}</span>
                      {page.promptCount > 1 ? <span>{page.promptCount} 记录</span> : null}
                      {page.sourceStale ? <span>需更新</span> : null}
                    </div>
                  </button>
                );
              })}
            </div>
          </aside>

          <section className="min-h-0 overflow-y-auto custom-scrollbar">
            {activePage ? (
              <div className="space-y-3">
                <div className="a4n-page-surface p-3">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <h4 className="font-mono text-base font-semibold text-book-text-main">
                          P{activePage.pageNumber}
                        </h4>
                        <PageStatusPill prompt={activePage.prompt} />
                        <span className="pyqt-pill !text-[10px]">
                          <Image size={11} />
                          {activePage.prompt.generated_image_id != null
                            ? `图片 #${activePage.prompt.generated_image_id}`
                            : "未生成图片"}
                        </span>
                        {activePage.sourceStale ? (
                          <span className="pyqt-pill !border-error/50 !text-[10px] !text-error">
                            <AlertTriangle size={11} />
                            来源旧
                          </span>
                        ) : null}
                        {activePage.promptCount > 1 ? (
                          <span className="pyqt-pill !border-warning/50 !text-[10px] !text-warning">
                            {activePage.promptCount} 记录
                          </span>
                        ) : null}
                      </div>
                      <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-book-text-muted">
                        <span>prompt_id {activePage.prompt.id}</span>
                        <span>{activePage.prompt.prompt_type}</span>
                        {activePage.source?.content_chars != null ? (
                          <span>来源正文 {activePage.source.content_chars} 字符</span>
                        ) : null}
                      </div>
                    </div>
                    <div className="flex flex-wrap items-center justify-end gap-2">
                      {renderGenerateButton(activePage)}
                      {canEditManga && onDeletePrompt ? (
                        <BookButton
                          size="sm"
                          variant="danger"
                          onClick={() => void onDeletePrompt(activePage.prompt)}
                          disabled={actionsDisabled || chapterLocked}
                          title={
                            activePromptDownstreamHint ?? "删除当前漫画整页提示词"
                          }
                        >
                          <Trash2 size={13} />
                          删除
                        </BookButton>
                      ) : null}
                    </div>
                  </div>
                </div>

                <div
                  role="tablist"
                  aria-label="整页漫画查看方式"
                  className="flex flex-wrap items-center gap-px border-b border-book-border/55"
                >
                  <button
                    type="button"
                    role="tab"
                    aria-selected={detailView === "prompt"}
                    data-active={detailView === "prompt" ? "1" : undefined}
                    className="pyqt-tab"
                    onClick={() => setDetailView("prompt")}
                  >
                    提示词与参考图
                  </button>
                  <button
                    type="button"
                    role="tab"
                    aria-selected={detailView === "image"}
                    data-active={detailView === "image" ? "1" : undefined}
                    className="pyqt-tab"
                    onClick={() => setDetailView("image")}
                  >
                    成图
                  </button>
                </div>

                {detailView === "image" ? (
                  <MangaPageImagePreview
                    page={activePage}
                    image={activePageImage}
                    loading={assetsLoading}
                    onOpenImage={setImagePreview}
                  />
                ) : (
                  <div className="grid grid-cols-1 gap-3 2xl:grid-cols-[minmax(0,1.08fr)_minmax(320px,0.92fr)]">
                    <section className="a4n-page-surface min-w-0 p-3">
                      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
                        <div className="flex items-center gap-1.5 text-[11px] font-semibold text-book-text-muted">
                          <FileText size={12} />
                          整页生成提示词
                        </div>
                        {canEditManga && editingPromptId === activePage.prompt.id ? (
                          <div className="flex items-center gap-1.5">
                            <BookButton
                              size="sm"
                              variant="secondary"
                              onClick={cancelPromptEdit}
                              disabled={actionsDisabled || promptSaving}
                            >
                              <X size={12} />
                              取消
                            </BookButton>
                            <BookButton
                              size="sm"
                              variant="primary"
                              onClick={() => void savePromptEdit(activePage)}
                              disabled={
                                actionsDisabled ||
                                promptSaving ||
                                !promptDraft.trim() ||
                                promptDraft.trim() === activePage.prompt.image_prompt.trim()
                              }
                              title={activePromptDownstreamHint}
                            >
                              {promptSaving ? (
                                <Loader2 size={12} className="animate-spin" />
                              ) : (
                                <Save size={12} />
                              )}
                              保存
                            </BookButton>
                          </div>
                        ) : canEditManga ? (
                          <div className="flex items-center gap-1.5">
                            <BookButton
                              size="sm"
                              variant="secondary"
                              onClick={() => void onOptimizePrompt?.(activePage.prompt)}
                              disabled={
                                chapterLocked ||
                                actionsDisabled ||
                                !onOptimizePrompt ||
                                optimizingActivePrompt
                              }
                              title={
                                activePromptDownstreamHint ?? "按用户要求优化当前漫画整页提示词"
                              }
                            >
                              {optimizingActivePrompt ? (
                                <Loader2 size={12} className="animate-spin" />
                              ) : (
                                <Wand2 size={12} />
                              )}
                              优化
                            </BookButton>
                            <BookButton
                              size="sm"
                              variant="secondary"
                              onClick={() => beginPromptEdit(activePage)}
                              disabled={
                                chapterLocked ||
                                actionsDisabled ||
                                !onUpdatePrompt
                              }
                              title={activePromptDownstreamHint}
                            >
                              <Pencil size={12} />
                              编辑
                            </BookButton>
                          </div>
                        ) : null}
                      </div>
                      {canEditManga && editingPromptId === activePage.prompt.id ? (
                        <textarea
                          value={promptDraft}
                          onChange={(event) => setPromptDraft(event.target.value)}
                          className="pyqt-input min-h-[260px] max-h-[52vh] w-full resize-y font-mono text-[12px] leading-relaxed"
                          disabled={actionsDisabled || promptSaving}
                        />
                      ) : (
                        <pre className="max-h-[40vh] min-h-[220px] overflow-y-auto whitespace-pre-wrap break-words rounded border border-book-border/45 bg-book-bg-tertiary/40 p-3 text-[12px] leading-relaxed text-book-text-main custom-scrollbar">
                          {activePage.prompt.image_prompt}
                        </pre>
                      )}
                    </section>

                    <div className="min-w-0 space-y-3">
                      <section>
                        <div className="mb-2 flex items-center gap-1.5 text-[11px] font-semibold text-book-text-muted">
                          <Link2 size={12} />
                          参考图绑定
                        </div>
                        <ReferenceBindingsPanel
                          page={activePage}
                          profiles={profiles}
                          portraits={portraits}
                          projectImages={projectImages}
                          assetsLoading={assetsLoading}
                          chapterLocked={chapterLocked || actionsDisabled || !canEditManga}
                          selectedCandidateBySlot={selectedCandidateBySlot}
                          onSelectCandidate={(slotKey, value) =>
                            setSelectedCandidateBySlot((prev) => ({ ...prev, [slotKey]: value }))
                          }
                          onBindReferenceImage={onBindReferenceImage}
                        />
                      </section>

                      <section>
                        <div className="mb-2 flex items-center gap-1.5 text-[11px] font-semibold text-book-text-muted">
                          <FileText size={12} />
                          构图设计
                        </div>
                        <LayoutSummaryGrid items={activePage.layoutItems} />
                      </section>

                      <section className="grid grid-cols-1 gap-2 md:grid-cols-2 2xl:grid-cols-1">
                        <div className="a4n-stat-tile p-3">
                          <div className="mb-1 flex items-center gap-1.5 text-[10px] font-semibold text-book-text-muted">
                            <Users size={11} />
                            角色
                          </div>
                          <div className="text-xs leading-relaxed text-book-text-main">
                            {activePage.characterNames.length > 0
                              ? activePage.characterNames.join("、")
                              : "—"}
                          </div>
                        </div>
                        <div className="a4n-stat-tile p-3">
                          <div className="mb-1 flex items-center gap-1.5 text-[10px] font-semibold text-book-text-muted">
                            <MessageSquareText size={11} />
                            文字
                          </div>
                          <div className="line-clamp-4 text-xs leading-relaxed text-book-text-main">
                            {activePage.dialogueText ?? "—"}
                          </div>
                        </div>
                      </section>

                      <section className="a4n-stat-tile p-3">
                        <div className="mb-1 text-[10px] font-semibold text-book-text-muted">
                          正文来源
                        </div>
                        <div className="truncate font-mono text-[11px] text-book-text-sub">
                          {activePage.source?.content_sha256 ?? "—"}
                        </div>
                      </section>
                    </div>
                  </div>
                )}
              </div>
            ) : null}
          </section>
        </div>
        <Modal
          isOpen={imagePreview != null}
          onClose={() => setImagePreview(null)}
          title={imagePreview ? `第 ${imagePreview.pageNumber} 页成图` : "成图"}
          subtitle={
            imagePreview ? (
              <span className="font-mono">
                #{imagePreview.image.id} · {imagePreview.image.model} · {imagePreview.image.width}x
                {imagePreview.image.height}
              </span>
            ) : null
          }
          icon={<Image size={15} />}
          maxWidthClassName="max-w-6xl"
          bodyClassName="bg-book-bg-tertiary p-0"
        >
          {imagePreview ? (
            <div className="flex min-h-[60vh] items-center justify-center p-3 sm:p-5">
              <img
                src={imagePreview.src}
                alt={`第 ${imagePreview.pageNumber} 页漫画成图`}
                className="max-h-[78vh] w-full object-contain"
              />
            </div>
          ) : null}
        </Modal>
    </div>
  );
};
