// 入口：挂载全部场景，暴露 window.__seek(t) 供渲染器逐帧求值，
// 并收集各场景登记的音效时间点（window.__cues）交给音频合成。
import { T, DURATION, FPS, CHAPTERS } from './timeline.js';
import { el, show } from './core.js';
import { createBackground } from './gl/background.js';
import { createFx } from './gl/fx.js';
import { createStory } from './gl/story.js';
import { SCENES } from './scenes/index.js';

const FONT_SPECS = [
  '200 40px "Noto Serif CJK SC"',
  '300 40px "Noto Serif CJK SC"',
  '500 40px "Noto Serif CJK SC"',
  '600 40px "Noto Serif CJK SC"',
  '900 40px "Noto Serif CJK SC"',
  '300 40px "Noto Sans CJK SC"',
  '400 40px "Noto Sans CJK SC"',
  '500 40px "Noto Sans CJK SC"',
  '700 40px "Noto Sans CJK SC"',
  '400 40px "JetBrainsMono NF"',
  '700 40px "JetBrainsMono NF"',
];

async function boot() {
  await Promise.all(FONT_SPECS.map((s) => document.fonts.load(s, '汉字Aa0')));
  const stage = document.getElementById('stage');
  const cues = [];
  const ctx = {
    stage,
    bg: createBackground(document.getElementById('bg')),
    fx: createFx(document.getElementById('fx')),
    story: createStory(),
    flashNode: document.getElementById('flash'),
    flash: 0,
    dust: 0.5,
    dustOpts: {},
    cue(t, kind, o = {}) {
      cues.push({ t: Math.round(t * 1e4) / 1e4, kind, ...o });
    },
  };
  const root = document.getElementById('scenes');
  for (const s of SCENES) {
    s.root = el('div', 'scene', root);
    await s.mount(s.root, ctx);
  }
  cues.sort((a, b) => a.t - b.t);

  window.__cues = { T, DURATION, FPS, cues, chapters: CHAPTERS };
  window.__sceneRanges = SCENES.map(s => ({ from: s.from, to: s.to }));
  window.__seek = (t) => {
    if (!Number.isFinite(t) || t < 0 || t > DURATION) throw new RangeError('时间必须在 0–123 秒之间');
    ctx.bg.reset();
    ctx.fx.clear();
    ctx.flash = 0;
    ctx.dust = 0.5;
    ctx.dustOpts = {};
    for (const s of SCENES) {
      const on = t >= s.from && t < s.to;
      show(s.root, on);
      if (on) s.update(t, ctx);
    }
    ctx.fx.dust(t, ctx.dust, ctx.dustOpts);
    ctx.bg.render(t);
    ctx.flashNode.style.opacity = Math.min(1, ctx.flash).toFixed(3);
    document.getElementById('illustration-note').style.opacity = t >= 12 && t < 112 ? '1' : '0';
    window.__time = t;
  };
  window.__seek(0);
  window.__ready = true;
}

boot().catch((e) => {
  console.error(e);
  window.__error = String(e && e.stack ? e.stack : e);
});
