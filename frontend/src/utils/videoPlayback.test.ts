import { describe, expect, it } from "vitest";
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

  it("rejects invalid media metadata and Base64 before loading a player", () => {
    for (const data of [
      { mime_type: "video/mp4", data_base64: "%%%" },
      { mime_type: "video/mp4", data_base64: "" },
      { mime_type: "text/html", data_base64: "AA==" },
      { mime_type: "video/mp4;base64,AA==", data_base64: "AA==" },
    ]) {
      expect(() => videoPlaybackDataToDataUrl(data)).toThrow();
    }
  });
});
