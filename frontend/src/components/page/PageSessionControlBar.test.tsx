import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { I18nProvider } from "../../i18n";
import type { PageSessionState } from "../../hooks/usePageSession";
import type { PageSessionView } from "../../api/pageSessions";
import { PageSessionControlBar } from "./PageSessionControlBar";

function session(status: PageSessionView["status"], agentId: string | null): PageSessionView {
  return {
    protocol_version: 1,
    ui_session_id: "session-1",
    mode: "page",
    status,
    client: "a4n-web",
    tab_id: "tab-1",
    page_kind: "novel-detail",
    route: "/novel/42",
    project_id: 42,
    chapter_id: 7,
    active_tab: "overview",
    selected_ids: {},
    dirty_state: false,
    local_state: {},
    context_revision: 3,
    agent_id: agentId,
    operation_id: status === "running" ? "operation-1" : null,
    last_operation: status === "running" ? {
      operation_id: "operation-1",
      ui_session_id: "session-1",
      agent_id: agentId,
      command: "list_projects",
      page_kind: "novel-detail",
      phase: "读取项目",
      target: {},
      status: "running",
      progress: 40,
      summary: null,
      result: null,
      error: null,
      started_at: "2026-08-23T00:00:00Z",
      updated_at: "2026-08-23T00:00:01Z",
    } : null,
    created_at: "2026-08-23T00:00:00Z",
    updated_at: "2026-08-23T00:00:01Z",
    last_seen_at: "2026-08-23T00:00:01Z",
    expires_at: "2026-08-23T01:00:00Z",
  };
}

function renderBar(overrides: Partial<PageSessionState>): string {
  const state: PageSessionState = {
    session: null,
    authorizationCode: null,
    loading: false,
    busy: false,
    error: null,
    locked: false,
    control: vi.fn(),
    authorize: vi.fn().mockResolvedValue(undefined),
    restart: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  return renderToStaticMarkup(
    <I18nProvider initialLocale="zh-CN">
      <PageSessionControlBar state={state} />
    </I18nProvider>,
  );
}

describe("PageSessionControlBar", () => {
  it("shows the running state and keeps takeover visible while the page is locked", () => {
    const html = renderBar({
      session: session("running", "agent-1"),
      locked: true,
    });

    expect(html).toContain('aria-label="页面 CLI 会话"');
    expect(html).toContain('data-page-session-status="running"');
    expect(html).toContain("暂停");
    expect(html).toContain("取消操作");
    expect(html).toContain("用户接管");
    expect(html).toContain("list_projects · 读取项目 · 40%");
  });

  it("renders the one-time code only in the waiting-for-attach control", () => {
    const html = renderBar({
      session: session("awaiting_attach", null),
      authorizationCode: "CODE-123",
    });

    expect(html).toContain("等待绑定");
    expect(html).toContain("CODE-123");
    expect(html).toContain("复制绑定码");
  });
});
