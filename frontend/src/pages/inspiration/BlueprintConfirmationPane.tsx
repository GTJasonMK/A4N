import React, { useRef, useState } from "react";
import { AlertTriangle, ArrowLeft, Check, Eye, RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { Modal } from "../../components/ui/Modal";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import type { Blueprint } from "../../api/blueprint";
import { Section } from "./shared";
import { useI18n } from "../../i18n";
import { useNovelDetailDirty } from "../novel-detail/novelDetailDirty";

interface BlueprintConfirmationPaneProps {
  blueprint: Blueprint;
  consistencyWarning?: string | null;
  refining: boolean;
  onRefine: (focus: string) => void;
  onPreviewRefine: (focus: string) => void;
  onConfirm: () => void;
  onCancel: () => void;
}

/** 蓝图确认页：正文负责阅读，底栏只突出最终确认。 */
export const BlueprintConfirmationPane: React.FC<BlueprintConfirmationPaneProps> = ({
  blueprint,
  consistencyWarning,
  refining,
  onRefine,
  onPreviewRefine,
  onConfirm,
  onCancel,
}) => {
  const { t } = useI18n();
  const [refineFocus, setRefineFocus] = useState("");
  useNovelDetailDirty("inspiration-blueprint-refine", refineFocus.trim().length > 0);
  const [refineModalOpen, setRefineModalOpen] = useState(false);
  const refineInputRef = useRef<HTMLTextAreaElement>(null);
  const refineReady = refineFocus.trim().length > 0;
  const closeRefineModal = async () => {
    if (refining) return;
    if (refineFocus.trim() && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("inspiration.adjustBlueprint"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    setRefineModalOpen(false);
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-5 sm:px-6 sm:py-6">
        <div className="mx-auto max-w-3xl">
          <header className="mb-4">
            <h2 className="font-serif text-lg font-semibold text-book-text-main">{t("inspiration.storyBlueprint")}</h2>
            <p className="mt-1 text-xs leading-relaxed text-book-text-muted">
              {[blueprint.genre, blueprint.style].filter(Boolean).join(" · ") || t("inspiration.firstStoryBlueprint")}
              <span className="mx-1.5 text-book-border">·</span>
              {consistencyWarning ? t("inspiration.readonlyBlueprint") : t("inspiration.confirmToProject")}
            </p>
          </header>

          {consistencyWarning ? (
            <div className="mb-4 flex items-start gap-2 border border-danger/35 bg-danger/5 px-3 py-2.5 text-sm text-danger">
              <AlertTriangle size={15} className="mt-0.5 shrink-0" />
              <span>{consistencyWarning}</span>
            </div>
          ) : null}

          <section className="a4n-flat-surface mb-4 overflow-hidden px-4 py-4 sm:px-5 sm:py-5">
            {blueprint.one_sentence_summary ? (
              <Section label={t("inspiration.storyCore")}>
                <div className="font-medium leading-relaxed">
                  {blueprint.one_sentence_summary}
                </div>
              </Section>
            ) : null}
            {blueprint.one_sentence_summary ? <div className="story-divider my-4" /> : null}
            <Section label={t("inspiration.fullSynopsis")}>
              <div className="whitespace-pre-wrap leading-relaxed">
                {blueprint.full_synopsis ||
                  t("inspiration.missingSynopsis")}
              </div>
            </Section>
          </section>

        </div>
      </div>

      <div className="shrink-0 border-t border-book-border/40 bg-book-bg-paper/90 px-4 py-3 backdrop-blur-xl sm:px-6">
        <div className="mx-auto flex max-w-3xl items-center gap-2">
          <Dropdown
            items={[
              {
                label: t("inspiration.backToChat"),
                icon: <ArrowLeft size={12} />,
                onClick: onCancel,
                disabled: refining,
              },
            ]}
          />
          {!consistencyWarning ? (
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => setRefineModalOpen(true)}
              disabled={refining}
              className="!text-xs"
            >
              <RefreshCw size={13} /> {t("inspiration.refine")}
            </BookButton>
          ) : null}
          {!consistencyWarning ? (
            <BookButton
              variant="primary"
              size="sm"
              onClick={onConfirm}
              disabled={refining}
              className="min-w-0 flex-1 !text-xs sm:ml-auto sm:flex-none"
            >
              <Check size={13} /> {t("inspiration.confirmProject")}
            </BookButton>
          ) : null}
        </div>
      </div>

      <Modal
        isOpen={refineModalOpen && !consistencyWarning}
        onClose={() => void closeRefineModal()}
        title={t("inspiration.adjustBlueprint")}
        icon={<RefreshCw size={15} />}
        maxWidthClassName="max-w-md"
        bodyClassName="px-4 py-4 sm:px-5"
        initialFocusRef={refineInputRef}
        footer={(
          <div className="flex w-full min-w-0 items-center gap-2 sm:justify-end">
            {!refining && refineReady ? (
              <Dropdown
                items={[
                  {
                    label: t("inspiration.viewRefineRequest"),
                    icon: <Eye size={12} />,
                    onClick: () => onPreviewRefine(refineFocus.trim()),
                  },
                ]}
              />
            ) : null}
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => {
                const focus = refineFocus.trim();
                if (!focus) return;
                onRefine(focus);
                setRefineModalOpen(false);
              }}
              disabled={refining || !refineReady}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {refining ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> {t("inspiration.refining")}
                </span>
              ) : (
                t("inspiration.submitRefine")
              )}
            </BookButton>
          </div>
        )}
      >
        <textarea
          ref={refineInputRef}
          value={refineFocus}
          onChange={(e) => setRefineFocus(e.target.value)}
          placeholder={t("inspiration.refinePlaceholder")}
          aria-label={t("inspiration.adjustBlueprint")}
          className="pyqt-input min-h-[112px] w-full resize-y !rounded-[14px] px-3 py-2.5 text-sm leading-relaxed"
          disabled={refining}
        />
      </Modal>
    </div>
  );
};
