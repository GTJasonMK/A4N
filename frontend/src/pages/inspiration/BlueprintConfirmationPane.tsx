import React, { useState } from "react";
import { AlertTriangle, ArrowLeft, Check, Eye, RefreshCw, X } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton, BookIconButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import type { Blueprint } from "../../api/blueprint";
import { Section } from "./shared";
import { useI18n } from "../../i18n";

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
  const [showRefineInput, setShowRefineInput] = useState(false);
  const refineReady = refineFocus.trim().length > 0;

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

          {!consistencyWarning && showRefineInput && (
            <section className="a4n-flat-surface mb-4 overflow-hidden p-4 sm:p-5">
              <div className="mb-3 flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <h3 className="text-sm font-semibold text-book-text-main">{t("inspiration.adjustBlueprint")}</h3>
                  <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
                    {t("inspiration.adjustHint")}
                  </p>
                </div>
                <BookIconButton
                  label={t("inspiration.collapseAdjust")}
                  size="sm"
                  onClick={() => setShowRefineInput(false)}
                  disabled={refining}
                >
                  <X size={13} />
                </BookIconButton>
              </div>
              <textarea
                autoFocus
                value={refineFocus}
                onChange={(e) => setRefineFocus(e.target.value)}
                placeholder={t("inspiration.refinePlaceholder")}
                className="pyqt-input min-h-[96px] w-full resize-y !rounded-[14px] px-3 py-2.5 text-sm leading-relaxed"
                disabled={refining}
              />
              <div className="mt-3 flex items-center gap-2">
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
                    setShowRefineInput(false);
                    setRefineFocus("");
                  }}
                  disabled={refining || !refineReady}
                  className="min-w-0 flex-1 !text-xs sm:ml-auto sm:flex-none"
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
            </section>
          )}
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
          {!consistencyWarning && !showRefineInput ? (
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => setShowRefineInput(true)}
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
    </div>
  );
};
