import React, { useState } from "react";
import { ListChecks, Loader2 } from "lucide-react";
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

/**
 * 灵感对话页。
 *
 * 页面只负责三阶段布局，项目加载、流式对话、蓝图生成与取消逻辑集中在
 * useInspirationChatController。
 */
export const InspirationChat: React.FC = () => {
  const chat = useInspirationChatController();
  const [progressOpen, setProgressOpen] = useState(false);

  if (!chat.project) {
    return (
      <div className="h-full flex items-center justify-center text-book-text-muted">
        <Loader2 size={20} className="animate-spin mr-2" /> 加载项目…
      </div>
    );
  }

  return (
    <div className="h-full w-full flex flex-col">
      <InspirationChatHeader
        projectTitle={chat.project.title}
        showConversationControls={chat.stage === "conversation"}
        generating={chat.generating}
        canTryGenerate={chat.canTryGenerate}
        canGenerate={chat.canGenerate}
        onOpenProgress={() => setProgressOpen(true)}
        onGenerateBlueprint={() => void chat.handleGenerateBlueprint()}
        onPreviewGenerateBlueprint={() => void chat.handlePreviewGenerateBlueprint()}
      />

      <div className="flex-1 min-h-0 flex">
        <div className="flex-1 min-h-0">
          {chat.stage === "conversation" && (
            <div className="flex h-full min-h-0">
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
                  showThinking={chat.sending}
                  messagesEndRef={chat.messagesEndRef}
                  placeholder={chat.inputPlaceholder}
                  inputActive={chat.inputActive}
                />
              </div>
              <InspirationGuidePanel
                progressSummaryLines={chat.progressSummaryLines}
                nextQuestion={chat.nextQuestion}
                isReady={chat.conversationComplete}
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
        title="生成首版蓝图"
        subtitle="正在把灵感对话整理为可写作的项目蓝图"
        status={chat.blueprintProgress.status}
        current={chat.blueprintProgress.current}
        total={chat.blueprintProgress.total}
        message={chat.blueprintProgress.message}
        error={chat.blueprintProgress.error}
        streamText={chat.blueprintStreamText}
        streamVersion={chat.blueprintStreamVersion}
        onClose={() => chat.setBlueprintProgressOpen(false)}
        closeLabel={chat.blueprintProgress.status === "success" ? "查看蓝图" : "关闭"}
        onCancel={
          chat.canCancelBlueprintGeneration
            ? () => void chat.handleCancelBlueprint()
            : undefined
        }
        cancelling={chat.blueprintCancelling}
        cancelLabel="停止生成"
      />
      <Modal
        isOpen={progressOpen && chat.stage === "conversation"}
        onClose={() => setProgressOpen(false)}
        title="创作进度"
        subtitle="已确认的信息会随对话自动整理"
        icon={<ListChecks size={15} />}
        maxWidthClassName="max-w-md"
        bodyClassName="px-4 py-4"
      >
        <InspirationProgressContent
          progressSummaryLines={chat.progressSummaryLines}
          nextQuestion={chat.nextQuestion}
          isReady={chat.conversationComplete}
          sending={chat.sending}
        />
      </Modal>
      <RequestPreviewModal
        open={chat.requestPreviewOpen}
        loading={chat.requestPreviewLoading}
        data={chat.requestPreviewData}
        fallbackTitle="灵感对话请求预览"
        onClose={() => chat.setRequestPreviewOpen(false)}
      />
      <RequestPreviewModal
        open={chat.blueprintRequestPreviewOpen}
        loading={chat.blueprintRequestPreviewLoading}
        data={chat.blueprintRequestPreviewData}
        fallbackTitle="蓝图请求预览"
        onClose={() => chat.setBlueprintRequestPreviewOpen(false)}
      />
    </div>
  );
};

export default InspirationChat;
