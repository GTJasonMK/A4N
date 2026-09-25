import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { AgentMessageView } from "../../api/agent";
import { AgentMessageList, createPendingAgentMessage } from "./AgentMessageList";
import { AgentMessageRow } from "./AgentMessageRow";

vi.mock("../../theme/activeTheme", async () => {
  const actual = await vi.importActual<typeof import("../../theme/activeTheme")>("../../theme/activeTheme");
  return {
    ...actual,
    getActiveThemeStreamDisplaySettings: () => ({
      enabled: true,
      rate: 30,
      mode: "fixed" as const,
    }),
  };
});

const timestamp = "2026-08-28T00:00:00Z";

function message(overrides: Partial<AgentMessageView> = {}): AgentMessageView {
  return {
    content: "",
    created_at: timestamp,
    id: "message-1",
    metadata: null,
    role: "assistant",
    run_id: "run-1",
    source_id: null,
    source_kind: "agent",
    status: "complete",
    thread_id: "thread-1",
    turn_id: "turn-1",
    updated_at: timestamp,
    ...overrides,
  };
}

describe("AgentMessageRow", () => {
  it("renders an empty assistant tool call as an identified operation", () => {
    const html = renderToStaticMarkup(
      <AgentMessageRow
        message={message({
          metadata: {
            tool_calls: [
              {
                id: "call-1",
                name: "list_projects",
                arguments: { limit: 10 },
              },
            ],
          },
        })}
        locale="zh-CN"
      />,
    );

    expect(html).toContain("执行轨迹");
    expect((html.match(/执行轨迹/g) ?? []).length).toBe(1);
    expect(html).toContain("list_projects");
    expect(html).toContain("查看参数");
    expect(html).toContain('data-agent-process-state="running"');
    expect(html).not.toContain("空消息");
  });

  it("keeps a tool result summary compact and puts raw data in details", () => {
    const html = renderToStaticMarkup(
      <AgentMessageRow
        message={message({
          role: "tool",
          content: JSON.stringify({ projects: [{ id: 1 }, { id: 2 }] }),
          metadata: {
            execution_id: "42",
            tool: "list_projects",
            tool_call_id: "call-1",
            result: { projects: [{ id: 1 }, { id: 2 }] },
          },
        })}
        locale="zh-CN"
      />,
    );

    expect(html).toContain("执行结果");
    expect(html).toContain("返回 projects 2 项");
    expect(html).toContain("调用 call-1");
    expect(html).toContain("执行 #42");
    expect(html).toContain("查看原始结果");
    expect(html).toContain("<details");
    expect(html.indexOf("查看原始结果")).toBeLessThan(
      html.indexOf("&quot;projects&quot;"),
    );
  });

  it("groups a matching tool result into the tool call bubble", () => {
    const html = renderToStaticMarkup(
      <AgentMessageList
        messages={[
          message({
            id: "assistant-tool-call",
            metadata: {
              tool_calls: [{
                id: "call-1",
                name: "list_projects",
                arguments: { limit: 10 },
              }],
            },
          }),
          message({
            id: "tool-result",
            role: "tool",
            content: JSON.stringify({ projects: [{ id: 1 }, { id: 2 }] }),
            metadata: {
              execution_id: "42",
              tool: "list_projects",
              tool_call_id: "call-1",
              result: { projects: [{ id: 1 }, { id: 2 }] },
            },
          }),
        ]}
        locale="zh-CN"
      />,
    );

    expect((html.match(/<article/g) ?? []).length).toBe(1);
    expect(html).toContain("list_projects");
    expect(html).toContain("执行结果");
    expect(html).toContain("返回 projects 2 项");
    expect(html).toContain("执行 #42");
    expect(html).toContain('data-agent-process-state="complete"');
    expect(html).not.toContain("<details open");
  });

  it("keeps multiple tool results paired and leaves unknown results visible", () => {
    const html = renderToStaticMarkup(
      <AgentMessageList
        messages={[
          message({
            id: "assistant-tool-call",
            metadata: {
              tool_calls: [
                { id: "call-read", name: "read_file", arguments: {} },
                { id: "call-write", name: "write_file", arguments: {} },
              ],
            },
          }),
          message({
            id: "tool-result-read",
            role: "tool",
            metadata: {
              tool: "read_file",
              tool_call_id: "call-read",
              result: { text: "read" },
            },
            content: "{\"text\":\"read\"}",
          }),
          message({
            id: "tool-result-unknown",
            role: "tool",
            metadata: {
              tool: "unknown_tool",
              tool_call_id: "call-missing",
              result: { status: "unknown" },
            },
            content: "{\"status\":\"unknown\"}",
          }),
          message({
            id: "tool-result-write",
            role: "tool",
            metadata: {
              tool: "write_file",
              tool_call_id: "call-write",
              result: { status: "ok" },
            },
            content: "{\"status\":\"ok\"}",
          }),
        ]}
        locale="zh-CN"
      />,
    );

    expect((html.match(/<article/g) ?? []).length).toBe(2);
    expect(html).toContain("read_file");
    expect(html).toContain("write_file");
    expect(html).toContain("unknown_tool");
  });

  it("describes a genuinely empty assistant message without pretending it is a reply", () => {
    const html = renderToStaticMarkup(
      <AgentMessageRow message={message()} locale="zh-CN" />,
    );

    expect(html).toContain("Agent 未返回文本");
    expect(html).not.toContain("（空消息）");
  });

  it("shows a thinking state as soon as an assistant stream exists", () => {
    const html = renderToStaticMarkup(
      <AgentMessageRow
        message={message({ status: "streaming" })}
        locale="zh-CN"
      />,
    );

    expect(html).toContain("正在思考");
    expect(html).toContain('role="status"');
    expect(html).not.toContain("Agent 未返回文本");
  });

  it("starts a completed message from its stream prefix", () => {
    const html = renderToStaticMarkup(
      <AgentMessageRow
        message={message({ content: "streamed reply" })}
        locale="zh-CN"
        wasStreamed
      />,
    );

    expect(html).toContain('role="status"');
    expect(html).not.toContain("streamed reply");
  });

  it("summarizes a generic object by fields instead of exposing inline JSON", () => {
    const html = renderToStaticMarkup(
      <AgentMessageRow
        message={message({
          role: "tool",
          content: JSON.stringify({ status: "ok", owner: "agent" }),
          metadata: {
            tool: "get_status",
            result: { status: "ok", owner: "agent" },
          },
        })}
        locale="zh-CN"
      />,
    );

    expect(html).toContain("返回 2 个字段：status、owner");
    expect(html.indexOf("返回 2 个字段")).toBeLessThan(
      html.indexOf("&quot;status&quot;"),
    );
  });

  it("shows the submitted command and current run state in the feed", () => {
    const html = renderToStaticMarkup(
      <AgentMessageList
        messages={[]}
        pendingMessages={[createPendingAgentMessage("thread-1", "继续处理", "client-1")]}
        activeRun={{ status: "waiting", created_at: timestamp, started_at: null, error: null }}
        locale="zh-CN"
      />,
    );

    expect(html).toContain("继续处理");
    expect((html.match(/继续处理/g) ?? []).length).toBe(1);
    expect(html).toContain("已提交，等待 Agent 接管");
    expect(html).toContain("等待你的确认");
  });

  it("times only an active run from started_at, never from its old created_at", () => {
    const html = renderToStaticMarkup(
      <AgentMessageList
        messages={[]}
        activeRun={{
          status: "running",
          created_at: "2020-01-01T00:00:00Z",
          started_at: new Date(Date.now() - 16_000).toISOString(),
          error: null,
        }}
        locale="zh-CN"
      />,
    );
    expect(html).toMatch(/1[5-7]s/);

    const long = renderToStaticMarkup(
      <AgentMessageList
        messages={[]}
        activeRun={{
          status: "running",
          created_at: "2020-01-01T00:00:00Z",
          started_at: new Date(Date.now() - (8 * 3_600 + 1 * 60 + 2) * 1_000).toISOString(),
          error: null,
        }}
        locale="zh-CN"
      />,
    );
    expect(long).toMatch(/8:01:0[1-3]/);

    const waiting = renderToStaticMarkup(
      <AgentMessageList
        messages={[]}
        activeRun={{
          status: "waiting",
          created_at: "2020-01-01T00:00:00Z",
          started_at: "2020-01-01T00:00:00Z",
          error: null,
        }}
        locale="zh-CN"
      />,
    );
    expect(waiting).not.toMatch(/\d+:\d{2}/);
  });

  it("shows terminal failures and their cause instead of a completion fallback", () => {
    const html = renderToStaticMarkup(
      <AgentMessageList
        messages={[]}
        activeRun={{ status: "failed", created_at: timestamp, started_at: null, error: "模型配置不可用" }}
        locale="zh-CN"
      />,
    );
    expect(html).toContain("Agent 任务失败");
    expect(html).toContain("模型配置不可用");
    expect(html).toContain('role="alert"');
    expect(html).not.toContain("Agent 已完成");
  });

  it("collapses long assistant replies for the compact domain console", () => {
    const html = renderToStaticMarkup(
      <AgentMessageList
        messages={[
          message({
            content: `# 需要注意：\n\n${Array.from(
              { length: 7 },
              (_, index) => `${index + 1}. 第 ${index + 1} 项需要在编辑器中确认后再继续。`,
            ).join("\n")}`,
          }),
        ]}
        locale="zh-CN"
        compactAssistantMessages
      />,
    );

    expect(html).toContain("需要注意： · 7 项");
    expect(html).toContain('class="agent-message-collapsible"');
    expect(html).not.toContain("<details open");
  });
});
