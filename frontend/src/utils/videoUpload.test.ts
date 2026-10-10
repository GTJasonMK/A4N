import { beforeEach, expect, it, vi } from "vitest";
import type { MessageTranslator } from "../i18n";
import { pickLocalFile } from "./fileUpload";
import { pickVideoUploadFile } from "./videoUpload";

vi.mock("./fileUpload", () => ({ pickLocalFile: vi.fn() }));

const pickLocalFileMock = vi.mocked(pickLocalFile);
const translate: MessageTranslator = (key) => key;

beforeEach(() => {
  pickLocalFileMock.mockReset();
});

it("uses generated video MIME aliases and extensions", async () => {
  const file = new File(["video"], "clip.M4V", { type: "video/x-m4v" });
  pickLocalFileMock.mockResolvedValue(file);

  await expect(pickVideoUploadFile(translate)).resolves.toEqual({
    file,
    mimeType: "video/x-m4v",
  });
  expect(pickLocalFileMock).toHaveBeenCalledWith(".mp4,.webm,.mov,.m4v");
});

it("rejects inherited object keys as video extensions", async () => {
  pickLocalFileMock.mockResolvedValue(new File(["video"], "clip.toString"));

  await expect(pickVideoUploadFile(translate)).rejects.toThrow(
    "video.upload.invalidType",
  );
});
