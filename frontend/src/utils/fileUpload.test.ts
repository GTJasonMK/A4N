import { expect, it, vi } from "vitest";
import { pickLocalFile } from "./fileUpload";

it("settles native file-picker cancellation without a window focus event", async () => {
  const input = Object.assign(new EventTarget(), { style: {}, click: vi.fn(), remove: vi.fn() });
  vi.stubGlobal("document", { createElement: () => input, body: { appendChild: vi.fn() } });
  vi.stubGlobal("window", { addEventListener: vi.fn(), removeEventListener: vi.fn() });
  try {
    const selection = pickLocalFile("image/png");
    input.dispatchEvent(new Event("cancel"));
    await expect(selection).resolves.toBeNull();
    expect(input.remove).toHaveBeenCalledOnce();
  } finally {
    vi.unstubAllGlobals();
  }
});
