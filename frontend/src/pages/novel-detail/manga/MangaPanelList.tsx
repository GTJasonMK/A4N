import React, { useEffect, useMemo, useState } from "react";
import { FileText,
  Image,
  ImagePlus,
  Link2,
  MessageSquareText,
  Pencil,
  RefreshCw,
  Save,
  Trash2,
  Users,
  Wand2,
  X,
  Eye, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type { Portrait } from "../../../api/portrait";
import type { BindMangaReferenceImageInput, MangaPanel, PipelineStatus } from "../../../api/manga";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { EmptyHint } from "../shared";
import {
  MangaPageImagePreview,
  type ImagePreviewTarget,
} from "./MangaPageImagePreview";
import {
  LayoutSummaryGrid,
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
  const hasMultiplePages = pagePrompts.length > 1;
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
  if (panelsLoading) {
    return (
      <div className="flex h-full min-h-0 items-center justify-center gap-2 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" /> 加载整页提示词…
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
    if (generated && !page.sourceStale) return null;
    return (
      <BookButton
        size="sm"
        variant={generated ? "secondary" : "primary"}
        onClick={() => void onGeneratePageImage(page.pageNumber)}
        disabled={actionsDisabled || chapterLocked || generating || generatingPageNumber != null}
      >
        {generating ? (
          <LoadingSpinner size="sm" />
        ) : generated ? (
          <RefreshCw size={13} />
        ) : (
          <ImagePlus size={13} />
        )}
        {generated ? "更新图片" : "生成图片"}
      </BookButton>
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
      ? "该提示词已有成图，修改、优化或删除时需要选择如何处理下游图片"
      : undefined;
  const optimizingActivePrompt =
    activePage != null && optimizingPromptId === activePage.prompt.id;
  const pageSummaryParts = [
    `${pagePrompts.length} 页`,
    `成图 ${linkedImageCount}/${pagePrompts.length}`,
    bindingCount > 0 ? `参考图 ${boundBindingCount}/${bindingCount}` : "",
    pipelineStatus
      ? `流水线 ${pipelineCompleted}/${pipelinePromptCount || pagePrompts.length}`
      : "",
    pipelineFailed > 0 ? `失败 ${pipelineFailed}` : "",
    pipelineStatus?.has_checkpoint ? `断点 ${pipelineCheckpointLabel}` : "",
    staleCount > 0 ? `${staleCount} 页需更新` : "",
  ].filter(Boolean);
  const activePromptMenuItems: DropdownItem[] = [];
  const activePageMenuItems: DropdownItem[] = [];
  if (activePage && canEditManga) {
    if (onPreviewPageImage) {
      activePageMenuItems.push({
        label: "查看生成图片请求",
        icon: <Eye size={12} />,
        disabled:
          actionsDisabled ||
          chapterLocked ||
          generatingPageNumber === activePage.pageNumber ||
          generatingPageNumber != null,
        onClick: () => void onPreviewPageImage(activePage.pageNumber),
      });
    }
    if (
      onGeneratePageImage &&
      activePage.prompt.generated_image_id != null &&
      !activePage.sourceStale
    ) {
      activePageMenuItems.push({
        label: "重新生成图片",
        icon: <RefreshCw size={12} />,
        disabled:
          actionsDisabled ||
          chapterLocked ||
          generatingPageNumber === activePage.pageNumber ||
          generatingPageNumber != null,
        onClick: () => void onGeneratePageImage(activePage.pageNumber),
      });
    }
    if (onDeletePrompt && !actionsDisabled && !chapterLocked) {
      if (activePageMenuItems.length > 0) activePageMenuItems.push({ type: "divider" });
      activePageMenuItems.push({
        label: "删除当前页提示词",
        icon: <Trash2 size={12} />,
        danger: true,
        onClick: () => void onDeletePrompt(activePage.prompt),
      });
    }
  }
  if (activePage && canEditManga && editingPromptId !== activePage.prompt.id) {
    if (onOptimizePrompt) {
      activePromptMenuItems.push({
        label: "优化提示词",
        icon: <Wand2 size={12} />,
        disabled: chapterLocked || actionsDisabled || optimizingActivePrompt,
        onClick: () => void onOptimizePrompt(activePage.prompt),
      });
    }
    if (onUpdatePrompt) {
      activePromptMenuItems.push({
        label: "编辑提示词",
        icon: <Pencil size={12} />,
        disabled: chapterLocked || actionsDisabled,
        onClick: () => beginPromptEdit(activePage),
      });
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
        <div className="mb-3 shrink-0">
          <h2 className="text-sm font-semibold text-book-text-main">整页漫画</h2>
          <p
            className={`mt-1 text-small leading-relaxed ${
              pipelineFailed > 0 || staleCount > 0
                ? "text-warning"
                : pipelineCompleted > 0
                  ? "text-book-text-sub"
                  : "text-book-text-muted"
            }`}
          >
            {pageSummaryParts.join(" · ")}
          </p>
        </div>

        <div
          className={`grid min-h-0 grid-cols-1 content-start gap-3 xl:flex-1 xl:content-stretch ${
            hasMultiplePages ? "xl:grid-cols-[180px_minmax(0,1fr)]" : ""
          }`}
        >
          {hasMultiplePages ? (
            <nav
              aria-label="漫画页列表"
              className="min-w-0 overflow-x-auto pb-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden xl:min-h-0 xl:overflow-x-hidden xl:overflow-y-auto xl:border-r xl:border-book-border/50 xl:pb-0 xl:pr-2"
            >
              <div className="flex min-w-max gap-1.5 xl:min-w-0 xl:flex-col xl:gap-0 xl:overflow-hidden xl:rounded-xl xl:border xl:border-book-border/50 xl:bg-book-bg-paper/30">
                {pagePrompts.map((page) => {
                  const selected = page.pageNumber === activePage?.pageNumber;
                  const imageLabel =
                    page.prompt.generated_image_id != null
                      ? "已生成"
                      : "未生成";
                  const pageStatus = statusLabel(page.prompt.status);
                  const pageMeta = [
                    imageLabel,
                    pageStatus !== "完成" && pageStatus !== "待生成" ? pageStatus : "",
                    page.promptCount > 1 ? `${page.promptCount} 条记录` : "",
                    page.sourceStale ? "需更新" : "",
                  ].filter(Boolean);
                  return (
                    <button
                      key={page.pageNumber}
                      type="button"
                      onClick={() => setSelectedPage(page.pageNumber)}
                      aria-current={selected ? "true" : undefined}
                      className={`min-w-[8.75rem] rounded-xl border px-3 py-2 text-left text-xs transition-colors xl:w-full xl:min-w-0 xl:rounded-none xl:border-0 ${
                        selected
                          ? "border-book-primary/55 bg-book-primary/10 text-book-primary xl:shadow-[inset_2px_0_0_rgb(var(--color-primary))]"
                          : "border-book-border/50 bg-book-bg-paper/45 text-book-text-sub hover:border-book-primary/35 hover:bg-book-bg-tertiary/55 hover:text-book-text-main xl:border-transparent xl:bg-transparent"
                      }`}
                    >
                      <span className="block text-xs font-semibold">第 {page.pageNumber} 页</span>
                      <div
                        className={`mt-0.5 truncate text-caption ${
                          selected ? "text-book-primary" : "text-book-text-muted"
                        }`}
                      >
                        {pageMeta.join(" · ")}
                      </div>
                    </button>
                  );
                })}
              </div>
            </nav>
          ) : null}

          <section className="min-h-0 overflow-visible xl:overflow-y-auto custom-scrollbar">
            {activePage ? (
              <article className="a4n-flat-surface min-w-0 overflow-hidden">
                <header className="border-b border-book-border/40 p-3">
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <div className="min-w-0">
                      <h3 className="text-sm font-semibold text-book-text-main">
                        第 {activePage.pageNumber} 页
                      </h3>
                      <p className="mt-1 text-small text-book-text-muted">
                        {[
                          statusLabel(activePage.prompt.status),
                          activePage.prompt.generated_image_id != null ? "已有成图" : "尚未生成",
                          activePage.sourceStale ? "来源需更新" : "",
                          activePage.promptCount > 1 ? `${activePage.promptCount} 条记录` : "",
                        ].filter(Boolean).join(" · ")}
                      </p>
                    </div>
                    <div className="flex items-center justify-end gap-2">
                      {generatingPageNumber === activePage.pageNumber &&
                      activePage.prompt.generated_image_id != null &&
                      !activePage.sourceStale ? (
                        <span
                          role="status"
                          className="inline-flex min-h-8 items-center gap-1.5 px-1 text-small text-book-text-muted"
                        >
                          <LoadingSpinner size="sm" /> 正在重新生成
                        </span>
                      ) : null}
                      {activePageMenuItems.length > 0 ? <Dropdown items={activePageMenuItems} /> : null}
                      {renderGenerateButton(activePage)}
                    </div>
                  </div>
                </header>

                <div className="border-b border-book-border/40 px-3 py-2">
                  <SegmentedControl
                    ariaLabel="整页漫画查看方式"
                    value={detailView}
                    onChange={setDetailView}
                    options={[
                      { id: "prompt", label: "提示词与参考图" },
                      { id: "image", label: "成图" },
                    ]}
                  />
                </div>

                <div className="p-3">
                  {detailView === "image" ? (
                    <MangaPageImagePreview
                      page={activePage}
                      image={activePageImage}
                      loading={assetsLoading}
                      onOpenImage={setImagePreview}
                    />
                  ) : (
                    <div className="grid grid-cols-1 gap-5 2xl:grid-cols-[minmax(0,1.08fr)_minmax(320px,0.92fr)]">
                    <section className="min-w-0">
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <h3 className="flex items-center gap-1.5 text-xs font-semibold text-book-text-main">
                          <FileText size={12} className="text-book-primary" />
                          整页生成提示词
                        </h3>
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
                                <LoadingSpinner size="sm" />
                              ) : (
                                <Save size={12} />
                              )}
                              保存
                            </BookButton>
                          </div>
                        ) : activePromptMenuItems.length > 0 ? (
                          <Dropdown items={activePromptMenuItems} />
                        ) : null}
                      </div>
                      {canEditManga && editingPromptId === activePage.prompt.id ? (
                        <textarea
                          value={promptDraft}
                          onChange={(event) => setPromptDraft(event.target.value)}
                          className="pyqt-input min-h-[260px] max-h-[52vh] w-full resize-y font-mono text-body-sm leading-relaxed"
                          disabled={actionsDisabled || promptSaving}
                        />
                      ) : (
                        <pre className="min-h-[220px] whitespace-pre-wrap break-words border-l-2 border-book-border/70 bg-book-bg-tertiary/25 py-2 pl-3 pr-2 text-body-sm leading-relaxed text-book-text-main">
                          {activePage.prompt.image_prompt}
                        </pre>
                      )}
                    </section>

                    <div className="min-w-0 divide-y divide-book-border/40 border-y border-book-border/40">
                      <section className="py-3 first:pt-0">
                        <h3 className="mb-2 flex items-center gap-1.5 text-xs font-semibold text-book-text-sub">
                          <Link2 size={12} />
                          参考图绑定
                        </h3>
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

                      <section className="py-3">
                        <h3 className="mb-2 flex items-center gap-1.5 text-xs font-semibold text-book-text-sub">
                          <FileText size={12} />
                          构图设计
                        </h3>
                        <LayoutSummaryGrid items={activePage.layoutItems} />
                      </section>

                      <section className="divide-y divide-book-border/35 py-3">
                        <div className="grid gap-1 px-3 py-2 md:grid-cols-[88px_minmax(0,1fr)] md:gap-3">
                          <div className="flex items-center gap-1.5 text-small font-medium text-book-text-muted">
                            <Users size={11} />
                            角色
                          </div>
                          <div className="text-xs leading-relaxed text-book-text-main">
                            {activePage.characterNames.length > 0
                              ? activePage.characterNames.join("、")
                              : "—"}
                          </div>
                        </div>
                        <div className="grid gap-1 px-3 py-2 md:grid-cols-[88px_minmax(0,1fr)] md:gap-3">
                          <div className="flex items-center gap-1.5 text-small font-medium text-book-text-muted">
                            <MessageSquareText size={11} />
                            文字
                          </div>
                          <div className="line-clamp-4 text-xs leading-relaxed text-book-text-main">
                            {activePage.dialogueText ?? "—"}
                          </div>
                        </div>
                      </section>
                    </div>
                  </div>
                  )}
                </div>
              </article>
            ) : null}
          </section>
        </div>
        <Modal
          isOpen={imagePreview != null}
          onClose={() => setImagePreview(null)}
          title={imagePreview ? `第 ${imagePreview.pageNumber} 页成图` : "成图"}
          subtitle={
            imagePreview
              ? `${imagePreview.image.width}×${imagePreview.image.height}`
              : null
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
