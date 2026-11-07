import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const screenshotDir = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR ?? fileURLToPath(
  new URL("../../../target/tmp/video-scrub/", import.meta.url),
);
mkdirSync(screenshotDir, { recursive: true });
const screenshotPath = (name) => `${screenshotDir}/${name}`;

const openHarness = async (query = "") => {
  await browser.url(`/e2e/video-scrub/${query}`);
  let lastState = null;
  try {
    await browser.waitUntil(async () => {
      lastState = await browser.execute(() => {
        const video = document.querySelector("video");
        return {
          ready:
            video instanceof HTMLVideoElement &&
            video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA &&
            Boolean(document.querySelector('[aria-label="视频剪辑时间轴"]')),
          error: window.__A4N_VIDEO_SCRUB_E2E_ERROR__ ?? document.querySelector('[role="alert"]')?.textContent ?? null,
          videoExists: video instanceof HTMLVideoElement,
          readyState: video?.readyState ?? null,
          networkState: video?.networkState ?? null,
          mediaError: video?.error?.message || video?.error?.code || null,
          sourceKind: video?.currentSrc.startsWith("data:") ? "data-url" : video?.currentSrc ? "external" : null,
          timelineExists: Boolean(document.querySelector('[aria-label="视频剪辑时间轴"]')),
          text: document.body.innerText.slice(0, 500),
        };
      });
      return lastState.ready || lastState.error;
    }, { timeout: 30_000 });
  } catch (cause) {
    throw new Error(
      `video scrub harness did not initialize: ${JSON.stringify(lastState)}`,
      { cause },
    );
  }
  const error = await browser.execute(
    () => window.__A4N_VIDEO_SCRUB_E2E_ERROR__ ?? document.querySelector('[role="alert"]')?.textContent ?? null,
  );
  assert.equal(error, null, error);
};

describe("video timeline scrubbing", () => {
  afterEach(async () => {
    await browser.execute(() => {
      for (const video of document.querySelectorAll("video")) {
        video.pause();
        video.removeAttribute("src");
        video.load();
      }
    });
  });

  it("reloads an unchanged assembly preview when updating from cache", async function () {
    if (process.env.A4N_WDIO_APP_BINARY) this.skip();
    await browser.url("/e2e/video-scrub/?scenario=assembly");
    await browser.waitUntil(() => browser.execute(() => document.querySelector("video")?.readyState >= 2));
    await browser.execute(() => {
      window.__assemblyReloads = 0;
      document.querySelector("video").addEventListener("loadstart", () => { window.__assemblyReloads += 1; });
    });
    for (let count = 1; count <= 2; count += 1) {
      await browser.execute(() => { document.querySelector("video").currentTime = 2; });
      await browser.waitUntil(() => browser.execute(() => !document.querySelector("video").seeking));
      await $("button=更新预览").click();
      let lastState = null;
      try {
        await browser.waitUntil(async () => {
          lastState = await browser.execute((expected) => {
            const video = document.querySelector("video");
            return {
              reloads: window.__assemblyReloads,
              expected,
              readyState: video?.readyState ?? null,
              currentTime: video?.currentTime ?? null,
              loadingOverlay: document.body.textContent.includes("正在加载预览"),
            };
          }, count);
          return lastState.reloads === count && lastState.readyState >= 2 && lastState.currentTime < 0.1 && !lastState.loadingOverlay;
        }, { timeout: 3000 });
      } catch (cause) {
        throw new Error(
          `identical cached media did not reload or left its loading overlay active: ${JSON.stringify(lastState)}`,
          { cause },
        );
      }
    }
  });

  it("seeks with the desktop native progress control, including after ending", async function () {
    await browser.setWindowSize(1440, 1000);
    await openHarness();
    const webkit = await browser.execute(() =>
      /AppleWebKit/i.test(navigator.userAgent) && !/Chrome|Chromium/i.test(navigator.userAgent),
    );
    if (!webkit) this.skip();
    await browser.execute(async () => {
      const video = document.querySelector("video");
      video.muted = true;
      await video.play();
    });
    await browser.waitUntil(() => browser.execute(() => document.querySelector("video").currentTime > 0.2));
    const rect = await browser.execute(() => {
      const video = document.querySelector("video");
      video.pause();
      video.scrollIntoView({ block: "center" });
      const bounds = video.getBoundingClientRect();
      return { left: bounds.left, bottom: bounds.bottom, width: bounds.width };
    });
    // WebKitGTK's native track lies between the transport/time and volume controls.
    const xAt = (ratio) => Math.round(rect.left + 152 + (rect.width - 270) * ratio);
    const y = Math.round(rect.bottom - 22);
    const pointer = (actions) => browser.performActions([
      { type: "pointer", id: "native-progress", parameters: { pointerType: "mouse" }, actions },
    ]);
    const samples = [];
    for (let cycle = 0; cycle < 3; cycle += 1) {
      if (cycle > 0) {
        await browser.execute(async () => {
          const video = document.querySelector("video");
          video.currentTime = video.duration - 0.15;
          await video.play();
        });
        await browser.waitUntil(() => browser.execute(() => document.querySelector("video").ended));
      }
      await pointer([
        { type: "pointerMove", x: xAt(0.1), y },
        { type: "pause", duration: 150 },
        { type: "pointerDown", button: 0 },
      ]);
      try {
        for (const ratio of [0.25, 0.75, 0.45]) {
          await pointer([{ type: "pointerMove", x: xAt(ratio), y, duration: 120 }]);
          let state;
          try {
            await browser.waitUntil(async () => {
              state = await browser.execute(() => {
                const video = document.querySelector("video");
                return {
                  time: video.currentTime,
                  duration: video.duration,
                  seeking: video.seeking,
                  seekable: Array.from({ length: video.seekable.length }, (_, index) => [video.seekable.start(index), video.seekable.end(index)]),
                  error: video.error?.message ?? null,
                };
              });
              return !state.seeking && Math.abs(state.time - state.duration * ratio) < 0.15;
            }, { timeout: 5000 });
          } catch (error) {
            throw new Error(`native progress stuck at cycle ${cycle}, ratio ${ratio}: ${JSON.stringify(state)}`, { cause: error });
          }
          const frame = await browser.execute(() => {
            const video = document.querySelector("video");
            const canvas = document.createElement("canvas");
            canvas.width = 32;
            canvas.height = 18;
            const context = canvas.getContext("2d");
            context.imageSmoothingEnabled = false;
            context.drawImage(video, 0, 0, 32, 18);
            return {
              time: video.currentTime,
              pixels: Array.from(context.getImageData(0, 0, 32, 18).data),
              overlay: Boolean(document.querySelector('img[data-scrub-preview-visible="true"]')),
              error: video.error?.message ?? null,
            };
          });
          const expected = spawnSync("ffmpeg", [
            "-hide_banner", "-loglevel", "error", "-ss", String(Math.floor(frame.time * 24) / 24),
            "-i", fileURLToPath(new URL("./playback-fixture.mp4", import.meta.url)),
            "-frames:v", "1", "-vf", "format=rgba,scale=32:18:flags=neighbor", "-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1",
          ], { timeout: 5000 });
          assert.equal(expected.status, 0, expected.stderr?.toString());
          assert.equal(expected.stdout.length, frame.pixels.length);
          const pixelError = frame.pixels.reduce((sum, value, index) =>
            sum + Math.abs(value - expected.stdout[index]), 0) / frame.pixels.length;
          samples.push({ cycle, ratio, time: frame.time, pixelError });
          assert.equal(frame.error, null);
          assert.equal(frame.overlay, false);
          assert.ok(pixelError < 8, `native frame did not follow time: ${JSON.stringify(samples)}`);
        }
      } finally {
        await pointer([{ type: "pointerUp", button: 0 }]);
      }
    }
    process.stdout.write(`A4N_NATIVE_PROGRESS_RESULT=${JSON.stringify(samples)}\n`);
  });

  it("cancels a captured marker drag when the marker is deleted or playback starts", async () => {
    for (const action of ["delete", "play"]) {
      await openHarness();
      const rect = await browser.execute(() => {
        const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
        timeline.scrollIntoView({ block: "center" });
        const bounds = timeline.getBoundingClientRect();
        return { left: bounds.left, top: bounds.top, width: bounds.width, height: bounds.height };
      });
      const xAt = (ratio) => Math.round(rect.left + rect.width * ratio);
      const y = Math.round(rect.top + rect.height / 2);
      const pointer = (actions) => browser.performActions([
        { type: "pointer", id: "cancel-marker", parameters: { pointerType: "mouse" }, actions },
      ]);
      await pointer([
        { type: "pointerMove", x: xAt(0.2), y },
        { type: "pointerDown", button: 0 },
        { type: "pointerUp", button: 0 },
      ]);
      await $('[aria-label="剪辑竖线 1"]').waitForDisplayed();
      await browser.waitUntil(() => browser.execute(() => {
        const video = document.querySelector("video");
        return !video.seeking && !document.querySelector('img[data-scrub-preview-visible="true"]');
      }));
      await pointer([
        { type: "pointerMove", x: xAt(0.2), y },
        { type: "pointerDown", button: 0 },
        { type: "pointerMove", x: xAt(0.6), y, duration: 120 },
      ]);
      await browser.waitUntil(() => browser.execute(() =>
        Boolean(document.querySelector('img[data-scrub-preview-visible="true"]')),
      ));
      try {
        if (action === "delete") {
          await browser.keys("Delete");
          assert.equal(await $('[aria-label="剪辑竖线 1"]').isExisting(), false);
          await browser.execute(() => {
            const video = document.querySelector("video");
            video.currentTime = video.duration * 0.3;
          });
          await browser.waitUntil(() => browser.execute(() =>
            !document.querySelector("video").seeking && !document.querySelector('img[data-scrub-preview-visible="true"]'),
          ), { timeout: 3000, timeoutMsg: "deleted marker kept the stale preview over native playback" });
        } else {
          await browser.execute(async () => {
            const video = document.querySelector("video");
            video.muted = true;
            await video.play();
          });
          await pointer([{ type: "pointerMove", x: xAt(0.8), y, duration: 120 }]);
          const state = await browser.execute(() => ({
            preview: Boolean(document.querySelector('img[data-scrub-preview-visible="true"]')),
            marker: Number(document.querySelector('[aria-label="剪辑竖线 1"]').getAttribute("aria-valuenow")),
            duration: document.querySelector("video").duration,
          }));
          assert.equal(state.preview, false, "cancelled pointer capture reactivated the preview");
          assert.ok(Math.abs(state.marker - state.duration * 0.6) < 0.1);
        }
      } finally {
        await pointer([{ type: "pointerUp", button: 0 }]);
      }
    }
  });

  it("keeps source audio in normal playback", async () => {
    await openHarness();
    await browser.execute(async () => {
      const video = document.querySelector("video");
      video.muted = true;
      await video.play();
    });
    await browser.waitUntil(
      () => browser.execute(() => {
        const video = document.querySelector("video");
        return video.webkitAudioDecodedByteCount > 0 || video.audioTracks?.length > 0;
      }),
      { timeout: 3_000, timeoutMsg: "normal playback lost the source audio track" },
    );
  });

  it("honors play while a timeline seek is settling", async () => {
    await openHarness();
    const result = await browser.executeAsync((done) => {
      const video = document.querySelector("video");
      const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
      const rect = timeline.getBoundingClientRect();
      video.muted = true;
      timeline.dispatchEvent(new MouseEvent("click", {
        bubbles: true,
        clientX: rect.left + rect.width * 0.2,
        clientY: rect.top + rect.height / 2,
      }));
      video.play().then(() => {
        setTimeout(() => done({
          paused: video.paused,
          currentTime: video.currentTime,
          expected: video.duration * 0.2,
          previewVisible: Boolean(document.querySelector('img[data-scrub-preview-visible="true"]')),
        }), 300);
      }).catch((error) => done({ playError: String(error) }));
    });
    assert.equal(result.playError, undefined, result.playError);
    assert.equal(result.paused, false, JSON.stringify(result));
    assert.ok(result.currentTime > result.expected, JSON.stringify(result));
    assert.equal(result.previewVisible, false, JSON.stringify(result));
  });

  it("honors a native seek that supersedes a timeline seek", async () => {
    await openHarness();
    const expected = await browser.execute(() => {
      const video = document.querySelector("video");
      const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
      const rect = timeline.getBoundingClientRect();
      timeline.dispatchEvent(new MouseEvent("click", {
        bubbles: true,
        clientX: rect.left + rect.width * 0.2,
        clientY: rect.top + rect.height / 2,
      }));
      video.currentTime = video.duration * 0.8;
      return video.currentTime;
    });
    await browser.waitUntil(
      () => browser.execute((target) => {
        const video = document.querySelector("video");
        return !video.seeking && Math.abs(video.currentTime - target) < 0.05 &&
          !document.querySelector('img[data-scrub-preview-visible="true"]');
      }, expected),
      { timeout: 3_000, timeoutMsg: "the previous timeline target overrode the native seek" },
    );
  });

  it("adds keyboard markers at the live position and scrubs again after ending", async () => {
    await openHarness();
    await browser.execute(async () => {
      const video = document.querySelector("video");
      video.muted = true;
      await video.play();
      document.querySelector('[aria-label="视频剪辑时间轴"]').focus();
    });
    await browser.waitUntil(
      () => browser.execute(() => document.querySelector("video").currentTime > 0.3),
    );
    const beforeMarker = await browser.execute(() => document.querySelector("video").currentTime);
    await browser.keys("Enter");
    const marker = await $('[aria-label="剪辑竖线 1"]');
    const markerTime = Number(await marker.getAttribute("aria-valuenow"));
    assert.ok(markerTime >= beforeMarker - 0.05, `marker ${markerTime} lagged behind playback ${beforeMarker}`);
    await browser.execute(() => {
      const video = document.querySelector("video");
      video.currentTime = video.duration - 0.15;
    });
    await browser.waitUntil(
      () => browser.execute(() => document.querySelector("video").ended),
    );
    await browser.execute(() => document.querySelector('[aria-label="剪辑竖线 1"]').focus());
    await browser.keys("ArrowRight");
    const target = Number(await marker.getAttribute("aria-valuenow"));
    await browser.waitUntil(
      () => browser.execute((time) => {
        const video = document.querySelector("video");
        return !video.seeking && Math.abs(video.currentTime - time) < 0.05 &&
          !document.querySelector('img[data-scrub-preview-visible="true"]');
      }, target),
      { timeout: 3_000, timeoutMsg: "scrubbing stayed locked after playback ended" },
    );
    await browser.execute(async () => { await document.querySelector("video").play(); });
    await browser.waitUntil(
      () => browser.execute((time) => document.querySelector("video").currentTime > time + 0.1, target),
    );
  });

  it("keeps marker keyboard events inside the timeline", async () => {
    await openHarness();
    const timeline = await $('[aria-label="视频剪辑时间轴"]');
    await timeline.waitForDisplayed();
    await browser.execute(() => document.querySelector("video").pause());
    await browser.execute(() => {
      const video = document.querySelector("video");
      video.currentTime = video.duration * 0.2;
      document.querySelector('[aria-label="视频剪辑时间轴"]').focus();
    });
    await browser.waitUntil(() => browser.execute(() => {
      const video = document.querySelector("video");
      return Boolean(video && !video.seeking && Math.abs(video.currentTime / video.duration - 0.2) < 0.05);
    }));
    await browser.keys("Enter");
    await browser.waitUntil(
      () => browser.execute(() => document.querySelectorAll('[role="slider"][aria-label^="剪辑竖线"]').length === 1),
    );

    await browser.execute(() => {
      const video = document.querySelector("video");
      video.currentTime = video.duration * 0.6;
      document.querySelector('[aria-label="剪辑竖线 1"]').focus();
    });
    await browser.waitUntil(() => browser.execute(() => !document.querySelector("video").seeking));
    await browser.keys("Enter");
    await browser.pause(80);
    assert.equal(
      await browser.execute(() => document.querySelectorAll('[role="slider"][aria-label^="剪辑竖线"]').length),
      1,
      "Enter on the marker must not add a timeline marker",
    );

    await browser.execute(() => {
      const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
      [...document.querySelectorAll("button")].find((button) => !button.disabled && !timeline.contains(button))?.focus();
    });
    await browser.keys("Delete");
    await browser.pause(80);
    assert.equal(
      await browser.execute(() => document.querySelectorAll('[role="slider"][aria-label^="剪辑竖线"]').length),
      1,
      "Delete outside the timeline must not remove its selected marker",
    );
  });

  it("keeps the playback controls usable at wide and narrow widths", async () => {
    try {
      for (const width of [1440, 1024, 390]) {
        await browser.setWindowSize(width, 900);
        await openHarness();
        const state = await browser.execute(() => {
          const timeline = document.querySelector('[aria-label="视频剪辑时间轴"]');
          timeline.scrollIntoView({ block: "center" });
          timeline.focus();
          return {
            width: window.innerWidth,
            overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
            focused: document.activeElement === timeline,
          };
        });
        assert.equal(state.width, width, `viewport did not resize to ${width}px`);
        assert.equal(state.overflow, 0, `horizontal overflow at ${width}px`);
        assert.equal(state.focused, true, `timeline could not receive focus at ${width}px`);
        await browser.keys("Enter");
        assert.equal(await $('[aria-label="剪辑竖线 1"]').isExisting(), true);
        await browser.saveScreenshot(screenshotPath(`playback-${width}.png`));
      }
    } finally {
      await browser.setWindowSize(1440, 1000);
    }
  });

  it("retries failed playback even when the returned media bytes are unchanged", async () => {
    await openHarness("?scenario=source-fallback");
    await browser.execute(() => {
      const video = document.querySelector("video");
      window.__failedVideo = video;
      window.__failedVideoSource = video.currentSrc;
      Object.defineProperty(video, "error", { value: { code: 3, message: "fixture decode failure" } });
      video.dispatchEvent(new Event("error"));
    });
    const retry = await $("button=重试加载");
    await retry.waitForDisplayed();
    await retry.click();
    await browser.waitUntil(() => browser.execute(() => {
      const video = document.querySelector("video");
      return video && video !== window.__failedVideo && video.readyState >= 2 && !video.error &&
        video.currentSrc === window.__failedVideoSource && !document.querySelector('[role="alert"]');
    }), { timeout: 3000, timeoutMsg: "retry reused the failed player for an identical source" });
  });

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
            const deadline = performance.now() + 3_000;
            while (video.currentTime <= beforePlay + 0.1 && performance.now() < deadline) {
              await wait(20);
            }
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
