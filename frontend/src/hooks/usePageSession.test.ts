import { describe, expect, it } from "vitest";
import { buildPageContext, pageKindForPath, pageSessionWithCurrentContext } from "./usePageSession";
import type { PageSessionView } from "../api/pageSessions";

describe("Page Session page context", () => {
  it("maps supported routes to stable page kinds", () => {
    expect(pageKindForPath("/")).toBe("novel-list");
    expect(pageKindForPath("/inspiration/3/chat")).toBe("inspiration-chat");
    expect(pageKindForPath("/novel/4/overview")).toBe("novel-detail");
    expect(pageKindForPath("/write/4")).toBe("writing-desk");
    expect(pageKindForPath("/canvas-project/4")).toBe("canvas-project");
    expect(pageKindForPath("/comic-project/4")).toBe("comic-project");
    expect(pageKindForPath("/galgame-project/4")).toBe("galgame-project");
    expect(pageKindForPath("/settings/about")).toBe("settings");
    expect(pageKindForPath("/agents")).toBe("agent-monitor");
    expect(pageKindForPath("/unknown")).toBe("page");
  });

  it("derives project, chapter, tab, and full route context from the URL", () => {
    expect(
      buildPageContext(
        {
          pathname: "/novel/42/overview",
          search: "?chapter_id=7",
          hash: "#world",
        },
        4,
      ),
    ).toEqual({
      page_kind: "novel-detail",
      route: "/novel/42/overview?chapter_id=7#world",
      project_id: 42,
      chapter_id: 7,
      active_tab: "world",
      selected_ids: {},
      dirty_state: false,
      local_state: {},
      context_revision: 4,
    });
  });

  it("rejects invalid IDs and carries forward page-local state", () => {
    expect(
      buildPageContext(
        { pathname: "/novel/0/overview", search: "?chapter=0", hash: "#" },
        5,
        {
          selected_ids: { project: [42] },
          dirty_state: true,
          local_state: { scroll_top: 120 },
        },
      ),
    ).toMatchObject({
      project_id: null,
      chapter_id: null,
      active_tab: null,
      selected_ids: { project: [42] },
      dirty_state: true,
      local_state: { scroll_top: 120 },
      context_revision: 5,
    });
  });

  it("does not let an older heartbeat overwrite a newer route or dirty state", () => {
    const current = buildPageContext(
      { pathname: "/video-project/7", hash: "#materials" },
      8,
      { selected_ids: { image: [4] }, dirty_state: true, local_state: { draft: "edited" } },
    );
    const oldSession = {
      ui_session_id: "session-1",
      context_revision: 7,
      route: "/video-project/7#frames",
      page_kind: "video-project",
      project_id: 7,
      chapter_id: null,
      active_tab: "frames",
      selected_ids: {},
      dirty_state: false,
      local_state: {},
    } as PageSessionView;

    expect(pageSessionWithCurrentContext(oldSession, current)).toMatchObject({
      context_revision: 8,
      route: "/video-project/7#materials",
      active_tab: "materials",
      selected_ids: { image: [4] },
      dirty_state: true,
      local_state: { draft: "edited" },
    });
  });
});
