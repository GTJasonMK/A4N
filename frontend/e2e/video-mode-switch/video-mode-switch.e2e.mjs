import assert from "node:assert/strict";

const segmentHeaderText = (segmentNumber) => browser.execute((target) => {
  const marker = [...document.querySelectorAll("span")]
    .find((element) => element.textContent?.trim() === `片段 ${target}`);
  return marker?.closest("button")?.innerText ?? "";
}, segmentNumber);

const persistedSnapshots = () => browser.execute(() =>
  window.__A4N_VIDEO_MODE_E2E__.calls
    .filter((call) => call.command === "persisted_recipe_snapshot")
    .map(({ target, modes }) => ({ target, modes })),
);

const segmentPlanStarts = () => browser.execute(() =>
  window.__A4N_VIDEO_MODE_E2E__.calls
    .filter((call) => call.command === "start_video_generation")
    .map((call) => call.args.input.operation.options),
);

const recipeUpdates = () => browser.execute(() =>
  window.__A4N_VIDEO_MODE_E2E__.calls
    .filter((call) => call.command === "update_chapter_video_prompt")
    .map((call) => call.args.input.segment_recipe),
);

const selectSegmentVideoCalls = () => browser.execute(() =>
  window.__A4N_VIDEO_MODE_E2E__.calls
    .filter((call) => call.command === "select_segment_video")
    .map((call) => ({
      videoId: call.args.videoId,
      segmentPlanPromptId: call.args.input.segment_plan_prompt_id,
      shotNumber: call.args.input.shot_number,
    })),
);

const adoptionSnapshots = () => browser.execute(() =>
  window.__A4N_VIDEO_MODE_E2E__.calls
    .filter((call) => call.command === "persisted_adoption_snapshot")
    .map(({ videoId, adopted }) => ({ videoId, adopted })),
);

const assembleChapterVideoCalls = () => browser.execute(() =>
  window.__A4N_VIDEO_MODE_E2E__.calls
    .filter((call) => call.command === "assemble_chapter_videos")
    .map((call) => ({
      projectId: call.args.projectId,
      chapterId: call.args.chapterId,
      workflowMode: call.args.workflowMode,
    })),
);

describe("video segment mode switching in Chromium", () => {
  it("persists only the clicked segment recipe", async () => {
    await browser.url("/e2e/video-mode-switch/");

    const firstGroup = await $('[aria-label="片段 1 生成方案"]');
    await firstGroup.waitForExist();
    assert.match(await segmentHeaderText(1), /首尾帧生成/);
    assert.match(await segmentHeaderText(2), /分镜生成/);

    await firstGroup.$('.//button[normalize-space()="Chat"]').click();
    await browser.waitUntil(async () => (await persistedSnapshots()).length === 1);
    assert.match(await segmentHeaderText(1), /Chat 生视频/);
    assert.match(await segmentHeaderText(2), /分镜生成/);

    const secondHeader = await $('//span[normalize-space()="片段 2"]/ancestor::button[1]');
    await secondHeader.click();
    const secondGroup = await $('[aria-label="片段 2 生成方案"]');
    await secondGroup.waitForExist();
    await secondGroup.$('.//button[normalize-space()="首尾帧"]').click();
    await browser.waitUntil(async () => (await persistedSnapshots()).length === 2);

    assert.match(await segmentHeaderText(1), /Chat 生视频/);
    assert.match(await segmentHeaderText(2), /首尾帧生成/);
    assert.deepEqual(await persistedSnapshots(), [
      { target: 1, modes: { 1: "chat_video", 2: "storyboard" } },
      { target: 2, modes: { 1: "chat_video", 2: "boundary_frames" } },
    ]);
  });

  it("retains a saved segment after cancellation and resumes from the next shot", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=cancel-resume");

    const generate = await $('//button[contains(normalize-space(), "生成片段规划")]');
    await generate.waitForEnabled();
    await generate.click();

    const savedSegment = await $('//*[normalize-space()="片段 1"]');
    await savedSegment.waitForExist();
    const stop = await $('//button[normalize-space()="停止生成"]');
    await stop.waitForClickable();
    await stop.click();

    const close = await $('//button[normalize-space()="关闭"]');
    await close.waitForClickable();
    await close.click();

    const resume = await $('//button[contains(normalize-space(), "继续生成")]');
    await resume.waitForEnabled();
    assert.match(await $("body").getText(), /50%.*已保存镜头 1.*1\/2 个镜头/s);
    assert.equal(await savedSegment.isExisting(), true);

    const startsBeforeResume = await segmentPlanStarts();
    assert.equal(startsBeforeResume.length, 1);
    const originalOptions = startsBeforeResume[0];

    const configuration = await $('//summary[contains(normalize-space(), "规划配置")]');
    await configuration.click();
    const style = await $('textarea[placeholder*="整体风格"]');
    await style.setValue("不应进入恢复请求的当前表单风格");
    await resume.click();

    await browser.waitUntil(async () => (await segmentPlanStarts()).length === 2);
    const startsAfterResume = await segmentPlanStarts();
    assert.deepEqual(startsAfterResume[1], originalOptions);
    assert.notEqual(startsAfterResume[1].style, await style.getValue());
    await browser.waitUntil(async () => (await $("body").getText()).includes("正在生成镜头 2"));

    const stopResumed = await $('//button[normalize-space()="停止生成"]');
    await stopResumed.waitForClickable();
    await stopResumed.click();
    const closeResumed = await $('//button[normalize-space()="关闭"]');
    await closeResumed.waitForClickable();
    await closeResumed.click();
    await resume.waitForEnabled();
    assert.equal(await savedSegment.isExisting(), true);
  });

  it("continues a boundary-frame segment with a later first-frame-chain segment", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=cross-mode-continuity");

    const firstGroup = await $('[aria-label="片段 1 生成方案"]');
    await firstGroup.waitForExist();
    assert.match(await segmentHeaderText(1), /首尾帧生成/);

    const secondHeader = await $('//span[normalize-space()="片段 2"]/ancestor::button[1]');
    await secondHeader.click();
    const secondGroup = await $('[aria-label="片段 2 生成方案"]');
    await secondGroup.waitForExist();
    await secondGroup.$('.//button[normalize-space()="首帧续接"]').click();

    await browser.waitUntil(async () => (await recipeUpdates()).length === 1);
    const [updatedRecipe] = await recipeUpdates();
    assert.equal(updatedRecipe.segment_number, 2);
    assert.equal(updatedRecipe.workflow_mode, "first_frame_chain");
    assert.equal(updatedRecipe.guidance_mode, "continuity");
    assert.equal(updatedRecipe.continuity_source_video_id, 501);
    assert.match(await segmentHeaderText(1), /首尾帧生成/);
    assert.match(await segmentHeaderText(2), /首帧生成/);

    const guidance = await $('[aria-label="片段 2 媒体引导"]');
    await guidance.waitForExist();
    assert.equal(
      await guidance.$('.//button[@data-active="1"]').getText(),
      "续接视频",
    );
    const source = await $('//select[option[contains(normalize-space(), "视频 #501")]]');
    await source.waitForExist();
    assert.equal(await source.getValue(), "501");
  });

  it("keeps old-plan videos out of candidates and only adopts the current plan video", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=stale-plan-adoption");

    const firstGroup = await $('[aria-label="片段 1 生成方案"]');
    await firstGroup.waitForExist();
    assert.match(await segmentHeaderText(1), /Chat 生视频/);

    const results = await $('//section[@aria-label="视频结果"]');
    await results.waitForExist();
    await browser.waitUntil(async () =>
      (await results.getText()).includes("当前规划候选视频"),
    );
    const resultsText = await results.getText();
    assert.match(resultsText, /当前规划候选视频/);
    assert.doesNotMatch(resultsText, /旧规划遗留视频/);
    assert.match(resultsText, /候选 1|镜头 1/);

    const adopt = await results.$('.//button[normalize-space()="采用"]');
    await adopt.waitForClickable();
    await adopt.click();

    await browser.waitUntil(async () => (await selectSegmentVideoCalls()).length === 1);
    assert.deepEqual(await selectSegmentVideoCalls(), [
      { videoId: 502, segmentPlanPromptId: 41, shotNumber: 1 },
    ]);
    assert.deepEqual(await adoptionSnapshots(), [
      { videoId: 502, adopted: { 1: 502, 2: null } },
    ]);

    await browser.waitUntil(async () =>
      (await results.getText()).includes("已采用"),
    );
    assert.doesNotMatch(await results.getText(), /旧规划遗留视频/);

    // 第二层：即使绕过 UI 直接调用采用 API，旧规划视频也必须被拒绝且不写回 adopted。
    const rejectResult = await browser.execute(async () => {
      try {
        await window.__TAURI_INTERNALS__.invoke("select_segment_video", {
          userId: 1,
          projectId: 1,
          chapterId: 2,
          videoId: 503,
          input: {
            segment_plan_prompt_id: 41,
            shot_number: 1,
          },
        });
        return { ok: true };
      } catch (error) {
        return {
          ok: false,
          message: error instanceof Error ? error.message : String(error),
        };
      }
    });
    assert.equal(rejectResult.ok, false);
    assert.match(rejectResult.message, /旧规划|失效身份/);
    assert.deepEqual(await adoptionSnapshots(), [
      { videoId: 502, adopted: { 1: 502, 2: null } },
    ]);
    assert.equal((await selectSegmentVideoCalls()).length, 2);
    assert.deepEqual((await selectSegmentVideoCalls())[1], {
      videoId: 503,
      segmentPlanPromptId: 41,
      shotNumber: 1,
    });
  });

  it("blocks mixed assembly when a segment only has a stale adopted video", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=mixed-assembly-gap");

    const previewTitle = await $('//h3[normalize-space()="成片预览"]');
    await previewTitle.waitForExist();
    const body = await $("body");
    await browser.waitUntil(async () =>
      (await body.getText()).includes("尚缺片段 2 的有效采用视频"),
    );
    assert.match(await body.getText(), /成片片段 1 有效视频|镜头 1/);
    assert.doesNotMatch(await body.getText(), /成片片段 2 有效视频/);

    const generate = await $('//button[contains(normalize-space(), "生成预览")]');
    await generate.waitForExist();
    assert.equal(await generate.isEnabled(), false);
    assert.equal((await assembleChapterVideoCalls()).length, 0);
  });

  it("assembles mixed preview only after every current-plan segment is adopted", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=mixed-assembly-ready");

    const previewTitle = await $('//h3[normalize-space()="成片预览"]');
    await previewTitle.waitForExist();
    const body = await $("body");
    await browser.waitUntil(async () =>
      (await body.getText()).includes("成片片段 1 有效视频"),
    );
    assert.match(await body.getText(), /成片片段 2 有效视频/);
    assert.doesNotMatch(await body.getText(), /尚缺片段/);

    const generate = await $('//button[contains(normalize-space(), "生成预览")]');
    await generate.waitForEnabled();
    await generate.click();

    await browser.waitUntil(async () => (await assembleChapterVideoCalls()).length === 1);
    assert.deepEqual(await assembleChapterVideoCalls(), [
      { projectId: 1, chapterId: 2, workflowMode: "mixed" },
    ]);
    await browser.waitUntil(async () =>
      (await body.getText()).includes("更新预览"),
    );
  });
});
