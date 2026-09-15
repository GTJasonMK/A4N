import React, { useState } from "react";
import { ListChecks } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { ProgressDialog } from "../components/feedback/ProgressDialog";
import { RequestPreviewModal } from "../components/feedback/RequestPreviewModal";
import { Modal } from "../components/ui/Modal";
import { BlueprintConfirmationPane } from "./inspiration/BlueprintConfirmationPane";
import { BlueprintDisplayPane } from "./inspiration/BlueprintDisplayPane";
import { ConversationPane } from "./inspiration/ConversationPane";
import { InspirationChatHeader } from "./inspiration/InspirationChatHeader";
import {
  InspirationGuidePanel,
  InspirationProgressContent,
} from "./inspiration/InspirationGuidePanel";
import { useInspirationChatController } from "./inspiration/useInspirationChatController";
import { useI18n } from "../i18n";

/**
 * 灵感对话页。
 *
 * 页面只负责三阶段布局，项目加载、流式对话、蓝图生成与取消逻辑集中在
 * useInspirationChatController。
 */
export const InspirationChat: React.FC = () => {
  const chat = useInspirationChatController();
  const { t } = useI18n();
  const [progressOpen, setProgressOpen] = useState(false);

  if (!chat.project) {
    return (
      <div className="h-full flex items-center justify-center text-book-text-muted">
        <LoadingSpinner size="lg" className="mr-2" /> {t("inspiration.loadingProject")}
      </div>
    );
  }

  return (
    <div className="h-full w-full min-w-0 flex flex-col">
      <InspirationChatHeader
        projectTitle={chat.project.title}
        showConversationControls={chat.stage === "conversation"}
        generating={chat.generating}
        canGenerate={chat.canGenerate}
        onOpenProgress={() => setProgressOpen(true)}
        onGenerateBlueprint={() => void chat.handleGenerateBlueprint()}
        onPreviewGenerateBlueprint={() => void chat.handlePreviewGenerateBlueprint()}
      />

      <div className="flex min-w-0 flex-1 min-h-0">
        <div className="min-w-0 flex-1 min-h-0">
          {chat.stage === "conversation" && (
            <div className="flex h-full min-h-0 min-w-0">
              <div className="min-w-0 flex-1">
                <ConversationPane
                  messages={chat.messages}
                  input={chat.input}
                  setInput={chat.setInput}
                  inputRef={chat.inputRef}
                  onSend={() => void chat.handleSend()}
                  onPreview={() => void chat.handlePreviewInspirationRequest()}
                  onSendOption={(text) => void chat.handleSend(text)}
                  onKeyDown={chat.handleKeyDown}
                  onCompositionStart={chat.handleCompositionStart}
                  onCompositionEnd={chat.handleCompositionEnd}
                  onInputFocus={chat.handleConversationFocus}
                  onInputBlur={chat.handleConversationBlur}
                  sending={chat.sending}
                  messagesViewportRef={chat.messagesViewportRef}
                  isFollowingLatest={chat.isFollowingLatest}
                  hasNewContent={chat.hasNewContent}
                  onScrollToLatest={chat.scrollToLatest}
                  placeholder={chat.inputPlaceholder}
                  inputActive={chat.inputActive}
                />
              </div>
              <InspirationGuidePanel
                progressSummaryLines={chat.progressSummaryLines}
                nextQuestion={chat.nextQuestion}
                checklistComplete={chat.conversationChecklistComplete}
                sending={chat.sending}
              />
            </div>
          )}

          {chat.stage === "confirm" && chat.blueprint && (
            <BlueprintConfirmationPane
              blueprint={chat.blueprint}
              consistencyWarning={chat.blueprintConsistencyWarning}
              refining={chat.refining}
              onRefine={(focus) => void chat.handleRefineBlueprint(focus)}
              onPreviewRefine={(focus) => void chat.handlePreviewRefineBlueprint(focus)}
              onConfirm={chat.handleConfirmBlueprint}
              onCancel={chat.handleBackToConversation}
            />
          )}

          {chat.stage === "display" && chat.blueprint && (
            <BlueprintDisplayPane blueprint={chat.blueprint} />
          )}
        </div>
      </div>

      <ProgressDialog
        open={chat.blueprintProgressOpen}
        title={
          chat.blueprintOperation === "refinement"
            ? t("inspiration.adjustBlueprint")
            : t("inspiration.blueprintTitle")
        }
        subtitle={
          chat.blueprintOperation === "refinement"
            ? t("inspiration.adjustHint")
            : t("inspiration.blueprintSubtitle")
        }
        status={chat.blueprintProgress.status}
        current={chat.blueprintProgress.current}
        total={chat.blueprintProgress.total}
        message={chat.blueprintProgress.message}
        error={chat.blueprintProgress.error}
        streamText={chat.blueprintStreamText}
        streaming={
          chat.blueprintProgressOpen
          && (chat.blueprintOperation != null || chat.blueprintStreamText.length > 0)
        }
        onClose={chat.closeBlueprintProgress}
        closeLabel={chat.blueprintProgress.status === "success" ? t("inspiration.viewBlueprint") : t("inspiration.close")}
        onCancel={
          chat.canCancelBlueprintOperation
            ? () => void chat.handleCancelBlueprint()
            : undefined
        }
        cancelling={chat.blueprintCancelling}
        cancelLabel={t("inspiration.stopGeneration")}
      />
      <Modal
        isOpen={progressOpen && chat.stage === "conversation"}
        onClose={() => setProgressOpen(false)}
        title={t("inspiration.progressTitle")}
        subtitle={t("inspiration.progressSubtitle")}
        icon={<ListChecks size={15} />}
        maxWidthClassName="max-w-md"
        bodyClassName="px-4 py-4"
      >
        <InspirationProgressContent
          progressSummaryLines={chat.progressSummaryLines}
          nextQuestion={chat.nextQuestion}
          checklistComplete={chat.conversationChecklistComplete}
          sending={chat.sending}
        />
      </Modal>
      <RequestPreviewModal
        open={chat.requestPreviewOpen}
        loading={chat.requestPreviewLoading}
        data={chat.requestPreviewData}
        fallbackTitle={t("inspiration.chatRequestPreview")}
        onClose={() => chat.setRequestPreviewOpen(false)}
      />
      <RequestPreviewModal
        open={chat.blueprintRequestPreviewOpen}
        loading={chat.blueprintRequestPreviewLoading}
        data={chat.blueprintRequestPreviewData}
        fallbackTitle={t("inspiration.blueprintRequestPreview")}
        onClose={() => chat.setBlueprintRequestPreviewOpen(false)}
      />
    </div>
  );
};
