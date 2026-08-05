import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const openHarness = async () => {
  if (process.env.A4N_WDIO_APP_BINARY)
    await browser.url("http://localhost:5173/e2e/video-scrub/");
  else await browser.url("/e2e/video-scrub/");
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
      const readFrame = () => {
        const preview = document.querySelector(
          'canvas[data-scrub-preview-visible="true"]',
        );
        if (!(preview instanceof HTMLCanvasElement)) {
          return {
            nativeVideo: true,
            frameHash: Math.round(video.currentTime * 1000),
            blackRatio: 0,
          };
        }
        const canvas = document.createElement("canvas");
        canvas.width = 160;
        canvas.height = 90;
        const context = canvas.getContext("2d", { willReadFrequently: true });
        if (!context) throw new Error("frame sampling context unavailable");
        context.drawImage(preview, 0, 0, canvas.width, canvas.height);
        const pixels = context.getImageData(
          0,
          0,
          canvas.width,
          canvas.height,
        ).data;
        let hash = 2166136261;
        let blackPixels = 0;
        for (let index = 0; index < pixels.length; index += 4) {
          hash = Math.imul(hash ^ pixels[index], 16777619);
          hash = Math.imul(hash ^ pixels[index + 1], 16777619);
          hash = Math.imul(hash ^ pixels[index + 2], 16777619);
          if (pixels[index] + pixels[index + 1] + pixels[index + 2] < 18) {
            blackPixels += 1;
          }
        }
        return {
          frameHash: hash >>> 0,
          blackRatio: blackPixels / (pixels.length / 4),
          nativeVideo: false,
        };
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
        dispatch("pointerdown", 0.12);
        await wait(20);
        const ratios = [0.28, 0.52, 0.81, 0.44, 0.17, 0.69];
        const samples = [];
        for (const ratio of ratios) {
          dispatch("pointermove", ratio);
          await settle(ratio);
          samples.push({
            ratio,
            currentTime: video.currentTime,
            expectedTime: video.duration * ratio,
            seeking: video.seeking,
            ...readFrame(),
          });
        }
        dispatch("pointerup", ratios.at(-1), 1, 0);
        await settle(ratios.at(-1));
        const finalTime = video.currentTime;
        const markerValue = Number(marker.getAttribute("aria-valuenow"));
        done({
          samples,
          finalTime,
          expectedFinalTime: video.duration * ratios.at(-1),
          markerValue,
          videoCount: document.querySelectorAll("video").length,
          canvasCount: document.querySelectorAll("canvas").length,
          canvasVisible: Boolean(
            document.querySelector('canvas[data-scrub-preview-visible="true"]'),
          ),
          paused: video.paused,
        });
      })().catch((error) => done({ error: String(error) }));
    });

    process.stdout.write(`A4N_VIDEO_SCRUB_RESULT=${JSON.stringify(result)}\n`);
    assert.equal(result.error, undefined, result.error);
    assert.equal(result.videoCount, 1);
    assert.equal(result.canvasCount, 1);
    assert.ok(result.samples.length >= 5);
    assert.ok(
      result.samples.every(
        (sample) =>
          !sample.seeking &&
          Math.abs(sample.currentTime - sample.expectedTime) < 0.05,
      ),
      `video did not follow intermediate markers: ${JSON.stringify(result)}`,
    );
    assert.ok(
      result.samples.every(
        (sample) => sample.nativeVideo || sample.blackRatio < 0.9,
      ),
      `video rendered a black frame: ${JSON.stringify(result)}`,
    );
    assert.ok(
      new Set(result.samples.map((sample) => sample.frameHash)).size >= 5,
      `rendered video frame remained frozen: ${JSON.stringify(result)}`,
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
      const readCanvasHash = () => {
        const video = document.querySelector("video");
        const preview = document.querySelector(
          'canvas[data-scrub-preview-visible="true"]',
        );
        if (!(preview instanceof HTMLCanvasElement)) {
          return video instanceof HTMLVideoElement
            ? Math.round(video.currentTime * 1000)
            : null;
        }
        const probe = document.createElement("canvas");
        probe.width = 80;
        probe.height = 45;
        const context = probe.getContext("2d", { willReadFrequently: true });
        if (!context) return null;
        context.drawImage(preview, 0, 0, probe.width, probe.height);
        const pixels = context.getImageData(
          0,
          0,
          probe.width,
          probe.height,
        ).data;
        let hash = 2166136261;
        for (let index = 0; index < pixels.length; index += 4) {
          hash = Math.imul(hash ^ pixels[index], 16777619);
          hash = Math.imul(hash ^ pixels[index + 1], 16777619);
          hash = Math.imul(hash ^ pixels[index + 2], 16777619);
        }
        return hash >>> 0;
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
        fire("pointerdown", 0.9, 1);
        await wait(10);
        const ratios = Array.from(
          { length: 70 },
          (_, index) => 0.9 - (0.84 * index) / 69,
        );
        const dragFrameHashes = [readCanvasHash()];
        for (const [index, ratio] of ratios.entries()) {
          fire("pointermove", ratio, 1);
          await wait(5);
          if (index % 5 === 0) dragFrameHashes.push(readCanvasHash());
        }
        fire("pointerup", ratios.at(-1), 0);
        const expected = video.duration * ratios.at(-1);
        for (let attempt = 0; attempt < 100; attempt += 1) {
          if (!video.seeking && Math.abs(video.currentTime - expected) < 0.05)
            break;
          await wait(20);
        }
        dragFrameHashes.push(readCanvasHash());
        done({
          currentTime: video.currentTime,
          expected,
          seeking: video.seeking,
          videoCount: document.querySelectorAll("video").length,
          canvasCount: document.querySelectorAll("canvas").length,
          canvasVisible: Boolean(
            document.querySelector('canvas[data-scrub-preview-visible="true"]'),
          ),
          dragFrameHashes: dragFrameHashes.filter(
            (hash) => typeof hash === "number",
          ),
        });
      })().catch((error) => done({ error: String(error) }));
    });

    process.stdout.write(
      `A4N_VIDEO_SCRUB_CONTINUOUS_RESULT=${JSON.stringify(result)}\n`,
    );
    assert.equal(result.error, undefined, result.error);
    assert.equal(result.videoCount, 1);
    assert.equal(result.canvasCount, 1);
    assert.equal(result.seeking, false);
    assert.ok(
      new Set(result.dragFrameHashes).size >= 3,
      `visible preview did not update during drag: ${JSON.stringify(result)}`,
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
    await browser.waitUntil(
      () =>
        browser
          .execute(() => {
            const video = document.querySelector("video");
            const canvas = document.querySelector(
              'canvas[data-scrub-preview-visible="true"]',
            );
            return {
              currentTime: video?.currentTime ?? null,
              duration: video?.duration ?? null,
              seeking: video?.seeking ?? null,
              canvasVisible: canvas instanceof HTMLCanvasElement,
            };
          })
          .then((value) =>
            value.currentTime != null &&
            value.duration != null &&
            !value.seeking &&
            Math.abs(value.currentTime - value.duration * 0.76) < 0.2
              ? value
              : false,
          ),
      { timeout: 10_000, timeoutMsg: "native pointer drag did not settle" },
    );
    const nativeVideo = await browser.$("video");
    const screenshotHash = async (path) => {
      await nativeVideo.saveScreenshot(path);
      return createHash("sha256")
        .update(readFileSync(path))
        .digest("hex");
    };
    const forwardFrameHash = await screenshotHash(
      "/tmp/a4n-webkit-forward.png",
    );
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
              ? !video.seeking && Math.abs(video.currentTime - video.duration * 0.2) < 0.2
              : false;
          })
          .then(Boolean),
      { timeout: 10_000, timeoutMsg: "native reverse drag did not settle" },
    );
    const reverseFrameHash = await screenshotHash(
      "/tmp/a4n-webkit-reverse.png",
    );
    assert.notEqual(
      reverseFrameHash,
      forwardFrameHash,
      "反向拖动后视频画面没有变化",
    );
    await browser.execute(() => {
      document.querySelector("video")?.pause();
    });
    const playButton = await browser.$('[aria-label="播放后处理预览"]');
    if (await playButton.isExisting()) {
      await playButton.click();
    } else {
      await browser.execute(() => void document.querySelector("video")?.play());
    }
    const startPlaybackTime = await browser.execute(
      () => document.querySelector("video")?.currentTime ?? null,
    );
    await browser.pause(250);
    const playbackState = await browser.execute(() => ({
      currentTime: document.querySelector("video")?.currentTime ?? null,
      paused: document.querySelector("video")?.paused ?? true,
      canvasVisible: Boolean(
        document.querySelector('canvas[data-scrub-preview-visible="true"]'),
      ),
      playbackHeadVisible: Boolean(
        document.querySelector('[data-playback-head="true"]'),
      ),
    }));
    assert.equal(playbackState.paused, false);
    assert.equal(playbackState.canvasVisible, false);
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
