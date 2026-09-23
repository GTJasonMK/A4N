import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const screenshotDir = fileURLToPath(
  new URL("../../../target/tmp/video-scrub/", import.meta.url),
);
mkdirSync(screenshotDir, { recursive: true });
const screenshotPath = (name) => `${screenshotDir}${name}`;

const openHarness = async (query = "") => {
  if (process.env.A4N_WDIO_APP_BINARY)
    await browser.url(`http://localhost:5173/e2e/video-scrub/${query}`);
  else await browser.url(`/e2e/video-scrub/${query}`);
  await browser.waitUntil(
    () =>
      browser
        .execute(() => {
          const video = document.querySelector("video");
          return {
            ready:
              video instanceof HTMLVideoElement &&
              video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA &&
              Boolean(document.querySelector('[aria-label="视频剪辑时间轴"]')),
            error: window.__A4N_VIDEO_SCRUB_E2E_ERROR__ ?? null,
          };
        })
        .then((state) => state.ready || state.error),
    { timeout: 30_000, timeoutMsg: "video scrub harness did not initialize" },
  );
  const error = await browser.execute(
    () => window.__A4N_VIDEO_SCRUB_E2E_ERROR__ ?? null,
  );
  assert.equal(error, null, error);
};

describe("video timeline scrubbing", () => {
  it("falls back to the source video when the scrub proxy fails", async () => {
    await openHarness("?scenario=source-fallback");
    const result = await browser.executeAsync((done) => {
      const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
      const video = document.querySelector("video");
      if (
        !(timeline instanceof HTMLElement) ||
        !(video instanceof HTMLVideoElement)
      ) {
        done({ error: "timeline or video missing" });
        return;
      }
      const rect = timeline.getBoundingClientRect();
      timeline.dispatchEvent(
        new MouseEvent("click", {
          bubbles: true,
          clientX: rect.left + rect.width * 0.4,
          clientY: rect.top + rect.height / 2,
        }),
      );
      void (async () => {
        for (let attempt = 0; attempt < 100; attempt += 1) {
          const expected = video.duration * 0.4;
          if (
            !video.seeking &&
            video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA &&
            Math.abs(video.currentTime - expected) < 0.05
          ) {
            const beforePlay = video.currentTime;
            video.muted = true;
            await video.play();
            await wait(150);
            video.pause();
            done({
              currentTime: video.currentTime,
              beforePlay,
              advanced: video.currentTime > beforePlay,
              sourceFallback: document.querySelector(
                'img[data-scrub-preview-visible="true"]',
              ) == null,
            });
            return;
          }
          await wait(20);
        }
        throw new Error(
          `source fallback seek did not settle: ${video.currentTime} vs ${video.duration * 0.4}`,
        );
      })().catch((error) => done({ error: String(error) }));
    });

    assert.equal(result.error, undefined, result.error);
    assert.ok(result.advanced, `source fallback did not play: ${JSON.stringify(result)}`);
    assert.equal(result.sourceFallback, true);
  });

  it("keeps the active source isolated when switching push batches", async () => {
    await openHarness("?scenario=multiple-pushes");
    const readState = () =>
      browser.execute(() => {
        const video = document.querySelector("video");
        return {
          videoId: video?.getAttribute("data-video-id") ?? null,
          readyState: video?.readyState ?? null,
          error: video?.error?.message ?? null,
          alert: document.querySelector('[role="alert"]')?.textContent ?? null,
        };
      });
    const waitForVideo = async (videoId, timeoutMsg) => {
      try {
        await browser.waitUntil(
          async () => {
            const state = await readState();
            return (
              state.videoId === videoId && state.readyState >= 2 && !state.alert
            );
          },
          { timeout: 10_000, timeoutMsg },
        );
      } catch (error) {
        throw new Error(
          `${timeoutMsg}: ${JSON.stringify(await readState())}; ${error}`,
        );
      }
    };
    await waitForVideo("9002", "latest push did not initialize");
    await $('[data-video-push-id="9001"]').click();
    await waitForVideo("9001", "first push did not initialize");
    await $('[data-video-push-id="9002"]').click();
    await waitForVideo("9002", "latest push did not recover");
  });

  it("uses one video element and follows every requested position", async () => {
    await openHarness();
    const result = await browser.executeAsync((done) => {
      const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
      const video = document.querySelector("video");
      if (
        !(timeline instanceof HTMLElement) ||
        !(video instanceof HTMLVideoElement)
      ) {
        done({ error: "timeline or video missing" });
        return;
      }
      const rect = timeline.getBoundingClientRect();
      const point = (ratio) => rect.left + rect.width * ratio;
      const dispatch = (type, ratio, pointerId = 1, buttons = 1) => {
        const target =
          document.querySelector('[aria-label="剪辑竖线 1"]') ?? timeline;
        target.dispatchEvent(
          new PointerEvent(type, {
            bubbles: true,
            pointerId,
            buttons,
            clientX: point(ratio),
            clientY: rect.top + rect.height / 2,
          }),
        );
      };
      const settle = async (ratio) => {
        const expected = video.duration * ratio;
        for (let attempt = 0; attempt < 100; attempt += 1) {
          if (
            !video.seeking &&
            video.dataset.scrubSeekPending !== "true" &&
            Math.abs(video.currentTime - expected) < 0.05 &&
            video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA
          ) {
            return;
          }
          await wait(20);
        }
        throw new Error(
          `seek did not settle: ${video.currentTime} vs ${expected}; readyState=${video.readyState}; seeking=${video.seeking}; networkState=${video.networkState}; duration=${video.duration}; error=${video.error?.code ?? "none"}`,
        );
      };
      const waitForPreview = async (ratio) => {
        const expectedTime = video.duration * ratio;
        const expectedFrame = Math.round(expectedTime * 12);
        for (let attempt = 0; attempt < 100; attempt += 1) {
          const image = document.querySelector(
            'img[data-scrub-preview-visible="true"]',
          );
          const markerValue = Number(
            document
              .querySelector('[aria-label="剪辑竖线 1"]')
              ?.getAttribute("aria-valuenow"),
          );
          const frameIndex = Number(
            image?.getAttribute("data-scrub-frame-index"),
          );
          if (
            image instanceof HTMLImageElement &&
            image.complete &&
            image.naturalWidth > 0 &&
            Math.abs(frameIndex - expectedFrame) <= 1 &&
            Math.abs(markerValue - expectedTime) < 0.05
          ) {
            return { frameIndex, markerValue, nativeTime: video.currentTime };
          }
          await wait(10);
        }
        throw new Error(`scrub preview did not reach ${expectedFrame}`);
      };

      void (async () => {
        timeline.dispatchEvent(
          new MouseEvent("click", {
            bubbles: true,
            clientX: point(0.12),
            clientY: rect.top + rect.height / 2,
          }),
        );
        for (let attempt = 0; attempt < 50; attempt += 1) {
          if (document.querySelector('[aria-label="剪辑竖线 1"]')) break;
          await wait(20);
        }
        const marker = document.querySelector('[aria-label="剪辑竖线 1"]');
        if (!(marker instanceof HTMLElement)) throw new Error("marker missing");
        await settle(0.12);
        let seekedDuringDrag = 0;
        const countSeeked = () => {
          seekedDuringDrag += 1;
        };
        video.addEventListener("seeked", countSeeked);
        dispatch("pointerdown", 0.12);
        await wait(20);
        const ratios = [0.28, 0.52, 0.81, 0.44, 0.17, 0.69];
        const samples = [];
        for (const ratio of ratios) {
          dispatch("pointermove", ratio);
          samples.push({ ratio, ...(await waitForPreview(ratio)) });
        }
        const seekedBeforeRelease = seekedDuringDrag;
        dispatch("pointerup", ratios.at(-1), 1, 0);
        await settle(ratios.at(-1));
        video.removeEventListener("seeked", countSeeked);
        const finalTime = video.currentTime;
        const markerValue = Number(marker.getAttribute("aria-valuenow"));
        done({
          samples,
          seekedBeforeRelease,
          finalTime,
          expectedFinalTime: video.duration * ratios.at(-1),
          markerValue,
          videoCount: document.querySelectorAll("video").length,
          canvasCount: document.querySelectorAll("canvas").length,
          paused: video.paused,
        });
      })().catch((error) => done({ error: String(error) }));
    });

    process.stdout.write(`A4N_VIDEO_SCRUB_RESULT=${JSON.stringify(result)}\n`);
    assert.equal(result.error, undefined, result.error);
    assert.equal(result.videoCount, 1);
    assert.equal(result.canvasCount, 0);
    assert.ok(result.samples.length >= 5);
    assert.equal(
      result.seekedBeforeRelease,
      0,
      `native video was still seeking during drag: ${JSON.stringify(result)}`,
    );
    assert.ok(
      result.samples.every(
        (sample) =>
          Number.isInteger(sample.frameIndex) &&
          Number.isFinite(sample.markerValue),
      ),
      `JPEG scrub preview did not follow markers: ${JSON.stringify(result)}`,
    );
    assert.ok(
      new Set(result.samples.map((sample) => sample.frameIndex)).size >= 5,
      `JPEG scrub preview remained frozen: ${JSON.stringify(result)}`,
    );
    assert.ok(
      Math.abs(result.finalTime - result.expectedFinalTime) < 0.05,
      `video did not settle on final marker: ${JSON.stringify(result)}`,
    );
    assert.ok(
      Math.abs(result.markerValue - result.expectedFinalTime) < 0.05,
      `marker and video diverged: ${JSON.stringify(result)}`,
    );
    assert.equal(result.paused, true);
  });

  it("converges after a high-frequency reverse drag", async () => {
    await openHarness();
    const result = await browser.executeAsync((done) => {
      const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
      const video = document.querySelector("video");
      if (
        !(timeline instanceof HTMLElement) ||
        !(video instanceof HTMLVideoElement)
      ) {
        done({ error: "timeline or video missing" });
        return;
      }
      const rect = timeline.getBoundingClientRect();
      const point = (ratio) => rect.left + rect.width * ratio;
      const fire = (type, ratio, buttons) => {
        const marker = document.querySelector('[aria-label="剪辑竖线 1"]');
        (marker ?? timeline).dispatchEvent(
          new PointerEvent(type, {
            bubbles: true,
            pointerId: 7,
            buttons,
            clientX: point(ratio),
            clientY: rect.top + rect.height / 2,
          }),
        );
      };
      void (async () => {
        timeline.dispatchEvent(
          new MouseEvent("click", {
            bubbles: true,
            clientX: point(0.9),
            clientY: rect.top + rect.height / 2,
          }),
        );
        for (let attempt = 0; attempt < 50; attempt += 1) {
          if (document.querySelector('[aria-label="剪辑竖线 1"]')) break;
          await wait(20);
        }
        if (!document.querySelector('[aria-label="剪辑竖线 1"]'))
          throw new Error("marker missing");
        const initialExpected = video.duration * 0.9;
        for (let attempt = 0; attempt < 100; attempt += 1) {
          if (
            !video.seeking &&
            video.dataset.scrubSeekPending !== "true" &&
            Math.abs(video.currentTime - initialExpected) < 0.05
          ) {
            break;
          }
          await wait(20);
        }
        let seekedDuringDrag = 0;
        const countSeeked = () => {
          seekedDuringDrag += 1;
        };
        video.addEventListener("seeked", countSeeked);
        fire("pointerdown", 0.9, 1);
        await wait(10);
        const ratios = Array.from(
          { length: 70 },
          (_, index) => 0.9 - (0.84 * index) / 69,
        );
        const observedFrames = [];
        for (const ratio of ratios) {
          fire("pointermove", ratio, 1);
          await wait(5);
          observedFrames.push(
            Number(
              document
                .querySelector('img[data-scrub-preview-visible="true"]')
                ?.getAttribute("data-scrub-frame-index"),
            ),
          );
        }
        const seekedBeforeRelease = seekedDuringDrag;
        fire("pointerup", ratios.at(-1), 0);
        const expected = video.duration * ratios.at(-1);
        for (let attempt = 0; attempt < 100; attempt += 1) {
          if (
            !video.seeking &&
            video.dataset.scrubSeekPending !== "true" &&
            Math.abs(video.currentTime - expected) < 0.05
          )
            break;
          await wait(20);
        }
        video.removeEventListener("seeked", countSeeked);
        done({
          currentTime: video.currentTime,
          expected,
          seeking: video.seeking,
          observedFrameCount: new Set(observedFrames.filter(Number.isFinite))
            .size,
          seekedBeforeRelease,
          videoCount: document.querySelectorAll("video").length,
          canvasCount: document.querySelectorAll("canvas").length,
        });
      })().catch((error) => done({ error: String(error) }));
    });

    process.stdout.write(
      `A4N_VIDEO_SCRUB_CONTINUOUS_RESULT=${JSON.stringify(result)}\n`,
    );
    assert.equal(result.error, undefined, result.error);
    assert.equal(result.videoCount, 1);
    assert.equal(result.canvasCount, 0);
    assert.equal(result.seeking, false);
    assert.equal(
      result.seekedBeforeRelease,
      0,
      `high-frequency drag still sought native video: ${JSON.stringify(result)}`,
    );
    assert.ok(
      result.observedFrameCount >= 6,
      `drag preview did not update continuously: ${JSON.stringify(result)}`,
    );
    assert.ok(
      Math.abs(result.currentTime - result.expected) < 0.05,
      `reverse drag did not settle: ${JSON.stringify(result)}`,
    );
  });

  it("follows a native pointer-captured drag", async () => {
    await openHarness();
    await browser.execute(() => {
      document
        .querySelector('[aria-label="视频剪辑时间轴"]')
        ?.scrollIntoView({ block: "center" });
    });
    const timelineRect = await browser.execute(() => {
      const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
      const rect = timeline?.getBoundingClientRect();
      return rect
        ? {
            left: rect.left,
            top: rect.top,
            width: rect.width,
            height: rect.height,
          }
        : null;
    });
    assert.ok(timelineRect && timelineRect.width > 0);
    const xAt = (ratio) => timelineRect.left + timelineRect.width * ratio;
    const y = timelineRect.top + timelineRect.height / 2;
    await browser.performActions([
      {
        type: "pointer",
        id: "native-marker-create",
        parameters: { pointerType: "mouse" },
        actions: [
          { type: "pointerMove", x: Math.round(xAt(0.2)), y: Math.round(y) },
          { type: "pointerDown", button: 0 },
          { type: "pointerUp", button: 0 },
        ],
      },
    ]);
    const marker = await browser.$('[aria-label="剪辑竖线 1"]');
    await marker.waitForDisplayed({ timeout: 10_000 });
    const nativeVideo = await browser.$("video");
    const screenshotHash = async (path) => {
      await nativeVideo.saveScreenshot(path);
      return createHash("sha256").update(readFileSync(path)).digest("hex");
    };
    const waitForRatio = (ratio, timeoutMsg) =>
      browser.waitUntil(
        () =>
          browser
            .execute((targetRatio) => {
              const video = document.querySelector("video");
              const marker = Number(
                document
                  .querySelector('[aria-label="剪辑竖线 1"]')
                  ?.getAttribute("aria-valuenow"),
              );
              const expected = (video?.duration ?? 0) * targetRatio;
              return Boolean(
                video &&
                !video.seeking &&
                video.dataset.scrubSeekPending !== "true" &&
                Math.abs(video.currentTime - expected) < 0.2 &&
                Math.abs(marker - expected) < 0.2,
              );
            }, ratio)
            .then(Boolean),
        { timeout: 10_000, timeoutMsg },
      );
    const waitForPreviewRatio = (ratio, timeoutMsg) =>
      browser.waitUntil(
        () =>
          browser
            .execute((targetRatio) => {
              const video = document.querySelector("video");
              const image = document.querySelector(
                'img[data-scrub-preview-visible="true"]',
              );
              const marker = Number(
                document
                  .querySelector('[aria-label="剪辑竖线 1"]')
                  ?.getAttribute("aria-valuenow"),
              );
              const expectedTime = (video?.duration ?? 0) * targetRatio;
              const expectedFrame = Math.round(expectedTime * 12);
              const frameIndex = Number(
                image?.getAttribute("data-scrub-frame-index"),
              );
              return Boolean(
                image instanceof HTMLImageElement &&
                image.complete &&
                image.naturalWidth > 0 &&
                Math.abs(frameIndex - expectedFrame) <= 1 &&
                Math.abs(marker - expectedTime) < 0.2,
              );
            }, ratio)
            .then(Boolean),
        { timeout: 10_000, timeoutMsg },
      );
    const isNativeWebKit = await browser.execute(
      () =>
        /AppleWebKit/i.test(navigator.userAgent) &&
        !/Chrome|Chromium/i.test(navigator.userAgent),
    );
    let forwardFrameHash;
    if (isNativeWebKit) {
      await browser.execute(() => {
        const video = document.querySelector("video");
        if (video) video.controls = false;
      });
      await waitForRatio(0.2, "initial marker did not settle");
      await browser.execute(() => {
        const video = document.querySelector("video");
        window.__A4N_NATIVE_DRAG_SEEKED__ = 0;
        video?.addEventListener("seeked", () => {
          window.__A4N_NATIVE_DRAG_SEEKED__ += 1;
        });
      });
      await browser.performActions([
        {
          type: "pointer",
          id: "native-marker-drag",
          parameters: { pointerType: "mouse" },
          actions: [
            { type: "pointerMove", x: Math.round(xAt(0.2)), y: Math.round(y) },
            { type: "pointerDown", button: 0 },
          ],
        },
      ]);
      const dragSamples = [];
      const dragRatios = [0.38, 0.76, 0.18, 0.76, 0.18, 0.9, 0.12, 0.82];
      for (const [index, ratio] of dragRatios.entries()) {
        await browser.performActions([
          {
            type: "pointer",
            id: "native-marker-drag",
            parameters: { pointerType: "mouse" },
            actions: [
              {
                type: "pointerMove",
                x: Math.round(xAt(ratio)),
                y: Math.round(y),
                duration: 150,
              },
            ],
          },
        ]);
        try {
          await waitForPreviewRatio(ratio, "held JPEG preview did not settle");
        } catch (error) {
          const state = await browser.execute(() => {
            const video = document.querySelector("video");
            return {
              currentTime: video?.currentTime ?? null,
              duration: video?.duration ?? null,
              paused: video?.paused ?? null,
              seeking: video?.seeking ?? null,
              pending: video?.dataset.scrubSeekPending ?? null,
              marker: document
                .querySelector('[aria-label="剪辑竖线 1"]')
                ?.getAttribute("aria-valuenow"),
            };
          });
          throw new Error(`${error}; state=${JSON.stringify(state)}`);
        }
        dragSamples.push({
          ratio,
          hash: await screenshotHash(
            screenshotPath(`a4n-webkit-drag-${index}.png`),
          ),
          state: await browser.execute(() => {
            const video = document.querySelector("video");
            return {
              currentTime: video?.currentTime ?? null,
              paused: video?.paused ?? null,
              seeking: video?.seeking ?? null,
              readyState: video?.readyState ?? null,
              error: video?.error?.message ?? null,
              frameIndex: document
                .querySelector('img[data-scrub-preview-visible="true"]')
                ?.getAttribute("data-scrub-frame-index"),
              marker: document
                .querySelector('[aria-label="剪辑竖线 1"]')
                ?.getAttribute("aria-valuenow"),
            };
          }),
        });
      }
      const rapidVisualSamples = [];
      for (const [index, ratio] of [
        0.14, 0.86, 0.22, 0.78, 0.3, 0.7,
      ].entries()) {
        await browser.performActions([
          {
            type: "pointer",
            id: "native-marker-drag",
            parameters: { pointerType: "mouse" },
            actions: [
              {
                type: "pointerMove",
                x: Math.round(xAt(ratio)),
                y: Math.round(y),
                duration: 5,
              },
            ],
          },
        ]);
        await waitForPreviewRatio(ratio, "rapid JPEG preview did not settle");
        const rapidScreenshotPath = screenshotPath(
          `a4n-webkit-rapid-${index}.png`,
        );
        rapidVisualSamples.push({
          ratio,
          hash: await screenshotHash(rapidScreenshotPath),
          bytes: readFileSync(rapidScreenshotPath).length,
          ...(await browser.execute(() => ({
            currentTime: document.querySelector("video")?.currentTime ?? null,
            frameIndex: document
              .querySelector('img[data-scrub-preview-visible="true"]')
              ?.getAttribute("data-scrub-frame-index"),
          }))),
        });
      }
      process.stdout.write(
        `A4N_VIDEO_SCRUB_WEBKIT_RAPID=${JSON.stringify(rapidVisualSamples)}\n`,
      );
      const stressRatios = Array.from({ length: 8 }, () => [
        0.12, 0.22, 0.32, 0.42, 0.52, 0.62, 0.72, 0.82, 0.92, 0.82, 0.72, 0.62,
        0.52, 0.42, 0.32, 0.22, 0.18,
      ]).flat();
      await browser.performActions([
        {
          type: "pointer",
          id: "native-marker-drag",
          parameters: { pointerType: "mouse" },
          actions: stressRatios.map((ratio) => ({
            type: "pointerMove",
            x: Math.round(xAt(ratio)),
            y: Math.round(y),
            duration: 25,
          })),
        },
      ]);
      await browser.performActions([
        {
          type: "pointer",
          id: "native-marker-drag",
          parameters: { pointerType: "mouse" },
          actions: [
            {
              type: "pointerMove",
              x: Math.round(xAt(0.18)),
              y: Math.round(y),
              duration: 150,
            },
          ],
        },
      ]);
      try {
        await waitForPreviewRatio(0.18, "rapid held JPEG drag did not settle");
      } catch (error) {
        let state;
        try {
          state = await browser.execute(() => {
            const video = document.querySelector("video");
            return {
              currentTime: video?.currentTime ?? null,
              duration: video?.duration ?? null,
              paused: video?.paused ?? null,
              seeking: video?.seeking ?? null,
              readyState: video?.readyState ?? null,
              networkState: video?.networkState ?? null,
              error: video?.error?.message ?? null,
              frameIndex: document
                .querySelector('img[data-scrub-preview-visible="true"]')
                ?.getAttribute("data-scrub-frame-index"),
              marker: document
                .querySelector('[aria-label="剪辑竖线 1"]')
                ?.getAttribute("aria-valuenow"),
            };
          });
        } catch (stateError) {
          state = { webdriverError: String(stateError) };
        }
        throw new Error(`${error}; state=${JSON.stringify(state)}`);
      }
      dragSamples.push({
        ratio: 0.18,
        hash: await screenshotHash(
          screenshotPath("a4n-webkit-drag-stress.png"),
        ),
        state: await browser.execute(() => {
          const video = document.querySelector("video");
          return {
            currentTime: video?.currentTime ?? null,
            paused: video?.paused ?? null,
            seeking: video?.seeking ?? null,
            readyState: video?.readyState ?? null,
            error: video?.error?.message ?? null,
            marker: document
              .querySelector('[aria-label="剪辑竖线 1"]')
              ?.getAttribute("aria-valuenow"),
          };
        }),
      });
      await browser.performActions([
        {
          type: "pointer",
          id: "native-marker-drag",
          parameters: { pointerType: "mouse" },
          actions: [
            {
              type: "pointerMove",
              x: Math.round(xAt(0.76)),
              y: Math.round(y),
              duration: 150,
            },
          ],
        },
      ]);
      await waitForPreviewRatio(0.76, "final held JPEG drag did not settle");
      forwardFrameHash = await screenshotHash(
        screenshotPath("a4n-webkit-drag-forward.png"),
      );
      const heldSeekedCount = await browser.execute(
        () => window.__A4N_NATIVE_DRAG_SEEKED__,
      );
      await browser.performActions([
        {
          type: "pointer",
          id: "native-marker-drag",
          parameters: { pointerType: "mouse" },
          actions: [{ type: "pointerUp", button: 0 }],
        },
      ]);
      await waitForRatio(0.76, "released native drag did not settle");
      process.stdout.write(
        `A4N_VIDEO_SCRUB_WEBKIT_DRAG=${JSON.stringify(dragSamples)}\n`,
      );
      assert.equal(heldSeekedCount, 0, "按住竖线拖动时仍在连续 seek 原生视频");
      assert.ok(
        new Set(rapidVisualSamples.map((sample) => sample.hash)).size >= 5,
        `快速拖动时 JPEG 画面卡在同一帧：${JSON.stringify(rapidVisualSamples)}`,
      );
      assert.ok(
        rapidVisualSamples.every((sample) => sample.bytes > 20_000),
        `快速拖动时出现黑帧：${JSON.stringify(rapidVisualSamples)}`,
      );
      for (const ratio of [0.76, 0.18]) {
        const matching = dragSamples.filter((sample) => sample.ratio === ratio);
        assert.equal(
          new Set(matching.map((sample) => sample.hash)).size,
          1,
          `往返拖回同一位置时画面不一致：${JSON.stringify(matching)}`,
        );
      }
      assert.notEqual(
        dragSamples[1].hash,
        dragSamples[2].hash,
        "按住竖线反向拖动时视频画面没有回退",
      );
      assert.equal(
        forwardFrameHash,
        dragSamples.find((sample) => sample.ratio === 0.76)?.hash,
        "压力拖动后画面没有恢复到最终竖线位置",
      );
    } else {
      await browser.performActions([
        {
          type: "pointer",
          id: "native-marker-drag",
          parameters: { pointerType: "mouse" },
          actions: [
            { type: "pointerMove", x: Math.round(xAt(0.2)), y: Math.round(y) },
            { type: "pointerDown", button: 0 },
            {
              type: "pointerMove",
              x: Math.round(xAt(0.76)),
              y: Math.round(y),
              duration: 300,
            },
            { type: "pointerUp", button: 0 },
          ],
        },
      ]);
      await waitForRatio(0.76, "native pointer drag did not settle");
      forwardFrameHash = await screenshotHash(
        screenshotPath("a4n-chromium-forward.png"),
      );
    }
    await browser.performActions([
      {
        type: "pointer",
        id: "native-marker-reverse-drag",
        parameters: { pointerType: "mouse" },
        actions: [
          { type: "pointerMove", x: Math.round(xAt(0.76)), y: Math.round(y) },
          { type: "pointerDown", button: 0 },
          {
            type: "pointerMove",
            x: Math.round(xAt(0.2)),
            y: Math.round(y),
            duration: 300,
          },
          { type: "pointerUp", button: 0 },
        ],
      },
    ]);
    await browser.waitUntil(
      () =>
        browser
          .execute(() => {
            const video = document.querySelector("video");
            return video
              ? !video.seeking &&
                  video.dataset.scrubSeekPending !== "true" &&
                  Math.abs(video.currentTime - video.duration * 0.2) < 0.2
              : false;
          })
          .then(Boolean),
      { timeout: 10_000, timeoutMsg: "native reverse drag did not settle" },
    );
    const reverseFrameHash = await screenshotHash(
      screenshotPath("a4n-webkit-reverse.png"),
    );
    assert.notEqual(
      reverseFrameHash,
      forwardFrameHash,
      "反向拖动后视频画面没有变化",
    );
    await browser.execute(() => {
      document.querySelector("video")?.pause();
    });
    await browser.execute(() => void document.querySelector("video")?.play());
    const startPlaybackTime = await browser.execute(
      () => document.querySelector("video")?.currentTime ?? null,
    );
    await browser.pause(250);
    const playbackState = await browser.execute(() => ({
      currentTime: document.querySelector("video")?.currentTime ?? null,
      paused: document.querySelector("video")?.paused ?? true,
      canvasCount: document.querySelectorAll("canvas").length,
      playbackHeadVisible: Boolean(
        document.querySelector('[data-playback-head="true"]'),
      ),
    }));
    assert.equal(playbackState.paused, false);
    assert.equal(playbackState.canvasCount, 0);
    assert.equal(playbackState.playbackHeadVisible, true);
    assert.ok(
      startPlaybackTime != null &&
        playbackState.currentTime != null &&
        playbackState.currentTime > startPlaybackTime,
      `playback cursor did not advance: ${JSON.stringify(playbackState)}`,
    );
    await browser.execute(() => document.querySelector("video")?.pause());
  });
});
