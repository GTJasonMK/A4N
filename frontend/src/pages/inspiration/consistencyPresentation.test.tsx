import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { Blueprint } from "../../api/blueprint";
import {
  generationTaskRecoveryIssueMessage,
  resolveFormalResultRecovery,
} from "../../utils/generationEventReducer";
import { BlueprintConfirmationPane } from "./BlueprintConfirmationPane";
import { ChatBubble } from "./ChatBubble";
import { I18nProvider, translateMessage } from "../../i18n";

const t = (key: Parameters<typeof translateMessage>[1], params?: Parameters<typeof translateMessage>[2]) =>
  translateMessage("zh-CN", key, params);

describe("generation consistency presentation", () => {
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
        onSendOption={vi.fn()}
        />
      </I18nProvider>,
    );

    expect(html).toContain("正在整理你的灵感…");
    expect(html).toContain('role="status"');
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
