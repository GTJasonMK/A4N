import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));

import { syncSceneProfiles } from "./sceneProfiles";

beforeEach(() => mocks.apiInvoke.mockReset());

describe("scene profile sync host contract", () => {
  it("sends only canonical input and optional task identity", async () => {
    await syncSceneProfiles(7, 3, "scene-sync");

    expect(mocks.apiInvoke).toHaveBeenCalledWith("sync_scene_profiles", {
      input: { project_id: 7, chapter_number: 3 },
      taskId: "scene-sync",
    });
  });
});
