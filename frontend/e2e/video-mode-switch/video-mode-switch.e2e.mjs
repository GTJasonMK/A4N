import assert from "node:assert/strict";

const segmentHeaderText = async (segmentNumber) => {
  const button = await pageSegmentButton(segmentNumber);
  await button.waitForDisplayed();
  return button.getText();
};

const pageSegmentButton = (segmentNumber) =>
  $(
    `//button[@data-video-segment="${segmentNumber}" and not(ancestor::*[@role="dialog"])]`,
  );

const segmentConfigDialog = (segmentNumber) =>
  $(
    `//div[@role="dialog" and .//h3[normalize-space()="片段 ${segmentNumber} · 配置"]]`,
  );

const revealByWheel = async (element) => {
  await element.waitForExist();
  for (let attempt = 0; attempt < 16; attempt++) {
    const position = await element.execute((node) => {
      const box = node.getBoundingClientRect();
      const cx = box.left + box.width / 2;
      const cy = box.top + box.height / 2;
      const hit = document.elementFromPoint(cx, cy);
      if (box.width > 0 && box.height > 0 && hit && (node === hit || node.contains(hit))) return { reached: true };
      let scroll = node.parentElement;
      while (scroll && (!/auto|scroll/.test(getComputedStyle(scroll).overflowY) || scroll.scrollHeight <= scroll.clientHeight + 1)) scroll = scroll.parentElement;
      if (!scroll) return { reached: false, scrollable: false, top: box.top, height: box.height };
      const bounds = scroll.getBoundingClientRect();
      const top = Math.max(0, bounds.top), bottom = Math.min(innerHeight, bounds.bottom);
      const span = bottom - top;
      return {
        reached: false, scrollable: span > 0,
        x: Math.round(Math.max(1, Math.min(innerWidth - 1, bounds.right - 4))),
        y: Math.round((top + bottom) / 2),
        deltaY: Math.round(Math.max(-span / 2, Math.min(span / 2, cy - (top + bottom) / 2))),
        top: box.top, height: box.height,
      };
    });
    if (position.reached) return;
    assert(position.scrollable && position.deltaY !== 0, `Control has no reachable scroll path: ${JSON.stringify(position)}`);
    await browser.performActions([{ type: "wheel", id: "video-mode-wheel", actions: [{
      type: "scroll", origin: "viewport", x: position.x, y: position.y, deltaX: 0, deltaY: position.deltaY, duration: 120,
    }] }]);
    await browser.releaseActions();
    await browser.pause(80);
  }
  assert.fail("Control did not become reachable through real wheel scrolling");
};

const openFrameMenu = async (dialog) => {
  const trigger = await dialog.$('button[aria-haspopup="menu"]');
  await trigger.waitForClickable();
  await trigger.click();
  await browser.waitUntil(async () => (await trigger.getAttribute("aria-expanded")) === "true");
  const id = await trigger.getAttribute("aria-controls");
  assert(id, "The open frame menu must expose its controlled menu");
  const menu = await $(`[role="menu"][id=${JSON.stringify(id)}]`);
  await menu.waitForDisplayed();
  return { trigger, menu };
};

const frameMenuItem = (menu, label) =>
  menu.$(`.//*[@role="menuitem" and normalize-space()=${JSON.stringify(label)}]`);

const openSegmentConfig = async (segmentNumber) => {
  const button = await pageSegmentButton(segmentNumber);
  await revealByWheel(button);
  await button.waitForClickable();
  if ((await button.getAttribute("aria-current")) !== "true") {
    await button.click();
    await browser.waitUntil(
      async () => (await button.getAttribute("aria-current")) === "true",
    );
  }
  await button.click();
  const dialog = await segmentConfigDialog(segmentNumber);
  await dialog.waitForExist();
  const group = await dialog.$(`[aria-label="片段 ${segmentNumber} 生成方案"]`);
  await group.waitForExist();
  return { button, dialog, group };
};

const closeSegmentConfig = async (segmentNumber) => {
  const dialog = await segmentConfigDialog(segmentNumber);
  const close = await dialog.$('button[aria-label="关闭"]');
  await close.waitForClickable();
  await close.click();
  await dialog.waitForExist({ reverse: true });
};

const persistedSnapshots = () =>
  browser.execute(() =>
    window.__A4N_VIDEO_MODE_E2E__.calls
      .filter((call) => call.command === "persisted_recipe_snapshot")
      .map(({ target, modes }) => ({ target, modes })),
  );

const segmentPlanStarts = () =>
  browser.execute(() =>
    window.__A4N_VIDEO_MODE_E2E__.calls
      .filter((call) => call.command === "start_video_generation")
      .map((call) => call.args.input.operation.options),
  );

const segmentPlanRetryTaskIds = () =>
  browser.execute(() =>
    window.__A4N_VIDEO_MODE_E2E__.calls
      .filter((call) => call.command === "start_video_generation")
      .map((call) => call.args.input.meta.retry_of_task_id),
  );

const recipeUpdates = () =>
  browser.execute(() =>
    window.__A4N_VIDEO_MODE_E2E__.calls
      .filter((call) => call.command === "update_chapter_video_segment_recipe")
      .map((call) => call.args.recipe),
  );

const postSelectRefreshes = () =>
  browser.execute(
    () =>
      window.__A4N_VIDEO_MODE_E2E__.calls.filter(
        (call) => call.command === "post_select_refresh_started",
      ).length,
  );

const framePromptMutations = () =>
  browser.execute(() =>
    window.__A4N_VIDEO_MODE_E2E__.calls
      .filter(
        (call) =>
          call.command === "update_chapter_video_prompt" ||
          call.command === "optimize_chapter_video_prompt",
      )
      .map((call) => ({ command: call.command, args: call.args })),
  );

const deletedImageIds = () =>
  browser.execute(() =>
    window.__A4N_VIDEO_MODE_E2E__.calls
      .filter((call) => call.command === "delete_image")
      .map((call) => call.args.input.image_id),
  );

const draftRecipeUpdates = () =>
  browser.execute(() =>
    window.__A4N_VIDEO_MODE_E2E__.calls
      .filter((call) => call.command === "persisted_draft_recipe_snapshot")
      .map((call) => call.mode),
  );

const selectSegmentVideoCalls = () =>
  browser.execute(() =>
    window.__A4N_VIDEO_MODE_E2E__.calls
      .filter((call) => call.command === "select_segment_video")
      .map((call) => ({
        videoId: call.args.videoId,
        segmentPlanPromptId: call.args.input.segment_plan_prompt_id,
        targetSegment: call.args.input.target_segment,
      })),
  );

const adoptionSnapshots = () =>
  browser.execute(() =>
    window.__A4N_VIDEO_MODE_E2E__.calls
      .filter((call) => call.command === "persisted_adoption_snapshot")
      .map(({ videoId, adopted }) => ({ videoId, adopted })),
  );

const assembleChapterVideoCalls = () =>
  browser.execute(() =>
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

    assert.equal(await $('[aria-label="片段 1 生成方案"]').isExisting(), false);
    const results = await $('//section[@aria-label="视频结果"]');
    await results.waitForExist();
    assert.match(await results.getText(), /还没有生成视频/);
    assert.match(await segmentHeaderText(1), /首尾帧生成/);
    assert.match(await segmentHeaderText(2), /分镜生成/);

    const { group: firstGroup } = await openSegmentConfig(1);
    await firstGroup.$('.//button[normalize-space()="Chat"]').click();
    await browser.waitUntil(
      async () => (await persistedSnapshots()).length === 1,
    );
    assert.match(await segmentHeaderText(1), /Chat 生视频/);
    assert.match(await segmentHeaderText(2), /分镜生成/);

    await closeSegmentConfig(1);
    const { group: secondGroup } = await openSegmentConfig(2);
    await secondGroup.$('.//button[normalize-space()="首尾帧"]').click();
    await browser.waitUntil(
      async () => (await persistedSnapshots()).length === 2,
    );

    assert.match(await segmentHeaderText(1), /Chat 生视频/);
    assert.match(await segmentHeaderText(2), /首尾帧生成/);
    assert.deepEqual(await persistedSnapshots(), [
      { target: 1, modes: { 1: "chat_video", 2: "storyboard" } },
      { target: 2, modes: { 1: "chat_video", 2: "boundary_frames" } },
    ]);
  });

  it("saves different segments independently instead of blocking on one global queue", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=parallel-recipe-saves");

    const { group: firstGroup } = await openSegmentConfig(1);
    await firstGroup.$('.//button[normalize-space()="Chat"]').click();
    assert.match(await segmentHeaderText(1), /Chat 生视频/);

    await closeSegmentConfig(1);
    const { group: secondGroup } = await openSegmentConfig(2);
    await secondGroup.$('.//button[normalize-space()="首尾帧"]').click();

    await browser.waitUntil(
      async () => (await persistedSnapshots()).length === 2,
    );
    assert.deepEqual(await persistedSnapshots(), [
      { target: 2, modes: { 1: "boundary_frames", 2: "boundary_frames" } },
      { target: 1, modes: { 1: "chat_video", 2: "boundary_frames" } },
    ]);
    assert.match(await segmentHeaderText(1), /Chat 生视频/);
    assert.match(await segmentHeaderText(2), /首尾帧生成/);
  });

  it("closes only the top modal and restores focus through the modal stack", async () => {
    await browser.url("/e2e/video-mode-switch/");

    const { dialog } = await openSegmentConfig(1);
    const referenceTrigger = await dialog.$(
      'button[aria-label="添加参考图或素材"]',
    );
    await referenceTrigger.waitForClickable();
    await referenceTrigger.click();

    const materialDialog = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="添加参考图 / 素材"]]',
    );
    await materialDialog.waitForExist();
    assert.equal(
      await browser.execute(() => document.body.style.overflow),
      "hidden",
    );

    await browser.keys(["Escape"]);
    await materialDialog.waitForExist({ reverse: true });
    await dialog.waitForExist();
    assert.equal(
      await browser.execute(() => document.body.style.overflow),
      "hidden",
    );
    assert.match(
      await browser.execute(() => document.activeElement?.textContent ?? ""),
      /添加第一张|继续添加/,
    );

    await browser.keys(["Escape"]);
    await dialog.waitForExist({ reverse: true });
    await browser.waitUntil(
      async () =>
        (await browser.execute(() => document.body.style.overflow)) !==
        "hidden",
    );
    await browser.waitUntil(
      async () =>
        (await browser.execute(() =>
          document.activeElement?.getAttribute("data-video-segment"),
        )) === "1",
    );
    assert.equal((await recipeUpdates()).length, 0);
  });

  it("shows required mode inputs as clickable slots", async () => {
    await browser.url("/e2e/video-mode-switch/");

    const { dialog } = await openSegmentConfig(1);
    assert.match(await dialog.getText(), /必需输入/);
    assert.equal(
      await dialog.$('button[aria-label="配置首帧"]').isExisting(),
      true,
    );
    assert.equal(
      await dialog.$('button[aria-label="配置尾帧"]').isExisting(),
      true,
    );
    assert.equal(
      await dialog.$('button[aria-label="添加参考图或素材"]').isExisting(),
      true,
    );
    const slotLayout = await browser.execute(() => {
      const slot = document.querySelector(
        '[role="dialog"] button[aria-label="配置首帧"]',
      );
      const preview = slot?.children[0]?.getBoundingClientRect();
      const rect = slot?.getBoundingClientRect();
      const endSlot = document.querySelector(
        '[role="dialog"] button[aria-label="配置尾帧"]',
      );
      const endPreview = endSlot?.children[0]?.getBoundingClientRect();
      const endRect = endSlot?.getBoundingClientRect();
      return {
        slotHeight: rect?.height ?? 0,
        previewWidth: preview?.width ?? 0,
        previewHeight: preview?.height ?? 0,
        endSlotHeight: endRect?.height ?? 0,
        endPreviewWidth: endPreview?.width ?? 0,
        endPreviewHeight: endPreview?.height ?? 0,
        topDifference: rect && endRect ? Math.abs(rect.top - endRect.top) : null,
        gap: rect && endRect ? endRect.left - rect.right : null,
      };
    });
    const referenceSlotLayout = await browser.execute(() => {
      const slot = document.querySelector(
        '[role="dialog"] button[aria-label="添加参考图或素材"]',
      );
      const rect = slot?.getBoundingClientRect();
      return { width: rect?.width ?? 0, height: rect?.height ?? 0 };
    });
    assert.ok(slotLayout.slotHeight <= 240, `Required frame slot exceeds its compact height: ${JSON.stringify(slotLayout)}`);
    assert.ok(slotLayout.endSlotHeight <= 240, `Required tail slot exceeds its compact height: ${JSON.stringify(slotLayout)}`);
    assert.ok(Math.abs(slotLayout.previewWidth / slotLayout.previewHeight - 16 / 9) < 0.02);
    assert.ok(Math.abs(slotLayout.endPreviewWidth / slotLayout.endPreviewHeight - 16 / 9) < 0.02);
    assert.ok(
      Math.abs(slotLayout.previewWidth - slotLayout.endPreviewWidth) <= 4,
      `The required frame pair must have matching widths: ${JSON.stringify(slotLayout)}`,
    );
    assert.ok(slotLayout.topDifference != null && slotLayout.topDifference <= 1);
    assert.ok(slotLayout.gap != null && slotLayout.gap >= 0);
    assert.ok(referenceSlotLayout.width >= 44 && referenceSlotLayout.height >= 44);

    const firstFrame = await dialog.$('button[aria-label="配置首帧"]');
    await firstFrame.click();
    const frameDialog = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="首帧配置"]]',
    );
    await frameDialog.waitForExist();
    await frameDialog
      .$('.//button[contains(normalize-space(), "准备首帧提示词")]')
      .waitForExist();
    await browser.keys(["Escape"]);
    await frameDialog.waitForExist({ reverse: true });
    await dialog.waitForExist();

    const referenceTrigger = await dialog.$(
      'button[aria-label="添加参考图或素材"]',
    );
    await revealByWheel(referenceTrigger);
    await referenceTrigger.click();
    const materialDialog = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="添加参考图 / 素材"]]',
    );
    await materialDialog.waitForExist();
    await materialDialog.$('.//button[normalize-space()="生成参考"]').click();
    await browser.waitUntil(
      async () => (await persistedSnapshots()).length === 1,
    );
    const closeMaterialDialog = await materialDialog.$(
      'button[aria-label="关闭"]',
    );
    await closeMaterialDialog.waitForClickable();
    await closeMaterialDialog.click();
    await materialDialog.waitForExist({ reverse: true });
    const updatedDialog = await segmentConfigDialog(1);
    await updatedDialog
      .$('button[aria-label="管理参考素材 701"]')
      .waitForExist();
    await updatedDialog
      .$('button[aria-label="添加参考图或素材"]')
      .waitForExist();
  });

  it("shows generated storyboard assets after fingerprint verification", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=storyboard-assets");
    const { dialog } = await openSegmentConfig(2);
    await browser.waitUntil(async () =>
      (await dialog.getText()).includes("输入已就绪"),
    );
    assert.doesNotMatch(await dialog.getText(), /请先准备当前片段的分镜提示词/);
    assert.match(
      await dialog.$('button[aria-label="配置分镜板"]').getText(),
      /已填充.*片段二分镜板/s,
    );
    assert.equal(
      await dialog.$('button[aria-label="配置分镜板"] img').isExisting(),
      true,
    );

    await dialog.$('button[aria-label="配置分镜板"]').click();
    const frameDialog = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="分镜板配置"]]',
    );
    await frameDialog.waitForExist();
    const prompt = await frameDialog.$(
      'textarea[aria-label="分镜板图片提示词"]',
    );
    await prompt.waitForExist();
    assert.equal(await prompt.getValue(), "storyboard");
    assert.equal(await prompt.getProperty("readOnly"), true);
    assert.equal(await prompt.isEnabled(), true, "A locked saved prompt must remain focusable and copyable");
    await prompt.click();
    await browser.keys(["Control", "a"]);
    await browser.releaseActions();
    assert.equal(await prompt.execute((element) =>
      element === document.activeElement && element.selectionStart === 0 && element.selectionEnd === element.value.length), true);
    await browser.keys("不得覆盖已有图片的提示词");
    assert.equal(await prompt.getValue(), "storyboard", "Read-only prompts must reject typing while an image exists");
    assert.equal(
      await frameDialog
        .$('.//button[normalize-space()="导入 图片"]')
        .isEnabled(),
      false,
    );
    assert.equal(
      await frameDialog
        .$('.//button[normalize-space()="生成图片"]')
        .isEnabled(),
      false,
    );
    const lockedMenu = await openFrameMenu(frameDialog);
    const lockedOptimize = await frameMenuItem(lockedMenu.menu, "优化提示词");
    assert.equal(await lockedOptimize.getAttribute("aria-disabled"), "true");
    await lockedOptimize.click();
    assert.equal(await $('//div[@role="dialog" and .//h3[normalize-space()="优化文字资产"]]').isExisting(), false);
    assert.equal((await framePromptMutations()).length, 0);
    await frameMenuItem(lockedMenu.menu, "删除分镜板图片").click();
    const deleteDialog = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="删除素材图"]]',
    );
    await deleteDialog.waitForDisplayed();
    await browser.waitUntil(async () => (await deleteDialog.getText()).includes("相关视频参考图会同步解除关联"), {
      timeout: 2500, timeoutMsg: "The deletion confirmation must visibly explain the reference unlinking before confirmation",
    });
    assert.match(await deleteDialog.getText(), /相关视频参考图会同步解除关联/);
    assert.deepEqual(await deletedImageIds(), [], "Opening the confirmation must not delete the image");
    await deleteDialog.$('.//button[normalize-space()="删除"]').click();
    await browser.waitUntil(async () => (await deletedImageIds()).length === 1);
    assert.deepEqual(await deletedImageIds(), [179]);
    await deleteDialog.waitForExist({ reverse: true });
    await frameDialog.waitForExist();
    try {
      await browser.waitUntil(async () => (await prompt.getProperty("readOnly")) === false);
    } catch (error) {
      const state = await browser.execute(() => ({
        commands: window.__A4N_VIDEO_MODE_E2E__.calls.map(call => call.command),
        readOnly: document.querySelector('textarea[aria-label="分镜板图片提示词"]')?.readOnly,
        alerts: [...document.querySelectorAll('[role="alert"]')].map(element => element.textContent),
      }));
      assert.fail(`Deleting image 179 did not unlock its saved prompt: ${error.message}; ${JSON.stringify(state)}`);
    }
    assert.equal(await prompt.isEnabled(), true);
    const unlockedMenu = await openFrameMenu(frameDialog);
    assert.equal(
      await frameMenuItem(unlockedMenu.menu, "删除分镜板图片").isExisting(),
      false,
    );
    assert.notEqual(await frameMenuItem(unlockedMenu.menu, "优化提示词").getAttribute("aria-disabled"), "true");
    await unlockedMenu.trigger.click();
    await unlockedMenu.menu.waitForExist({ reverse: true });
    assert.equal(
      await frameDialog
        .$('.//button[normalize-space()="生成图片"]')
        .isEnabled(),
      true,
    );

    const edited = "编辑后的分镜图片提示词，保留人物、空间关系和镜头构图。";
    await prompt.setValue(edited);
    const generate = await frameDialog.$(
      './/button[normalize-space()="生成图片"]',
    );
    assert.equal(await generate.isEnabled(), false);
    await frameDialog.$('.//button[normalize-space()="保存更改"]').click();
    await browser.waitUntil(
      async () => (await framePromptMutations()).length === 1,
    );
    assert.equal(
      (await framePromptMutations())[0].args.input.visual_prompt,
      edited,
    );
    assert.equal(await prompt.getValue(), edited);

    const optimizeMenu = await openFrameMenu(frameDialog);
    const optimize = await frameMenuItem(optimizeMenu.menu, "优化提示词");
    assert.notEqual(await optimize.getAttribute("aria-disabled"), "true");
    await optimize.click();
    const optimizeDialog = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="优化文字资产"]]',
    );
    await optimizeDialog.waitForExist();
    await optimizeDialog.$("textarea").setValue("强化画面层次");
    await optimizeDialog.$('.//button[normalize-space()="开始优化"]').click();
    const progressDialog = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="优化提示词"]]',
    );
    await progressDialog.waitForExist();
    await browser.waitUntil(async () =>
      (await progressDialog.getText()).includes("正在调用模型并校验优化结果"),
    );
    await browser.waitUntil(
      async () => (await framePromptMutations()).length === 2,
    );
    await browser.waitUntil(async () =>
      (await progressDialog.getText()).includes("任务已完成"),
    );
    await progressDialog.$('.//button[normalize-space()="关闭"]').click();
    await progressDialog.waitForExist({ reverse: true });
    assert.equal(
      (await framePromptMutations())[1].args.input.instruction,
      "强化画面层次",
    );
    const optimized = `${edited}\n优化要求：强化画面层次`;
    await browser.waitUntil(async () => (await prompt.getValue()) === optimized);
    assert.equal(await prompt.getValue(), optimized, "Optimization must preserve the complete saved prompt and append the requested refinement");
  });

  it("confirms dirty prompt edits before closing and discards them only after confirmation", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=stale-plan-adoption");

    const { dialog } = await openSegmentConfig(1);
    const prompt = await dialog.$(
      './/label[.//span[normalize-space()="Chat 视频提示词"]]//textarea',
    );
    await prompt.waitForExist();
    await prompt.setValue("尚未保存的提示词修改");
    const dirtyNotice = await dialog.$('.//div[not(*) and contains(normalize-space(), "当前编辑内容未保存")]');
    await revealByWheel(dirtyNotice);
    assert.match(await dirtyNotice.getText(), /当前编辑内容未保存.*生成视频前需要先保存/s);

    await browser.keys(["Escape"]);
    let confirm = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="放弃未保存的提示词编辑"]]',
    );
    await confirm.waitForExist();

    await browser.keys(["Escape"]);
    await confirm.waitForExist({ reverse: true });
    await dialog.waitForExist();
    assert.equal(await prompt.getValue(), "尚未保存的提示词修改");

    await browser.keys(["Escape"]);
    confirm = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="放弃未保存的提示词编辑"]]',
    );
    await confirm.waitForExist();
    const discard = await confirm.$(
      './/button[normalize-space()="放弃并关闭"]',
    );
    await discard.click();
    await dialog.waitForExist({ reverse: true });

    const reopened = await openSegmentConfig(1);
    const reopenedPrompt = await reopened.dialog.$(
      './/label[.//span[normalize-space()="Chat 视频提示词"]]//textarea',
    );
    await reopenedPrompt.waitForExist();
    assert.equal(await reopenedPrompt.getValue(), "最终提示词");
    await closeSegmentConfig(1);
  });

  it("saves dirty reference-media settings before closing without duplicating results", async () => {
    await browser.url(
      "/e2e/video-mode-switch/?scenario=reference-media-layout",
    );

    const pageResults = await $$('//section[@aria-label="视频结果"]');
    assert.equal(pageResults.length, 1);
    const { dialog } = await openSegmentConfig(1);
    const referenceWorkspace = await dialog.$(
      './/section[@aria-label="参考媒体生成设置"]',
    );
    await referenceWorkspace.waitForExist();
    assert.equal((await dialog.$$('section[aria-label="视频结果"]')).length, 0);

    const prompt = await referenceWorkspace.$(
      'textarea[placeholder*="整体画面"]',
    );
    const savedPrompt = "参考媒体关闭前保存测试".repeat(10);
    await prompt.setValue(savedPrompt);
    await closeSegmentConfig(1);
    await browser.waitUntil(async () => (await recipeUpdates()).length === 1);
    const [updatedRecipe] = await recipeUpdates();
    assert.equal(updatedRecipe.segment_number, 1);
    assert.equal(updatedRecipe.reference_media.prompt, savedPrompt);
    assert.equal((await $$('//section[@aria-label="视频结果"]')).length, 1);
  });

  it("shows an adopted reference-media video without a final prompt", async () => {
    await browser.url(
      "/e2e/video-mode-switch/?scenario=reference-media-adopted-video",
    );
    const results = await $('//section[@aria-label="视频结果"]');
    await results.waitForExist();
    await browser.execute(() =>
      window.__A4N_VIDEO_MODE_E2E__.publishReferenceMediaVideo(),
    );
    const second = await pageSegmentButton(2);
    await second.click();
    await browser.waitUntil(
      async () => await results.$('[data-video-preview-id="709"]').isExisting(),
    );
  });

  it("keeps the current reference-media segment open when draft saving fails", async () => {
    await browser.url(
      "/e2e/video-mode-switch/?scenario=reference-media-save-failure",
    );

    const { dialog } = await openSegmentConfig(1);
    const prompt = await dialog.$(
      './/section[@aria-label="参考媒体生成设置"]//textarea[@placeholder and contains(@placeholder, "整体画面")]',
    );
    await prompt.setValue("这个草稿必须在切换片段前保存".repeat(8));

    await browser.execute(() => {
      const target = [
        ...document.querySelectorAll('button[data-video-segment="2"]'),
      ].find((button) => !button.closest('[role="dialog"]'));
      target?.click();
    });
    await browser.waitUntil(async () => (await recipeUpdates()).length >= 1);
    await dialog.waitForExist();
    assert.equal(await segmentConfigDialog(2).isExisting(), false);
    assert.equal(
      await pageSegmentButton(1).getAttribute("aria-current"),
      "true",
    );
    assert.match(await $("body").getText(), /参考媒体草稿保存失败测试/);
  });

  it("selects a segment first, follows its adopted video, and opens config on the second click", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=segment-selection");

    const results = await $('//section[@aria-label="视频结果"]');
    await results.waitForExist();
    await browser.waitUntil(
      async () => await results.$('[data-video-preview-id="701"]').isExisting(),
    );
    assert.doesNotMatch(
      await results.getText(),
      /片段 1 普通候选视频|成片记录/,
    );

    const second = await pageSegmentButton(2);
    await second.click();
    await browser.waitUntil(
      async () => (await second.getAttribute("aria-current")) === "true",
    );
    assert.equal(await segmentConfigDialog(2).isExisting(), false);
    await browser.waitUntil(
      async () => await results.$('[data-video-preview-id="704"]').isExisting(),
    );
    assert.doesNotMatch(
      await results.getText(),
      /片段 2 普通候选视频|成片记录/,
    );

    await second.click();
    await segmentConfigDialog(2).waitForExist();
  });

  it("places compact segment controls above the full-width workspace", async () => {
    await browser.url("/e2e/video-mode-switch/");
    const { dialog } = await openSegmentConfig(1);
    const layout = await browser.execute(() => {
      const modal = [...document.querySelectorAll('[role="dialog"]')].find(
        (node) =>
          node.querySelector("h3")?.textContent?.includes("片段 1 · 配置"),
      );
      const setup = modal?.querySelector('[aria-label="片段配置选项"]');
      const workspace = modal?.querySelector('[aria-label="片段生成工作区"]');
      const setupRect = setup?.getBoundingClientRect();
      const workspaceRect = workspace?.getBoundingClientRect();
      return {
        setupTag: setup?.tagName ?? "",
        setupLeft: setupRect?.left ?? 0,
        setupRight: setupRect?.right ?? 0,
        setupBottom: setupRect?.bottom ?? 0,
        workspaceLeft: workspaceRect?.left ?? 0,
        workspaceRight: workspaceRect?.right ?? 0,
        workspaceTop: workspaceRect?.top ?? 0,
      };
    });
    assert.equal(layout.setupTag, "SECTION");
    assert.ok(Math.abs(layout.setupLeft - layout.workspaceLeft) <= 1);
    assert.ok(Math.abs(layout.setupRight - layout.workspaceRight) <= 1);
    assert.ok(layout.setupBottom <= layout.workspaceTop + 1);
    await dialog.$('button[aria-label="关闭"]').click();
    await dialog.waitForExist({ reverse: true });

    await browser.url("/e2e/video-mode-switch/?scenario=stale-plan-adoption");
    const versionsDialog = (await openSegmentConfig(1)).dialog;
    const versionLayout = await browser.execute(() => {
      const list = document.querySelector(
        '[role="dialog"] ul[aria-label="片段 1 提示词版本"]',
      );
      const item = list?.querySelector("li");
      const listRect = list?.getBoundingClientRect();
      const itemRect = item?.getBoundingClientRect();
      return {
        count: list?.children.length ?? 0,
        widthDifference:
          listRect && itemRect
            ? Math.abs(listRect.width - itemRect.width)
            : null,
      };
    });
    assert.equal(versionLayout.count, 1);
    assert.ok((versionLayout.widthDifference ?? Number.POSITIVE_INFINITY) <= 4);
    await versionsDialog.$('button[aria-label="关闭"]').click();
    await versionsDialog.waitForExist({ reverse: true });
  });

  it("uses a single-column bottom workspace on a mobile viewport", async () => {
    await browser.setWindowSize(390, 844);
    try {
      await browser.url("/e2e/video-mode-switch/");
      const mobileTabs = await $('[aria-label="视频生成移动工作区"]');
      await mobileTabs.waitForExist();
      assert.match(await mobileTabs.getText(), /片段/);
      assert.match(await mobileTabs.getText(), /视频/);

      const { dialog } = await openSegmentConfig(1);
      const layout = await browser.execute(() => {
        const modal = [...document.querySelectorAll('[role="dialog"]')].find(
          (node) =>
            node.querySelector("h3")?.textContent?.includes("片段 1 · 配置"),
        );
        const setup = modal?.querySelector('[aria-label="片段配置选项"]');
        const workspace = modal?.querySelector('[aria-label="片段生成工作区"]');
        const modalRect = modal?.getBoundingClientRect();
        const setupRect = setup?.getBoundingClientRect();
        const workspaceRect = workspace?.getBoundingClientRect();
        return {
          viewportHeight: window.innerHeight,
          width: modalRect?.width ?? 0,
          height: modalRect?.height ?? 0,
          bottom: modalRect?.bottom ?? 0,
          setupBottom: setupRect?.bottom ?? 0,
          workspaceTop: workspaceRect?.top ?? 0,
        };
      });
      assert.ok(layout.width >= 380 && layout.width <= 390);
      assert.ok(layout.height >= layout.viewportHeight - 20);
      assert.ok(layout.bottom >= layout.viewportHeight - 1);
      assert.ok(layout.setupBottom <= layout.workspaceTop + 1);
      await dialog.$('button[aria-label="关闭"]').click();
      await dialog.waitForExist({ reverse: true });
    } finally {
      await browser.setWindowSize(1440, 1000);
    }
  });

  it("retains a saved segment after cancellation and resumes from the next shot", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=cancel-resume");

    const planning = await $('section[aria-label="视频片段规划"]');
    await planning.waitForExist();
    const generate = await $(
      '//button[contains(normalize-space(), "规划此场景")]',
    );
    await generate.waitForEnabled();
    await revealByWheel(generate);
    await generate.click();

    const savedSegment = await $('//*[normalize-space()="片段 1"]');
    await savedSegment.waitForExist();
    const stop = await $('//button[normalize-space()="停止生成"]');
    await stop.waitForClickable();
    await stop.click();

    const close = await $('//button[normalize-space()="关闭"]');
    await close.waitForClickable();
    await close.click();

    let resume = await $('//button[contains(normalize-space(), "继续生成")]');
    await resume.waitForEnabled();
    assert.match(await $("body").getText(), /50%.*已保存镜头 1.*1\/2 个镜头/s);
    assert.equal(await $('//*[normalize-space()="片段 1"]').isExisting(), true);
    assert.equal((await planning.$$("ol > li")).length, 1);
    assert.equal(await planning.$("ol > li p").getText(), "镜头 1 的已保存片段");

    const videoTab = await $(
      '//button[@role="tab" and normalize-space()="片段生成"]',
    );
    await videoTab.waitForClickable();
    await videoTab.click();
    await browser.waitUntil(async () => (await videoTab.getAttribute("aria-selected")) === "true");
    await $('section[aria-label="视频结果"]').waitForExist();
    // Cancelled drafts remain on the planning page until resumed or published.
    assert.equal(await pageSegmentButton(1).isExisting(), false);
    assert.deepEqual(await draftRecipeUpdates(), []);

    const planningTab = await $(
      '//button[@role="tab" and normalize-space()="片段规划"]',
    );
    await planningTab.click();
    await browser.waitUntil(async () => (await planningTab.getAttribute("aria-selected")) === "true");
    await planning.waitForExist();
    assert.equal(await planning.$("ol > li p").getText(), "镜头 1 的已保存片段");
    resume = await $('//button[contains(normalize-space(), "继续生成")]');
    await resume.waitForExist();

    const startsBeforeResume = await segmentPlanStarts();
    assert.equal(startsBeforeResume.length, 1);
    const originalOptions = startsBeforeResume[0];

    const configuration = await $(
      '//summary[contains(normalize-space(), "规划配置")]',
    );
    await revealByWheel(configuration);
    await configuration.click();
    const style = await $('textarea[placeholder*="整体风格"]');
    await revealByWheel(style);
    await style.setValue("不应进入恢复请求的当前表单风格");
    await revealByWheel(resume);
    await resume.click();

    await browser.waitUntil(
      async () => (await segmentPlanStarts()).length === 2,
    );
    const startsAfterResume = await segmentPlanStarts();
    assert.deepEqual(startsAfterResume[1], originalOptions);
    assert.notEqual(startsAfterResume[1].style, await style.getValue());
    assert.deepEqual(await segmentPlanRetryTaskIds(), [
      null,
      "00000000-0000-4000-8000-000000000001",
    ]);
    await browser.waitUntil(async () =>
      (await $("body").getText()).includes("正在生成镜头 2"),
    );

    const stopResumed = await $('//button[normalize-space()="停止生成"]');
    await stopResumed.waitForClickable();
    await stopResumed.click();
    const closeResumed = await $('//button[normalize-space()="关闭"]');
    await closeResumed.waitForClickable();
    await closeResumed.click();
    resume = await $('//button[contains(normalize-space(), "继续生成")]');
    await resume.waitForEnabled();
    assert.equal(await $('//*[normalize-space()="片段 1"]').isExisting(), true);
    assert.equal((await planning.$$("ol > li")).length, 1);
    assert.equal(await planning.$("ol > li p").getText(), "镜头 1 的已保存片段");
    assert.deepEqual(await browser.execute(() => window.__A4N_VIDEO_MODE_E2E__.calls
      .filter((call) => call.command === "cancel_generation_task")
      .map((call) => call.args.input.task_id)), [
      "00000000-0000-4000-8000-000000000001",
      "00000000-0000-4000-8000-000000000002",
    ]);
  });

  it("keeps published planning content visible while normal planning runs", async () => {
    await browser.url(
      "/e2e/video-mode-switch/?scenario=planning-content-running",
    );

    const planning = await $('section[aria-label="视频片段规划"]');
    await planning.waitForExist();
    const published = await planning.$('//*[normalize-space()="片段一"]');
    await published.waitForExist();

    const generate = await planning.$(
      './/button[contains(normalize-space(), "规划此场景")]',
    );
    await generate.waitForEnabled();
    await generate.click();

    const draft = await planning.$('//*[normalize-space()="片段 2"]');
    await draft.waitForExist();
    assert.equal(await published.isExisting(), true);
    assert.match(await planning.getText(), /生成中草稿.*2 个片段/s);

    const stop = await $('//button[normalize-space()="停止生成"]');
    await stop.waitForClickable();
    await stop.click();
    const close = await $('//button[normalize-space()="关闭"]');
    await close.waitForClickable();
    await close.click();
  });

  it("continues a boundary-frame segment with a later first-frame-chain segment", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=cross-mode-continuity");

    assert.match(await segmentHeaderText(1), /首尾帧生成/);

    const { dialog, group: secondGroup } = await openSegmentConfig(2);
    await secondGroup.$('.//button[normalize-space()="首帧续接"]').click();

    await browser.waitUntil(async () => (await recipeUpdates()).length === 1);
    const [updatedRecipe] = await recipeUpdates();
    assert.equal(updatedRecipe.segment_number, 2);
    assert.equal(updatedRecipe.workflow_mode, "first_frame_chain");
    assert.equal(updatedRecipe.guidance_mode, "continuity");
    assert.equal(updatedRecipe.continuity_source_video_id, null);
    assert.match(await segmentHeaderText(1), /首尾帧生成/);
    assert.match(await segmentHeaderText(2), /首帧续接/);

    const firstFrameSource = await dialog.$('[aria-label="首帧来源"]');
    await firstFrameSource.waitForExist();
    const independent = await firstFrameSource.$(
      './/button[normalize-space()="独立生成"]',
    );
    await independent.waitForClickable();
    await independent.click();
    await browser.waitUntil(async () => (await recipeUpdates()).length === 2);
    const [, independentRecipe] = await recipeUpdates();
    assert.equal(independentRecipe.guidance_mode, "system_frames");
    assert.equal(independentRecipe.continuity_source_video_id, null);
    assert.equal(
      await dialog.$('button[aria-label="配置首帧"]').isExisting(),
      true,
    );
    assert.equal(
      await dialog.$('button[aria-label="配置续接首帧"]').isExisting(),
      false,
    );

    const continuity = await dialog.$(
      './/*[@aria-label="首帧来源"]//button[normalize-space()="尾帧续接"]',
    );
    await continuity.waitForClickable();
    await continuity.click();
    await browser.waitUntil(async () => (await recipeUpdates()).length === 3);
    const continuityRecipe = (await recipeUpdates())[2];
    assert.equal(continuityRecipe.guidance_mode, "continuity");

    const continuitySlot = await dialog.$('button[aria-label="配置续接首帧"]');
    await continuitySlot.waitForClickable();
    await continuitySlot.click();
    const continuityDialog = await $(
      '//div[@role="dialog" and .//h3[normalize-space()="续接首帧配置"]]',
    );
    await continuityDialog.waitForExist();
    const source = await continuityDialog.$('button[role="combobox"]');
    await source.waitForDisplayed();
    assert.equal(await source.getText(), "选择更早片段已采用的视频…");
    await source.waitForEnabled();
    await source.click();
    const option = await $('//div[@role="option" and contains(., "501")]');
    await option.waitForClickable();
    await option.click();
    await browser.waitUntil(async () => (await recipeUpdates()).length === 4);
    await browser.execute(() => window.dispatchEvent(new Event("focus")));
    await browser.waitUntil(async () => (await postSelectRefreshes()) === 1);
    const sourceRecipe = (await recipeUpdates())[3];
    assert.equal(sourceRecipe.continuity_source_video_id, 501);
    await browser.waitUntil(async () => (await source.getText()).includes("501"));
  });

  it("keeps old-plan videos out of candidates and only adopts the current plan video", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=stale-plan-adoption");

    const { group: firstGroup } = await openSegmentConfig(1);
    assert.match(await segmentHeaderText(1), /Chat 生视频/);
    assert.equal(
      await firstGroup
        .$('.//button[@data-active="1" and normalize-space()="Chat"]')
        .isExisting(),
      true,
    );
    await closeSegmentConfig(1);

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

    await browser.waitUntil(
      async () => (await selectSegmentVideoCalls()).length === 1,
    );
    assert.deepEqual(await selectSegmentVideoCalls(), [
      {
        videoId: 502,
        segmentPlanPromptId: 41,
        targetSegment: {
          source_shot_id: "unit_1_shot_1",
          segment_index_in_shot: 1,
        },
      },
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
            target_segment: {
              source_shot_id: "unit_1_shot_1",
              segment_index_in_shot: 1,
            },
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
      targetSegment: {
        source_shot_id: "unit_1_shot_1",
        segment_index_in_shot: 1,
      },
    });
  });

  it("keeps prompt content visible while video generation is running", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=stale-plan-adoption");

    const { dialog } = await openSegmentConfig(1);
    const prompt = await dialog.$(
      './/label[.//span[normalize-space()="Chat 视频提示词"]]//textarea',
    );
    await prompt.waitForExist();
    await browser.execute(() => {
      window.__A4N_VIDEO_MODE_E2E__.setFinalPromptStatus("running");
      window.dispatchEvent(new Event("focus"));
    });
    await browser.waitUntil(async () =>
      (await dialog.getText()).includes("当前提示词正在运行，以下内容暂时只读"),
    );
    await prompt.waitForExist();
    assert.equal(await prompt.isEnabled(), false);
    assert.equal(await prompt.getValue(), "最终提示词");
  });

  it("assembles the valid prefix when a later adopted video is stale", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=mixed-assembly-gap");
    const assemblyTab = await $('//button[@role="tab" and normalize-space()="成片合成"]');
    await assemblyTab.waitForClickable();
    await assemblyTab.click();
    await browser.waitUntil(async () => (await assemblyTab.getAttribute("aria-selected")) === "true");

    const previewTitle = await $('//h3[normalize-space()="成片预览"]');
    await previewTitle.waitForExist();
    const body = await $("body");
    await browser.waitUntil(async () =>
      (await body.getText()).includes("计划片段 2 尚无有效视频；本次预览仅包含已采纳素材"),
    );
    assert.match(await body.getText(), /成片片段 1 有效视频|镜头 1/);
    assert.doesNotMatch(await body.getText(), /成片片段 2 有效视频/);

    const generate = await $(
      '//button[contains(normalize-space(), "生成预览")]',
    );
    await generate.waitForExist();
    await generate.waitForEnabled();
    await generate.click();
    await browser.waitUntil(
      async () => (await assembleChapterVideoCalls()).length === 1,
    );
    assert.deepEqual(await assembleChapterVideoCalls(), [
      { projectId: 1, chapterId: 2, workflowMode: "mixed" },
    ]);
    await browser.waitUntil(async () =>
      (await body.getText()).includes("更新预览"),
    );
  });

  it("assembles all valid videos when every current-plan segment is adopted", async () => {
    await browser.url("/e2e/video-mode-switch/?scenario=mixed-assembly-ready");
    const assemblyTab = await $('//button[@role="tab" and normalize-space()="成片合成"]');
    await assemblyTab.waitForClickable();
    await assemblyTab.click();
    await browser.waitUntil(async () => (await assemblyTab.getAttribute("aria-selected")) === "true");

    const previewTitle = await $('//h3[normalize-space()="成片预览"]');
    await previewTitle.waitForExist();
    const body = await $("body");
    await browser.waitUntil(async () =>
      (await body.getText()).includes("成片片段 1 有效视频"),
    );
    assert.match(await body.getText(), /成片片段 2 有效视频/);
    assert.doesNotMatch(await body.getText(), /本次预览仅包含已采纳素材/);

    const generate = await $(
      '//button[contains(normalize-space(), "生成预览")]',
    );
    await generate.waitForEnabled();
    await generate.click();

    await browser.waitUntil(
      async () => (await assembleChapterVideoCalls()).length === 1,
    );
    assert.deepEqual(await assembleChapterVideoCalls(), [
      { projectId: 1, chapterId: 2, workflowMode: "mixed" },
    ]);
    await browser.waitUntil(async () =>
      (await body.getText()).includes("更新预览"),
    );
  });

  it("hides inert background scrollbars while segment configuration is open", async () => {
    await browser.url("/e2e/video-mode-switch/");
    await openSegmentConfig(1);

    const state = await browser.execute(() => {
      const root = document.getElementById("root");
      const segmentList = [...(root?.querySelectorAll(".custom-scrollbar") ?? [])].find(
        (element) => element.querySelector("button[data-video-segment]"),
      );
      return {
        rootInert: root?.hasAttribute("inert") ?? false,
        scrollbarDisplay: segmentList
          ? getComputedStyle(segmentList, "::-webkit-scrollbar").display
          : null,
      };
    });

    assert.equal(state.rootInert, true);
    assert.equal(state.scrollbarDisplay, "none");
  });
});
