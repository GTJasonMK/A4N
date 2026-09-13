import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { AgentMessageView } from "../../api/agent";
import { AgentMessageList } from "./AgentMessageList";
import { AgentMessageRow } from "./AgentMessageRow";

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

    expect(html).toContain("工具调用");
    expect((html.match(/工具调用/g) ?? []).length).toBe(1);
    expect(html).toContain("list_projects");
    expect(html).toContain("查看参数");
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

    expect(html).toContain("工具结果");
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
    expect(html).toContain("工具结果");
    expect(html).toContain("返回 projects 2 项");
    expect(html).toContain("执行 #42");
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
});
