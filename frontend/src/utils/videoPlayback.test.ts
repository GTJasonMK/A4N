import { describe, expect, it, vi } from "vitest";
import { videoPlaybackDataToBlob, videoPlaybackDataToDataUrl } from "./videoPlayback";

describe("inline video playback data", () => {
  it("preserves the same media bytes for playback and download", async () => {
    const data = { mime_type: "video/mp4", data_base64: "AAECA/7/" };
    const response = await fetch(videoPlaybackDataToDataUrl(data));
    expect(response.headers.get("content-type")).toBe("video/mp4");
    expect(new Uint8Array(await response.arrayBuffer())).toEqual(
      new Uint8Array(await videoPlaybackDataToBlob(data).arrayBuffer()),
    );
  });

  it("rejects invalid media metadata before loading a player", () => {
    for (const data of [
      { mime_type: "video/mp4", data_base64: "" },
      { mime_type: "text/html", data_base64: "AA==" },
      { mime_type: "video/mp4;base64,AA==", data_base64: "AA==" },
    ]) {
      expect(() => videoPlaybackDataToDataUrl(data)).toThrow();
    }
  });

  it("preserves a large playback payload without changing media bytes", async () => {
    const data = {
      mime_type: "video/mp4",
      data_base64: `${"AAAA".repeat(1024 * 1024)}AAECA/7/`,
    };
    const response = await fetch(videoPlaybackDataToDataUrl(data));
    const bytes = new Uint8Array(await response.arrayBuffer());

    expect(Array.from(bytes.slice(-6))).toEqual([0, 1, 2, 3, 254, 255]);
  });

  it("leaves inline Base64 decoding to the media consumer", () => {
    const decode = vi.spyOn(globalThis, "atob");
    const data = { mime_type: "video/mp4", data_base64: "AAECA/7/" };

    try {
      expect(videoPlaybackDataToDataUrl(data)).toBe("data:video/mp4;base64,AAECA/7/");
      expect(decode).not.toHaveBeenCalled();
    } finally {
      decode.mockRestore();
    }
  });
});
