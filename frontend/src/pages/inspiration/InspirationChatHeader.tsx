import React from "react";
import { Eye, ListChecks, Sparkles } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BackButton } from "../../components/ui/BackButton";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { PageHeader } from "../../components/shell/PageHeader";
import { useI18n } from "../../i18n";

interface InspirationChatHeaderProps {
  projectTitle: string;
  showConversationControls: boolean;
  generating: boolean;
  canGenerate: boolean;
  onOpenProgress: () => void;
  onGenerateBlueprint: () => void;
  onPreviewGenerateBlueprint: () => void;
}

/** 灵感对话顶栏：只保留页面身份、响应方式和生成主操作。 */
export const InspirationChatHeader: React.FC<InspirationChatHeaderProps> = ({
  projectTitle,
  showConversationControls,
  generating,
  canGenerate,
  onOpenProgress,
  onGenerateBlueprint,
  onPreviewGenerateBlueprint,
}) => {
  const { t } = useI18n();
  return (
  <PageHeader
    className="a4n-app-page-header--detail h-auto shrink-0 !px-3 py-2 sm:!px-[var(--a4n-content-gutter)]"
    leading={<BackButton iconOnly title={t("nav.backToPrevious")} />}
    title={t("novel.inspiration")}
    subtitle={projectTitle}
    actions={showConversationControls ? (
      <>
        <div className="lg:hidden">
          <Dropdown
            items={[
              {
                label: t("inspiration.progressTitle"),
                icon: <ListChecks size={12} />,
                onClick: onOpenProgress,
              },
              { type: "divider" },
              {
                label: t("inspiration.blueprintRequestPreview"),
                icon: <Eye size={12} />,
                onClick: onPreviewGenerateBlueprint,
                disabled: !canGenerate,
              },
            ]}
          />
        </div>
        <div className="hidden lg:block">
          <Dropdown
            items={[
              {
                label: t("inspiration.blueprintRequestPreview"),
                icon: <Eye size={12} />,
                onClick: onPreviewGenerateBlueprint,
                disabled: !canGenerate,
              },
            ]}
          />
        </div>
        <BookButton
          variant="primary"
          size="sm"
          onClick={onGenerateBlueprint}
          disabled={!canGenerate}
          title={t("inspiration.blueprintTitle")}
          className="min-w-0 flex-1 !min-h-8 !px-2.5 !text-xs sm:flex-none"
        >
          {generating ? (
            <span className="inline-flex items-center gap-1">
              <LoadingSpinner size="sm" />
              <span>{t("workflow.generating")}</span>
            </span>
          ) : (
            <span className="inline-flex items-center gap-1">
              <Sparkles size={12} />
              <span>{t("inspiration.blueprintTitle")}</span>
            </span>
          )}
        </BookButton>
      </>
    ) : undefined}
  />
  );
};
