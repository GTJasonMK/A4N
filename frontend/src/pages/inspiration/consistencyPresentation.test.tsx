import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { Blueprint } from "../../api/blueprint";
import {
  generationTaskRecoveryIssueMessage,
  resolveFormalResultRecovery,
} from "../../utils/generationEventReducer";
import { BlueprintConfirmationPane } from "./BlueprintConfirmationPane";
import { ChatBubble } from "./ChatBubble";
import { ConversationPane } from "./ConversationPane";
import { buildBlueprintGenerationInput } from "./useBlueprintWorkflow";
import { generationFailureMessage } from "./inspirationChatUtils";
import type { Message } from "./shared";
import { I18nProvider, translateMessage } from "../../i18n";

const t = (key: Parameters<typeof translateMessage>[1], params?: Parameters<typeof translateMessage>[2]) =>
  translateMessage("zh-CN", key, params);

describe("generation consistency presentation", () => {
  it("builds a random blueprint request without collected inspiration", () => {
    const input = buildBlueprintGenerationInput({
      project: {
        id: 7,
        title: "随机故事",
        status: "draft",
        project_type: "novel",
        description: "",
        target_chapters: 0,
      },
      messages: [],
      latestConversationState: undefined,
      progressSummary: undefined,
      nextQuestion: undefined,
    });

    expect(input).toMatchObject({
      genre: "自由创作",
      target_length: 100,
      variables: {
        conversation_state: {},
        inspiration_transcript: "",
        generation_instruction: expect.stringContaining("自由随机生成"),
      },
    });
  });

  it("uses collected inspiration without requiring a complete checklist", () => {
    const input = buildBlueprintGenerationInput({
      project: {
        id: 7,
        title: "未完成灵感",
        status: "draft",
        project_type: "novel",
        description: "",
        target_chapters: 60,
      },
      messages: [
        { id: "assistant-1", role: "assistant", content: "先说说你的想法", createdAt: 1 },
        { id: "user-1", role: "user", content: "一座每天都会遗忘居民的城市", createdAt: 2 },
      ],
      latestConversationState: { genre: "悬疑" },
      progressSummary: "已确认核心火花",
      nextQuestion: "主角为什么留在这里？",
    });

    expect(input).toMatchObject({
      genre: "悬疑",
      target_length: 60,
      variables: {
        inspiration_transcript: expect.stringContaining("一座每天都会遗忘居民的城市"),
      },
    });
  });

  it("drops streamed option metadata when an inspiration reply fails", () => {
    const failed = generationFailureMessage("message-options", "生成失败");

    expect(failed).toEqual(expect.objectContaining({
      id: "message-options",
      role: "system",
      content: "生成失败",
    }));
    expect(failed).not.toHaveProperty("streaming");
    expect(failed).not.toHaveProperty("options");
    expect(failed).not.toHaveProperty("streamingOptions");
    expect(failed).not.toHaveProperty("optionsPending");
    expect(failed).not.toHaveProperty("isComplete");
    expect(failed).not.toHaveProperty("rawContent");
  });

  it("renders one meaningful status inside an empty streaming reply", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <ChatBubble
        message={{
          id: "message-streaming",
          role: "assistant",
          content: "",
          streaming: true,
          createdAt: 1,
        }}
        canSelectOptions={false}
        onSendOption={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(html).toContain("正在整理你的灵感…");
    expect(html).toContain('role="status"');
  });

  it("renders completed assistant Markdown as formatted content", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <ChatBubble
          message={{
            id: "message-markdown",
            role: "assistant",
            content: "## 结论\n\n- 第一项\n- **第二项**",
            createdAt: 1,
          }}
          canSelectOptions={false}
          onSendOption={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(html).toContain("<h4");
    expect(html).toContain("<ul");
    expect(html).toContain("<strong>第二项</strong>");
    expect(html).not.toContain("## 结论");
  });

  it("renders non-interactive option cards as disabled", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <ChatBubble
          message={{
            id: "message-options",
            role: "assistant",
            content: "请选择一个方向",
            options: [{
              label: "方向一",
              description: "说明",
              keyElements: ["要素一", "要素二"],
              value: "选择：方向一\n说明\n关键要素：要素一、要素二",
            }],
            createdAt: 1,
          }}
          canSelectOptions={false}
          onSendOption={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(optionCardTags(html)).toEqual([
      expect.stringContaining('disabled=""'),
    ]);
  });

  it("keeps the latest unanswered history option card interactive after reload", () => {
    const [card] = optionCardTags(renderConversation([optionMessage("history-42")]));

    expect(card).toBeDefined();
    expect(card).not.toContain('disabled=""');
  });

  it("keeps a selected historical option card disabled", () => {
    const [card] = optionCardTags(renderConversation([
      optionMessage("history-42"),
      { id: "history-43", role: "user", content: "方向一", createdAt: 2 },
    ]));

    expect(card).toContain('disabled=""');
  });

  it("keeps the latest checklist-complete option card interactive", () => {
    const [card] = optionCardTags(renderConversation([{
      ...optionMessage("history-42"),
      isComplete: true,
    }]));

    expect(card).toBeDefined();
    expect(card).not.toContain('disabled=""');
  });

  it("does not reactivate an earlier option card after a failed turn", () => {
    const [card] = optionCardTags(renderConversation([
      optionMessage("message-options"),
      { id: "system-error", role: "system", content: "发送失败", createdAt: 2 },
    ]));

    expect(card).toContain('disabled=""');
  });

  it("keeps a readonly recovered option card disabled", () => {
    const [card] = optionCardTags(renderConversation([{
      ...optionMessage("message-options"),
      consistencyWarning: "正式结果只供核对",
    }]));

    expect(card).toContain('disabled=""');
  });

  it("keeps the persisted inspiration message visible with an explicit warning", () => {
    const recovery = resolveFormalResultRecovery({
      t,
      recoveryIssue: "consistency_error",
      consistent: false,
      hasPersisted: true,
      hasResultRef: true,
      domain: "inspiration_message",
    });
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <ChatBubble
        message={{
          id: "message-1",
          role: "assistant",
          content: "正式消息正文",
          consistencyWarning: recovery.warning ?? recovery.message,
          createdAt: 1,
        }}
        canSelectOptions={false}
        onSendOption={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(html).toContain(generationTaskRecoveryIssueMessage("consistency_error", t));
    expect(html).toContain("以下内容来自正式消息，只供核对");
    expect(html).toContain("正式消息正文");
  });

  it("renders an inconsistent persisted blueprint without mutation actions", () => {
    const recovery = resolveFormalResultRecovery({
      t,
      recoveryIssue: "expired",
      consistent: false,
      hasPersisted: true,
      hasResultRef: true,
      domain: "blueprint",
    });
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <BlueprintConfirmationPane
        blueprint={blueprint()}
        consistencyWarning={recovery.warning}
        refining={false}
        onRefine={vi.fn()}
        onPreviewRefine={vi.fn()}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(html).toContain("当前展示正式蓝图，只供核对");
    expect(html).toContain("正式蓝图正文");
    expect(html).not.toContain("调整蓝图");
    expect(html).not.toContain("确认进入项目");
  });

  it("marks chapter and optimize recovery with the same readonly / unrecoverable semantics", () => {
    const chapterReadonly = resolveFormalResultRecovery({
      t,
      recoveryIssue: "corrupt",
      consistent: false,
      hasPersisted: true,
      hasResultRef: true,
      domain: "chapter",
    });
    expect(chapterReadonly.kind).toBe("readonly");
    expect(chapterReadonly.message).toContain("已只读加载正式章节供核对");

    const chapterUnrecoverable = resolveFormalResultRecovery({
      t,
      recoveryIssue: "expired",
      consistent: false,
      hasPersisted: false,
      hasResultRef: false,
      domain: "chapter",
    });
    expect(chapterUnrecoverable.kind).toBe("unrecoverable");
    expect(chapterUnrecoverable.message).toContain("且没有正式结果引用，无法恢复");

    const optimizeReadonly = resolveFormalResultRecovery({
      t,
      recoveryIssue: null,
      consistent: false,
      hasPersisted: true,
      hasResultRef: true,
      domain: "optimize_turn",
    });
    expect(optimizeReadonly.kind).toBe("readonly");
    expect(optimizeReadonly.shouldMarkConsistencyError).toBe(true);
    expect(optimizeReadonly.message).toContain("已只读加载正式 Turn 供核对");
    expect(optimizeReadonly.warning).toContain("已只读加载正式 Turn 供核对");

    const optimizeUnrecoverable = resolveFormalResultRecovery({
      t,
      recoveryIssue: "expired",
      consistent: false,
      hasPersisted: false,
      hasResultRef: true,
      domain: "optimize_turn",
    });
    expect(optimizeUnrecoverable.kind).toBe("unrecoverable");
    expect(optimizeUnrecoverable.message).toContain("正式 Turn 不可用");
    expect(optimizeUnrecoverable.warning).toBeNull();
  });
});

function blueprint(): Blueprint {
  return {
    id: 1,
    project_id: 1,
    title: "正式蓝图",
    genre: "剧情",
    full_synopsis: "正式蓝图正文",
    style: null,
    tone: null,
    target_audience: null,
    one_sentence_summary: null,
    world_setting: null,
    current_version: "revision-1",
    total_chapters: null,
    chapters_per_part: null,
    source_review_status: "not_applicable",
  };
}

function optionMessage(id: string): Message {
  return {
    id,
    role: "assistant",
    content: "请选择一个方向",
    options: [{
      label: "方向一",
      description: "说明",
      keyElements: ["要素一", "要素二"],
      value: "选择：方向一\n说明\n关键要素：要素一、要素二",
    }],
    createdAt: 1,
  };
}

function renderConversation(messages: Message[]): string {
  return renderToStaticMarkup(
    <I18nProvider initialLocale="zh-CN">
      <ConversationPane
        messages={messages}
        input=""
        setInput={vi.fn()}
        inputRef={{ current: null }}
        onSend={vi.fn()}
        onPreview={vi.fn()}
        onSendOption={vi.fn()}
        onKeyDown={vi.fn()}
        onCompositionStart={vi.fn()}
        onCompositionEnd={vi.fn()}
        onInputFocus={vi.fn()}
        onInputBlur={vi.fn()}
        sending={false}
        messagesViewportRef={{ current: null }}
        isFollowingLatest
        hasNewContent={false}
        onScrollToLatest={vi.fn()}
        placeholder="输入灵感"
        inputActive={false}
      />
    </I18nProvider>,
  );
}

function optionCardTags(html: string): string[] {
  return html.match(/<button type="button"[^>]*class="inspiration-option-card[^>]*>/g) ?? [];
}
