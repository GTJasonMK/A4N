import { describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ fetch: vi.fn() }));
vi.mock(".", () => ({ authenticatedWebFetch: mocks.fetch, apiInvoke: vi.fn() }));
vi.mock("../utils/runtimeMode", () => ({ shouldUseHttpBackend: () => true }));

import { subscribeVisualPlanEdits } from "./visualPlanEdits";

const event = (sequence: number, projectId = 7, plan = {}) => `id: ${sequence}\nevent: visual_plan_edit\ndata: ${JSON.stringify({ plan, edit_receipt: { sequence, project_id: projectId, source_chapter_id: 11, after: { record_id: 3 } } })}\n\n`;

describe("visual plan edit stream", () => {
  it("accepts a document larger than the generic generation event limit", async () => {
    const plan = { points: Array.from({ length: 100 }, (_, id) => ({ id: String(id), description: "分镜描述".repeat(100) })) };
    mocks.fetch.mockResolvedValueOnce(new Response(event(1, 7, plan)));
    const received = vi.fn();
    await subscribeVisualPlanEdits(7, 11, 0, "source", 3, received, new AbortController().signal);
    expect(received.mock.calls[0][0].plan).toEqual(plan);
  });
  it("deduplicates replayed receipts and preserves the resource fence", async () => {
    mocks.fetch.mockResolvedValueOnce(new Response(event(1) + event(1) + event(2)));
    const received = vi.fn();
    await subscribeVisualPlanEdits(7, 11, 0, "source", 3, received, new AbortController().signal);
    expect(received).toHaveBeenCalledTimes(2);
    mocks.fetch.mockResolvedValueOnce(new Response(event(3, 8)));
    await expect(subscribeVisualPlanEdits(7, 11, 2, "source", 3, received, new AbortController().signal)).rejects.toThrow("身份");
    expect(received).toHaveBeenCalledTimes(2);
  });
});
