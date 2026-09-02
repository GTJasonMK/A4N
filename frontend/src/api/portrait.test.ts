import { beforeEach, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  apiInvoke: vi.fn(),
  start: vi.fn(),
}));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));
vi.mock("./generationTasks", () => ({
  generationClient: { start: mocks.start },
}));

import {
  generatePortraitPrompt,
  getPrimaryPortrait,
  listPortraitsByProject,
  previewGenerateAgeVariantRequest,
  previewGenerateMissingPortraitsRequest,
  previewGeneratePortraitPromptRequest,
  previewGeneratePortraitRequest,
  previewRegeneratePortraitRequest,
  setPrimaryPortrait,
  startPortraitGeneration,
  uploadPortrait,
} from "./portrait";

beforeEach(() => {
  mocks.apiInvoke.mockReset().mockResolvedValue(null);
  mocks.start.mockReset().mockImplementation((start) =>
    start({ start_request_id: "portrait-request", retry_of_task_id: null }),
  );
});

it("uses authenticated Host identity and one typed input envelope for portrait commands", async () => {
  await startPortraitGeneration({
    project_id: 7,
    operation: { kind: "generate_missing", width: 1024, height: 1024 },
  });
  await listPortraitsByProject(7);
  await getPrimaryPortrait(8);
  await setPrimaryPortrait(9);
  await uploadPortrait({
    project_id: 7,
    profile_id: 8,
    file_name: "portrait.png",
    mime_type: "image/png",
    data_base64: "AA==",
    portrait_type: "full_body",
    prompt: "portrait",
    expression: null,
    pose: null,
    set_primary: true,
  });
  await generatePortraitPrompt({
    project_id: 7,
    profile_id: 8,
    portrait_type: "full_body",
    variables: null,
  });
  await previewGeneratePortraitPromptRequest({
    project_id: 7,
    profile_id: 8,
    portrait_type: "full_body",
    variables: null,
  });
  await previewGeneratePortraitRequest({
    project_id: 7,
    character_name: "林岚",
    style: null,
    expression: "微笑",
    pose: null,
    reference_image_ids: [],
    width: 1024,
    height: 1024,
  });
  await previewRegeneratePortraitRequest({
    portrait_id: 9,
    reference_image_ids: [],
    width: 1024,
    height: 1024,
  });
  await previewGenerateMissingPortraitsRequest({
    project_id: 7,
    width: 1024,
    height: 1024,
  });
  await previewGenerateAgeVariantRequest({
    portrait_id: 9,
    age: "少年",
    reference_image_ids: [],
    width: 1024,
    height: 1024,
  });

  expect(mocks.apiInvoke.mock.calls.map(([command]) => command)).toEqual([
    "start_portrait_generation",
    "list_portraits_by_project",
    "get_primary_portrait",
    "set_primary_portrait",
    "upload_portrait",
    "generate_portrait_prompt",
    "preview_generate_portrait_prompt_request",
    "preview_generate_portrait_request",
    "preview_regenerate_portrait_request",
    "preview_generate_missing_portraits_request",
    "preview_generate_age_variant_request",
  ]);
  for (const [, args] of mocks.apiInvoke.mock.calls) {
    expect(args).toHaveProperty("input");
    expect(args).not.toHaveProperty("userId");
  }
});
