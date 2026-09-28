import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const screenshotDir = fileURLToPath(
  new URL("../../../target/tmp/real-postprocess/", import.meta.url),
);
mkdirSync(screenshotDir, { recursive: true });

describe("real A4N postprocess window", () => {
  it("reports the actual app route and postprocess controls", async () => {
    await browser.waitUntil(
      () =>
        browser
          .execute(() => (document.body?.innerText?.trim().length ?? 0) > 0)
          .then(Boolean),
      { timeout: 30_000, timeoutMsg: "real A4N window did not initialize" },
    );
    await browser.waitUntil(
      () =>
        browser
          .execute(() => document.body?.innerText?.includes("进入视频项目") ?? false)
          .then(Boolean),
      { timeout: 60_000, timeoutMsg: "real project list did not finish loading" },
    );
    const beforeOpen = await browser.execute(() => ({
      url: location.href,
      text: document.body?.innerText ?? "",
      buttons: [...document.querySelectorAll("button")].map((item) => ({
        textContent: item.textContent,
        innerText: item.innerText,
        html: item.outerHTML.slice(0, 240),
      })),
    }));
    process.stdout.write(`A4N_REAL_BEFORE_OPEN=${JSON.stringify(beforeOpen)}\n`);
    const openedProject = await browser.execute(() => {
      const button = [...document.querySelectorAll('[role="button"]')].find(
        (item) =>
          (item.textContent ?? "").includes("视频项目") &&
          (item.textContent ?? "").includes("视频已生成"),
      );
      if (!(button instanceof HTMLElement)) return false;
      button.click();
      return true;
    });
    assert.equal(openedProject, true, "real video project entry was not found");
    await browser.waitUntil(
      () => browser.getUrl().then((url) => url.includes("/video-project/")),
      { timeout: 30_000, timeoutMsg: "real video project did not open" },
    );
    await browser.pause(8_000);
    const openedPostprocess = await browser.execute(() => {
      const tab = [...document.querySelectorAll('[role="tab"]')].find(
        (item) => item.textContent?.includes("成片后处理"),
      );
      if (!(tab instanceof HTMLElement)) return false;
      tab.click();
      return true;
    });
    assert.equal(openedPostprocess, true, "postprocess tab was not found");
    await browser.pause(8_000);
    const state = await browser.execute(() => ({
      url: location.href,
      title: document.title,
      text: document.body?.innerText?.slice(0, 8000) ?? "",
      videos: document.querySelectorAll("video").length,
      timelines: document.querySelectorAll('[aria-label="视频剪辑时间轴"]').length,
      buttons: [...document.querySelectorAll("button")].slice(0, 80).map((button) => button.innerText),
    }));
    process.stdout.write(`A4N_REAL_APP_STATE=${JSON.stringify(state)}\n`);
    assert.ok(state.text.length > 0);

    for (let attempt = 0; attempt < 3; attempt += 1) {
      await browser.execute(() => {
        const marker = document.querySelector('[role="slider"][aria-label^="剪辑竖线"]');
        if (!(marker instanceof HTMLElement)) return;
        marker.focus();
        marker.dispatchEvent(
          new KeyboardEvent("keydown", {
            key: "Delete",
            bubbles: true,
            cancelable: true,
          }),
        );
      });
      await browser.pause(80);
    }
    await browser.waitUntil(
      () =>
        browser
          .execute(
            () =>
              document.querySelectorAll(
                '[role="slider"][aria-label^="剪辑竖线"]',
              ).length === 0,
          )
          .then(Boolean),
      { timeout: 2_000, timeoutMsg: "real marker was not deleted" },
    );

    const timeline = await browser.$('[aria-label="视频剪辑时间轴"]');
    await timeline.waitForDisplayed({ timeout: 10_000 });
    const timelineRect = await browser.execute(() => {
      const element = document.querySelector('[aria-label="视频剪辑时间轴"]');
      const rect = element?.getBoundingClientRect();
      return rect
        ? { left: rect.left, top: rect.top, width: rect.width, height: rect.height }
        : null;
    });
    assert.ok(timelineRect && timelineRect.width > 0, "real timeline has no bounds");
    process.stdout.write(
      `A4N_REAL_TIMELINE=${JSON.stringify(
        await browser.execute(() => ({
          innerWidth,
          innerHeight,
          devicePixelRatio,
          timeline: document
            .querySelector('[aria-label="视频剪辑时间轴"]')
            ?.getBoundingClientRect()
            .toJSON(),
          markers: [
            ...document.querySelectorAll(
              '[role="slider"][aria-label^="剪辑竖线"]',
            ),
          ].map((item) => ({
            rect: item.getBoundingClientRect().toJSON(),
            value: item.getAttribute("aria-valuenow"),
          })),
        })),
      )}\n`,
    );
    const nativeVideo = await browser.$("video");
    const screenshotHash = async (label) => {
      const path = `${screenshotDir}a4n-real-postprocess-${label}.png`;
      await nativeVideo.saveScreenshot(path);
      return createHash("sha256").update(readFileSync(path)).digest("hex");
    };

    const sample = async (label) => {
      const value = await browser.execute((sampleLabel) => {
        const video = document.querySelector("video");
        return {
          label: sampleLabel,
          userAgent: navigator.userAgent,
          currentTime: video?.currentTime ?? null,
          duration: video?.duration ?? null,
          paused: video?.paused ?? null,
          seeking: video?.seeking ?? null,
          readyState: video?.readyState ?? null,
          networkState: video?.networkState ?? null,
          videoWidth: video?.videoWidth ?? null,
          videoHeight: video?.videoHeight ?? null,
          error: video?.error?.message ?? null,
          scrubSeekPending: video?.dataset.scrubSeekPending === "true",
          canvasCount: document.querySelectorAll("canvas").length,
        };
      }, label);
      process.stdout.write(`A4N_REAL_SCRUB=${JSON.stringify(value)}\n`);
      return value;
    };

    const waitForRatio = (ratio, timeoutMsg) =>
      browser.waitUntil(
        () =>
          browser
            .execute((targetRatio) => {
              const video = document.querySelector("video");
              const expected = (video?.duration ?? 0) * targetRatio;
              return Boolean(
                video &&
                  !video.seeking &&
                  video.dataset.scrubSeekPending !== "true" &&
                  Math.abs(video.currentTime - expected) < 0.25,
              );
            }, ratio)
            .then(Boolean),
        { timeout: 10_000, timeoutMsg },
      );
    await browser
      .action("pointer", { parameters: { pointerType: "mouse" } })
      .move({
        origin: timeline,
        x: Math.round((0.2 - 0.5) * timelineRect.width),
        y: 0,
      })
      .down({ button: 0 })
      .up({ button: 0 })
      .perform();
    await browser.pause(100);
    const markerSample = await sample("after-marker-20");
    await waitForRatio(0.2, "real first marker did not present");
    await browser.execute(() => {
      const video = document.querySelector("video");
      if (video) video.currentTime = video.duration * 0.5;
    });
    await browser.waitUntil(
      () => browser.execute(() => {
        const video = document.querySelector("video");
        return Boolean(video && !video.seeking && Math.abs(video.currentTime / video.duration - 0.5) < 0.05);
      }),
      { timeout: 10_000, timeoutMsg: "native seek before marker keyboard checks did not settle" },
    );
    const focusedMarker = await browser.execute(() => {
      const marker = document.querySelector('[role="slider"][aria-label^="剪辑竖线"]');
      if (!(marker instanceof HTMLElement)) return false;
      marker.focus();
      marker.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
      return true;
    });
    assert.equal(focusedMarker, true, "keyboard marker was not available");
    await browser.pause(80);
    const enterOnMarkerCount = await browser.execute(() =>
      document.querySelectorAll('[role="slider"][aria-label^="剪辑竖线"]').length,
    );
    assert.equal(enterOnMarkerCount, 1, "Enter on a marker added an unrelated timeline marker");
    const focusedOutsideTimeline = await browser.execute(() => {
      const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
      const marker = timeline?.querySelector('[role="slider"][aria-label^="剪辑竖线"]');
      const outsideButton = [...document.querySelectorAll("button")].find((button) => !timeline?.contains(button));
      if (!(marker instanceof HTMLElement) || !(outsideButton instanceof HTMLElement)) return false;
      marker.focus();
      outsideButton.focus();
      outsideButton.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true, cancelable: true }));
      return true;
    });
    assert.equal(focusedOutsideTimeline, true, "an outside button was not available for the Delete key check");
    await browser.pause(80);
    const deleteOutsideTimelineCount = await browser.execute(() =>
      document.querySelectorAll('[role="slider"][aria-label^="剪辑竖线"]').length,
    );
    assert.equal(deleteOutsideTimelineCount, 1, "Delete outside the timeline removed the selected marker");
    const markerSettledSample = await sample("after-marker-20-1100ms");
    await browser.pause(2_000);
    const markerStableSample = await sample("after-marker-20-3100ms");
    const markerFrameHash = await screenshotHash("marker-20");
    assert.ok(
      markerSettledSample.currentTime != null &&
        Math.abs(markerSettledSample.currentTime - 0.2 * markerSettledSample.duration) < 0.25,
      "real seek did not settle near the requested first marker",
    );
    assert.equal(markerSettledSample.paused, true, "scrub decode kept playing after seek");
    assert.ok(
      markerStableSample.currentTime != null &&
        Math.abs(markerStableSample.currentTime - markerSettledSample.currentTime) < 0.05,
      "real seek drifted after it settled",
    );

    const marker = await browser.$('[role="slider"][aria-label="剪辑竖线 1"]');
    await marker.waitForDisplayed({ timeout: 10_000 });
    await browser
      .action("pointer", { parameters: { pointerType: "mouse" } })
      .move({ origin: marker })
      .down({ button: 0 })
      .move({
        origin: timeline,
        x: Math.round((0.55 - 0.5) * timelineRect.width),
        y: 0,
        duration: 100,
      })
      .move({
        origin: timeline,
        x: Math.round((0.8 - 0.5) * timelineRect.width),
        y: 0,
        duration: 100,
      })
      .up({ button: 0 })
      .perform();
    const dragSample = await sample("immediately-after-drag");
    await waitForRatio(0.8, "real drag target did not present");
    const dragSettledSample = await sample("after-drag-100ms");
    await browser.pause(500);
    await sample("after-drag-600ms");
    await browser.pause(1_000);
    await sample("after-drag-1600ms");
    const dragFrameHash = await screenshotHash("drag-80");
    assert.ok(
      dragSettledSample.currentTime != null &&
        Math.abs(dragSettledSample.currentTime - 0.8 * dragSettledSample.duration) < 0.25,
      "real drag did not settle near the requested final marker",
    );
    assert.equal(dragSettledSample.paused, true, "real drag left decode playback running");
    assert.equal(dragSettledSample.canvasCount, 0, "scrub canvas must not exist");
    assert.notEqual(dragFrameHash, markerFrameHash, "native video frame did not change");
    assert.notEqual(dragSample.currentTime, markerStableSample.currentTime);

    const rapidPath = [0.25, 0.85, 0.15, 0.9, 0.35, 0.7, 0.2, 0.8];
    const rapidDrag = browser.action("pointer", {
      parameters: { pointerType: "mouse" },
    });
    rapidDrag.move({ origin: marker }).down({ button: 0 });
    for (const ratio of rapidPath) {
      rapidDrag.move({
        origin: timeline,
        x: Math.round((ratio - 0.5) * timelineRect.width),
        y: 0,
        duration: 5,
      });
    }
    rapidDrag.up({ button: 0 });
    await rapidDrag.perform();
    const rapidTarget = rapidPath[rapidPath.length - 1];
    await waitForRatio(rapidTarget, "real continuous rapid drag did not settle");
    const rapidFinal = {
      ...(await sample("continuous-rapid-final")),
      frameHash: await screenshotHash("continuous-rapid-final"),
    };
    await browser.pause(1_000);
    const rapidStable = await sample("continuous-rapid-stable");
    assert.ok(
      rapidFinal.currentTime != null &&
        rapidFinal.duration != null &&
        Math.abs(rapidFinal.currentTime - rapidFinal.duration * rapidTarget) < 0.3 &&
        rapidFinal.canvasCount === 0 &&
        rapidFinal.error == null &&
        !rapidFinal.scrubSeekPending,
      `real continuous rapid drag failed: ${JSON.stringify(rapidFinal)}`,
    );
    assert.ok(
      rapidStable.error == null &&
        rapidStable.videoWidth > 0 &&
        Math.abs(rapidStable.currentTime - rapidFinal.currentTime) < 0.05,
      `real continuous rapid drag became unstable: ${JSON.stringify(rapidStable)}`,
    );

    const reverseTargets = [0.15, 0.85, 0.2, 0.8, 0.3, 0.7, 0.25, 0.75];
    const reverseSamples = [];
    for (const [index, ratio] of reverseTargets.entries()) {
      const reverseDrag = browser.action("pointer", {
        parameters: { pointerType: "mouse" },
      });
      reverseDrag
        .move({ origin: marker })
        .down({ button: 0 })
        .move({
          origin: timeline,
          x: Math.round((ratio - 0.5) * timelineRect.width),
          y: 0,
          duration: 20,
        })
        .up({ button: 0 });
      await reverseDrag.perform();
      await waitForRatio(ratio, `real reverse drag did not present ${ratio}`);
      reverseSamples.push({
        ...(await sample(`reverse-${index}-${ratio}`)),
        frameHash: await screenshotHash(`reverse-${index}-${ratio}`),
      });
    }
    assert.ok(
      reverseSamples.every(
        (item, index) =>
          item.currentTime != null &&
          item.duration != null &&
          Math.abs(item.currentTime - item.duration * reverseTargets[index]) < 0.3 &&
          item.canvasCount === 0 &&
          item.error == null &&
          item.videoWidth > 0 &&
          !item.scrubSeekPending,
      ),
      `real reverse drag did not follow decoded frames: ${JSON.stringify(reverseSamples)}`,
    );
    assert.ok(
      new Set(reverseSamples.map((item) => item.frameHash)).size >= 5,
      `native video frame remained frozen: ${JSON.stringify(reverseSamples)}`,
    );

    if (process.env.A4N_REAL_STRESS === "1") {
      const stressTargets = [
        0.08, 0.92, 0.16, 0.84, 0.24, 0.76, 0.32, 0.68, 0.4, 0.6,
      ];
      const stressHashes = [];
      for (let cycle = 0; cycle < 40; cycle += 1) {
        const target = stressTargets[cycle % stressTargets.length];
        const stressDrag = browser.action("pointer", {
          parameters: { pointerType: "mouse" },
        });
        stressDrag
          .move({ origin: marker })
          .down({ button: 0 });
        for (let step = 1; step <= 80; step += 1) {
          const ratio =
            stressTargets[(cycle + step) % stressTargets.length] *
              (step / 80) +
            target * (1 - step / 80);
          stressDrag.move({
            origin: timeline,
            x: Math.round((ratio - 0.5) * timelineRect.width),
            y: 0,
            duration: 2,
          });
        }
        stressDrag
          .move({
            origin: timeline,
            x: Math.round((target - 0.5) * timelineRect.width),
            y: 0,
            duration: 2,
          })
          .up({ button: 0 });
        await stressDrag.perform();
        await waitForRatio(target, `real stress drag did not settle ${cycle}`);
        const stressState = await sample(`stress-${cycle}`);
        assert.equal(stressState.error, null);
        assert.equal(stressState.paused, true);
        assert.equal(stressState.canvasCount, 0);
        stressHashes.push(await screenshotHash(`stress-${cycle}`));
      }
      assert.ok(
        new Set(stressHashes).size >= 8,
        `real stress drag rendered a frozen/black frame: ${JSON.stringify(stressHashes)}`,
      );

      const xAt = (ratio) => timelineRect.left + timelineRect.width * ratio;
      const y = timelineRect.top + timelineRect.height / 2;
      await browser.performActions([
        {
          type: "pointer",
          id: "real-held-stress",
          parameters: { pointerType: "mouse" },
          actions: [
            { type: "pointerMove", x: Math.round(xAt(0.6)), y: Math.round(y) },
            { type: "pointerDown", button: 0 },
          ],
        },
      ]);
      for (let index = 0; index < 200; index += 1) {
        const target = index % 2 === 0 ? 0.08 + (index % 20) * 0.04 : 0.92 - (index % 20) * 0.04;
        await browser.performActions([
          {
            type: "pointer",
            id: "real-held-stress",
            parameters: { pointerType: "mouse" },
            actions: [
              {
                type: "pointerMove",
                x: Math.round(xAt(target)),
                y: Math.round(y),
                duration: 20,
              },
            ],
          },
        ]);
        await waitForRatio(target, `real held stress did not settle ${index}`);
      }
      await browser.performActions([
        {
          type: "pointer",
          id: "real-held-stress",
          parameters: { pointerType: "mouse" },
          actions: [{ type: "pointerUp", button: 0 }],
        },
      ]);
      const heldStressState = await sample("held-stress-final");
      assert.equal(heldStressState.error, null);
      assert.equal(heldStressState.paused, true);
    }
  });
});
