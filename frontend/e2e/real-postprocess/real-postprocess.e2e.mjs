import assert from "node:assert/strict";

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
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "Delete", bubbles: true }),
        );
      });
      await browser.pause(80);
    }

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

    const sample = async (label) => {
      const value = await browser.execute((sampleLabel) => {
        const video = document.querySelector("video");
        const canvas = document.querySelector(
          'canvas[data-scrub-preview-visible="true"]',
        );
        const readPixel = (target) => {
          if (!(target instanceof HTMLCanvasElement) || target.width <= 0 || target.height <= 0) {
            return null;
          }
          const context = target.getContext("2d");
          if (!context) return null;
          const pixel = context.getImageData(
            Math.floor(target.width / 2),
            Math.floor(target.height / 2),
            1,
            1,
          ).data;
          return [...pixel];
        };
        return {
          label: sampleLabel,
          currentTime: video?.currentTime ?? null,
          duration: video?.duration ?? null,
          paused: video?.paused ?? null,
          seeking: video?.seeking ?? null,
          readyState: video?.readyState ?? null,
          networkState: video?.networkState ?? null,
          videoWidth: video?.videoWidth ?? null,
          videoHeight: video?.videoHeight ?? null,
          canvasPixel: readPixel(canvas),
          videoPixel: (() => {
            if (!video || video.videoWidth <= 0 || video.videoHeight <= 0) return null;
            const probe = document.createElement("canvas");
            probe.width = 1;
            probe.height = 1;
            const context = probe.getContext("2d");
            if (!context) return null;
            try {
              context.drawImage(video, 0, 0, 1, 1);
              return [...context.getImageData(0, 0, 1, 1).data];
            } catch {
              return null;
            }
          })(),
        };
      }, label);
      process.stdout.write(`A4N_REAL_SCRUB=${JSON.stringify(value)}\n`);
      return value;
    };

    const xAt = (ratio) => timelineRect.left + timelineRect.width * ratio;
    await browser.performActions([
      {
        type: "pointer",
        id: "real-timeline",
        parameters: { pointerType: "mouse" },
        actions: [
          { type: "pointerMove", x: Math.round(xAt(0.2)), y: Math.round(timelineRect.top + timelineRect.height / 2) },
          { type: "pointerDown", button: 0 },
          { type: "pointerUp", button: 0 },
        ],
      },
    ]);
    await browser.pause(100);
    const markerSample = await sample("after-marker-20");
    await browser.pause(1_000);
    const markerSettledSample = await sample("after-marker-20-1100ms");
    await browser.pause(2_000);
    const markerStableSample = await sample("after-marker-20-3100ms");
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
    await browser.performActions([
      {
        type: "pointer",
        id: "real-marker",
        parameters: { pointerType: "mouse" },
        actions: [
          { type: "pointerMove", x: Math.round(xAt(0.2)), y: Math.round(timelineRect.top + timelineRect.height / 2) },
          { type: "pointerDown", button: 0 },
          { type: "pointerMove", x: Math.round(xAt(0.55)), y: Math.round(timelineRect.top + timelineRect.height / 2), duration: 100 },
          { type: "pointerMove", x: Math.round(xAt(0.8)), y: Math.round(timelineRect.top + timelineRect.height / 2), duration: 100 },
          { type: "pointerUp", button: 0 },
        ],
      },
    ]);
    const dragSample = await sample("immediately-after-drag");
    await browser.pause(100);
    const dragSettledSample = await sample("after-drag-100ms");
    await browser.pause(500);
    await sample("after-drag-600ms");
    await browser.pause(1_000);
    await sample("after-drag-1600ms");
    assert.ok(
      dragSettledSample.currentTime != null &&
        Math.abs(dragSettledSample.currentTime - 0.8 * dragSettledSample.duration) < 0.25,
      "real drag did not settle near the requested final marker",
    );
    assert.equal(dragSettledSample.paused, true, "real drag left decode playback running");
    assert.notDeepEqual(
      dragSettledSample.canvasPixel,
      markerStableSample.canvasPixel,
      "real scrub canvas did not change between two positions",
    );
    assert.notEqual(dragSample.currentTime, markerStableSample.currentTime);

    await browser.performActions([
      {
        type: "pointer",
        id: "real-high-frequency-marker",
        parameters: { pointerType: "mouse" },
        actions: [
          { type: "pointerMove", x: Math.round(xAt(0.8)), y: Math.round(timelineRect.top + timelineRect.height / 2) },
          { type: "pointerDown", button: 0 },
        ],
      },
    ]);
    const highFrequencySamples = [];
    for (const ratio of [0.25, 0.4, 0.6, 0.85]) {
      await browser.performActions([
        {
          type: "pointer",
          id: "real-high-frequency-marker",
          parameters: { pointerType: "mouse" },
          actions: [
            { type: "pointerMove", x: Math.round(xAt(ratio)), y: Math.round(timelineRect.top + timelineRect.height / 2), duration: 40 },
          ],
        },
      ]);
      await browser.pause(120);
      highFrequencySamples.push(await sample(`high-frequency-${ratio}`));
    }
    await browser.performActions([
      {
        type: "pointer",
        id: "real-high-frequency-marker",
        parameters: { pointerType: "mouse" },
        actions: [{ type: "pointerUp", button: 0 }],
      },
    ]);
    assert.ok(
      highFrequencySamples.every(
        (item, index) =>
          item.currentTime != null &&
          item.duration != null &&
          Math.abs(item.currentTime - item.duration * [0.25, 0.4, 0.6, 0.85][index]) < 0.3 &&
          item.canvasPixel?.some((channel) => channel > 10),
      ),
      `real high-frequency drag did not follow decoded frames: ${JSON.stringify(highFrequencySamples)}`,
    );
  });
});
