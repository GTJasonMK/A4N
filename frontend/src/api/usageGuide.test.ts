import { beforeEach, expect, it, vi } from "vitest";

const apiInvoke = vi.hoisted(() => vi.fn());

vi.mock("./index", () => ({ apiInvoke }));
vi.mock("./authStorage", () => ({ requireCurrentUserId: () => 42 }));

import { getUsageGuide, updateUsageGuide } from "./usageGuide";

beforeEach(() => apiInvoke.mockReset());

it("uses the canonical guide commands and caller context", async () => {
  const document = {
    blocks: [
      {
        kind: "text" as const,
        text: "guide",
        image_path: null,
        file_name: null,
        mime_type: null,
        data_base64: null,
      },
    ],
  };
  const updated = {
    blocks: [{ ...document.blocks[0], text: "updated" }],
  };
  apiInvoke.mockResolvedValueOnce(document).mockResolvedValueOnce(updated);

  await expect(getUsageGuide()).resolves.toEqual(document);
  await expect(updateUsageGuide(updated)).resolves.toEqual(updated);

  expect(apiInvoke).toHaveBeenNthCalledWith(1, "get_usage_guide");
  expect(apiInvoke).toHaveBeenNthCalledWith(2, "update_usage_guide", {
    callerUserId: 42,
    input: updated,
  });
});
