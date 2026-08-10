import { expect, it } from "vitest";
import type { UsageGuideBlock } from "../../api/generated/config";
import {
  insertUsageGuideImage,
  removeUsageGuideImage,
} from "./GuideTab";

it("inserts an image at the text cursor and restores text when removed", () => {
  const image: UsageGuideBlock = {
    kind: "image",
    text: null,
    image_path: null,
    file_name: "step.png",
    mime_type: "image/png",
    data_base64: "image-data",
  };
  const inserted = insertUsageGuideImage(
    {
      blocks: [
        {
          kind: "text",
          text: "前后",
          image_path: null,
          file_name: null,
          mime_type: null,
          data_base64: null,
        },
      ],
    },
    { blockIndex: 0, offset: 1 },
    image,
  );

  expect(inserted.blocks.map((block) => block.kind)).toEqual([
    "text",
    "image",
    "text",
  ]);
  expect(inserted.blocks[0].text).toBe("前");
  expect(inserted.blocks[2].text).toBe("后");
  expect(removeUsageGuideImage(inserted, 1).blocks).toEqual([
    {
      kind: "text",
      text: "前后",
      image_path: null,
      file_name: null,
      mime_type: null,
      data_base64: null,
    },
  ]);
});
