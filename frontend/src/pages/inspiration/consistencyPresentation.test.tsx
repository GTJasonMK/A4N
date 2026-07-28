import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { Blueprint } from "../../api/blueprint";
import {
  generationTaskRecoveryIssueMessage,
  resolveFormalResultRecovery,
} from "../../utils/generationEventReducer";
import { BlueprintConfirmationPane } from "./BlueprintConfirmationPane";
import { ChatBubble } from "./ChatBubble";

describe("generation consistency presentation", () => {
  it("keeps the persisted inspiration message visible with an explicit warning", () => {
    const recovery = resolveFormalResultRecovery({
      recoveryIssue: "consistency_error",
      consistent: false,
      hasPersisted: true,
      hasResultRef: true,
      domain: "inspiration_message",
    });
    const html = renderToStaticMarkup(
      <ChatBubble
        message={{
          id: "message-1",
          role: "assistant",
          content: "正式消息正文",
          consistencyWarning: recovery.warning ?? recovery.message,
          createdAt: 1,
        }}
        onSendOption={vi.fn()}
      />,
    );

    expect(html).toContain(generationTaskRecoveryIssueMessage("consistency_error"));
    expect(html).toContain("以下内容来自正式消息，只供核对");
    expect(html).toContain("正式消息正文");
  });

  it("renders an inconsistent persisted blueprint without mutation actions", () => {
    const recovery = resolveFormalResultRecovery({
      recoveryIssue: "expired",
      consistent: false,
      hasPersisted: true,
      hasResultRef: true,
      domain: "blueprint",
    });
    const html = renderToStaticMarkup(
      <BlueprintConfirmationPane
        blueprint={blueprint()}
        consistencyWarning={recovery.warning}
        refining={false}
        onRefine={vi.fn()}
        onPreviewRefine={vi.fn()}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    expect(html).toContain("当前展示正式蓝图，只供核对");
    expect(html).toContain("正式蓝图正文");
    expect(html).not.toContain("调整蓝图");
    expect(html).not.toContain("确认进入项目");
  });

  it("marks chapter and optimize recovery with the same readonly / unrecoverable semantics", () => {
    const chapterReadonly = resolveFormalResultRecovery({
      recoveryIssue: "corrupt",
      consistent: false,
      hasPersisted: true,
      hasResultRef: true,
      domain: "chapter",
    });
    expect(chapterReadonly.kind).toBe("readonly");
    expect(chapterReadonly.message).toContain("已只读加载正式章节供核对");

    const chapterUnrecoverable = resolveFormalResultRecovery({
      recoveryIssue: "expired",
      consistent: false,
      hasPersisted: false,
      hasResultRef: false,
      domain: "chapter",
    });
    expect(chapterUnrecoverable.kind).toBe("unrecoverable");
    expect(chapterUnrecoverable.message).toContain("且没有正式结果引用，无法恢复");

    const optimizeReadonly = resolveFormalResultRecovery({
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
    current_version: "revision-1",
    needs_part_outlines: false,
    total_chapters: null,
    chapters_per_part: null,
    avatar_svg: null,
    avatar_animal: null,
    created_at: null,
    updated_at: null,
  };
}
