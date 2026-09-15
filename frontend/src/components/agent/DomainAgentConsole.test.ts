import { describe, expect, it } from "vitest";
import {
  isAgentThreadOperational,
  type AgentThreadView,
} from "../../api/agent";
import {
  domainAgentBinding,
  findDomainAgentThread,
  matchesEditorSourceScope,
  matchesDomainAgentThread,
} from "./DomainAgentConsole";

const timestamp = "2026-08-28T00:00:00Z";

function thread(
  overrides: Partial<AgentThreadView> = {},
): AgentThreadView {
  return {
    bound_resource: { kind: "chapter", id: "11", label: null, revision: null, hash: null },
    capability: "content_optimization",
    context_summary: null,
    created_at: timestamp,
    default_model_config_id: null,
    id: "thread-1",
    kind: "subagent",
    next_event_sequence: 1,
    next_turn_sequence: 1,
    owner_user_id: 1,
    protocol_version: 1,
    revision: 1,
    scope: {
      resource_kind: "chapter",
      resource_id: "11",
      project_id: 7,
      chapter_id: 11,
      allowed_capabilities: ["content_optimization"],
      allowed_tools: [],
    },
    status: "active",
    title: null,
    updated_at: timestamp,
    ...overrides,
  };
}

describe("domain Agent binding", () => {
  it("matches editor requests after a same-resource view remount", () => {
    const source = {
      chapter_id: 11,
      editor_revision: "revision-1",
      entity_id: null,
      keyframe_id: null,
      project_id: 7,
      scene_id: null,
      shot_id: null,
      view_instance_id: "view-1",
    };
    expect(matchesEditorSourceScope(source, 7, 11)).toBe(true);
    expect(matchesEditorSourceScope(source, 8, 11)).toBe(false);
    expect(matchesEditorSourceScope(source, 7, 12)).toBe(false);
    expect(matchesEditorSourceScope(source, 7, null)).toBe(false);
  });

  it("builds the canonical chapter and project bindings", () => {
    const optimization = domainAgentBinding("content_optimization", 7, 11);
    const canvas = domainAgentBinding("canvas", 7);

    expect(optimization).toMatchObject({
      capability: "content_optimization",
      projectId: 7,
      chapterId: 11,
      resourceKind: "chapter",
      resourceId: "11",
    });
    expect(
      optimization &&
        `subagent:${optimization.capability}:${optimization.resourceKind}:${optimization.resourceId}`,
    ).toBe("subagent:content_optimization:chapter:11");
    expect(canvas).toMatchObject({
      capability: "canvas",
      projectId: 7,
      chapterId: null,
      resourceKind: "project",
      resourceId: "7",
    });
    expect(
      canvas && `subagent:${canvas.capability}:${canvas.resourceKind}:${canvas.resourceId}`,
    ).toBe("subagent:canvas:project:7");
  });

  it("rejects invalid or cross-domain scope", () => {
    expect(domainAgentBinding("content_optimization", 7)).toBeNull();
    expect(domainAgentBinding("content_optimization", 0, 11)).toBeNull();
    expect(domainAgentBinding("canvas", 7, 11)).toBeNull();
    expect(domainAgentBinding("director", 7)).toBeNull();
    expect(domainAgentBinding("director", 7, 11)).not.toBeNull();
  });

  it("keeps the Director thread bound to the chapter", () => {
    expect(domainAgentBinding("director", 7, 11)).toMatchObject({
      capability: "director",
      projectId: 7,
      chapterId: 11,
      resourceKind: "chapter",
      resourceId: "11",
    });
  });

  it("matches only the same capability, project, and resource", () => {
    const binding = domainAgentBinding("content_optimization", 7, 11);
    expect(binding).not.toBeNull();
    if (!binding) return;

    expect(matchesDomainAgentThread(thread(), binding)).toBe(true);
    expect(
      matchesDomainAgentThread(
        thread({
          scope: {
            ...thread().scope,
            project_id: 8,
          },
        }),
        binding,
      ),
    ).toBe(false);
    expect(
      matchesDomainAgentThread(
        thread({
          bound_resource: { kind: "chapter", id: "12", label: null, revision: null, hash: null },
          scope: { ...thread().scope, resource_id: "12" },
        }),
        binding,
      ),
    ).toBe(false);
    expect(matchesDomainAgentThread(thread({ kind: "main" }), binding)).toBe(false);
  });

  it("prefers the active canonical thread", () => {
    const binding = domainAgentBinding("content_optimization", 7, 11);
    expect(binding).not.toBeNull();
    if (!binding) return;

    const archived = thread({ id: "archived", status: "archived" });
    const active = thread({ id: "active" });
    expect(findDomainAgentThread([archived, active], binding)?.id).toBe("active");
    expect(findDomainAgentThread([archived], binding)).toBeNull();
  });

  it("keeps a non-archived thread operational even if an older server reports idle", () => {
    expect(isAgentThreadOperational(thread({ status: "active" }))).toBe(true);
    expect(isAgentThreadOperational({ status: "idle" })).toBe(true);
    expect(isAgentThreadOperational(thread({ status: "archived" }))).toBe(false);
  });
});
